// fullcheck: carrega TODOS os .rpkg do jogo (uniao de RRIDs) e testa:
//  (a) se os hash_value do mod existem em qualquer chunk;
//  (b) para paths conhecidos, qual plataforma de hashing casa com o jogo.
// Uso: fullcheck <Resources dir>

use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::{PlatformTag, RuntimeResourceID};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);

    let mut all: HashSet<RuntimeResourceID> = HashSet::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&res_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rpkg").unwrap_or(false))
        .collect();
    files.sort();
    for p in &files {
        match ResourcePackage::from_file(p) {
            Ok(pkg) => {
                for r in pkg.resources().keys() {
                    all.insert(*r);
                }
            }
            Err(e) => eprintln!("  ERRO {}: {}", p.display(), e),
        }
    }
    eprintln!("RRIDs unicos em TODO o jogo: {}", all.len());

    // (a) hash_value do mod
    eprintln!("\n(a) hash_value do mod existem no jogo?");
    for h in ["00F7E8D794A3AA7F", "00BE5AE6CDE9C9C1", "00000766399116B2"] {
        let rrid = RuntimeResourceID::from_hex_string(h)?;
        eprintln!("   {} -> {}", h, all.contains(&rrid));
    }

    // (b) paths conhecidos (dependencias citadas pelo mod) com varias plataformas
    eprintln!("\n(b) qual plataforma de hashing casa com o jogo?");
    let paths = [
        "[assembly:/localization/hitman6.sweet].pc_dialogsoundtemplatelist",
        "[assembly:/localization/hitman6.sweet].pc_cascadinglanguagedependencies",
    ];
    let platforms = ["pc", "mac", "osx", "ios", "PS4", "XboxOne"];
    for p in paths {
        eprintln!("  {}", p);
        // hash literal (sem mexer em plataforma)
        let lit = RuntimeResourceID::from_raw_string(p);
        eprintln!("     [literal] {} -> {}", lit.to_hex_string(), all.contains(&lit));
        for plat in platforms {
            if let Ok(rid) = ResourceID::from_str(p) {
                let rrid = RuntimeResourceID::from_resource_id_with_platform(&rid, plat, PlatformTag::None);
                if all.contains(&rrid) {
                    eprintln!("     [{}] {} -> ACHOU ✅", plat, rrid.to_hex_string());
                } else {
                    eprintln!("     [{}] {} -> nao", plat, rrid.to_hex_string());
                }
            }
        }
    }
    Ok(())
}
