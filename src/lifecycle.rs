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
use std::path::{Path, PathBuf};
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

/// Átállás az adatmappára: a `config.json` és a `wallpapers` mappa korábban az exe mellett
/// volt (`old`), most az adatmappában (`new`, %LOCALAPPDATA%\CyberShell) van, mert a
/// program mappája lehet írásvédett (pl. Store-ból telepítve). Az első induláskor az exe
/// melletti példányokat átmásoljuk; az eredetiek biztonsági mentésként megmaradnak. Ami az
/// adatmappában már megvan, ahhoz nem nyúlunk. Igazat ad, ha a háttérképek átköltöztek.
pub(crate) fn migrate_data(old: &Path, new: &Path) -> bool {
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same(old, new) {
        return false;
    }
    for name in ["config.json", "config.backup.json"] {
        let (from, to) = (old.join(name), new.join(name));
        if from.is_file() && !to.exists() {
            match std::fs::copy(&from, &to) {
                Ok(_) => log(format!("átköltöztetve: {} → {}", from.display(), to.display())),
                Err(e) => log(format!("nem sikerült átmásolni ({}): {e}", from.display())),
            }
        }
    }
    let (from, to) = (old.join("wallpapers"), new.join("wallpapers"));
    if !from.is_dir() || to.exists() {
        return false;
    }
    if let Err(e) = std::fs::create_dir_all(&to) {
        log(format!("a háttérképek mappája nem hozható létre ({}): {e}", to.display()));
        return false;
    }
    let (mut ok, mut failed) = (0, 0);
    for f in std::fs::read_dir(&from).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()) {
        let Some(name) = f.file_name() else { continue };
        match std::fs::copy(&f, to.join(name)) {
            Ok(_) => ok += 1,
            Err(e) => {
                failed += 1;
                log(format!("nem sikerült átmásolni ({}): {e}", f.display()));
            }
        }
    }
    log(format!("háttérképek átköltöztetve: {} → {} ({ok} kép, {failed} hiba)", from.display(), to.display()));
    true
}

/// A gyári háttérképeket a mappába írja, ha hiányoznak, vagy ha a tartalmuk eltér a beépítettől
/// (a méret eltérése elég; azonos méretnél a tartalmat hasonlítjuk össze). Visszaadja, hányat írt.
fn sync_builtin_wallpapers(dir: &Path, walls: &[(&str, &[u8])]) -> usize {
    let mut n = 0;
    for (name, data) in walls {
        let p = dir.join(name);
        let same = match std::fs::metadata(&p) {
            Ok(m) if m.len() == data.len() as u64 => std::fs::read(&p).is_ok_and(|d| d == *data),
            _ => false,
        };
        if !same && std::fs::write(&p, data).is_ok() {
            n += 1;
        }
    }
    n
}

/// A gyári háttérképek fájlnevének eleje: így ismerjük fel egy korábbi változat gyári képeit is.
const BUILTIN_WALL_PREFIX: &str = "cybershell_background_";

/// Egy korábbi változat már nem beépített gyári háttérképeinek törlése (pl. a v2.0 12 képéből
/// a v2.1-ben 8 maradt). Csak a gyári elnevezésű képekhez nyúl, a sajátokhoz nem. Visszaadja, hányat törölt.
fn remove_obsolete_wallpapers(dir: &Path, walls: &[(&str, &[u8])]) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut n = 0;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let builtin_name = name.to_lowercase().starts_with(BUILTIN_WALL_PREFIX);
        if builtin_name && !walls.iter().any(|(w, _)| w.eq_ignore_ascii_case(&name)) && std::fs::remove_file(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// A konfiguráció előkészítése induláskor: a régi helyről átköltöztetett adatok, a gyári
/// háttérképek, a config.json betöltése (hibánál alapbeállítás), az „Indulás a géppel”
/// összehangolása a Windowszal, végül a kiegészített fájl visszaírása. A `custom` a --config
/// kapcsoló fájlja (ilyenkor nincs költöztetés). Visszaadja a beállítást és a fájl helyét.
pub(crate) fn prepare_config(base_dir: &Path, custom: Option<PathBuf>, legacy_autostart: bool) -> (Config, PathBuf) {
    // A beállítások és a háttérképek az adatmappában vannak (a program mappája lehet
    // írásvédett); a korábban az exe mellett tartottakat az első induláskor átmásoljuk.
    let walls_moved = custom.is_none() && migrate_data(base_dir, &data_dir());
    let cfg_path = custom.unwrap_or_else(|| data_dir().join("config.json"));
    // A gyári háttérképek kicsomagolása: a hiányzók (első induláskor mind, egy újabb változat
    // új képei a meglévők mellé), és azok is, amelyek tartalma eltér az exe-be építettől (pl. egy
    // újabb változatban ugyanazon a néven másik kép van).
    let wall_dir = cfg_path.parent().map(|p| p.join("wallpapers")).unwrap_or_else(|| PathBuf::from("wallpapers"));
    if std::fs::create_dir_all(&wall_dir).is_ok() {
        let n = sync_builtin_wallpapers(&wall_dir, crate::embedded::WALLPAPERS);
        if n > 0 {
            log(format!("{n} gyári háttérkép kicsomagolva vagy frissítve: {}", wall_dir.display()));
        }
        let old = remove_obsolete_wallpapers(&wall_dir, crate::embedded::WALLPAPERS);
        if old > 0 {
            log(format!("{old} korábbi gyári háttérkép törölve: {}", wall_dir.display()));
        }
    }
    let mut loaded_ok = true;
    let mut cfg = match config::load(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            log(&e);
            loaded_ok = false;
            let c = default_config();
            if !cfg_path.exists() {
                let _ = config::save(&cfg_path, &c);
            }
            c
        }
    };
    // „Indulás a géppel”: a konfigurációs fájl az irányadó. Ha még nincs benne,
    // a Windows jelenlegi beállítását vesszük át. Bekapcsolt állapotban a bejegyzést
    // mindig frissítjük (aktuális exe-útvonal, az Intézőre váró indítás).
    let registered = autostart::is_enabled() || legacy_autostart;
    let mut autostart_changed = false;
    match cfg.start_with_windows {
        None => cfg.start_with_windows = Some(registered),
        // a Windowsban (Feladatkezelő) kikapcsolták: ezt tiszteletben tartjuk
        Some(true) if autostart::disabled_in_windows() => {
            log("indítás a Windowszal: a Windowsban letiltották, kikapcsolva");
            cfg.start_with_windows = Some(false);
            autostart_changed = true;
        }
        Some(want) => {
            if (want || registered)
                && let Err(e) = autostart::set(want)
            {
                log(format!("indítás a Windowszal – hiba: {e}"));
            }
        }
    }
    // Átköltöztetett háttérképek: a beállításban és a Windowsban is az új helyükre mutasson
    // a kép, hogy a régi mappa törölhető legyen.
    let mut wallpaper_moved = false;
    if walls_moved {
        let old_walls = base_dir.join("wallpapers");
        if let Some(p) = moved_path(&cfg.wallpaper, &old_walls, &wall_dir) {
            cfg.wallpaper = p;
            wallpaper_moved = true;
        }
        // a COM itt még nem fut a fő szálon: a háttérkép-kezelőt külön szálon érjük el
        let (old, new) = (old_walls.clone(), wall_dir.clone());
        let moved = std::thread::spawn(move || {
            let _com = crate::wmi::ComThread::init();
            let p = moved_path(&crate::sys::current_wallpaper(), &old, &new)?;
            Some(crate::sys::set_wallpaper(&p).map(|()| p))
        })
        .join()
        .ok()
        .flatten();
        match moved {
            Some(Ok(p)) => log(format!("a Windows háttérképe az új helyére állítva: {p}")),
            Some(Err(e)) => log(format!("a Windows háttérképe nem állítható át: {e}")),
            None => {}
        }
    }
    // A fájlba minden opció kerüljön bele (a hiányzókat alapértékkel kiegészítjük).
    if loaded_ok && (autostart_changed || wallpaper_moved || config::needs_rewrite(&cfg_path, &cfg)) {
        match config::save(&cfg_path, &cfg) {
            Ok(()) => log("konfigurációs fájl kiegészítve az összes opcióval"),
            Err(e) => log(e),
        }
    }
    (cfg, cfg_path)
}

/// Ha a kép a régi mappában van, és az új mappában megvan ugyanilyen nevű, az új útvonala.
pub(crate) fn moved_path(path: &str, old_dir: &Path, new_dir: &Path) -> Option<String> {
    let p = Path::new(path);
    let norm = |d: &Path| d.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    if path.is_empty() || p.parent().map(norm) != Some(norm(old_dir)) {
        return None;
    }
    let n = new_dir.join(p.file_name()?);
    n.is_file().then(|| n.to_string_lossy().to_string())
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
    c.normalize_edges();
    c.resolve_scheme();
    // Az alap ikonlista (assets/default-apps.json, az exe-be építve). Ami ezen a gépen máshol
    // van, azt az indításkori ellenőrzés megkeresi; ami nincs telepítve, az rejtve marad.
    c.apps = serde_json::from_str(include_str!("../assets/default-apps.json")).unwrap_or_default();
    if c.apps.is_empty() {
        c.apps = vec![config::AppEntry {
            name: "Fájlok".into(),
            path: "explorer.exe".into(),
            icon: "folder".into(),
            maximize: Some(false),
            ..Default::default()
        }];
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_wallpapers_refresh() {
        let d = temp("walls");
        let walls: &[(&str, &[u8])] = &[("a.jpg", b"uj-a"), ("b.jpg", b"uj-b"), ("c.jpg", b"ccc")];
        std::fs::write(d.join("a.jpg"), "regi-a").unwrap(); // más méret
        std::fs::write(d.join("b.jpg"), "xx-b").unwrap(); // azonos méret, más tartalom
        std::fs::write(d.join("c.jpg"), "ccc").unwrap(); // azonos
        std::fs::write(d.join("sajat.jpg"), "sajat").unwrap(); // saját kép: nem nyúlunk hozzá
        assert_eq!(sync_builtin_wallpapers(&d, walls), 2);
        assert_eq!(std::fs::read(d.join("a.jpg")).unwrap(), b"uj-a");
        assert_eq!(std::fs::read(d.join("b.jpg")).unwrap(), b"uj-b");
        assert_eq!(std::fs::read(d.join("sajat.jpg")).unwrap(), b"sajat");
        assert_eq!(sync_builtin_wallpapers(&d, walls), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn obsolete_wallpapers_removed() {
        let d = temp("walls-old");
        let walls: &[(&str, &[u8])] = &[("CyberShell_background_01.jpg", b"a")];
        for f in ["CyberShell_background_01.jpg", "CyberShell_background_12.jpg", "sajat.jpg"] {
            std::fs::write(d.join(f), "x").unwrap();
        }
        assert_eq!(remove_obsolete_wallpapers(&d, walls), 1);
        assert!(d.join("CyberShell_background_01.jpg").exists());
        assert!(!d.join("CyberShell_background_12.jpg").exists());
        assert!(d.join("sajat.jpg").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Üres ideiglenes mappa a teszthez (a végén töröljük).
    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cybershell-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn data_migration() {
        let root = temp("migrate");
        let (old, new) = (root.join("exe"), root.join("data"));
        std::fs::create_dir_all(old.join("wallpapers")).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("config.json"), "{\"apps\": []}").unwrap();
        std::fs::write(old.join("wallpapers").join("sajat.jpg"), "kép").unwrap();
        std::fs::write(old.join("wallpapers").join("tech-plates.jpg"), "gyári").unwrap();

        assert!(migrate_data(&old, &new));
        assert_eq!(std::fs::read_to_string(new.join("config.json")).unwrap(), "{\"apps\": []}");
        assert_eq!(std::fs::read_to_string(new.join("wallpapers").join("sajat.jpg")).unwrap(), "kép");
        assert!(new.join("wallpapers").join("tech-plates.jpg").is_file());
        // az eredetiek megmaradnak
        assert!(old.join("config.json").is_file() && old.join("wallpapers").join("sajat.jpg").is_file());

        // másodszorra már nem nyúl semmihez: az adatmappa példánya az irányadó
        std::fs::write(new.join("config.json"), "{\"apps\": [1]}").unwrap();
        assert!(!migrate_data(&old, &new));
        assert_eq!(std::fs::read_to_string(new.join("config.json")).unwrap(), "{\"apps\": [1]}");

        // ugyanaz a mappa: nincs teendő
        assert!(!migrate_data(&new, &new));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn moved_wallpaper() {
        let root = temp("moved");
        let (old, new) = (root.join("exe").join("wallpapers"), root.join("data").join("wallpapers"));
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("tech-plates.jpg"), "").unwrap();
        let in_old = old.join("tech-plates.jpg").to_string_lossy().to_string();
        let expected = new.join("tech-plates.jpg").to_string_lossy().to_string();
        assert_eq!(moved_path(&in_old, &old, &new), Some(expected.clone()));
        // a kis- és nagybetű nem számít (Windows-útvonal)
        assert_eq!(moved_path(&in_old.to_uppercase(), &old, &new).map(|p| p.to_lowercase()), Some(expected.to_lowercase()));
        // ami nem a régi mappában van, vagy nincs meg az új helyen, marad
        assert_eq!(moved_path(r"C:\Képek\tech-plates.jpg", &old, &new), None);
        assert_eq!(moved_path(&old.join("nincs.jpg").to_string_lossy(), &old, &new), None);
        assert_eq!(moved_path("", &old, &new), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
