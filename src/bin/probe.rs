// probe: (1) distribuicao do byte alto dos RRIDs do jogo (tag de plataforma?);
//        (2) testa paths conhecidos do packagedefinition com varias plataformas
//            pra achar qual hashing casa com o jogo (valida a ferramenta tambem).
// Uso: probe <Resources dir>

use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::{PlatformTag, RuntimeResourceID};
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);

    let mut all: HashSet<RuntimeResourceID> = HashSet::new();
    let mut top_byte: BTreeMap<u8, usize> = BTreeMap::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&res_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rpkg").unwrap_or(false))
        .collect();
    files.sort();
    for p in &files {
        let fname = p.file_name().unwrap().to_string_lossy().to_string();
        if fname == "chunk0patch2.rpkg" { continue; }
        if let Ok(pkg) = ResourcePackage::from_file(p) {
            for r in pkg.resources().keys() {
                all.insert(*r);
                let hex = r.to_hex_string();
                let tb = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                *top_byte.entry(tb).or_default() += 1;
            }
        }
    }
    eprintln!("RRIDs unicos no jogo (sem nosso patch): {}", all.len());
    eprintln!("Distribuicao do byte alto dos RRIDs do jogo:");
    for (b, n) in &top_byte {
        eprintln!("  0x{:02X} : {}", b, n);
    }

    // paths conhecidos, EXATOS, do packagedefinition (existem no chunk0 com certeza)
    let paths = [
        "[assembly:/_pro/scenes/frontend/boot.entity].entitytemplate",
        "[assembly:/_pro/scenes/frontend/mainmenu.entity].entitytemplate",
        "[assembly:/common/globalresources.ini].resourceidx",
    ];
    let platforms = ["pc", "mac", "osx", "ios", "win32", "dx11", "dx12"];
    eprintln!("\nTestando paths conhecidos (do packagedefinition):");
    for p in paths {
        eprintln!("  {}", p);
        let lit = RuntimeResourceID::from_raw_string(p);
        eprintln!("     [literal] {} -> {}", lit.to_hex_string(), all.contains(&lit));
        for plat in platforms {
            if let Ok(rid) = ResourceID::from_str(p) {
                let rrid = RuntimeResourceID::from_resource_id_with_platform(&rid, plat, PlatformTag::None);
                let mark = if all.contains(&rrid) { "ACHOU ✅" } else { "nao" };
                eprintln!("     [{}] {} -> {}", plat, rrid.to_hex_string(), mark);
            }
        }
    }
    Ok(())
}
