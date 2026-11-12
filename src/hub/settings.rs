//! A teljes képernyős beállítások: adatok, megjelenítés, üzenetek, rendszerterhelés és háttérkép.

use crate::GITHUB_URL;
use crate::WEBSITE_URL;
use crate::hub::Hub;
use crate::hub::extra_key;
use crate::log;
use crate::version_label;
use crate::web::nav_url;
use crate::web::percent_encode;
use crate::web::push;
use crate::web::settings_url;
use crate::{appbar, autostart, clipboard, config, keyhook, sys, wintrack};
use serde_json::json;
use std::path::{Path, PathBuf};
use tao::window::Window;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos,
    ShowWindow,
};
use wry::WebView;

/// A teljes képernyős beállítások ablaka.
pub(crate) struct SettingsWin {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
}

impl Hub {
    pub(crate) fn settings_eval(&self, payload: &serde_json::Value) {
        if let Some(s) = &self.settings {
            push(&s.webview, payload);
        }
    }

    /// A beállításokhoz szükséges összes adat: kapcsolók, appok (telepítettséggel), futó programok.
    pub(crate) fn send_settings(&mut self) {
        if self.settings.is_none() {
            return;
        }
        let apps: Vec<serde_json::Value> = self
            .cfg
            .apps
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let mut j = self.app_json(i, a);
                j["path"] = json!(a.path);
                j["args"] = json!(a.args);
                j["rawIcon"] = json!(a.icon);
                j["place"] = json!(if a.on_right() { "right" } else { "left" });
                j["match"] = json!(a.match_exe);
                j["available"] = json!(self.available.get(i).copied().unwrap_or(true));
                j["canExeIcon"] = json!(a.resolved_path().to_lowercase().ends_with(".exe"));
                j["autostart"] = json!(a.autostart);
                j
            })
            .collect();
        // futó, de nem kitűzött programok – ezekből egy kattintással lehet ikont csinálni
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        let mut running: Vec<serde_json::Value> = Vec::new();
        for w in &wins {
            if w.path.is_empty()
                || self.matchers.iter().any(|m| m.matches(w))
                || extra_key(w).is_none()
                || w.exe == "applicationframehost.exe"
            {
                continue;
            }
            if running.iter().any(|r| r["path"] == json!(w.path)) {
                continue;
            }
            let name = self.program_name(&w.path);
            running.push(json!({
                "name": name,
                "path": w.path,
                "img": format!("/__icon/{}", percent_encode(&w.path.replace('\\', "/"))),
            }));
        }
        let current_wallpaper = match sys::current_wallpaper() {
            cur if cur.is_empty() => self.cfg.wallpaper.clone(),
            cur => cur,
        };
        let payload = json!({
            "type": "settings",
            "autostart": autostart::is_enabled(),
            "layout": { "icons": self.cfg.icons_edge, "info": self.cfg.info_edge },
            "winKey": self.cfg.start_menu.win_key,
            "clipboard": self.cfg.start_menu.clipboard,
            "appearance": self.cfg.appearance,
            "health": self.health_json(),
            "sections": self.cfg.sections,
            "language": self.cfg.language,
            "version": version_label(),
            "apps": apps,
            "running": running,
            "theme": self.theme_json(),
            "wallpapers": self.wallpaper_list(),
            "wallpaper": current_wallpaper,
        });
        self.settings_eval(&payload);
    }

    pub(crate) fn settings_show(&mut self, animate: bool) {
        self.peek_hide();
        self.fly_hide(false);
        self.launcher_hide(false);
        // csak a középső munkaterületet takarja, az oldalsó panelek és a keretek látszanak
        let mon = self.work_rect();
        if let Some(s) = self.settings.as_mut() {
            unsafe {
                let _ =
                    SetWindowPos(s.hwnd, Some(HWND_TOPMOST), mon.left, mon.top, mon.right - mon.left, mon.bottom - mon.top, SWP_SHOWWINDOW);
                let _ = SetForegroundWindow(s.hwnd);
            }
            let was_visible = s.visible;
            s.visible = true;
            if !was_visible {
                log("beállítások megnyitva");
            }
        }
        self.send_settings();
        // a felület beúszó animációval jelenik meg
        if animate {
            self.settings_eval(&json!({ "type": "show" }));
        }
        self.eval(self.info_edge(), &json!({ "type": "settings-open", "on": true }));
    }

    /// Az elsődleges képernyő munkaterülete a sávok nélkül (ahogy az elrendezés kiszámolta;
    /// a Windows munkaterület-adata egy panelváltás után még a régi lehet).
    pub(crate) fn work_rect(&self) -> RECT {
        if self.work.right > self.work.left && self.work.bottom > self.work.top { self.work } else { appbar::work_area() }
    }

    /// A nyitott beállítások ablakát a munkaterülethez igazítja (fókusz és animáció nélkül).
    pub(crate) fn settings_fit(&self) {
        let Some(s) = self.settings.as_ref().filter(|s| s.visible) else { return };
        let r = self.work_rect();
        unsafe {
            let _ = SetWindowPos(s.hwnd, Some(HWND_TOPMOST), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOACTIVATE);
        }
    }

    /// Processzor- és memóriaterhelés a beállítások kijelzőjének.
    pub(crate) fn send_sys(&mut self) {
        let now = sys::cpu_times();
        let cpu = match (self.cpu_prev, now) {
            (Some((pi, pt)), Some((ni, nt))) if nt > pt => {
                let total = (nt - pt) as f64;
                let idle = ni.saturating_sub(pi) as f64;
                Some(((1.0 - idle / total) * 100.0).clamp(0.0, 100.0))
            }
            _ => None,
        };
        self.cpu_prev = now;
        let (used, total) = sys::memory().unwrap_or((0, 0));
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
        self.settings_eval(&json!({
            "type": "sys",
            "cpu": cpu.map(|c| (c * 10.0).round() / 10.0),
            "memUsed": used,
            "memTotal": total,
            "cores": cores,
        }));
    }

    /// A beállítások mindig felül lévő ablaka eltakarná az onnan megnyitott
    /// programot (Intéző, böngésző, jegyzettömb), ezért ilyenkor hátrébb engedjük.
    /// A következő megnyitáskor újra felülre kerül.
    pub(crate) fn settings_lower(&self) {
        if let Some(s) = &self.settings {
            unsafe {
                let _ = SetWindowPos(s.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }

    /// Ha nyitva vannak a beállítások, kiúsznak (pl. egy programikonra kattintottak).
    pub(crate) fn settings_dismiss(&mut self) {
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_eval(&json!({ "type": "hide" }));
        }
    }

    pub(crate) fn settings_hide(&mut self) {
        if let Some(s) = self.settings.as_mut() {
            unsafe {
                let _ = ShowWindow(s.hwnd, SW_HIDE);
            }
            s.visible = false;
        }
        self.eval(self.info_edge(), &json!({ "type": "settings-open", "on": false }));
    }

    /// A felülettől kapott app-listából Config-bejegyzések (a meglévő, nem szerkeszthető
    /// mezőket – pl. args, match – megtartjuk).
    pub(crate) fn apps_from_settings(&self, list: &serde_json::Value) -> Vec<config::AppEntry> {
        let mut out = Vec::new();
        for item in list.as_array().into_iter().flatten() {
            let mut a: config::AppEntry = item["id"].as_u64().and_then(|i| self.cfg.apps.get(i as usize).cloned()).unwrap_or_default();
            if let Some(s) = item["name"].as_str() {
                a.name = s.trim().to_string();
            }
            if let Some(s) = item["path"].as_str()
                && a.path != s
            {
                a.path = s.to_string();
                a.match_exe.clear();
            }
            if let Some(s) = item["args"].as_str() {
                a.args = s.to_string();
            }
            if let Some(s) = item["icon"].as_str() {
                a.icon = s.to_string();
            }
            if let Some(s) = item["color"].as_str() {
                a.color = s.to_string();
            }
            a.place = if item["place"] == "right" { "right".into() } else { String::new() };
            a.autostart = item["autostart"].as_bool().unwrap_or(false);
            if !a.name.is_empty() && !a.path.is_empty() {
                out.push(a);
            }
        }
        out
    }

    pub(crate) fn on_settings_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let kind = msg["type"].as_str().unwrap_or("");
        if kind != "log" {
            log(format!("beállítások: {kind} {}", msg["name"].as_str().unwrap_or("")));
        }
        match kind {
            "ready" => {
                self.send_settings();
                // nyelvváltás után a már látható felület animáció nélkül jelenik meg újra
                if self.settings.as_ref().is_some_and(|s| s.visible) {
                    self.settings_eval(&json!({ "type": "show", "instant": true }));
                }
            }
            // a felület a kiúszó animáció végén kéri az elrejtést
            "close" => self.settings_hide(),
            "language" => {
                let lang = if msg["lang"] == "en" { "en" } else { "hu" };
                if self.cfg.language != lang || self.cfg.language_auto {
                    self.cfg.language = lang.to_string();
                    self.cfg.language_auto = false;
                    log(format!("nyelv: {lang}"));
                    // A memóriában lévő beállításból töltjük újra a felületeket (nem a fájlból
                    // olvassuk vissza), így a mentés sikerétől függetlenül átvált a nyelv.
                    self.save_config();
                    self.apply_config(None);
                    if let Some(s) = &self.settings {
                        let url = settings_url(&self.cfg);
                        log(format!("beállítások újratöltése: {url}"));
                        let _ = s.webview.load_url(&nav_url(&url));
                    }
                }
            }
            "cmd" => match msg["name"].as_str().unwrap_or("") {
                // a config.json újraolvasása (kézi szerkesztés után) és az összes felület újratöltése;
                // a beállítások nyitva maradnak, egy üzenet jelzi, hogy megtörtént
                "reload" => {
                    if self.reload() {
                        let t = self.tr("A config.json újraolvasva, a felületek újratöltve.", "config.json re-read, all panels reloaded.");
                        self.settings_eval(&json!({ "type": "toast", "text": t }));
                    }
                }
                "config" => {
                    self.settings_lower();
                    self.open_config();
                }
                "website" => {
                    self.settings_lower();
                    wintrack::shell_open(WEBSITE_URL, "", "", false);
                }
                "github" => {
                    self.settings_lower();
                    wintrack::shell_open(GITHUB_URL, "", "", false);
                }
                "exit" => self.exit_requested = true,
                "autostart" => self.set_autostart(msg["on"].as_bool().unwrap_or(false)),
                _ => {}
            },
            // a panelek helye: az ikonsáv és az információs panel nem lehet ugyanazon a szélen
            "layout" => {
                let icons = msg["icons"].as_str().unwrap_or("");
                let info = msg["info"].as_str().unwrap_or("");
                let ok = |e: &str| config::EDGE_NAMES.contains(&e);
                if ok(icons) && ok(info) && icons != info {
                    log(format!("panelek helye: ikonsáv {icons}, információs panel {info}"));
                    self.cfg.icons_edge = icons.to_string();
                    self.cfg.info_edge = info.to_string();
                    if self.save_config() {
                        self.reload();
                    }
                }
                self.send_settings();
            }
            "appearance" => {
                let v = msg["value"].as_str().unwrap_or("dark");
                if ["dark", "light", "auto"].contains(&v) && self.cfg.appearance != v {
                    self.cfg.appearance = v.to_string();
                    log(format!("megjelenés: {v}"));
                    self.cfg.resolve_scheme();
                    self.save_config();
                    self.push_theme();
                }
                self.send_settings();
            }
            "health" => {
                let enabled = msg["enabled"].as_bool().unwrap_or(self.cfg.health.enabled);
                let minutes = msg["minutes"].as_u64().map(|m| (m as u32).clamp(5, 480)).unwrap_or(self.cfg.health.minutes);
                let changed = enabled != self.cfg.health.enabled || minutes != self.cfg.health.minutes;
                self.cfg.health.enabled = enabled;
                self.cfg.health.minutes = minutes;
                if changed {
                    log(format!("egészség: {}, {minutes} perc", if enabled { "be" } else { "ki" }));
                    self.save_config();
                    self.health_restart();
                }
                self.send_settings();
            }
            "health-reset" => {
                self.health_restart();
                self.send_settings();
            }
            "clipboard" => {
                self.cfg.start_menu.clipboard = msg["on"].as_bool().unwrap_or(false);
                clipboard::set_enabled(self.cfg.start_menu.clipboard);
                log(format!("vágólap-előzmények: {}", if self.cfg.start_menu.clipboard { "be" } else { "ki" }));
                self.save_config();
                self.send_settings();
                self.send_clips();
            }
            "winkey" => {
                self.cfg.start_menu.win_key = msg["on"].as_bool().unwrap_or(false);
                keyhook::set_enabled(self.cfg.start_menu.win_key);
                if !self.cfg.start_menu.win_key {
                    self.launcher_hide(false);
                }
                log(format!("saját Start menü: {}", if self.cfg.start_menu.win_key { "be" } else { "ki" }));
                self.save_config();
                self.send_settings();
            }
            "sections" => {
                if let Ok(mut s) = serde_json::from_value::<config::Sections>(msg["sections"].clone()) {
                    s.normalize();
                    self.cfg.sections = s;
                    if self.save_config() {
                        self.eval(self.info_edge(), &json!({ "type": "sections", "sections": self.cfg.sections }));
                    }
                }
            }
            "save-apps" => {
                let apps = self.apps_from_settings(&msg["apps"]);
                log(format!("ikonok mentve: {} db", apps.len()));
                self.cfg.apps = apps;
                if self.save_config() {
                    self.reload();
                    let t = self.tr("Az ikonok elmentve.", "Icons saved.");
                    self.toast(&t);
                }
                self.send_settings();
            }
            "browse" => {
                let owner = self.settings.as_ref().map(|s| s.hwnd).unwrap_or_default();
                if let Some(p) = sys::pick_program(owner) {
                    let is_exe = p.path.to_lowercase().ends_with(".exe");
                    self.settings_eval(&json!({
                        "type": "picked",
                        "app": {
                            "name": p.name,
                            "path": p.path,
                            "args": p.args,
                            "icon": if is_exe { "exe" } else { "" },
                            "img": if is_exe { format!("/__icon/{}", percent_encode(&p.path.replace('\\', "/"))) } else { String::new() },
                            "canExeIcon": is_exe,
                        }
                    }));
                }
            }
            "wall-set" => {
                let p = msg["path"].as_str().unwrap_or("").to_string();
                self.apply_wallpaper(&p);
            }
            "wall-add" => {
                let owner = self.settings.as_ref().map(|s| s.hwnd).unwrap_or_default();
                if let Some(file) = sys::pick_image(owner) {
                    match self.import_wallpaper(&file) {
                        Ok(p) => self.apply_wallpaper(&p),
                        Err(e) => {
                            log(format!("háttérkép másolása sikertelen: {e}"));
                            self.toast(&e);
                        }
                    }
                }
            }
            "wall-folder" => {
                let d = self.wallpaper_dir();
                let _ = std::fs::create_dir_all(&d);
                self.settings_lower();
                wintrack::shell_open(&d.to_string_lossy(), "", "", false);
            }
            "log" => log(format!("[beállítások] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    pub(crate) fn wallpaper_dir(&self) -> PathBuf {
        self.cfg_path.parent().map(|p| p.join("wallpapers")).unwrap_or_else(|| PathBuf::from("wallpapers"))
    }

    pub(crate) fn is_wallpaper_file(p: &Path) -> bool {
        p.is_file() && p.extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| sys::WALLPAPER_EXTS.contains(&e.as_str()))
    }

    /// A háttérképmappa képei, név szerint rendezve.
    pub(crate) fn wallpaper_list(&self) -> Vec<serde_json::Value> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.wallpaper_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| Self::is_wallpaper_file(p))
            .collect();
        files.sort();
        files
            .iter()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                json!({ "path": s, "url": format!("/__file/{}", percent_encode(&s.replace('\\', "/"))) })
            })
            .collect()
    }

    /// Egy tetszőleges helyről választott képet bemásol a háttérképmappába.
    pub(crate) fn import_wallpaper(&self, file: &str) -> Result<String, String> {
        let src = PathBuf::from(file);
        let dir = self.wallpaper_dir();
        if src.parent().is_some_and(|p| p.eq(&dir)) {
            return Ok(file.to_string());
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "kep".into());
        let ext = src.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_else(|| "jpg".into());
        let mut dest = dir.join(format!("{stem}.{ext}"));
        let mut n = 2;
        while dest.exists() {
            dest = dir.join(format!("{stem}-{n}.{ext}"));
            n += 1;
        }
        std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    }

    pub(crate) fn apply_wallpaper(&mut self, path: &str) {
        if !Self::is_wallpaper_file(Path::new(path)) {
            let t = self.tr("A kép nem található.", "Image not found.");
            self.toast(&t);
            return;
        }
        match sys::set_wallpaper(path) {
            Ok(()) => {
                log(format!("háttérkép: {path}"));
                if self.cfg.wallpaper != path {
                    self.cfg.wallpaper = path.to_string();
                    self.save_config();
                }
                let t = self.tr("Háttérkép beállítva.", "Wallpaper set.");
                self.toast(&t);
            }
            Err(e) => {
                log(format!("háttérkép hiba: {e}"));
                let t = self.tr("A háttérkép nem állítható be", "Couldn't set the wallpaper");
                self.toast(&format!("{t}: {e}"));
            }
        }
        self.send_settings();
    }
}
