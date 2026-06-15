// chunkmap: para cada recurso do mod, descobre em QUAIS particoes montadas
// (chunk2..chunk30 — chunk0/1 sao multi-parte e o rpkg-rs nao monta) o RRID existe.
// Objetivo: saber se os recursos do mod pertencem a chunks de localizacao
// (entao chunk0patch2 nao os sobrescreveria) ou nao (entao sao chunk0/chunk1).
//
// Uso: chunkmap <Resources dir> <content_dir>

use rpkg_rs::resource::partition_manager::{PartitionManager, PartitionState};
use rpkg_rs::resource::pdefs::PackageDefinitionSource;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use rpkg_rs::WoaVersion;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const META_SUFFIX: &str = ".meta.json";

fn collect_meta_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            collect_meta_files(&path, out)?;
        } else if ft.is_file() && path.to_string_lossy().ends_with(META_SUFFIX) {
            out.push(path);
        }
    }
    Ok(())
}

#[allow(deprecated)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);
    let mod_dir = PathBuf::from(&args[2]);

    let source = PackageDefinitionSource::from_file(res_dir.join("packagedefinition.txt"), WoaVersion::HM3)?;
    let mut pm = PartitionManager::new(res_dir.clone(), &source)?;
    pm.mount_partitions(|_c: usize, _s: &PartitionState| {})?;
    eprintln!("Particoes montadas (chunk2..30): {}", pm.partitions.len());

    let mut metas = Vec::new();
    collect_meta_files(&mod_dir, &mut metas)?;
    eprintln!("Recursos do mod: {}", metas.len());

    let mut per_partition: BTreeMap<String, usize> = BTreeMap::new();
    let mut found_in_mounted = 0usize;
    let mut not_in_mounted = 0usize; // = chunk0/chunk1/novo
    let mut examples: Vec<String> = Vec::new();

    for meta_path in &metas {
        let meta: Value = serde_json::from_str(&fs::read_to_string(meta_path)?)?;
        let hv = meta["hash_value"].as_str().ok_or("sem hash")?;
        let rtype = meta["hash_resource_type"].as_str().unwrap_or("?");
        let rrid = RuntimeResourceID::from_hex_string(hv)?;
        let parts = pm.partitions_with_resource(&rrid);
        if parts.is_empty() {
            not_in_mounted += 1;
        } else {
            found_in_mounted += 1;
            for pid in &parts {
                let name = pm
                    .find_partition(pid.clone())
                    .and_then(|p| p.partition_info().name.clone())
                    .unwrap_or_else(|| pid.to_string());
                *per_partition.entry(name).or_default() += 1;
            }
            if examples.len() < 15 {
                let names: Vec<String> = parts
                    .iter()
                    .map(|pid| {
                        pm.find_partition(pid.clone())
                            .and_then(|p| p.partition_info().name.clone())
                            .unwrap_or_else(|| pid.to_string())
                    })
                    .collect();
                examples.push(format!("{} {} -> {:?}", rtype, hv, names));
            }
        }
    }

    eprintln!("\n== RESULTADO ==");
    eprintln!(
        "Recursos do mod presentes em chunks de localizacao montados (chunk2..30): {}",
        found_in_mounted
    );
    eprintln!(
        "Recursos NAO presentes em chunk2..30 (logo pertencem a chunk0/chunk1 ou sao novos): {}",
        not_in_mounted
    );
    eprintln!("\nDistribuicao dos que estao em chunks de localizacao (por particao):");
    for (name, n) in &per_partition {
        eprintln!("  {} : {}", name, n);
    }
    if !examples.is_empty() {
        eprintln!("\nExemplos (recurso do mod que vive numa chunk de localizacao):");
        for e in &examples {
            eprintln!("  {}", e);
        }
    }
    Ok(())
}
