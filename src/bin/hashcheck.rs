// hashcheck: computa RRIDs (pc) de paths conhecidos que o mod referencia como
// dependencia (logo existem no jogo) e verifica se estao no chunk0/chunk1.
// Se NAO estiverem, o porte Mac usa hashing diferente do "pc" do Windows.
// Uso: hashcheck <Resources dir>

use rpkg_rs::misc::resource_id::ResourceID;
use rpkg_rs::resource::resource_package::ResourcePackage;
use rpkg_rs::resource::runtime_resource_id::{PlatformTag, RuntimeResourceID};
use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn rrids_of(res_dir: &Path, files: &[&str]) -> HashSet<RuntimeResourceID> {
    let mut set = HashSet::new();
    for f in files {
        let p = res_dir.join(f);
        if let Ok(pkg) = ResourcePackage::from_file(&p) {
            for rrid in pkg.resources().keys() {
                set.insert(*rrid);
            }
        }
    }
    set
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let res_dir = PathBuf::from(&args[1]);

    let set0 = rrids_of(
        &res_dir,
        &[
            "chunk0part1.rpkg", "chunk0part2.rpkg", "chunk0part3.rpkg",
            "chunk0part4.rpkg", "chunk0part5.rpkg", "chunk0part6.rpkg", "chunk0patch1.rpkg",
        ],
    );
    let set1 = rrids_of(
        &res_dir,
        &["chunk1part1.rpkg", "chunk1part2.rpkg", "chunk1part3.rpkg", "chunk1patch1.rpkg"],
    );
    eprintln!("chunk0 RRIDs={}  chunk1 RRIDs={}", set0.len(), set1.len());

    // paths que o mod referencia como dependencia (existem no jogo com certeza)
    let paths = [
        "[assembly:/localization/hitman6.sweet].pc_dialogsoundtemplatelist",
        "[assembly:/localization/hitman6.sweet].pc_cascadinglanguagedependencies",
        "[assembly:/sound/wwise/originals/voices/english(us)/italy/7000_mamba/7000_story/dr7081_bakerclosingshop_bakerassmale002_002.wav].pc_wes",
    ];

    for p in paths {
        let rid = ResourceID::from_str(p)?;
        let rrid = RuntimeResourceID::from_resource_id_with_platform(&rid, "pc", PlatformTag::None);
        let in0 = set0.contains(&rrid);
        let in1 = set1.contains(&rrid);
        eprintln!(
            "{}\n   pc-RRID={} | chunk0={} chunk1={}",
            p, rrid.to_hex_string(), in0, in1
        );
    }
    Ok(())
}
