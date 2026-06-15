// distribute: descobre a QUE CHUNK pertence cada recurso do mod, lendo todos os
// rpkgs do jogo. Saida: quantos recursos do mod vao em cada chunk (= quais
// chunkNpatch2 precisamos construir). Uso: distribute <Resources dir> <content_dir>

use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
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

fn chunk_of(filename: &str) -> Option<u32> {
    let rest = filename.strip_prefix("chunk")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);
    let mod_dir = PathBuf::from(&args[2]);

    // conjunto de RRIDs do mod
    let mut metas = Vec::new();
    collect_meta_files(&mod_dir, &mut metas)?;
    let mut modset: HashSet<RuntimeResourceID> = HashSet::new();
    for m in &metas {
        let meta: Value = serde_json::from_str(&fs::read_to_string(m)?)?;
        let hv = meta["hash_value"].as_str().ok_or("sem hash")?;
        modset.insert(RuntimeResourceID::from_hex_string(hv)?);
    }
    eprintln!("Recursos do mod (unicos): {}", modset.len());

    // para cada chunk, o conjunto de RRIDs do mod que aparecem nele
    let mut per_chunk: BTreeMap<u32, HashSet<RuntimeResourceID>> = BTreeMap::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&res_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rpkg").unwrap_or(false))
        .collect();
    files.sort();
    for p in &files {
        let fname = p.file_name().unwrap().to_string_lossy().to_string();
        // IGNORA nosso proprio patch instalado (senao da falso positivo)
        if fname == "chunk0patch2.rpkg" {
            eprintln!("(ignorando nosso patch: {})", fname);
            continue;
        }
        let chunk = match chunk_of(&fname) {
            Some(c) => c,
            None => continue,
        };
        if let Ok(pkg) = ResourcePackage::from_file(p) {
            let entry = per_chunk.entry(chunk).or_default();
            for r in pkg.resources().keys() {
                if modset.contains(r) {
                    entry.insert(*r);
                }
            }
        }
    }

    eprintln!("\n== DISTRIBUICAO DOS RECURSOS DO MOD POR CHUNK ==");
    let mut covered: HashSet<RuntimeResourceID> = HashSet::new();
    for (chunk, set) in &per_chunk {
        if set.is_empty() {
            continue;
        }
        eprintln!("  chunk{:<2} : {} recursos do mod", chunk, set.len());
        for r in set {
            covered.insert(*r);
        }
    }
    eprintln!("\nTotal de recursos do mod cobertos por algum chunk: {} / {}", covered.len(), modset.len());
    let missing = modset.len() - covered.len();
    eprintln!("Recursos do mod NAO encontrados em nenhum chunk (novos): {}", missing);
    Ok(())
}
