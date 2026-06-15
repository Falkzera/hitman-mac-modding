// diag: monta o jogo igual a engine (packagedefinition + patches) e verifica,
// para uma amostra de recursos do mod, em quais patches o recurso aparece e se
// a versao "vencedora" (a mais alta) bate com a do nosso mod.
//
// Uso: diag <Contents/Resources dir> <content_dir do mod> [tamanho_amostra=12]

use rpkg_rs::resource::partition_manager::{PartitionManager, PartitionState};
use rpkg_rs::resource::pdefs::PackageDefinitionSource;
use rpkg_rs::resource::resource_partition::PatchId;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use rpkg_rs::WoaVersion;
use serde_json::Value;
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
    if args.len() < 3 {
        eprintln!("uso: diag <Resources dir> <content_dir> [amostra=12]");
        std::process::exit(2);
    }
    let res_dir = PathBuf::from(&args[1]);
    let mod_dir = PathBuf::from(&args[2]);
    let sample: usize = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(12);

    let pdef = res_dir.join("packagedefinition.txt");
    let source = PackageDefinitionSource::from_file(pdef, WoaVersion::HM3)?;

    // patchlevel conforme packagedefinition (como a engine le)
    let infos = source.read()?;
    for p in &infos {
        let name = p.name.clone().unwrap_or_default();
        if name == "super" || p.patch_level >= 2 {
            eprintln!(
                "packagedefinition -> particao '{}' patchlevel={}",
                name, p.patch_level
            );
        }
    }

    eprintln!("Montando particoes a partir de {} ...", res_dir.display());
    let mut pm = PartitionManager::new(res_dir.clone(), &source)?;
    pm.mount_partitions(|_cur: usize, _st: &PartitionState| {})?;
    eprintln!("Particoes montadas: {}", pm.partitions.len());

    // dump de cada particao montada: nome + PatchIds dos pacotes + nº recursos
    for part in &pm.partitions {
        let info = part.partition_info();
        let mut pkgs: Vec<String> = part.packages.keys().map(|p| format!("{:?}", p)).collect();
        pkgs.sort();
        eprintln!(
            "  part id={} name={:?} patches={} recursos={} pacotes={:?}",
            info.id,
            info.name,
            part.num_patches(),
            part.latest_resources().len(),
            pkgs
        );
    }

    // amostra cobrindo os 3 tipos (DLGE/LOCR/RTLV)
    let mut all = Vec::new();
    collect_meta_files(&mod_dir, &mut all)?;
    all.sort();
    let per = (sample / 3).max(1);
    let mut metas: Vec<PathBuf> = Vec::new();
    for ext in [".DLGE.meta.json", ".LOCR.meta.json", ".RTLV.meta.json"] {
        let of_type: Vec<&PathBuf> = all
            .iter()
            .filter(|p| p.to_string_lossy().ends_with(ext))
            .collect();
        if of_type.is_empty() {
            continue;
        }
        let step = (of_type.len() / per).max(1);
        let mut i = 0;
        while i < of_type.len() && metas.iter().filter(|m| m.to_string_lossy().ends_with(ext)).count() < per {
            metas.push(of_type[i].clone());
            i += step;
        }
    }
    let step = 1usize;
    let sample = metas.len();

    let mut checked = 0usize;
    let mut has_patch2 = 0usize;
    let mut winner_is_ours = 0usize;
    let mut not_found = 0usize;

    let mut i = 0usize;
    while i < metas.len() && checked < sample {
        let meta_path = &metas[i];
        i += step;
        let meta: Value = serde_json::from_str(&fs::read_to_string(meta_path)?)?;
        let hv = meta["hash_value"].as_str().ok_or("sem hash_value")?;
        let rtype = meta["hash_resource_type"].as_str().unwrap_or("?");
        let rrid = RuntimeResourceID::from_hex_string(hv)?;
        let s = meta_path.to_string_lossy();
        let src = fs::read(PathBuf::from(&s[..s.len() - META_SUFFIX.len()]))?;
        checked += 1;

        let mut found = false;
        for part in &pm.partitions {
            let idx = part.resource_patch_indices(&rrid);
            if idx.is_empty() {
                continue;
            }
            found = true;
            let names: Vec<String> = idx
                .iter()
                .map(|pid| part.partition_info().filename(*pid))
                .collect();
            let p2 = idx
                .iter()
                .any(|pid| matches!(pid, PatchId::Patch(n) if *n == 2));
            if p2 {
                has_patch2 += 1;
            }
            let win_ok = matches!(part.read_resource(&rrid), Ok(d) if d == src);
            if win_ok {
                winner_is_ours += 1;
            }
            eprintln!(
                "{} {} -> {:?} | tem chunk0patch2: {} | vencedor==nosso: {}",
                rtype, hv, names, p2, win_ok
            );
        }
        if !found {
            not_found += 1;
            eprintln!("{} {} -> NAO ENCONTRADO em particao montada", rtype, hv);
        }
    }

    eprintln!(
        "\nRESUMO: amostra={} | com chunk0patch2={} | vencedor==nosso mod={} | nao encontrados={}",
        checked, has_patch2, winner_is_ours, not_found
    );
    Ok(())
}
