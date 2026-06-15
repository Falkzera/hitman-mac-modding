// rtlv: (1) conta por tipo o que esta instalado nos nossos patches;
//       (2) investiga por que RTLV nao casa: pareia mod-RTLV (original Windows) com
//           o RTLV do Mac mais parecido e mostra QUANTOS/ONDE os bytes diferem.
// Uso: rtlv <Mac Resources> <Windows Runtime> <content_dir mod>
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

fn rpkgs(d: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(d).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|x| x=="rpkg").unwrap_or(false)).collect()).unwrap_or_default();
    v.sort(); v
}

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    let mac = PathBuf::from(&a[1]);
    let win = PathBuf::from(&a[2]);
    let moddir = PathBuf::from(&a[3]);

    // (1) contagem por tipo dos nossos patches instalados
    println!("== INSTALADO (nossos chunk*patch2.rpkg) por tipo ==");
    let mut inst: BTreeMap<String,usize> = BTreeMap::new();
    for p in rpkgs(&mac) {
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if !(n.starts_with("chunk") && n.ends_with("patch2.rpkg")) { continue; }
        if let Ok(pk)=ResourcePackage::from_file(&p) {
            for (_,i) in pk.resources().iter() { *inst.entry(i.data_type()).or_default()+=1; }
        }
    }
    for (t,c) in &inst { println!("  {} : {}", t, c); }

    // (2) investigacao RTLV
    println!("\n== RTLV: por que nao casa? ==");
    // windows pkgs (p/ ler original ingles do mod)
    let mut wf=rpkgs(&win); wf.reverse();
    let mut wp=Vec::new(); for p in &wf { if let Ok(pk)=ResourcePackage::from_file(p){wp.push(pk);} }
    let read_win=|r:&RuntimeResourceID|->Option<Vec<u8>>{ for pk in &wp { if pk.resources().contains_key(r){ if let Ok(d)=pk.read_resource(r){return Some(d);}}} None };
    // mac RTLV: (blob)
    let mut mac_rtlv: Vec<Vec<u8>>=Vec::new();
    for p in rpkgs(&mac) {
        if p.file_name().unwrap().to_string_lossy().ends_with("patch2.rpkg") { continue; }
        if let Ok(pk)=ResourcePackage::from_file(&p) {
            for (r,i) in pk.resources().iter() { if i.data_type()=="RTLV" { if let Ok(d)=pk.read_resource(r){mac_rtlv.push(d);} } }
        }
    }
    println!("RTLV no Mac: {}", mac_rtlv.len());

    // pega alguns RTLV do mod
    let mut metas=Vec::new();
    fn collect(d:&Path,o:&mut Vec<PathBuf>){ if let Ok(rd)=fs::read_dir(d){for e in rd.flatten(){let p=e.path(); if p.is_dir(){collect(&p,o)} else if p.to_string_lossy().ends_with(".RTLV.meta.json"){o.push(p)}}}}
    collect(&moddir,&mut metas);
    for m in metas.iter().take(5) {
        let v:Value=serde_json::from_str(&fs::read_to_string(m)?)?;
        let x=RuntimeResourceID::from_hex_string(v["hash_value"].as_str().unwrap())?;
        let nrefs=v["hash_reference_data"].as_array().map(|a|a.len()).unwrap_or(0);
        let eng=match read_win(&x){Some(e)=>e,None=>{println!("  {} sem original Windows",v["hash_value"].as_str().unwrap());continue;}};
        // acha o RTLV do Mac de mesmo tamanho com menos diferencas
        let mut best:Option<(usize,usize)>=None; // (idx, diffcount)
        for (idx,mb) in mac_rtlv.iter().enumerate() {
            if mb.len()!=eng.len() { continue; }
            let diff=eng.iter().zip(mb).filter(|(a,b)|a!=b).count();
            if best.map(|(_,d)|diff<d).unwrap_or(true){best=Some((idx,diff));}
        }
        match best {
            Some((idx,diff))=>{
                let mb=&mac_rtlv[idx];
                let offs:Vec<usize>=eng.iter().zip(mb).enumerate().filter(|(_,(a,b))|a!=b).map(|(i,_)|i).collect();
                println!("  RTLV {} | refs={} | win {}B | melhor Mac (mesmo tam) difere em {} bytes", v["hash_value"].as_str().unwrap(), nrefs, eng.len(), diff);
                if !offs.is_empty(){ println!("     offsets diferentes (primeiros): {:?}", &offs[..offs.len().min(20)]); }
            }
            None=>println!("  RTLV {} | win {}B | nenhum RTLV Mac do mesmo tamanho", v["hash_value"].as_str().unwrap(), eng.len()),
        }
    }
    Ok(())
}
