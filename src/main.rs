// build-patch: monta um chunkNpatchM.rpkg (RPKGv2) a partir de recursos
// compilados + .meta.json no formato RPKG-Tool (layout do Simple Mod Framework).
//
// Uso:
//   build-patch <content_dir> <output.rpkg> [patch_level=2] [chunk_id=0]
//
// Varre <content_dir> recursivamente por arquivos "*.meta.json"; para cada um,
// o recurso compilado e o mesmo caminho sem o sufixo ".meta.json".
// Nao modifica nada do jogo: so le o mod e grava o .rpkg no caminho de saida.

use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::package_builder::{PackageBuilder, PackageResourceBuilder};
use rpkg_rs::resource::resource_package::{
    ChunkType, PackageVersion, ResourceReferenceFlags, ResourceReferenceFlagsStandard,
};
use rpkg_rs::resource::resource_partition::PatchId;
use rpkg_rs::resource::runtime_resource_id::{PlatformTag, RuntimeResourceID};
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const META_SUFFIX: &str = ".meta.json";

fn collect_meta_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            collect_meta_files(&path, out)?;
        } else if ft.is_file() {
            if path.to_string_lossy().ends_with(META_SUFFIX) {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn resource_path_for_meta(meta: &Path) -> PathBuf {
    let s = meta.to_string_lossy();
    PathBuf::from(&s[..s.len() - META_SUFFIX.len()])
}

fn parse_reference(h: &str) -> Result<RuntimeResourceID, Box<dyn Error>> {
    // Referencia pode vir como ioi-string "[assembly:/...].type" ou como hash hex.
    if h.starts_with('[') {
        let rid = ResourceID::from_str(h)?;
        Ok(RuntimeResourceID::from_resource_id_with_platform(
            &rid,
            "pc",
            PlatformTag::None,
        ))
    } else {
        Ok(RuntimeResourceID::from_hex_string(h)?)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("uso: build-patch <content_dir> <output.rpkg> [patch_level=2] [chunk_id=0]");
        std::process::exit(2);
    }
    let content_dir = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let patch_level: usize = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(2);
    let chunk_id: u8 = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(0);

    eprintln!("Varrendo {} ...", content_dir.display());
    let mut metas: Vec<PathBuf> = Vec::new();
    collect_meta_files(&content_dir, &mut metas)?;
    metas.sort();
    eprintln!("Encontrados {} arquivos .meta.json", metas.len());

    let mut builder = PackageBuilder::new(chunk_id, ChunkType::Standard);
    builder.with_patch_id(&PatchId::Patch(patch_level));

    let mut added = 0usize;
    let mut errors = 0usize;
    let mut shown = 0usize;

    for meta_path in &metas {
        match build_one(meta_path) {
            Ok(resource) => {
                builder.with_resource(resource);
                added += 1;
                if added % 4000 == 0 {
                    eprintln!("  ... {} recursos", added);
                }
            }
            Err(e) => {
                errors += 1;
                if shown < 20 {
                    eprintln!("  ERRO em {}: {}", meta_path.display(), e);
                    shown += 1;
                }
            }
        }
    }

    eprintln!("Recursos adicionados: {}  | erros: {}", added, errors);

    if added == 0 {
        return Err("nenhum recurso adicionado; abortando".into());
    }

    eprintln!(
        "Gravando patch (chunk{} patch{}) em {} ...",
        chunk_id,
        patch_level,
        output.display()
    );
    builder.build_to_file(PackageVersion::RPKGv2, &output)?;
    eprintln!("OK");
    Ok(())
}

fn build_one(meta_path: &Path) -> Result<PackageResourceBuilder, Box<dyn Error>> {
    let res_path = resource_path_for_meta(meta_path);
    let meta: Value = serde_json::from_str(&fs::read_to_string(meta_path)?)?;

    let hash_value = meta["hash_value"].as_str().ok_or("meta sem hash_value")?;
    let rtype = meta["hash_resource_type"]
        .as_str()
        .ok_or("meta sem hash_resource_type")?;

    let data = fs::read(&res_path)
        .map_err(|e| format!("nao consegui ler recurso {}: {}", res_path.display(), e))?;

    let rrid = RuntimeResourceID::from_hex_string(hash_value)?;
    // compression_level=None (sem compressao), should_scramble=false
    let mut resource = PackageResourceBuilder::from_memory(rrid, rtype, data, None, false)?;

    if let Some(refs) = meta["hash_reference_data"].as_array() {
        for r in refs {
            let h = r["hash"].as_str().ok_or("referencia sem hash")?;
            let flag_str = r["flag"].as_str().unwrap_or("1F");
            let flag_byte = u8::from_str_radix(flag_str, 16)?;
            let ref_rrid = parse_reference(h)?;
            let flags = ResourceReferenceFlags::Standard(
                ResourceReferenceFlagsStandard::from_bits(flag_byte),
            );
            resource.with_reference(ref_rrid, flags);
        }
    }

    Ok(resource)
}
