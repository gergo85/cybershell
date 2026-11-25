//! A háttérben futó programok: az értesítési terület (a Windows tálcáján a dátum melletti
//! ikonsor) programjai közül azok, amelyek most futnak. A Windows minden ott megjelent ikont
//! megjegyez a `HKCU\Control Panel\NotifyIconSettings` alatt (a program útvonala, kitűzték-e a
//! tálcára, a sorrendje és az ikon legutóbbi képe PNG-ként); a lista ebből és a futó folyamatok
//! listájából áll össze. A panel Háttérprogramok modulja mutatja.

use crate::{reg, wintrack};
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Registry::HKEY_CURRENT_USER;
use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::GUID;

const ROOT: &str = r"Control Panel\NotifyIconSettings";

/// Egy háttérben futó program az értesítési területen.
#[derive(Clone, Debug)]
pub struct TrayApp {
    /// A bejegyzés azonosítója a nyilvántartásban (az ikon képét ezzel kérjük le).
    pub id: String,
    /// A program teljes elérési útja.
    pub path: String,
    /// Az ikon képének ujjlenyomata (ha változik, a felület újratölti a képet); None: nincs kép.
    pub icon: Option<u64>,
    /// A tálcára kitűzött (nem a rejtett ikonok közötti) ikon.
    pub promoted: bool,
    /// A sorrendje a Windows értesítési területén.
    pub order: u32,
}

/// A most futó, az értesítési területen ikont tartó programok: elöl a tálcára kitűzöttek,
/// a Windows sorrendjében. Egy program csak egyszer szerepel.
pub fn list() -> Vec<TrayApp> {
    let running = running_paths();
    let own = std::env::current_exe().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut out: Vec<TrayApp> = Vec::new();
    for id in reg::subkeys(HKEY_CURRENT_USER, ROOT) {
        let key = format!(r"{ROOT}\{id}");
        let Some(raw) = reg::get_string(HKEY_CURRENT_USER, &key, "ExecutablePath") else { continue };
        let path = resolve_known_folder(&raw);
        let lower = path.to_lowercase();
        // a Windows Intéző saját ikonjai (pl. hardver biztonságos eltávolítása) nem programok
        if !running.contains(&lower) || lower == own || lower.ends_with(r"\explorer.exe") {
            continue;
        }
        let icon = reg::get_binary(HKEY_CURRENT_USER, &key, "IconSnapshot").filter(|d| !d.is_empty()).map(|d| {
            let mut h = DefaultHasher::new();
            d.hash(&mut h);
            h.finish()
        });
        let item = TrayApp {
            id,
            path,
            icon,
            promoted: reg::get_dword(HKEY_CURRENT_USER, &key, "IsPromoted").unwrap_or(0) != 0,
            order: reg::get_dword(HKEY_CURRENT_USER, &key, "UIOrder").unwrap_or(u32::MAX),
        };
        match out.iter_mut().find(|a| a.path.eq_ignore_ascii_case(&item.path)) {
            // ugyanannak a programnak több bejegyzése is lehet (pl. frissítések után): a képpel bírót tartjuk meg
            Some(a) => {
                if a.icon.is_none() && item.icon.is_some() {
                    *a = item;
                }
            }
            None => out.push(item),
        }
    }
    out.sort_by_key(|a| (!a.promoted, a.order));
    out
}

/// Az ikon legutóbbi képe (PNG) a nyilvántartásból.
pub fn icon_png(id: &str) -> Option<Vec<u8>> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    reg::get_binary(HKEY_CURRENT_USER, &format!(r"{ROOT}\{id}"), "IconSnapshot").filter(|d| d.starts_with(b"\x89PNG"))
}

/// A futó folyamatok programjainak útvonala kisbetűvel.
fn running_paths() -> HashSet<String> {
    wintrack::process_paths().into_iter().map(|(_, p)| p).collect()
}

/// A nyilvántartás egy ismert mappával kezdődő útvonalat is tárolhat (pl.
/// `{6D809377-…}\WindowsApps\…` a Program Files alatt): ilyenkor a mappa valódi helyével.
fn resolve_known_folder(path: &str) -> String {
    let Some((guid, tail)) = path.strip_prefix('{').and_then(|r| r.split_once('}')) else { return path.to_string() };
    let Ok(id) = GUID::try_from(guid) else { return path.to_string() };
    match unsafe { SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None) } {
        Ok(p) => {
            let base = unsafe { p.to_string() }.unwrap_or_default();
            unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
            format!("{base}{tail}")
        }
        Err(_) => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_known_folder;

    #[test]
    fn known_folder_paths() {
        let pf = resolve_known_folder(r"{6D809377-6AF0-444B-8957-A3773F02200E}\WinRAR\WinRAR.exe");
        assert!(pf.to_lowercase().ends_with(r"program files\winrar\winrar.exe"), "{pf}");
        assert_eq!(resolve_known_folder(r"C:\x\y.exe"), r"C:\x\y.exe");
        assert_eq!(resolve_known_folder(r"{nem-guid}\y.exe"), r"{nem-guid}\y.exe");
    }

    #[test]
    #[ignore = "a gép valódi értesítési területét olvassa"]
    fn live_tray() {
        for a in super::list() {
            println!("{} | promoted={} order={} icon={:?}", a.path, a.promoted, a.order, a.icon);
        }
    }
}
