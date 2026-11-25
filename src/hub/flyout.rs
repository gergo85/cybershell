//! Az információs panel melletti oldalpanel: részletes időjárás, naptár és Események
//! (iCal-naptárak), valamint a Notesz szerkesztője.

use crate::UserEvent;
use crate::hub::Hub;
use crate::hub::layout::is_vertical;
use crate::hub::layout::popup_pos;
use crate::log;
use crate::send;
use crate::web::PopupParts;
use crate::web::push;
use crate::{appbar, config, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow,
};
use wry::WebView;

/// Az információs panel mellett kinyíló oldalpanel (időjárás, naptár, Események, Notesz).
pub(crate) struct FlyWin {
    /// Az ablak, amiben a felület fut: csak életben tartjuk (eldobásakor bezárulna).
    #[expect(dead_code, reason = "csak az ablak élettartama miatt tároljuk")]
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
    /// Mikor nyílt meg (a fókuszvesztést csak egy kis idő után figyeljük).
    pub(crate) shown_at: Instant,
    /// Megnyitás óta megkapta-e már a fókuszt (attól kezdve zárul be, ha máshova kattintanak).
    pub(crate) focused: bool,
    /// Mikor zárult be fókuszvesztés miatt (az időjárásra kattintás ne nyissa újra rögtön).
    pub(crate) hidden_at: Option<Instant>,
    /// Mit mutat: "weather", "calendar", "events" vagy "note", és melyik sáv mellett nyílt meg.
    pub(crate) kind: &'static str,
    pub(crate) bar: usize,
    /// A kérő szekció közepe a sáv mentén (fizikai pixel): a tartalomhoz igazított magasságnál
    /// (Események) ehhez képest marad a helyén.
    pub(crate) along: i32,
}

/// Az oldalpanel mérete CSS-pixelben.
pub(crate) const FLY_W: f64 = 380.0;
pub(crate) const FLY_H: f64 = 640.0;

impl FlyWin {
    pub(crate) fn new((window, webview, hwnd): PopupParts) -> Self {
        FlyWin {
            window,
            webview,
            hwnd,
            visible: false,
            shown_at: Instant::now(),
            focused: false,
            hidden_at: None,
            kind: "weather",
            bar: 0,
            along: 0,
        }
    }
}

/// A csatolt naptárak letöltése: a legutóbbi eredmény (ideje és a naptárak JSON-ja),
/// folyamatban van-e, és mikor frissítjük legközelebb az Események modulnak.
pub(crate) struct CalendarCache {
    pub(crate) cache: Option<(Instant, String)>,
    pub(crate) busy: bool,
    pub(crate) next: Instant,
}

impl CalendarCache {
    pub(crate) fn new(first: Instant) -> Self {
        CalendarCache { cache: None, busy: false, next: first }
    }
}

impl Hub {
    pub(crate) fn fly_eval(&self, payload: &serde_json::Value) {
        if let Some(f) = &self.fly {
            push(&f.webview, payload);
        }
    }

    pub(crate) fn fly_visible(&self) -> bool {
        self.fly.as_ref().is_some_and(|f| f.visible)
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
        // csak ezt az egy kattintást nyeljük el, a következő újra nyit
        if !f.visible && f.kind == kind && f.hidden_at.is_some_and(|t| t.elapsed() < Duration::from_millis(450)) {
            if let Some(f) = self.fly.as_mut() {
                f.hidden_at = None;
            }
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale, bar_mon)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale, b.mon)) else { return };
        // A teljes képernyős állapot beragadhat (pl. egy rövid ideig teljes képernyős ablak után
        // nem jön vissza a jelzés). Ha a panelre rá lehetett kattintani, az látszik, tehát az
        // oldalpanel is kinyílhat; a jelzést csak naplózzuk.
        if self.fullscreen.contains(&bar_mon) {
            log(format!("oldalpanel ({kind}): a képernyő teljes képernyősnek jelölt, de a panelre kattintottak, megnyílik"));
        }
        // nyitott beállítások: azok kiúsznak, az oldalpanel pedig megnyílik
        if self.settings_visible() {
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
        f.focused = false;
        f.hidden_at = None;
        f.kind = kind;
        f.bar = idx;
        f.along = (along * scale).round() as i32;
        unsafe {
            let _ = SetWindowPos(f.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_SHOWWINDOW);
            let _ = SetForegroundWindow(f.hwnd);
        }
        log(format!("oldalpanel megnyitva: {kind}"));
        let payload = if kind == "note" {
            json!({ "type": "note-open", "note": self.cfg.note, "theme": self.theme_json(), "language": self.cfg.language })
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
        self.eval_info(&json!({ "type": "fly-state", "open": false }));
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
        if let Some((at, data)) = &self.calendar.cache
            && !force
            && at.elapsed() < Duration::from_secs(10 * 60)
        {
            let payload = json!({ "type": "cal-data", "feeds": serde_json::from_str::<serde_json::Value>(data).unwrap_or_default() });
            self.cal_broadcast(&payload);
            return;
        }
        if self.cfg.calendar.feeds.is_empty() {
            self.calendar.cache = None;
            self.cal_broadcast(&json!({ "type": "cal-data", "feeds": [] }));
            return;
        }
        if self.calendar.busy {
            return;
        }
        self.calendar.busy = true;
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
        self.calendar.busy = false;
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
        self.calendar.cache = Some((Instant::now(), raw));
        self.cal_broadcast(&json!({ "type": "cal-data", "feeds": feeds }));
    }

    /// A letöltött naptárak az oldalpanelnek és (ha látszik az Események modul) az információs panelnek.
    fn cal_broadcast(&self, payload: &serde_json::Value) {
        self.fly_eval(payload);
        if self.cfg.sections.shows("events") {
            self.eval_info(payload);
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
                if url.contains("google.com") {
                    "Google Naptár".to_string()
                } else {
                    self.tr("Naptár", "Calendar", "Kalendarz", "Kalender", "Calendario")
                }
            });
        let color = COLORS[self.cfg.calendar.feeds.len() % COLORS.len()].to_string();
        log(format!("naptár csatolva: {name}"));
        self.cfg.calendar.feeds.push(config::CalendarFeed { name, url, color });
        self.save_config();
        self.calendar.cache = None;
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
            self.calendar.cache = None;
            self.send_calendar_feeds();
            self.calendar_fetch(true);
        }
    }

    /// Jegyzet: a szöveg, a teendők és a választott fül a config.json-ba kerül, és minden
    /// képernyő információs panelje megkapja; ha a változás nem a szerkesztőből jött (pl. a
    /// panelen pipáltak ki egy teendőt vagy fület váltottak), a nyitott szerkesztő is.
    pub(crate) fn note_update(&mut self, change: impl FnOnce(&mut crate::config::Note), from_editor: bool) {
        let mut n = self.cfg.note.clone();
        change(&mut n);
        n.normalize();
        if n == self.cfg.note {
            return;
        }
        self.cfg.note = n;
        self.save_config();
        self.eval_info(&json!({ "type": "note", "note": self.cfg.note }));
        if !from_editor && self.fly.as_ref().is_some_and(|f| f.visible && f.kind == "note") {
            self.fly_eval(&json!({ "type": "note-sync", "note": self.cfg.note }));
        }
    }

    /// Az Események listája a tartalmához igazítja a magasságát (kevés eseménynél ne maradjon
    /// üres hely alul): `css_h` a felület által mért magasság CSS-pixelben, legfeljebb a teljes.
    pub(crate) fn fly_fit(&mut self, css_h: f64) {
        let Some(f) = self.fly.as_ref().filter(|f| f.visible && f.kind == "events") else { return };
        let (bar, along) = (f.bar, f.along);
        let Some((bar_rect, bar_edge, bar_scale)) = self.bars.get(bar).map(|b| (b.rect, b.edge, b.scale)) else { return };
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);
        let w = (FLY_W * scale).round() as i32;
        let full = ((FLY_H * scale).round() as i32).min(work.bottom - work.top - 12);
        let h = ((css_h.max(160.0) * scale).ceil() as i32).min(full);
        let (left, top) = popup_pos(bar_rect, bar_edge, w, h, along, work);
        if let Some(f) = self.fly.as_ref() {
            unsafe {
                let _ = SetWindowPos(f.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_NOACTIVATE);
            }
        }
    }

    pub(crate) fn on_fly_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        match msg["type"].as_str().unwrap_or("") {
            "close" => self.fly_hide(false),
            "fly-fit" => self.fly_fit(msg["h"].as_f64().unwrap_or(FLY_H)),
            // Jegyzet: a szöveg a config.json-ba kerül, és minden képernyő információs panelje megkapja
            "note-set" => {
                let text: String = msg["text"].as_str().unwrap_or("").chars().take(2000).collect();
                self.note_update(|n| n.text = text, true);
            }
            // Teendők: a teljes lista (hozzáadás, pipálás, törlés, sorrend után)
            "tasks-set" => {
                let Ok(tasks) = serde_json::from_value::<Vec<crate::config::Task>>(msg["tasks"].clone()) else { return };
                self.note_update(|n| n.tasks = tasks, true);
            }
            "note-tab" => {
                let tab = msg["tab"].as_str().unwrap_or("note").to_string();
                self.note_update(|n| n.tab = tab, true);
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
