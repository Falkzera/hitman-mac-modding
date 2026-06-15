// ownership: descobre se os recursos do mod pertencem ao chunk0 (super) ou
// chunk1 (base), lendo os rpkgs diretamente (sem PartitionManager, que nao
// monta multi-parte). Uso: ownership <Resources dir> <content_dir>

use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use serde_json::Value;
use std::collections::HashSet;
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

fn rrids_of(res_dir: &Path, files: &[&str]) -> HashSet<RuntimeResourceID> {
    let mut set = HashSet::new();
    for f in files {
        let p = res_dir.join(f);
        if !p.exists() {
            continue;
        }
        match ResourcePackage::from_file(&p) {
            Ok(pkg) => {
                for rrid in pkg.resources().keys() {
                    set.insert(*rrid);
                }
                eprintln!("  lido {} ({} recursos acumulados)", f, set.len());
            }
            Err(e) => eprintln!("  ERRO lendo {}: {}", f, e),
        }
    }
    set
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);
    let mod_dir = PathBuf::from(&args[2]);

    eprintln!("Lendo chunk0 (super): part1..6 + patch1 ...");
    let set0 = rrids_of(
        &res_dir,
        &[
            "chunk0part1.rpkg",
            "chunk0part2.rpkg",
            "chunk0part3.rpkg",
            "chunk0part4.rpkg",
            "chunk0part5.rpkg",
            "chunk0part6.rpkg",
            "chunk0patch1.rpkg",
        ],
    );
    eprintln!("chunk0 total RRIDs: {}", set0.len());

    eprintln!("Lendo chunk1 (base): part1..3 + patch1 ...");
    let set1 = rrids_of(
        &res_dir,
        &[
            "chunk1part1.rpkg",
            "chunk1part2.rpkg",
            "chunk1part3.rpkg",
            "chunk1patch1.rpkg",
        ],
    );
    eprintln!("chunk1 total RRIDs: {}", set1.len());

    let mut metas = Vec::new();
    collect_meta_files(&mod_dir, &mut metas)?;

    let (mut only0, mut only1, mut both, mut neither) = (0usize, 0usize, 0usize, 0usize);
    for meta_path in &metas {
        let meta: Value = serde_json::from_str(&fs::read_to_string(meta_path)?)?;
        let hv = meta["hash_value"].as_str().ok_or("sem hash")?;
        let rrid = RuntimeResourceID::from_hex_string(hv)?;
        let in0 = set0.contains(&rrid);
        let in1 = set1.contains(&rrid);
        match (in0, in1) {
            (true, true) => both += 1,
            (true, false) => only0 += 1,
            (false, true) => only1 += 1,
            (false, false) => neither += 1,
        }
    }

    eprintln!("\n== A QUE CHUNK PERTENCEM OS {} RECURSOS DO MOD ==", metas.len());
    eprintln!("so chunk0 (super)  : {}", only0);
    eprintln!("so chunk1 (base)   : {}  <-- se >0, precisam de chunk1patch, nao chunk0patch", only1);
    eprintln!("em ambos           : {}", both);
    eprintln!("em nenhum (novos)  : {}", neither);
    Ok(())
}
