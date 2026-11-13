//! Az exe ikonjának beágyazása (assets/cybershell.ico, a forrása assets/cybershell.svg), valamint
//! a felület (`ui` mappa) és a gyári háttérképek (`wallpapers` mappa) beágyazása, hogy az exe
//! magában is elinduljon (pl. csak az exe-t másolták egy másik gépre).
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A mappa fájljai (almappákkal együtt), a mappához viszonyított, perjeles útvonallal.
fn files(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, base, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push((rel.to_string_lossy().replace('\\', "/"), p.clone()));
        }
    }
}

fn table(name: &str, list: &[(String, PathBuf)], code: &mut String) {
    writeln!(code, "pub static {name}: &[(&str, &[u8])] = &[").unwrap();
    for (rel, path) in list {
        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        let abs = abs.to_string_lossy().trim_start_matches(r"\\?\").to_string();
        writeln!(code, "    ({rel:?}, include_bytes!({abs:?})),").unwrap();
    }
    writeln!(code, "];").unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=assets/cybershell.rc");
    println!("cargo:rerun-if-changed=assets/cybershell.ico");
    embed_resource::compile("assets/cybershell.rc", embed_resource::NONE).manifest_optional().unwrap();

    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut ui = Vec::new();
    files(&root.join("ui"), &root.join("ui"), &mut ui);
    // a gyári háttérképek (a felhasználó által később felvettek nem kerülnek bele)
    let defaults = ["blue-hexagons.jpg", "circuit-lines.jpg", "dust-bridge.jpg", "hud-interface.jpg", "motherboard.jpg", "tech-plates.jpg"];
    let walls: Vec<(String, PathBuf)> =
        defaults.iter().map(|n| (n.to_string(), root.join("wallpapers").join(n))).filter(|(_, p)| p.is_file()).collect();

    println!("cargo:rerun-if-changed=ui");
    for (_, p) in ui.iter().chain(walls.iter()) {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let mut code = String::from("// A build.rs készíti: a beágyazott felület és háttérképek.\n");
    table("UI_FILES", &ui, &mut code);
    table("WALLPAPERS", &walls, &mut code);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(out, code).unwrap();
}
