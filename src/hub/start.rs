//! A saját Start menü és kereső: a telepített programok listája, megnyitás, parancsok.

use crate::UserEvent;
use crate::hub::Hub;
use crate::hub::layout::popup_pos;
use crate::log;
use crate::send;
use crate::web::percent_encode;
use crate::web::push;
use crate::{appbar, apps, clipboard, config, wintrack};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, HWND_TOPMOST, SW_HIDE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
};
use wry::WebView;

/// A saját Start menü és kereső ablaka. Két elrendezése van: "start" (az információs
/// panel mellett, a Start gomb magasságában) és "search" (a munkaterület közepén felül).
pub(crate) struct LaunchWin {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
    pub(crate) shown_at: Instant,
    pub(crate) hidden_at: Option<Instant>,
    pub(crate) mode: &'static str,
    /// Az előtte aktív ablak: a vágólapelem ide kerül beillesztésre.
    pub(crate) prev: Option<HWND>,
}

/// A Start menü és a kereső mérete CSS-pixelben.
pub(crate) const START_W: f64 = 700.0;
pub(crate) const START_H: f64 = 780.0;
pub(crate) const SEARCH_W: f64 = 720.0;
pub(crate) const SEARCH_H: f64 = 680.0;

impl Hub {
    pub(crate) fn launcher_eval(&self, payload: &serde_json::Value) {
        if let Some(l) = &self.launcher {
            push(&l.webview, payload);
        }
    }

    /// A telepített programok újrafelsorolása háttérszálon (két percnél régebbi lista esetén,
    /// vagy ha kérik). Közben a menü a meglévő listával működik.
    pub(crate) fn apps_refresh(&mut self, force: bool) {
        if self.apps_busy || (!force && self.apps_at.is_some_and(|t| t.elapsed() < Duration::from_secs(120))) {
            return;
        }
        self.apps_busy = true;
        apps::refresh(|list| send(UserEvent::AppsReady(list)));
    }

    pub(crate) fn on_apps_ready(&mut self, list: Vec<apps::StartApp>) {
        if self.start_apps.len() != list.len() {
            log(format!("Start menü: {} program", list.len()));
        }
        self.apps_busy = false;
        self.apps_at = Some(Instant::now());
        self.start_apps = list;
        // a programnevek újra a friss listából (pl. a Notepad.exe neve „Jegyzettömb”)
        self.extra_names.clear();
        self.send_launcher_data();
    }

    /// A menü adatai: programok, kitűzöttek, legutóbbi fájlok.
    pub(crate) fn send_launcher_data(&self) {
        if self.launcher.is_none() {
            return;
        }
        let list: Vec<serde_json::Value> = self
            .start_apps
            .iter()
            .map(|a| {
                json!({
                    "id": a.id,
                    "name": a.name,
                    "target": a.target,
                    "img": format!("/__appicon/{}", percent_encode(&a.id)),
                    "exe": a.target.to_lowercase().ends_with(".exe"),
                })
            })
            .collect();
        let recent: Vec<serde_json::Value> =
            apps::recent_files(40).into_iter().map(|(name, path)| json!({ "name": name, "path": path })).collect();
        self.launcher_eval(&json!({
            "type": "launcher-data",
            "apps": list,
            "loading": self.apps_at.is_none(),
            "pinned": self.cfg.start_menu.pinned,
            "recent": recent,
            "user": apps::user_name(),
            "theme": self.theme_json(),
            "language": self.cfg.language,
            "clips": self.clips_json(),
        }));
    }

    /// A vágólap-előzmények a felületnek. A szöveg a megjelenítéshez és a kereséshez rövidítve
    /// megy át; beillesztéskor a teljes szöveg kerül vissza a vágólapra (azonosító szerint).
    fn clips_json(&self) -> serde_json::Value {
        if !self.cfg.start_menu.clipboard {
            return json!(null);
        }
        let list: Vec<serde_json::Value> = clipboard::list()
            .into_iter()
            .map(|c| {
                let source = if c.source.is_empty() {
                    String::new()
                } else {
                    self.extra_names
                        .get(&c.source.to_lowercase())
                        .cloned()
                        .or_else(|| apps::name_for_path(&self.start_apps, &c.source))
                        .unwrap_or_else(|| wintrack::display_name(&c.source))
                };
                let chars = c.text.chars().count();
                json!({ "id": c.id, "text": c.text.chars().take(4000).collect::<String>(), "len": chars, "at": c.at, "source": source })
            })
            .collect();
        json!(list)
    }

    /// Új vágólap-elem: a Start menü listája frissül.
    pub(crate) fn send_clips(&self) {
        if self.launcher.is_some() {
            self.launcher_eval(&json!({ "type": "launcher-clips", "clips": self.clips_json() }));
        }
    }

    /// A Start menü vagy a kereső megnyitása (ha ugyanaz már nyitva van, bezárul).
    /// `idx`: a kérő sáv; Win billentyűnél az egérmutató képernyőjén lévő információs panel.
    pub(crate) fn launcher_toggle(&mut self, mode: &'static str, idx: Option<usize>) {
        let Some(l) = self.launcher.as_ref() else { return };
        if l.visible && l.mode == mode {
            self.launcher_hide(false);
            return;
        }
        // épp most zárult be, mert a gombra kattintva elvesztette a fókuszt: ez zárás volt
        if !l.visible && l.mode == mode && idx.is_some() && l.hidden_at.is_some_and(|t| t.elapsed() < Duration::from_millis(450)) {
            return;
        }
        let info = self.info_edge();
        let idx = idx.filter(|i| self.bars.get(*i).is_some_and(|b| b.edge == info)).or_else(|| {
            let mut pt = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut pt);
            }
            let inside = |r: RECT| pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom;
            self.bars
                .iter()
                .position(|b| {
                    b.edge == info && b.registered && inside(appbar::work_area_of(b.rect))
                        || b.edge == info && b.registered && inside(b.rect)
                })
                .or_else(|| self.bars.iter().position(|b| b.edge == info && b.mon == 0))
        });
        let Some(idx) = idx else { return };
        let Some((bar_rect, bar_edge, bar_scale)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale)) else { return };
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_eval(&json!({ "type": "hide" }));
        }
        self.peek_hide();
        self.fly_hide(false);
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);
        let (ww, wh) = (work.right - work.left, work.bottom - work.top);
        let (w, h, left, top) = if mode == "start" {
            let w = ((START_W * scale).round() as i32).min(ww - 12);
            let h = ((START_H * scale).round() as i32).min(wh - 12);
            // a panel mellett (alatta, fölötte), az Indítás gombhoz igazítva
            let along = self.quick_along.get(&(idx, "start")).copied().unwrap_or(0.0);
            let (left, top) = popup_pos(bar_rect, bar_edge, w, h, (along * scale).round() as i32, work);
            (w, h, left, top)
        } else {
            let w = ((SEARCH_W * scale).round() as i32).min(ww - 24);
            let h = ((SEARCH_H * scale).round() as i32).min(wh - 24);
            (w, h, work.left + (ww - w) / 2, work.top + ((wh - h) as f64 * 0.2).round() as i32)
        };
        let fg = unsafe { GetForegroundWindow() };
        let Some(l) = self.launcher.as_mut() else { return };
        if fg != l.hwnd && !fg.is_invalid() {
            l.prev = Some(fg);
        }
        l.visible = true;
        l.mode = mode;
        l.shown_at = Instant::now();
        l.hidden_at = None;
        unsafe {
            let _ = SetWindowPos(l.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_SHOWWINDOW);
        }
        // a billentyűzet a keresőmezőbe menjen (Win billentyűnél a Windows nem mindig engedi)
        wintrack::activate(l.hwnd);
        let _ = l.webview.focus();
        log(format!("{} megnyitva", if mode == "start" { "Start menü" } else { "kereső" }));
        self.launcher_eval(&json!({ "type": "launcher-open", "mode": mode }));
        self.send_launcher_data();
        self.launcher_bar = Some(idx);
        self.eval_info_at(
            Some(idx),
            &json!({ "type": "launcher-state", "mode": mode, "open": true }),
            &json!({ "type": "launcher-state", "open": false }),
        );
        self.apps_refresh(false);
    }

    /// Bezáruljon-e a Start menü (kereső), mert máshova került a fókusz: csak ha az új
    /// előtérablak ugyanazon a képernyőn van, mint a menü. Az asztalnál (ami minden képernyőn
    /// ott van) az egérmutató képernyője dönt. Több monitornál így a másik képernyőn indított
    /// program vagy ott tett kattintás nem zárja be.
    pub(crate) fn launcher_lost_to(&self, fg: HWND) -> bool {
        use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint, MonitorFromWindow};
        let Some(l) = self.launcher.as_ref() else { return false };
        if fg.is_invalid() || fg == l.hwnd {
            return false;
        }
        let cls = wintrack::class_name(fg);
        unsafe {
            let own = MonitorFromWindow(l.hwnd, MONITOR_DEFAULTTONEAREST);
            let other = if cls == "Progman" || cls == "WorkerW" {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
            } else {
                MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST)
            };
            own == other
        }
    }

    pub(crate) fn launcher_hide(&mut self, lost_focus: bool) {
        let Some(l) = self.launcher.as_mut() else { return };
        if !l.visible {
            return;
        }
        l.visible = false;
        l.hidden_at = lost_focus.then(Instant::now);
        unsafe {
            let _ = ShowWindow(l.hwnd, SW_HIDE);
        }
        self.launcher_eval(&json!({ "type": "launcher-hidden" }));
        self.eval(self.info_edge(), &json!({ "type": "launcher-state", "open": false }));
    }

    pub(crate) fn on_launcher_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let kind = msg["type"].as_str().unwrap_or("");
        let text = |k: &str| msg[k].as_str().unwrap_or("").to_string();
        if !matches!(kind, "ready" | "log") {
            log(format!("Start menü: {kind} {}", msg["id"].as_str().or(msg["name"].as_str()).unwrap_or("")));
        }
        match kind {
            "ready" => {
                if let Some(l) = &self.launcher
                    && l.visible
                {
                    let mode = l.mode;
                    self.launcher_eval(&json!({ "type": "launcher-open", "mode": mode }));
                }
                self.send_launcher_data();
            }
            "close" => self.launcher_hide(false),
            // vágólap-elem: beillesztés az előző ablakba, csak másolás, törlés
            "clip-paste" | "clip-copy" => {
                let id = msg["id"].as_u64().unwrap_or(0);
                if !clipboard::copy(id) {
                    let t = self.tr("Nem sikerült a vágólapra tenni", "Could not copy to the clipboard");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": t }));
                    return;
                }
                if kind == "clip-copy" {
                    let t = self.tr("Vágólapra másolva", "Copied to the clipboard");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": t }));
                    return;
                }
                let prev = self.launcher.as_ref().and_then(|l| l.prev);
                self.launcher_hide(false);
                if let Some(p) = prev {
                    wintrack::activate(p);
                    // a Ctrl+V a fókusz átadása után menjen
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_millis(120));
                        wintrack::press_keys(&[wintrack::VK_CONTROL, b'V']);
                    });
                }
            }
            "recent-delete" => {
                if !apps::remove_recent(&text("path")) {
                    let t = self.tr("Nem sikerült levenni a listáról", "Could not remove it from the list");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": t }));
                }
                self.send_launcher_data();
            }
            "recent-clear" => {
                let n = apps::clear_recent();
                log(format!("legutóbbi fájlok törölve a listáról: {n}"));
                self.send_launcher_data();
            }
            "clip-delete" => {
                clipboard::remove(msg["id"].as_u64().unwrap_or(0));
                self.send_clips();
            }
            "clip-clear" => {
                clipboard::clear();
                self.send_clips();
            }
            "refresh" => self.apps_refresh(true),
            // egy telepített program indítása (rendszergazdaként: csak asztali programnál)
            "run-app" => {
                let id = text("id");
                let Some(app) = self.start_apps.iter().find(|a| a.id == id).cloned().or_else(|| apps::find(&id)) else { return };
                let admin = msg["admin"].as_bool().unwrap_or(false);
                self.launcher_hide(false);
                let ok = if admin && !app.target.is_empty() {
                    wintrack::shell_verb("runas", &app.target, "")
                } else {
                    wintrack::shell_open(&format!("shell:AppsFolder\\{}", app.id), "", "", false)
                };
                log(format!("Start menü: indítás – {} -> {}", app.name, if ok { "ok" } else { "HIBA" }));
                if !ok && !admin {
                    let t = self.tr("Nem indítható", "Cannot start");
                    self.toast(&format!("{t}: {}", app.name));
                }
            }
            "open-location" => {
                let id = text("id");
                if let Some(app) = self.start_apps.iter().find(|a| a.id == id)
                    && !app.target.is_empty()
                {
                    let target = app.target.clone();
                    self.launcher_hide(false);
                    wintrack::shell_open("explorer.exe", &format!("/select,\"{target}\""), "", false);
                }
            }
            // legutóbbi fájl, mappa, webcím vagy beállításoldal megnyitása
            "open" => {
                let target = text("target");
                let lower = target.to_lowercase();
                let allowed = lower.starts_with("http://")
                    || lower.starts_with("https://")
                    || lower.starts_with("ms-settings:")
                    || lower.starts_with("shell:")
                    || Path::new(&config::expand_env(&target)).exists();
                if !allowed {
                    let t = self.tr("Nem található", "Not found");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": format!("{t}: {target}") }));
                    return;
                }
                let target = config::expand_env(&target);
                self.launcher_hide(false);
                if msg["reveal"].as_bool().unwrap_or(false) {
                    wintrack::shell_open("explorer.exe", &format!("/select,\"{target}\""), "", false);
                } else if !wintrack::shell_open(&target, "", "", false) {
                    let t = self.tr("Nem nyitható meg", "Can't open");
                    self.toast(&format!("{t}: {target}"));
                }
            }
            // parancs, ahogy a Futtatás ablakban (Win+R): program és paraméterei
            "run-cmd" => {
                let line = text("text");
                let line = line.trim();
                let (file, args) = if let Some(rest) = line.strip_prefix('"') {
                    match rest.split_once('"') {
                        Some((f, a)) => (f.to_string(), a.trim().to_string()),
                        None => (rest.to_string(), String::new()),
                    }
                } else {
                    match line.split_once(' ') {
                        Some((f, a)) => (f.to_string(), a.trim().to_string()),
                        None => (line.to_string(), String::new()),
                    }
                };
                if file.is_empty() {
                    return;
                }
                let admin = msg["admin"].as_bool().unwrap_or(false);
                let ok = wintrack::shell_verb(if admin { "runas" } else { "open" }, &config::expand_env(&file), &config::expand_env(&args));
                log(format!("Start menü: parancs – {file} -> {}", if ok { "ok" } else { "HIBA" }));
                if ok {
                    self.launcher_hide(false);
                } else if !admin {
                    let t = self.tr("A Windows nem találja", "Windows can't find");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": format!("{t}: {file}") }));
                }
            }
            // webes keresés az alapértelmezett böngészőben
            "web" => {
                let q = text("q");
                if q.trim().is_empty() {
                    return;
                }
                let tpl = if self.cfg.start_menu.web_search.contains("{q}") {
                    self.cfg.start_menu.web_search.clone()
                } else {
                    config::StartMenu::default().web_search
                };
                let enc: String = percent_encode(q.trim()).replace('/', "%2F").replace(':', "%3A");
                self.launcher_hide(false);
                wintrack::shell_open(&tpl.replace("{q}", &enc), "", "", false);
            }
            "pin" | "unpin" => {
                let id = text("id");
                let pins = &mut self.cfg.start_menu.pinned;
                pins.retain(|p| *p != id);
                if kind == "pin" && !id.is_empty() {
                    pins.push(id);
                }
                self.save_config();
                self.launcher_eval(&json!({ "type": "launcher-pins", "pinned": self.cfg.start_menu.pinned }));
            }
            "pin-order" => {
                if let Some(list) = msg["pinned"].as_array() {
                    let order: Vec<String> = list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
                    let mut a = order.clone();
                    let mut b = self.cfg.start_menu.pinned.clone();
                    a.sort();
                    b.sort();
                    if a == b {
                        self.cfg.start_menu.pinned = order;
                        self.save_config();
                    }
                }
            }
            // a program ikonja a CyberShell bal sávjára kerül
            "add-bar" => {
                let id = text("id");
                let Some(app) = self.start_apps.iter().find(|a| a.id == id).cloned() else { return };
                let is_exe = app.target.to_lowercase().ends_with(".exe");
                let path = if is_exe { app.target.clone() } else { format!("shell:AppsFolder\\{}", app.id) };
                if self.cfg.apps.iter().any(|a| a.resolved_path().eq_ignore_ascii_case(&path)) {
                    let t = self.tr("Már rajta van a sávon", "Already on the bar");
                    self.launcher_eval(&json!({ "type": "launcher-toast", "text": format!("{t}: {}", app.name) }));
                    return;
                }
                self.cfg.apps.push(config::AppEntry {
                    name: app.name.chars().take(24).collect(),
                    path,
                    icon: if is_exe { "exe".into() } else { String::new() },
                    match_title: if is_exe { String::new() } else { app.name.clone() },
                    ..Default::default()
                });
                log(format!("Start menü: a sávra került – {}", app.name));
                self.launcher_hide(false);
                if self.save_config() {
                    self.reload();
                    let t = self.tr("Hozzáadva a sávhoz", "Added to the bar");
                    self.toast(&format!("{t}: {}", app.name));
                }
            }
            "power" => {
                let action = text("action");
                self.launcher_hide(false);
                self.power(&action);
            }
            "settings" => {
                self.settings_bar = self.launcher_bar;
                self.launcher_hide(false);
                self.settings_show(true);
            }
            "log" => log(format!("[Start menü] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }
}
