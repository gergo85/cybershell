//! A teljes képernyős beállítások: adatok, megjelenítés, üzenetek, rendszerterhelés és háttérkép.

use crate::GITHUB_URL;
use crate::STORE_URL;
use crate::hub::extra_key;
use crate::hub::{Hub, WindowWatch};
use crate::log;
use crate::version_label;
use crate::web::PopupParts;
use crate::web::nav_url;
use crate::web::push;
use crate::web::settings_url;
use crate::web::{exe_icon_url, file_url};
use crate::{appbar, autostart, clipboard, config, hardware, keyhook, sys, winsync, wintrack};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos,
    ShowWindow,
};
use wry::WebView;

/// A CyberShell saját (az exe-be épített) háttérképe-e ez a fájl (a fájlnév alapján, a kis- és
/// nagybetűtől függetlenül); az Alapállapot ezeket hagyja meg.
fn builtin_wallpaper(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    crate::embedded::WALLPAPERS.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name))
}

/// A teljes képernyős beállítások ablaka.
pub(crate) struct SettingsWin {
    /// Az ablak, amiben a felület fut: csak életben tartjuk (eldobásakor bezárulna).
    #[expect(dead_code, reason = "csak az ablak élettartama miatt tároljuk")]
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
}

impl SettingsWin {
    pub(crate) fn new((window, webview, hwnd): PopupParts) -> Self {
        SettingsWin { window, webview, hwnd, visible: false }
    }
}

impl Hub {
    pub(crate) fn settings_eval(&self, payload: &serde_json::Value) {
        if let Some(s) = &self.settings {
            push(&s.webview, payload);
        }
    }

    /// Rövid értesítés a beállítások alján.
    pub(crate) fn settings_toast(&self, text: &str) {
        self.settings_eval(&json!({ "type": "toast", "text": text }));
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
                j["match"] = json!(a.match_exe);
                j["available"] = json!(self.available.get(i).copied().unwrap_or(true));
                let exe = a.resolved_path().to_lowercase().ends_with(".exe");
                j["canExeIcon"] = json!(exe);
                // a program saját ikonja (az ikonválasztó első eleme), akkor is, ha most más ikon van beállítva
                if exe {
                    j["exeImg"] = json!(self.icon_for("exe", &a.resolved_path()).1);
                }
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
                "img": self.program_icon(&w.path),
            }));
        }
        let current_wallpaper = match sys::current_wallpaper() {
            cur if cur.is_empty() => self.cfg.wallpaper.clone(),
            cur => cur,
        };
        // a panelek elrendezésének előnézetéhez: az elsődleges képernyő mérete (logikai pixel)
        let mons = appbar::monitors();
        let screen = mons.first().map(|m| {
            let s = (m.dpi as f64 / 96.0).max(0.5);
            json!({ "w": ((m.rect.right - m.rect.left) as f64 / s).round(), "h": ((m.rect.bottom - m.rect.top) as f64 / s).round() })
        });
        let c = &self.cfg;
        // a naptárak titkos címét nem küldjük ki, csak a kiszolgáló nevét
        let calendars: Vec<serde_json::Value> = c
            .calendar
            .feeds
            .iter()
            .map(|f| {
                let host = f.url.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or("");
                json!({ "name": f.name, "color": f.color, "host": host })
            })
            .collect();
        let extra = json!({
            "screen": screen,
            "monitors": mons.len(),
            "bars": {
                "icons": c.left_width, "info": c.right_width, "frame": c.frame,
                "ranges": {
                    "iconsWidth": config::ICONS_RANGE, "infoWidth": c.info_range(),
                    "frame.top": config::FRAME_TB_RANGE, "frame.bottom": config::FRAME_TB_RANGE,
                    "frame.left": config::FRAME_SIDE_RANGE, "frame.right": config::FRAME_SIDE_RANGE,
                },
                "defaults": { "icons": config::Config::default().left_width, "info": config::Config::default().right_width, "frame": config::Frame::default() },
            },
            "allMonitors": c.all_monitors,
            "hideTaskbar": c.hide_taskbar,
            "activity": { "autoStart": c.activity.auto_start, "sound": c.activity.sound },
            "calendars": calendars,
            "webSearch": c.start_menu.web_search,
            "neonBorder": c.neon_border,
            "windowsSync": c.windows_sync,
            "terminalFound": winsync::terminal_installed(),
            "scheme": c.scheme,
            "themes": { "dark": c.theme, "light": c.theme_light, "defaultDark": config::Theme::default(), "defaultLight": config::Theme::light() },
        });
        if self.hardware.is_none() && !self.hardware_busy && Instant::now() >= self.hardware_retry {
            self.hardware_busy = true;
            std::thread::spawn(|| crate::send(crate::UserEvent::Hardware(hardware::query())));
        }
        let mut payload = json!({
            "type": "settings",
            "hardware": self.hardware,
            "autostart": autostart::is_enabled(),
            "layout": { "icons": self.cfg.icons_edge, "info": self.cfg.info_edge },
            "winKey": self.cfg.start_menu.win_key,
            "clipboard": self.cfg.start_menu.clipboard,
            "desktopMenu": self.cfg.desktop_menu,
            "desktopIconMenu": self.cfg.desktop_icon_menu,
            "appearance": self.cfg.appearance,
            "health": self.health_json(),
            "weather": { "city": self.cfg.weather.city, "unit": self.cfg.weather.unit, "wind": self.cfg.weather.wind },
            "currency": self.cfg.currency,
            "dateTime": self.cfg.date_time,
            // akkumulátoros gép: a Laptop modul csak ekkor szerepel a Modulok listájában
            "laptop": sys::battery().is_some(),
            // a csatlakoztatott, töltöttséget jelző Bluetooth-eszközök száma: a Perifériák modul
            // csak ekkor szerepel a Modulok listájában
            "periph": serde_json::from_str::<serde_json::Value>(&self.periph_feed.sent)
                .ok()
                .and_then(|v| v["devices"].as_array().map(|a| a.len()))
                .unwrap_or(0),
            "iconLabels": self.cfg.icon_labels,
            "tooltips": self.cfg.tooltips,
            "sections": self.cfg.sections,
            "language": self.cfg.language,
            "version": version_label(),
            "apps": apps,
            "running": running,
            "theme": self.theme_json(),
            "wallpapers": self.wallpaper_list(),
            "wallpaper": current_wallpaper,
        });
        if let (Some(p), Some(e)) = (payload.as_object_mut(), extra.as_object()) {
            p.extend(e.clone());
        }
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
        self.eval_info_at(
            self.settings_bar,
            &json!({ "type": "settings-open", "on": true }),
            &json!({ "type": "settings-open", "on": false }),
        );
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
        let cpu = sys::cpu_percent(self.cpu_prev, now);
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

    pub(crate) fn settings_visible(&self) -> bool {
        self.settings.as_ref().is_some_and(|s| s.visible)
    }

    /// Ugyanazon a képernyőn van-e az ablak, mint a nyitott beállítások. Kis méretre tett
    /// ablaknál a Windows a visszaállított helyét veszi alapul.
    pub(crate) fn on_settings_monitor(&self, hwnd: HWND) -> bool {
        let Some(s) = self.settings.as_ref().filter(|s| s.visible) else { return false };
        unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == MonitorFromWindow(s.hwnd, MONITOR_DEFAULTTONEAREST) }
    }

    /// Programikonra kattintás után: a beállítások csak akkor úsznak ki, ha az előhozott ablak
    /// ugyanazon a képernyőn van (több monitornál a másik képernyőn nyíló program mellett nyitva
    /// maradnak).
    pub(crate) fn settings_dismiss_for(&mut self, hwnd: HWND) {
        if self.on_settings_monitor(hwnd) {
            self.settings_dismiss();
        }
    }

    /// Nyitott beállítások mellett indított program: az első új ablakát figyeljük, és csak akkor
    /// úsznak ki a beállítások, ha az ugyanazon a képernyőn jelenik meg (lásd `Hub::poll`).
    /// `known`: az indítás előtt már meglévő ablakok.
    pub(crate) fn settings_dismiss_on_launch(&mut self, known: HashSet<isize>) {
        if self.settings_visible() {
            self.settings_watch = Some(WindowWatch::new(known, Duration::from_secs(20)));
        }
    }

    /// Ha nyitva vannak a beállítások, kiúsznak (pl. egy programikonra kattintottak).
    pub(crate) fn settings_dismiss(&mut self) {
        if self.settings_visible() {
            self.settings_eval(&json!({ "type": "hide" }));
        }
    }

    pub(crate) fn settings_hide(&mut self) {
        let mut hidden = None;
        if let Some(s) = self.settings.as_mut() {
            unsafe {
                let _ = ShowWindow(s.hwnd, SW_HIDE);
            }
            hidden = s.visible.then_some(s.hwnd);
            s.visible = false;
        }
        self.eval_info(&json!({ "type": "settings-open", "on": false }));
        if let Some(h) = hidden {
            self.refocus_later(h);
        }
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
                if self.settings_visible() {
                    self.settings_eval(&json!({ "type": "show", "instant": true }));
                }
            }
            // a felület a kiúszó animáció végén kéri az elrejtést
            "close" => self.settings_hide(),
            "language" => {
                let lang = match msg["lang"].as_str() {
                    Some("en") => "en",
                    Some("pl") => "pl",
                    Some("de") => "de",
                    Some("es") => "es",
                    _ => "hu",
                };
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
                        let t = self.tr(
                            "A config.json újraolvasva, a felületek újratöltve.",
                            "config.json re-read, all panels reloaded.",
                            "Ponownie wczytano config.json, panele przeładowane.",
                            "config.json neu eingelesen, alle Panels neu geladen.",
                            "config.json releído; todos los paneles recargados.",
                        );
                        self.settings_toast(&t);
                    }
                }
                "config" => {
                    self.settings_lower();
                    self.open_config();
                }
                "store" => {
                    self.settings_lower();
                    wintrack::shell_open(STORE_URL, "", "", false);
                }
                "github" => {
                    self.settings_lower();
                    wintrack::shell_open(GITHUB_URL, "", "", false);
                }
                "exit" => self.exit_requested = true,
                "reset-all" => self.reset_all(),
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
                    self.cfg.info_edge_auto = false;
                    // fent vagy lent a panelsáv legfeljebb 100 pont vastag
                    let r = self.cfg.info_range();
                    self.cfg.right_width = self.cfg.right_width.clamp(r.0, r.1);
                    if self.save_config() {
                        self.reload();
                    }
                }
                self.send_settings();
            }
            "appearance" => {
                let v = msg["value"].as_str().unwrap_or("dark");
                if ["dark", "light", "auto", "windows"].contains(&v) && self.cfg.appearance != v {
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
            // Ikonnév mutatása: a programok neve az ikonjuk alatt
            "icon-labels" => {
                let on = msg["on"].as_bool().unwrap_or(true);
                if on != self.cfg.icon_labels {
                    self.cfg.icon_labels = on;
                    log(format!("ikonnevek: {}", if on { "be" } else { "ki" }));
                    self.save_config();
                    self.eval(self.icons_edge(), &json!({ "type": "icon-labels", "on": on }));
                }
                self.send_settings();
            }
            // Időjárás: a település (szabad szöveg), a mértékegység (°C/°F) és a szél kijelzése
            "weather" => {
                let mut changed = false;
                if let Some(city) = msg["city"].as_str().map(str::trim).filter(|c| !c.is_empty()) {
                    let city: String = city.chars().take(80).collect();
                    if city != self.cfg.weather.city {
                        // a kézzel megadott koordináták a régi településhez tartoztak
                        self.cfg.weather.city = city;
                        self.cfg.weather.latitude = None;
                        self.cfg.weather.longitude = None;
                        changed = true;
                    }
                }
                if let Some(unit) = msg["unit"].as_str().filter(|u| ["c", "f"].contains(u))
                    && unit != self.cfg.weather.unit
                {
                    self.cfg.weather.unit = unit.to_string();
                    changed = true;
                }
                if let Some(wind) = msg["wind"].as_bool()
                    && wind != self.cfg.weather.wind
                {
                    self.cfg.weather.wind = wind;
                    changed = true;
                }
                if changed {
                    log(format!("időjárás: {}, °{}", self.cfg.weather.city, self.cfg.weather.unit.to_uppercase()));
                    self.save_config();
                    self.eval_info(&json!({ "type": "weather", "weather": self.cfg.weather }));
                }
                self.send_settings();
            }
            // Valuta: a panel listáiban választható pénznemek
            "currency" => {
                let mut c = self.cfg.currency.clone();
                c.list =
                    msg["list"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                c.normalize();
                if c.list != self.cfg.currency.list {
                    log(format!("valuták: {}", c.list.join(", ")));
                    self.cfg.currency = c;
                    self.save_config();
                    self.eval_info(&json!({ "type": "currency", "currency": self.cfg.currency }));
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
            "deskmenu" | "deskiconmenu" => {
                self.set_desktop_menu(msg["type"] == "deskiconmenu", msg["on"].as_bool().unwrap_or(false));
                self.save_config();
                self.send_settings();
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
                    self.configure_monitor();
                    // az újonnan bekapcsolt Perifériák és Események azonnal kapjon adatot
                    self.periph_feed.next = Instant::now();
                    self.calendar.next = Instant::now();
                    if self.save_config() {
                        self.eval_info(&json!({ "type": "sections", "sections": self.cfg.sections }));
                    }
                    self.send_tray(true);
                    self.send_recent(true);
                }
            }
            // a csak a config.json-ban szereplő beállítások (méretek, keret, színek, szinkron…)
            "set" => {
                self.on_set(msg["key"].as_str().unwrap_or(""), &msg["value"]);
                self.send_settings();
            }
            "cal-add" => {
                let t = match self.calendar_add(&msg) {
                    Ok(()) => self.tr(
                        "Naptár csatolva.",
                        "Calendar connected.",
                        "Kalendarz połączony.",
                        "Kalender verbunden.",
                        "Calendario conectado.",
                    ),
                    Err("dup") => self.tr(
                        "Ez a naptár már csatolva van.",
                        "This calendar is already connected.",
                        "Ten kalendarz jest już połączony.",
                        "Dieser Kalender ist schon verbunden.",
                        "Este calendario ya está conectado.",
                    ),
                    Err(_) => self.tr(
                        "Ez nem iCal-cím. A Google Naptár „Titkos cím iCal formátumban” mezőjét másold be (https://…).",
                        "That is not an iCal address. Paste the “Secret address in iCal format” from Google Calendar (https://…).",
                        "To nie jest adres iCal. Wklej „Tajny adres w formacie iCal” z Kalendarza Google (https://…).",
                        "Das ist keine iCal-Adresse. Füge die „Privatadresse im iCal-Format“ aus Google Kalender ein (https://…).",
                        "Esa no es una dirección iCal. Pega la «Dirección secreta en formato iCal» de Google Calendar (https://…).",
                    ),
                };
                self.settings_toast(&t);
                self.send_settings();
            }
            "cal-remove" => {
                self.calendar_remove(msg["i"].as_u64().unwrap_or(u64::MAX) as usize);
                self.send_settings();
            }
            "cal-edit" => {
                let i = msg["i"].as_u64().unwrap_or(u64::MAX) as usize;
                if let Some(f) = self.cfg.calendar.feeds.get_mut(i) {
                    if let Some(n) = msg["name"].as_str().map(str::trim).filter(|n| !n.is_empty()) {
                        f.name = n.chars().take(40).collect();
                    }
                    if let Some(c) = msg["color"].as_str().filter(|c| config::is_hex_color(c)) {
                        f.color = c.to_lowercase();
                    }
                    self.save_config();
                    self.calendar.cache = None;
                    self.send_calendar_feeds();
                }
                self.send_settings();
            }
            // a Google Naptár beállításai (itt található a titkos iCal-cím)
            "cal-help" => {
                self.settings_lower();
                wintrack::shell_open("https://calendar.google.com/calendar/r/settings", "", "", false);
            }
            "save-apps" => {
                let apps = self.apps_from_settings(&msg["apps"]);
                log(format!("ikonok mentve: {} db", apps.len()));
                self.cfg.apps = apps;
                if self.save_config() {
                    self.reload();
                    let t = self.tr("Az ikonok elmentve.", "Icons saved.", "Ikony zapisane.", "Symbole gespeichert.", "Iconos guardados.");
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
                            "img": if is_exe { exe_icon_url(&p.path) } else { String::new() },
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
            // Alapállapot: a később felvett képek a Lomtárba kerülnek, a CyberShell saját képei maradnak
            "wall-reset" => {
                let extra: Vec<PathBuf> = self.wallpaper_files().into_iter().filter(|p| !builtin_wallpaper(p)).collect();
                let n = extra.len();
                if n > 0 {
                    match sys::recycle(&extra) {
                        Ok(()) => log(format!("háttérképek alapállapotban: {n} kép a Lomtárba")),
                        Err(e) => {
                            log(format!("háttérképek törlése sikertelen: {e}"));
                            let t = self.tr(
                                "A képek nem törölhetők",
                                "Couldn't remove the images",
                                "Nie można usunąć obrazów",
                                "Die Bilder konnten nicht entfernt werden",
                                "No se pudieron quitar las imágenes",
                            );
                            self.settings_toast(&format!("{t}: {e}"));
                        }
                    }
                }
                self.send_settings();
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

    /// Egy beállítás módosítása a beállítások felületéről. Csak az itt felsorolt kulcsokat
    /// fogadjuk el, mindegyiket érvényes tartományra szorítva.
    pub(crate) fn on_set(&mut self, key: &str, v: &serde_json::Value) {
        enum Then {
            Nothing,
            Relayout,
            Theme,
            Sync,
        }
        let num = |r: (u32, u32)| v.as_f64().map(|n| (n.round().max(0.0) as u32).clamp(r.0, r.1));
        let then = match key {
            "iconsWidth" | "infoWidth" => {
                let Some(n) = num(if key == "iconsWidth" { config::ICONS_RANGE } else { self.cfg.info_range() }) else { return };
                let slot = if key == "iconsWidth" { &mut self.cfg.left_width } else { &mut self.cfg.right_width };
                if *slot == n {
                    return;
                }
                *slot = n;
                Then::Relayout
            }
            "frame.top" | "frame.bottom" | "frame.left" | "frame.right" => {
                let range = if key == "frame.top" || key == "frame.bottom" { config::FRAME_TB_RANGE } else { config::FRAME_SIDE_RANGE };
                let Some(n) = num(range) else { return };
                let f = &mut self.cfg.frame;
                let slot = match key {
                    "frame.top" => &mut f.top,
                    "frame.bottom" => &mut f.bottom,
                    "frame.left" => &mut f.left,
                    _ => &mut f.right,
                };
                if *slot == n {
                    return;
                }
                *slot = n;
                Then::Relayout
            }
            // Méretek: alapállapot (a két sáv és a négy keret alapértéke)
            "sizesReset" => {
                let d = config::Config::default();
                self.cfg.left_width = d.left_width;
                self.cfg.right_width = d.right_width;
                self.cfg.frame = config::Frame::default();
                Then::Relayout
            }
            "allMonitors" => {
                let Some(b) = v.as_bool() else { return };
                self.cfg.all_monitors = b;
                Then::Relayout
            }
            "hideTaskbar" => {
                let Some(b) = v.as_bool() else { return };
                self.set_hide_taskbar(b);
                Then::Nothing
            }
            // Súgó szövegek: minden felület azonnal elrejti vagy visszateszi a title-öket
            "tooltips" => {
                let Some(b) = v.as_bool() else { return };
                self.cfg.tooltips = b;
                log(format!("súgó szövegek: {}", if b { "be" } else { "ki" }));
                self.broadcast_theme();
                Then::Nothing
            }
            "activity.autoStart" => {
                let Some(b) = v.as_bool() else { return };
                self.cfg.activity.auto_start = b;
                Then::Nothing
            }
            "activity.sound" => {
                let Some(b) = v.as_bool() else { return };
                self.cfg.activity.sound = b;
                Then::Nothing
            }
            "webSearch" => {
                let Some(t) = v.as_str().map(str::trim) else { return };
                if !(t.starts_with("https://") || t.starts_with("http://")) || !t.contains("{q}") || t.len() > 500 {
                    let m = self.tr(
                        "A webes kereső címe https://-sel kezdődjön, és legyen benne {q}.",
                        "The web search address must start with https:// and contain {q}.",
                        "Adres wyszukiwarki musi zaczynać się od https:// i zawierać {q}.",
                        "Die Adresse der Websuche muss mit https:// beginnen und {q} enthalten.",
                        "La dirección de búsqueda web debe empezar por https:// y contener {q}.",
                    );
                    self.settings_toast(&m);
                    return;
                }
                self.cfg.start_menu.web_search = t.to_string();
                Then::Nothing
            }
            // Dátum és idő: minden felület azonnal átvált (a téma üzenetével, újratöltés nélkül)
            "dateTime.dateFormat" => {
                let Some(f) = v.as_str().filter(|f| config::DATE_FORMATS.contains(f)) else { return };
                self.cfg.date_time.date_format = f.to_string();
                self.broadcast_theme();
                Then::Nothing
            }
            "dateTime.hour12" => {
                let Some(b) = v.as_bool() else { return };
                self.cfg.date_time.hour12 = b;
                self.broadcast_theme();
                Then::Nothing
            }
            "neonBorder" => {
                let Some(m) = v.as_str().filter(|m| ["off", "active", "all"].contains(m)) else { return };
                self.cfg.neon_border = m.to_string();
                if m == "off" {
                    self.reset_borders();
                }
                Then::Nothing
            }
            // Windows kiemelőszín: ki, vagy a téma melyik színe
            "windowsSync.accentColor" => {
                let Some(m) = v.as_str().filter(|m| ["off", "main", "alt", "hot"].contains(m)) else { return };
                let w = &mut self.cfg.windows_sync;
                w.accent = m != "off";
                if m != "off" {
                    w.accent_color = m.to_string();
                }
                Then::Sync
            }
            "windowsSync.darkMode" | "windowsSync.accent" | "windowsSync.terminal" => {
                let Some(b) = v.as_bool() else { return };
                let w = &mut self.cfg.windows_sync;
                match key {
                    "windowsSync.darkMode" => w.dark_mode = b,
                    "windowsSync.accent" => w.accent = b,
                    _ => w.terminal = b,
                }
                Then::Sync
            }
            "themeReset" => {
                match v.as_str() {
                    Some("dark") => self.cfg.theme = config::Theme::default(),
                    Some("light") => self.cfg.theme_light = config::Theme::light(),
                    _ => return,
                }
                Then::Theme
            }
            k if k.starts_with("theme.") || k.starts_with("themeLight.") => {
                let Some(c) = v.as_str().filter(|c| config::is_hex_color(c)) else { return };
                let (scheme, field) = k.split_once('.').unwrap_or_default();
                let t = if scheme == "theme" { &mut self.cfg.theme } else { &mut self.cfg.theme_light };
                let slot = match field {
                    "main" => &mut t.main,
                    "alt" => &mut t.alt,
                    "hot" => &mut t.hot,
                    "bg" => &mut t.bg,
                    "text" => &mut t.text,
                    _ => return,
                };
                *slot = c.to_lowercase();
                Then::Theme
            }
            _ => return,
        };
        log(format!("beállítás: {key} = {v}"));
        if !self.save_config() {
            return;
        }
        match then {
            Then::Relayout => self.apply_config(None),
            Then::Theme => self.push_theme(),
            Then::Sync => self.sync_windows(),
            Then::Nothing => {}
        }
    }

    /// A gép alkatrészei megérkeztek a háttérszálról.
    pub(crate) fn on_hardware(&mut self, v: Option<serde_json::Value>) {
        self.hardware_busy = false;
        match v {
            Some(v) => {
                log(format!("rendszerkomponensek: {}", v["cpu"]["name"].as_str().unwrap_or("?")));
                self.hardware = Some(v);
                self.send_settings();
            }
            None => {
                log("rendszerkomponensek: nem kérdezhetők le (öt perc múlva újra megpróbáljuk)");
                self.hardware_retry = Instant::now() + Duration::from_secs(5 * 60);
            }
        }
    }

    /// Minden beállítás alapértékre. A programok (Ikonok), a Start menü kitűzött elemei, a
    /// naptárak, a jegyzet, a háttérkép, az időjárás helye és mértékegysége, és az indulás a géppel megmarad; a régi fájl
    /// `config.backup.json` néven a mellé kerül.
    pub(crate) fn reset_all(&mut self) {
        let backup = self.cfg_path.with_file_name("config.backup.json");
        if let Err(e) = std::fs::copy(&self.cfg_path, &backup) {
            log(format!("a config.json mentése nem sikerült: {e}"));
        }
        let old = std::mem::take(&mut self.cfg);
        let mut c = config::Config {
            apps: old.apps,
            calendar: old.calendar,
            note: old.note,
            wallpaper: old.wallpaper,
            start_with_windows: old.start_with_windows,
            ..config::Config::default()
        };
        c.start_menu.pinned = old.start_menu.pinned;
        c.weather.city = old.weather.city;
        c.weather.latitude = old.weather.latitude;
        c.weather.longitude = old.weather.longitude;
        c.weather.unit = old.weather.unit;
        c.finish();
        self.cfg = c;
        log("minden beállítás alaphelyzetbe állítva (mentés: config.backup.json)");
        if self.cfg.hide_taskbar != self.want_hidden_taskbar {
            let on = self.cfg.hide_taskbar;
            self.set_hide_taskbar(on);
        }
        self.reset_borders();
        self.save_config();
        self.health_restart();
        self.apply_config(None);
        self.push_theme();
        if let Some(s) = &self.settings {
            let _ = s.webview.load_url(&nav_url(&settings_url(&self.cfg)));
        }
        let t = self.tr(
            "Minden beállítás alaphelyzetben. A korábbi fájl: config.backup.json",
            "Everything is back to defaults. The previous file: config.backup.json",
            "Wszystkie ustawienia przywrócono do domyślnych. Poprzedni plik: config.backup.json",
            "Alles ist auf Standard zurückgesetzt. Die vorherige Datei: config.backup.json",
            "Todo ha vuelto a los valores predeterminados. El archivo anterior: config.backup.json",
        );
        self.toast(&t);
    }

    /// A Windows tálcájának elrejtése vagy visszaadása menet közben.
    pub(crate) fn set_hide_taskbar(&mut self, on: bool) {
        self.cfg.hide_taskbar = on;
        if on && self.orig_taskbar.is_none() {
            self.orig_taskbar = Some(appbar::taskbar_state());
            appbar::hide_taskbar();
        } else if !on && let Some(st) = self.orig_taskbar.take() {
            appbar::restore_taskbar(st);
        }
        self.want_hidden_taskbar = on;
        self.save_state();
        self.layout(true);
    }

    pub(crate) fn wallpaper_dir(&self) -> PathBuf {
        self.cfg_path.parent().map(|p| p.join("wallpapers")).unwrap_or_else(|| PathBuf::from("wallpapers"))
    }

    pub(crate) fn is_wallpaper_file(p: &Path) -> bool {
        p.is_file() && p.extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| sys::WALLPAPER_EXTS.contains(&e.as_str()))
    }

    /// A háttérképmappa képei, név szerint rendezve.
    fn wallpaper_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.wallpaper_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| Self::is_wallpaper_file(p))
            .collect();
        files.sort();
        files
    }

    /// A háttérképmappa képei a beállítások felületének.
    pub(crate) fn wallpaper_list(&self) -> Vec<serde_json::Value> {
        self.wallpaper_files()
            .iter()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                json!({
                    "path": s,
                    "url": file_url(&s),
                    "builtin": builtin_wallpaper(p),
                })
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
            let t = self.tr(
                "A kép nem található.",
                "Image not found.",
                "Nie znaleziono obrazu.",
                "Bild nicht gefunden.",
                "No se encontró la imagen.",
            );
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
            }
            Err(e) => {
                log(format!("háttérkép hiba: {e}"));
                let t = self.tr(
                    "A háttérkép nem állítható be",
                    "Couldn't set the wallpaper",
                    "Nie można ustawić tapety",
                    "Der Hintergrund konnte nicht festgelegt werden",
                    "No se pudo poner el fondo",
                );
                self.toast(&format!("{t}: {e}"));
            }
        }
        self.send_settings();
    }
}
