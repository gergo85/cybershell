//! Az információs panel melletti oldalpanel: részletes időjárás és naptár (iCal-naptárak).

use crate::UserEvent;
use crate::hub::Hub;
use crate::hub::layout::is_vertical;
use crate::hub::layout::popup_pos;
use crate::log;
use crate::send;
use crate::web::push;
use crate::{appbar, config, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SW_HIDE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow};
use wry::WebView;

/// Részletes időjárás: az információs panel mellett kinyíló oldalpanel.
pub(crate) struct FlyWin {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
    /// Mikor nyílt meg (a fókuszvesztést csak egy kis idő után figyeljük).
    pub(crate) shown_at: Instant,
    /// Mikor zárult be fókuszvesztés miatt (az időjárásra kattintás ne nyissa újra rögtön).
    pub(crate) hidden_at: Option<Instant>,
    /// Mit mutat: "weather", "calendar", "events" vagy "note", és melyik sáv mellett nyílt meg.
    pub(crate) kind: &'static str,
    pub(crate) bar: usize,
}

/// Az oldalpanel mérete CSS-pixelben.
pub(crate) const FLY_W: f64 = 380.0;
pub(crate) const FLY_H: f64 = 640.0;

impl Hub {
    pub(crate) fn fly_eval(&self, payload: &serde_json::Value) {
        if let Some(f) = &self.fly {
            push(&f.webview, payload);
        }
    }

    /// Az időjárásra vagy a dátumra kattintottak: a kérő panel mellett, a szekció magasságában
    /// kinyílik az oldalpanel. Ha ugyanaz már nyitva van, bezárul.
    pub(crate) fn fly_open(&mut self, idx: usize, kind: &'static str, msg: &serde_json::Value) {
        let Some(f) = self.fly.as_ref() else { return };
        if f.visible && f.kind == kind && f.bar == idx {
            self.fly_hide(false);
            return;
        }
        // épp most zárult be, mert erre a szekcióra kattintva elvesztette a fókuszt: ez zárás volt
        if !f.visible && f.kind == kind && f.hidden_at.is_some_and(|t| t.elapsed() < Duration::from_millis(450)) {
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale, bar_mon)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale, b.mon)) else { return };
        if self.fullscreen.contains(&bar_mon) {
            return;
        }
        // nyitott beállítások: azok kiúsznak, az oldalpanel pedig megnyílik
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_eval(&json!({ "type": "hide" }));
        }
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);
        let w = (FLY_W * scale).round() as i32;
        let h = ((FLY_H * scale).round() as i32).min(work.bottom - work.top - 12);
        // a szekció közepéhez igazodik (függőleges panelnél magasságban, vízszintesnél oldalirányban)
        let mid = |a: &str, b: &str| (msg[a].as_f64().unwrap_or(0.0) + msg[b].as_f64().unwrap_or(0.0)) / 2.0;
        let along = if is_vertical(bar_edge) { mid("top", "bottom") } else { mid("left", "right") };
        let (left, top) = popup_pos(bar_rect, bar_edge, w, h, (along * scale).round() as i32, work);
        self.peek_hide();
        self.launcher_hide(false);
        let Some(f) = self.fly.as_mut() else { return };
        f.visible = true;
        f.shown_at = Instant::now();
        f.hidden_at = None;
        f.kind = kind;
        f.bar = idx;
        unsafe {
            let _ = SetWindowPos(f.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_SHOWWINDOW);
            let _ = SetForegroundWindow(f.hwnd);
        }
        log(format!("oldalpanel megnyitva: {kind}"));
        let payload = if kind == "note" {
            json!({ "type": "note-open", "text": self.cfg.note.text, "theme": self.theme_json(), "language": self.cfg.language })
        } else if kind == "calendar" || kind == "events" {
            let t = if kind == "events" { "events-open" } else { "calendar-open" };
            json!({ "type": t, "feeds": self.calendar_feed_list(), "theme": self.theme_json(), "language": self.cfg.language })
        } else {
            json!({ "type": "weather-open", "weather": self.cfg.weather, "theme": self.theme_json(), "language": self.cfg.language })
        };
        self.fly_eval(&payload);
        self.eval_info_at(
            Some(idx),
            &json!({ "type": "fly-state", "kind": kind, "open": true }),
            &json!({ "type": "fly-state", "open": false }),
        );
    }

    pub(crate) fn fly_hide(&mut self, lost_focus: bool) {
        let Some(f) = self.fly.as_mut() else { return };
        if !f.visible {
            return;
        }
        f.visible = false;
        f.hidden_at = lost_focus.then(Instant::now);
        unsafe {
            let _ = ShowWindow(f.hwnd, SW_HIDE);
        }
        let hwnd = f.hwnd;
        self.eval(self.info_edge(), &json!({ "type": "fly-state", "open": false }));
        if !lost_focus {
            self.refocus_later(hwnd);
        }
    }

    /// A csatolt naptárak a felületnek: név és szín (a titkos címet nem küldjük ki).
    pub(crate) fn calendar_feed_list(&self) -> serde_json::Value {
        json!(self.cfg.calendar.feeds.iter().map(|f| json!({ "name": f.name, "color": f.color })).collect::<Vec<_>>())
    }

    pub(crate) fn send_calendar_feeds(&self) {
        self.fly_eval(&json!({ "type": "cal-feeds", "feeds": self.calendar_feed_list() }));
    }

    /// A naptárak letöltése háttérszálon (tíz percig a legutóbbit adjuk vissza).
    pub(crate) fn calendar_fetch(&mut self, force: bool) {
        if let Some((at, data)) = &self.cal_cache
            && !force
            && at.elapsed() < Duration::from_secs(10 * 60)
        {
            let payload = json!({ "type": "cal-data", "feeds": serde_json::from_str::<serde_json::Value>(data).unwrap_or_default() });
            self.cal_broadcast(&payload);
            return;
        }
        if self.cfg.calendar.feeds.is_empty() {
            self.cal_cache = None;
            self.cal_broadcast(&json!({ "type": "cal-data", "feeds": [] }));
            return;
        }
        if self.cal_busy {
            return;
        }
        self.cal_busy = true;
        let feeds: Vec<(String, String, String)> =
            self.cfg.calendar.feeds.iter().map(|f| (f.name.clone(), f.url.clone(), f.color.clone())).collect();
        std::thread::spawn(move || {
            let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).build().new_agent();
            let out: Vec<serde_json::Value> = feeds
                .iter()
                .map(|(name, url, color)| {
                    let res = agent.get(url).header("User-Agent", "CyberShell").call();
                    match res {
                        Ok(mut r) => match r.body_mut().with_config().limit(20 * 1024 * 1024).read_to_string() {
                            Ok(text) if text.contains("BEGIN:VCALENDAR") => {
                                json!({ "name": name, "color": color, "ok": true, "ics": text })
                            }
                            Ok(_) => json!({ "name": name, "color": color, "ok": false, "error": "not-ics" }),
                            Err(_) => json!({ "name": name, "color": color, "ok": false, "error": "read" }),
                        },
                        Err(ureq::Error::StatusCode(code)) => {
                            json!({ "name": name, "color": color, "ok": false, "error": format!("HTTP {code}") })
                        }
                        Err(_) => json!({ "name": name, "color": color, "ok": false, "error": "offline" }),
                    }
                })
                .collect();
            send(UserEvent::CalendarData(serde_json::Value::Array(out).to_string()));
        });
    }

    pub(crate) fn on_calendar_data(&mut self, raw: String) {
        self.cal_busy = false;
        let feeds: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        let summary: Vec<String> = feeds
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| {
                format!(
                    "{}: {}",
                    f["name"].as_str().unwrap_or("?"),
                    if f["ok"] == true { "ok" } else { f["error"].as_str().unwrap_or("hiba") }
                )
            })
            .collect();
        log(format!("naptár letöltve – {}", summary.join(", ")));
        self.cal_cache = Some((Instant::now(), raw));
        self.cal_broadcast(&json!({ "type": "cal-data", "feeds": feeds }));
    }

    /// A letöltött naptárak az oldalpanelnek és (ha látszik az Események modul) az információs panelnek.
    fn cal_broadcast(&self, payload: &serde_json::Value) {
        self.fly_eval(payload);
        if self.cfg.sections.shows("events") {
            self.eval(self.info_edge(), payload);
        }
    }

    /// Új naptár csatolása (Google Naptár: „Titkos cím iCal formátumban”).
    /// Hibánál a hiba kódja: "url" (nem iCal-cím) vagy "dup" (már csatolva van).
    pub(crate) fn calendar_add(&mut self, msg: &serde_json::Value) -> Result<(), &'static str> {
        let mut url = msg["url"].as_str().unwrap_or("").trim().to_string();
        if let Some(rest) = url.strip_prefix("webcal://") {
            url = format!("https://{rest}");
        }
        if !url.starts_with("https://") || url.len() > 2000 {
            return Err("url");
        }
        if self.cfg.calendar.feeds.iter().any(|f| f.url == url) {
            return Err("dup");
        }
        const COLORS: [&str; 6] = ["#00e1ff", "#f3e600", "#ff2a6d", "#7ee787", "#b26bff", "#ffb02e"];
        let name =
            msg["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(40).collect()).unwrap_or_else(|| {
                if url.contains("google.com") { "Google Naptár".to_string() } else { self.tr("Naptár", "Calendar", "Kalendarz") }
            });
        let color = COLORS[self.cfg.calendar.feeds.len() % COLORS.len()].to_string();
        log(format!("naptár csatolva: {name}"));
        self.cfg.calendar.feeds.push(config::CalendarFeed { name, url, color });
        self.save_config();
        self.cal_cache = None;
        self.send_calendar_feeds();
        self.calendar_fetch(true);
        Ok(())
    }

    /// Egy csatolt naptár eltávolítása (sorszám szerint).
    pub(crate) fn calendar_remove(&mut self, i: usize) {
        if i < self.cfg.calendar.feeds.len() {
            let f = self.cfg.calendar.feeds.remove(i);
            log(format!("naptár eltávolítva: {}", f.name));
            self.save_config();
            self.cal_cache = None;
            self.send_calendar_feeds();
            self.calendar_fetch(true);
        }
    }

    pub(crate) fn on_fly_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        match msg["type"].as_str().unwrap_or("") {
            "close" => self.fly_hide(false),
            // Jegyzet: a szöveg a config.json-ba kerül, és minden képernyő információs panelje megkapja
            "note-set" => {
                let text: String = msg["text"].as_str().unwrap_or("").chars().take(2000).collect();
                if text != self.cfg.note.text {
                    self.cfg.note.text = text;
                    self.save_config();
                    self.eval(self.info_edge(), &json!({ "type": "note", "text": self.cfg.note.text }));
                }
            }
            "cal-fetch" => self.calendar_fetch(msg["force"].as_bool().unwrap_or(false)),
            "cal-add" => {
                if let Err(e) = self.calendar_add(&msg) {
                    self.fly_eval(&json!({ "type": "cal-error", "error": e }));
                }
            }
            "cal-remove" => self.calendar_remove(msg["i"].as_u64().unwrap_or(u64::MAX) as usize),
            // a Google Naptár beállításai (itt található a titkos iCal-cím)
            "cal-help" => {
                wintrack::shell_open("https://calendar.google.com/calendar/r/settings", "", "", false);
                self.fly_hide(false);
            }
            "log" => log(format!("[időjárás] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }
}
