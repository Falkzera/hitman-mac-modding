// remap: gera patches nativos do Mac remapeando as traducoes do mod (IDs Windows)
// para IDs do Mac via content-bridge. Processa LOCR + DLGE (RTLV nao casa, fica fora).
// DLGE leva as REFERENCIAS do original Mac (os IDs de ref corretos do Mac).
// Um chunkNpatch2.rpkg por chunk (LOCR+DLGE combinados).
// Uso: remap <Mac Resources> <Windows Runtime> <content_dir mod> <out_dir>
// NAO toca no jogo: grava em <out_dir>.

use rpkg_rs::resource::package_builder::{PackageBuilder, PackageResourceBuilder};
use rpkg_rs::resource::resource_package::{ChunkType, PackageVersion, ResourcePackage, ResourceReferenceFlags};
use rpkg_rs::resource::resource_partition::PatchId;
use rpkg_rs::resource::runtime_resource_id::RuntimeResourceID;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const META: &str = ".meta.json";
const TYPES: [&str; 2] = ["LOCR", "DLGE"]; // RTLV excluido (blob difere entre plataformas)

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
    let mut v: Vec<PathBuf> = fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|x| x=="rpkg").unwrap_or(false)).collect()).unwrap_or_default();
    v.sort(); v
}
fn chunk_of(fname: &str) -> Option<u32> {
    fname.strip_prefix("chunk")?.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
}

type Refs = Vec<(RuntimeResourceID, ResourceReferenceFlags)>;

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    let mac = PathBuf::from(&a[1]);
    let win = PathBuf::from(&a[2]);
    let mod_dir = PathBuf::from(&a[3]);
    let out = PathBuf::from(&a[4]);
    fs::create_dir_all(&out)?;

    // 1) Windows: todos pacotes DESC (patch alto antes -> versao mais recente)
    let mut wf = rpkgs(&win); wf.sort(); wf.reverse();
    let mut win_pkgs: Vec<ResourcePackage> = Vec::new();
    for p in &wf { if let Ok(pk)=ResourcePackage::from_file(p){ win_pkgs.push(pk);} }
    eprintln!("win pacotes: {}", win_pkgs.len());
    let read_win = |rrid: &RuntimeResourceID| -> Option<Vec<u8>> {
        for pk in &win_pkgs { if pk.resources().contains_key(rrid) { if let Ok(d)=pk.read_resource(rrid){return Some(d);} } }
        None
    };

    // 2) Mac: por tipo, conteudo ingles -> (ID-Mac, chunk, refs). Versao mais recente.
    let mut mac_maps: HashMap<&str, HashMap<Vec<u8>, (RuntimeResourceID,u32,Refs)>> = HashMap::new();
    for t in TYPES { mac_maps.insert(t, HashMap::new()); }
    let mut macfiles = rpkgs(&mac); macfiles.sort(); macfiles.reverse();
    for p in &macfiles {
        let fname = p.file_name().unwrap().to_string_lossy().to_string();
        if fname == "chunk0patch2.rpkg" { continue; } // ignora patch instalado por nos
        let chunk = match chunk_of(&fname) { Some(c)=>c, None=>continue };
        if let Ok(pk) = ResourcePackage::from_file(p) {
            for (rrid, info) in pk.resources().iter() {
                let ty = info.data_type();
                let key = match TYPES.iter().find(|t| **t==ty) { Some(t)=>*t, None=>continue };
                if let Ok(d) = pk.read_resource(rrid) {
                    mac_maps.get_mut(key).unwrap().entry(d)
                        .or_insert_with(|| (*rrid, chunk, info.references().clone()));
                }
            }
        }
    }
    for t in TYPES { eprintln!("mac {} conteudos: {}", t, mac_maps[t].len()); }

    // 3) mod -> ingles Windows -> ID-Mac -> blob PT (+ refs do Mac p/ DLGE)
    let mut ms = Vec::new(); metas(&mod_dir, &mut ms);
    let mut per_chunk: HashMap<u32, Vec<PackageResourceBuilder>> = HashMap::new();
    let mut matched: HashMap<&str, usize> = HashMap::new();
    let mut not_win=0usize; let mut no_mac=0usize;
    for m in &ms {
        let v: Value = serde_json::from_str(&fs::read_to_string(m)?)?;
        let ty = v["hash_resource_type"].as_str().unwrap_or("");
        let key = match TYPES.iter().find(|t| **t==ty) { Some(t)=>*t, None=>continue };
        let x = RuntimeResourceID::from_hex_string(v["hash_value"].as_str().unwrap())?;
        let eng = match read_win(&x) { Some(e)=>e, None=>{not_win+=1; continue;} };
        let (y, chunk, refs) = match mac_maps[key].get(&eng) { Some(t)=>t.clone(), None=>{no_mac+=1; continue;} };
        let s = m.to_string_lossy(); let pt_path = PathBuf::from(&s[..s.len()-META.len()]);
        let pt = fs::read(&pt_path)?;
        let mut res = PackageResourceBuilder::from_memory(y, key, pt, None, false)?;
        for (rr, fl) in &refs { res.with_reference(*rr, *fl); }
        per_chunk.entry(chunk).or_default().push(res);
        *matched.entry(key).or_default() += 1;
    }
    for t in TYPES { eprintln!("remapeados {}: {}", t, matched.get(t).copied().unwrap_or(0)); }
    eprintln!("nao-no-Windows: {} | sem-ID-Mac: {}", not_win, no_mac);

    // 4) um chunkNpatch2.rpkg por chunk (chunk28=dugong eh addon; resto standard)
    let mut chunks: Vec<u32> = per_chunk.keys().copied().collect(); chunks.sort();
    println!("\n== PATCHES GERADOS em {} ==", out.display());
    let mut total=0;
    for c in &chunks {
        let resources = per_chunk.remove(c).unwrap();
        let n = resources.len(); total+=n;
        let ct = if *c==28 { ChunkType::Addon } else { ChunkType::Standard };
        let mut b = PackageBuilder::new(*c as u8, ct);
        b.with_patch_id(&PatchId::Patch(2));
        for r in resources { b.with_resource(r); }
        b.build_to_file(PackageVersion::RPKGv2, out.join(format!("chunk{}patch2.rpkg", c)))?;
        println!("  chunk{}patch2.rpkg : {} recursos", c, n);
    }
    println!("\nTotal: {} recursos em {} chunks.", total, chunks.len());
    println!("Chunks p/ subir patchlevel=2: {:?}", chunks);
    Ok(())
}
