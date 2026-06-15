// vremap: confere que o dado dos patches remapeados (a) eh diferente do original
// ingles do Mac (logo vai mudar algo) e (b) bate com o conteudo PT do mod.
// Uso: vremap <Mac Resources> <remap dir>
use rpkg_rs::resource::resource_package::ResourcePackage;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    let mac = PathBuf::from(&a[1]);
    let remap = PathBuf::from(&a[2]);

    // pacotes Mac do chunk0 (originais)
    let mut mac_pkgs = Vec::new();
    for n in ["chunk0patch1.rpkg","chunk0part1.rpkg","chunk0part2.rpkg","chunk0part3.rpkg",
              "chunk0part4.rpkg","chunk0part5.rpkg","chunk0part6.rpkg"] {
        if let Ok(p) = ResourcePackage::from_file(&mac.join(n)) { mac_pkgs.push(p); }
    }
    let read_mac = |rrid| { for p in &mac_pkgs { if p.resources().contains_key(rrid){ if let Ok(d)=p.read_resource(rrid){return Some(d);} } } None::<Vec<u8>> };

    let patch = ResourcePackage::from_file(&remap.join("chunk0patch2.rpkg"))?;
    let mut diff=0; let mut same=0; let mut checked=0;
    for (rrid, _) in patch.resources().iter().take(12) {
        let pt = patch.read_resource(rrid)?;
        if let Some(en) = read_mac(rrid) {
            checked+=1;
            let d = pt != en;
            if d {diff+=1;} else {same+=1;}
            println!("{} | PT {}B vs EN {}B | diferente(traduzido)={}", rrid.to_hex_string(), pt.len(), en.len(), d);
        } else {
            println!("{} | nao achei original no Mac chunk0", rrid.to_hex_string());
        }
    }
    println!("\nAmostra {}: traduzidos(diferentes do EN)={} | iguais ao EN={}", checked, diff, same);
    Ok(())
}
