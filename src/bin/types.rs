// types: conta os tipos de recurso presentes no jogo do Mac.
// Uso: types <Resources dir>
use rpkg_rs::resource::resource_package::ResourcePackage;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&res_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rpkg").unwrap_or(false))
        .collect();
    files.sort();
    for p in &files {
        if p.file_name().unwrap().to_string_lossy() == "chunk0patch2.rpkg" { continue; }
        if let Ok(pkg) = ResourcePackage::from_file(p) {
            for (_, info) in pkg.resources().iter() {
                *counts.entry(format!("{:?}", info.data_type())).or_default() += 1;
            }
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("Tipos de recurso no jogo do Mac (top 30):");
    for (t, n) in v.iter().take(30) {
        println!("  {:<8} {}", t, n);
    }
    // foca em localizacao
    for t in ["\"LOCR\"", "\"DLGE\"", "\"RTLV\""] {
        let n = v.iter().find(|(k, _)| k == t).map(|(_, n)| *n).unwrap_or(0);
        println!("LOCALIZACAO {} : {}", t, n);
    }
    Ok(())
}
