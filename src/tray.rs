//! A háttérben futó programok: az értesítési terület (a Windows tálcáján a dátum melletti
//! ikonsor) programjai közül azok, amelyek most futnak. A Windows minden ott megjelent ikont
//! megjegyez a `HKCU\Control Panel\NotifyIconSettings` alatt (a program útvonala, kitűzték-e a
//! tálcára, a sorrendje és az ikon legutóbbi képe PNG-ként); a lista ebből és a futó folyamatok
//! listájából áll össze. A panel Háttérprogramok modulja mutatja.

use crate::wintrack;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, REG_VALUE_TYPE, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW,
    RegGetValueW, RegOpenKeyExW,
};
use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::{GUID, HSTRING, PWSTR};

const ROOT: &str = r"Control Panel\NotifyIconSettings";

/// Egy háttérben futó program az értesítési területen.
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
    for id in subkeys() {
        let key = format!(r"{ROOT}\{id}");
        let Some(raw) = reg_string(&key, "ExecutablePath") else { continue };
        let path = resolve_known_folder(&raw);
        let lower = path.to_lowercase();
        // a Windows Intéző saját ikonjai (pl. hardver biztonságos eltávolítása) nem programok
        if !running.contains(&lower) || lower == own || lower.ends_with(r"\explorer.exe") {
            continue;
        }
        let icon = reg_binary(&key, "IconSnapshot").filter(|d| !d.is_empty()).map(|d| {
            let mut h = DefaultHasher::new();
            d.hash(&mut h);
            h.finish()
        });
        let item = TrayApp {
            id,
            path,
            icon,
            promoted: reg_dword(&key, "IsPromoted").unwrap_or(0) != 0,
            order: reg_dword(&key, "UIOrder").unwrap_or(u32::MAX),
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
    reg_binary(&format!(r"{ROOT}\{id}"), "IconSnapshot").filter(|d| d.starts_with(b"\x89PNG"))
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

/// A NotifyIconSettings bejegyzéseinek azonosítói.
fn subkeys() -> Vec<String> {
    let mut out = Vec::new();
    let mut key = HKEY::default();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(ROOT), None, KEY_READ, &mut key) }.is_err() {
        return out;
    }
    for i in 0.. {
        let mut buf = [0u16; 64];
        let mut len = buf.len() as u32;
        let r = unsafe { RegEnumKeyExW(key, i, Some(PWSTR(buf.as_mut_ptr())), &mut len, None, None, None, None) };
        if r == ERROR_NO_MORE_ITEMS || r.is_err() {
            break;
        }
        out.push(String::from_utf16_lossy(&buf[..len as usize]));
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    out
}

fn reg_string(key: &str, value: &str) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    };
    if r.is_err() {
        return None;
    }
    let n = (len as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..n]).trim().to_string()).filter(|s| !s.is_empty())
}

fn reg_dword(key: &str, value: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut len = 4u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut u32 as *mut _),
            Some(&mut len),
        )
    };
    r.is_ok().then_some(v)
}

fn reg_binary(key: &str, value: &str) -> Option<Vec<u8>> {
    let (k, v) = (HSTRING::from(key), HSTRING::from(value));
    let mut len = 0u32;
    let mut kind = REG_VALUE_TYPE::default();
    if unsafe { RegGetValueW(HKEY_CURRENT_USER, &k, &v, RRF_RT_REG_BINARY, Some(&mut kind), None, Some(&mut len)) }.is_err() || len == 0 {
        return None;
    }
    let mut data = vec![0u8; len as usize];
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, &k, &v, RRF_RT_REG_BINARY, None, Some(data.as_mut_ptr() as *mut _), Some(&mut len)) };
    if r.is_err() {
        return None;
    }
    data.truncate(len as usize);
    Some(data)
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
