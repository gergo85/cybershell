//! Indulás és leállás körüli teendők: átállás a régi névről, egy korábbi futás maradványainak
//! eltakarítása, az őrfolyamat, vészleállás, alapbeállítás és a Claude Code állapotsor-parancsa.

use crate::APP_NAME;
use crate::CLEANUP;
use crate::HOTKEY_EXIT;
use crate::LOG;
use crate::config::Config;
use crate::data_dir;
use crate::log;
use crate::state_path;
use crate::{appbar, autostart, config, usage};
use serde_json::json;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_HOTKEY};
use windows::core::w;

/// Átállás a régi névről (v1.2-ig CyberHub): ha a régi program még fut, szabályosan
/// leállítjuk; az adatmappáját (claude.ai-belépés, napló) átnevezzük, a régi
/// „Indulás a géppel” bejegyzést töröljük. Igazat ad, ha a régi bejegyzés be volt kapcsolva.
pub(crate) fn migrate_legacy() -> bool {
    unsafe {
        if let Ok(h) = FindWindowW(None, w!("CyberHub left"))
            && !h.is_invalid()
        {
            let _ = PostMessageW(Some(h), WM_HOTKEY, WPARAM(HOTKEY_EXIT as usize), LPARAM(0));
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(8) && FindWindowW(None, w!("CyberHub left")).is_ok_and(|h| !h.is_invalid()) {
                std::thread::sleep(Duration::from_millis(150));
            }
            // az őrfolyamat és a WebView2 is engedje el a mappát
            std::thread::sleep(Duration::from_millis(1200));
        }
    }
    if let Ok(base) = std::env::var("LOCALAPPDATA").map(PathBuf::from) {
        let (old, new) = (base.join("CyberHub"), base.join(APP_NAME));
        if old.is_dir() && !new.exists() {
            for _ in 0..10 {
                if std::fs::rename(&old, &new).is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    autostart::remove_legacy()
}

/// Ha egy korábbi futás nem tudott rendesen kilépni, itt rendbe tesszük a képernyőt.
pub(crate) fn recover_stale() -> bool {
    let p = state_path();
    let Ok(text) = std::fs::read_to_string(&p) else {
        return false;
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = v["hwnds"].as_array() {
            for h in arr.iter().filter_map(|x| x.as_i64()) {
                appbar::unregister(HWND(h as isize as *mut core::ffi::c_void));
            }
        }
        if let Some(s) = v["taskbarState"].as_u64() {
            appbar::restore_taskbar(s as u32);
        }
    }
    let _ = std::fs::remove_file(&p);
    log("előző futás maradványai eltávolítva");
    true
}

/// Őrfolyamat: megvárja, amíg a fő példány kilép – akár összeomlás vagy a
/// Feladatkezelőből leállítás miatt –, és ha az nem tudott rendet rakni, rendet rak.
pub(crate) fn guard(pid: u32) {
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(h) => {
                WaitForSingleObject(h, INFINITE);
                let _ = CloseHandle(h);
            }
            Err(_) => return,
        }
    }
    if let Ok(f) = std::fs::OpenOptions::new().append(true).open(data_dir().join("cybershell.log")) {
        *LOG.lock().unwrap() = Some(f);
    }
    std::thread::sleep(Duration::from_millis(300));
    let running = || unsafe { FindWindowW(None, w!("CyberShell left")) }.map(|h| !h.is_invalid()).unwrap_or(false);
    if running() {
        return;
    }
    if state_path().exists() {
        log("őr: a program nem lépett ki rendesen, visszaállítás");
        recover_stale();
    }
    // Néhány másodpercig figyeljük, hogy a tálca tényleg látható maradjon.
    for _ in 0..25 {
        if running() {
            return;
        }
        if !appbar::taskbar_visible() {
            log("őr: a tálca rejtve maradt, megjelenítés");
            appbar::show_taskbar();
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

pub(crate) fn emergency_cleanup() {
    if let Ok(c) = CLEANUP.lock()
        && let Some((hwnds, tb)) = c.as_ref()
    {
        for h in hwnds {
            appbar::unregister(HWND(*h as *mut core::ffi::c_void));
        }
        if let Some(s) = tb {
            appbar::restore_taskbar(*s);
        }
    }
}

pub(crate) fn default_config() -> Config {
    let mut c = Config::default();
    c.resolve_language();
    c.resolve_scheme();
    c.apps = vec![config::AppEntry {
        name: "Fájlok".into(),
        path: "explorer.exe".into(),
        icon: "folder".into(),
        match_title: String::new(),
        maximize: Some(false),
        ..Default::default()
    }];
    c
}

/// Claude Code állapotsor-parancsa: a Claude Code a standard bemeneten átadja az
/// aktuális adatokat (köztük az 5 órás és heti korlátot). Ezeket elmentjük a
/// panelnek, és egy rövid sort írunk vissza, amit a Claude Code megjelenít.
pub(crate) fn statusline() {
    use std::io::Read;
    let mut input = String::new();
    let read = std::io::stdin().read_to_string(&mut input);
    let v: serde_json::Value = serde_json::from_str(input.trim_start_matches('\u{feff}')).unwrap_or(serde_json::Value::Null);
    if std::env::var("CYBERSHELL_DEBUG").is_ok() {
        let _ = std::fs::write(
            data_dir().join("statusline-debug.txt"),
            format!("read={:?} len={} rate_limits={}", read.map_err(|e| e.kind()), input.len(), v["rate_limits"].is_object()),
        );
    }
    let rl = &v["rate_limits"];
    if rl.is_object() {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let out = json!({ "five_hour": rl["five_hour"], "seven_day": rl["seven_day"], "writtenAt": now });
        let path = usage::limits_file();
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, out.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(m) = v["model"]["display_name"].as_str() {
        parts.push(m.to_string());
    }
    if let Some(p) = rl["five_hour"]["used_percentage"].as_f64() {
        parts.push(format!("5h {}%", p.round()));
    }
    if let Some(p) = rl["seven_day"]["used_percentage"].as_f64() {
        parts.push(format!("hét {}%", p.round()));
    }
    print!("{}", parts.join(" · "));
}
