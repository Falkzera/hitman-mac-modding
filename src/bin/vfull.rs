// vfull: valida os patches remapeados: IDs de recurso E de referencia devem existir no Mac.
// Uso: vfull <Mac Resources> <remap-full dir>
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn rpkgs(d: &PathBuf) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(d).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|x| x=="rpkg").unwrap_or(false)).collect()).unwrap_or_default();
    v.sort(); v
}

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    let mac = PathBuf::from(&a[1]);
    let remap = PathBuf::from(&a[2]);

    // IDs do Mac original (sem patches nossos)
    let installed: HashSet<&str> = ["chunk0patch2.rpkg"].into_iter().collect();
    let mut macset: HashSet<RuntimeResourceID> = HashSet::new();
    for p in rpkgs(&mac) {
        if installed.contains(p.file_name().unwrap().to_str().unwrap()) { continue; }
        if let Ok(pk)=ResourcePackage::from_file(&p) { for r in pk.resources().keys(){ macset.insert(*r);} }
    }
    eprintln!("Mac IDs: {}", macset.len());

    let mut res_in=0u64; let mut res_out=0u64; let mut ref_in=0u64; let mut ref_out=0u64;
    for p in rpkgs(&remap) {
        let pk = ResourcePackage::from_file(&p)?;
        for (rrid, info) in pk.resources().iter() {
            if macset.contains(rrid) { res_in+=1; } else { res_out+=1; }
            for (rr, _) in info.references() {
                if macset.contains(rr) { ref_in+=1; } else { ref_out+=1; }
            }
        }
    }
    println!("Recursos:   no Mac = {} | FORA do Mac = {}", res_in, res_out);
    println!("Referencias: no Mac = {} | FORA do Mac = {}", ref_in, ref_out);
    if res_out==0 && ref_out==0 { println!("VALIDACAO OK ✅ (todo recurso e toda referencia existem no Mac)"); }
    else { println!("ATENCAO: ha IDs fora do Mac ❌"); }
    Ok(())
}
