// bridge: testa o "content-bridge". Para recursos do mod, le o ORIGINAL ingles no
// Windows (mesmo ID), descomprime, e procura o MESMO conteudo (bytes) entre os
// recursos do Mac. Mede a taxa de casamento -> viabilidade do remap por conteudo.
// Uso: bridge <Mac Resources dir> <Windows Runtime dir> <content_dir do mod>

use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const META: &str = ".meta.json";

fn metas(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { metas(&p, out); }
            else if p.to_string_lossy().ends_with(META) { out.push(p); }
        }
    }
}

fn rpkgs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir).map(|rd| rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x=="rpkg").unwrap_or(false))
        .collect()).unwrap_or_default();
    v.sort();
    v
}

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    let mac = PathBuf::from(&a[1]);
    let win = PathBuf::from(&a[2]);
    let mod_dir = PathBuf::from(&a[3]);

    // 1) tipos dos recursos do mod
    let mut ms = Vec::new(); metas(&mod_dir, &mut ms);
    let mut mod_by_type: HashMap<String, Vec<RuntimeResourceID>> = HashMap::new();
    for m in &ms {
        let v: Value = serde_json::from_str(&fs::read_to_string(m)?)?;
        let t = v["hash_resource_type"].as_str().unwrap_or("?").to_string();
        let rrid = RuntimeResourceID::from_hex_string(v["hash_value"].as_str().unwrap())?;
        mod_by_type.entry(t).or_default().push(rrid);
    }
    for (t, v) in &mod_by_type { eprintln!("mod {} : {}", t, v.len()); }

    // 2) Windows: TODOS os pacotes, ordenados DESC (patch alto antes da base -> versao mais recente)
    let mut win_pkgs: Vec<(String, ResourcePackage)> = Vec::new();
    let mut win_files = rpkgs(&win);
    win_files.sort();
    win_files.reverse(); // chunkNpatchK (K alto) antes de chunkN base
    for p in &win_files {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        match ResourcePackage::from_file(p) {
            Ok(pkg) => win_pkgs.push((name, pkg)),
            Err(e) => eprintln!("win ERRO {}: {}", name, e),
        }
    }
    eprintln!("win pacotes carregados: {}", win_pkgs.len());
    let read_win = |rrid: &RuntimeResourceID| -> Option<Vec<u8>> {
        for (_, pkg) in &win_pkgs {
            if pkg.resources().contains_key(rrid) {
                if let Ok(d) = pkg.read_resource(rrid) { return Some(d); }
            }
        }
        None
    };

    // 3) Mac: conjunto de conteudos (bytes descomprimidos) por tipo
    fn build_mac_content(mac: &Path, types: &[&str]) -> HashMap<String, HashSet<Vec<u8>>> {
        let mut map: HashMap<String, HashSet<Vec<u8>>> = HashMap::new();
        for t in types { map.insert(t.to_string(), HashSet::new()); }
        for p in rpkgs(mac) {
            if p.file_name().unwrap().to_string_lossy()=="chunk0patch2.rpkg" { continue; }
            if let Ok(pkg) = ResourcePackage::from_file(&p) {
                for (rrid, info) in pkg.resources().iter() {
                    let ty = format!("{:?}", info.data_type());
                    let ty = ty.trim_matches('"').to_string();
                    if let Some(set) = map.get_mut(&ty) {
                        if let Ok(d) = pkg.read_resource(rrid) { set.insert(d); }
                    }
                }
            }
        }
        map
    }
    eprintln!("lendo conteudo do Mac (LOCR/DLGE/RTLV)...");
    let mac_content = build_mac_content(&mac, &["LOCR","DLGE","RTLV"]);
    for (t, s) in &mac_content { eprintln!("mac {} conteudos: {}", t, s.len()); }

    // 4) para cada tipo, ver quantos originais Windows do mod casam com conteudo Mac
    eprintln!("\n== CASAMENTO content-bridge (mod->Windows original->Mac) ==");
    for t in ["LOCR","DLGE","RTLV"] {
        let ids = match mod_by_type.get(t) { Some(v)=>v, None=>continue };
        let macset = &mac_content[t];
        let mut not_in_win=0; let mut matched=0; let mut nomatch=0;
        let sample: Vec<_> = ids.iter().collect();
        for rrid in &sample {
            match read_win(rrid) {
                None => not_in_win+=1,
                Some(content) => {
                    if macset.contains(&content) { matched+=1; } else { nomatch+=1; }
                }
            }
        }
        eprintln!("{}: amostra={} | nao-achado-no-Windows={} | CASOU-no-Mac={} | leu-Windows-mas-sem-match-Mac={}",
            t, sample.len(), not_in_win, matched, nomatch);
    }
    Ok(())
}
