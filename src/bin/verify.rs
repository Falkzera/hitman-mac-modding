// verify: reabre um patch rpkg com rpkg-rs e confere contra a fonte do mod.
// Uso: verify <patch.rpkg> <content_dir>

use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("uso: verify <patch.rpkg> <content_dir>");
        std::process::exit(2);
    }
    let rpkg_path = PathBuf::from(&args[1]);
    let mod_dir = PathBuf::from(&args[2]);

    let data = fs::read(&rpkg_path)?;
    println!("rpkg: {} ({} bytes)", rpkg_path.display(), data.len());
    // is_patch = true (forca leitura do formato de patch, com lista de delecoes)
    let rpkg = ResourcePackage::from_memory(data, true)?;

    let res = rpkg.resources();
    println!("Recursos no rpkg: {}", res.len());

    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    for (_rrid, info) in res.iter() {
        *by_type.entry(format!("{:?}", info.data_type())).or_default() += 1;
    }
    println!("Distribuicao por tipo:");
    for (t, n) in &by_type {
        println!("  {} : {}", t, n);
    }
    println!("Unneeded (delecoes): {}", rpkg.unneeded_resource_ids().len());

    // Checagem byte-a-byte de TODOS os recursos contra a fonte.
    let mut metas = Vec::new();
    collect_meta_files(&mod_dir, &mut metas)?;
    let mut ok = 0usize;
    let mut mismatch = 0usize;
    let mut missing = 0usize;
    for meta_path in &metas {
        let meta: Value = serde_json::from_str(&fs::read_to_string(meta_path)?)?;
        let hv = meta["hash_value"].as_str().ok_or("sem hash_value")?;
        let rrid = RuntimeResourceID::from_hex_string(hv)?;
        let s = meta_path.to_string_lossy();
        let src = fs::read(PathBuf::from(&s[..s.len() - META_SUFFIX.len()]))?;
        match rpkg.read_resource(&rrid) {
            Ok(d) => {
                if d == src {
                    ok += 1;
                } else {
                    mismatch += 1;
                    if mismatch <= 10 {
                        eprintln!("MISMATCH {} (rpkg {} vs src {})", hv, d.len(), src.len());
                    }
                }
            }
            Err(e) => {
                missing += 1;
                if missing <= 10 {
                    eprintln!("MISSING {} : {}", hv, e);
                }
            }
        }
    }
    println!(
        "Byte-a-byte -> OK: {} | mismatch: {} | missing: {}",
        ok, mismatch, missing
    );
    if mismatch == 0 && missing == 0 && ok == res.len() {
        println!("VERIFICACAO PASSOU ✅");
        Ok(())
    } else {
        println!("VERIFICACAO FALHOU ❌");
        std::process::exit(1);
    }
}
