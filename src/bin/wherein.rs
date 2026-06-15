// wherein: para cada hash dado, lista TODOS os rpkg que o contem (via from_file).
// Uso: wherein <Resources dir> <hash1> [hash2 ...]

use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);
    let hashes: Vec<(String, RuntimeResourceID)> = args[2..]
        .iter()
        .map(|h| (h.clone(), RuntimeResourceID::from_hex_string(h).unwrap()))
        .collect();

    let mut files: Vec<PathBuf> = fs::read_dir(&res_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rpkg").unwrap_or(false))
        .collect();
    files.sort();

    let mut hits: Vec<(String, Vec<String>)> =
        hashes.iter().map(|(h, _)| (h.clone(), Vec::new())).collect();

    for p in &files {
        let fname = p.file_name().unwrap().to_string_lossy().to_string();
        if let Ok(pkg) = ResourcePackage::from_file(p) {
            let keys = pkg.resources();
            for (i, (_, rrid)) in hashes.iter().enumerate() {
                if keys.contains_key(rrid) {
                    hits[i].1.push(fname.clone());
                }
            }
        }
    }

    for (h, fs_) in &hits {
        println!("{} -> {:?}", h, if fs_.is_empty() { vec!["NENHUM".to_string()] } else { fs_.clone() });
    }
    Ok(())
}
