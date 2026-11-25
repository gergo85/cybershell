//! Az információs panel moduljainak programoldala (a felületük: ui/modules/*.js): a modulok
//! üzenetei (Eszköztár, laptop, hangerő, zene, Főkapcsoló, frissítés, oldalpanel) és az élő
//! adataik (hangerő, akkumulátor, zene, Codex), valamint az aktivitás-jelzés.

use crate::hub::Hub;
use crate::log;
use crate::web::push;
use crate::{UserEvent, apps, codex, media, monitor, periph, send, sys, tray, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION;

/// Egy rendszeresen frissülő panel-adat (akkumulátor, hangerő, Háttérprogramok, Codex,
/// Perifériák): a felületnek legutóbb küldött üzenet (üres: még nincs; újratöltés után ezt
/// küldjük vissza), a következő frissítés ideje, és hogy épp fut-e a háttérszálas lekérdezés
/// (a fő szálon mért adatoknál mindig hamis).
pub(crate) struct Feed {
    pub(crate) sent: String,
    pub(crate) next: Instant,
    pub(crate) busy: bool,
}

impl Feed {
    pub(crate) fn new(first: Instant) -> Self {
        Feed { sent: String::new(), next: first, busy: false }
    }
}

impl Hub {
    /// A panel moduljainak üzenetei. Igazat ad, ha az üzenet ide tartozott.
    pub(crate) fn on_panel_ipc(&mut self, idx: usize, kind: &str, msg: &serde_json::Value) -> bool {
        match (kind, msg["name"].as_str().unwrap_or("")) {
            // Eszköztár: Számológép, Jegyzettömb, Képmetsző (a Keresés a "search" parancs)
            ("cmd", "tool") => {
                self.settings_dismiss();
                let target = match msg["tool"].as_str().unwrap_or("") {
                    "calc" => "calc.exe",
                    "notepad" => "notepad.exe",
                    "snip" => r"shell:AppsFolder\Microsoft.ScreenSketch_8wekyb3d8bbwe!App",
                    _ => "",
                };
                if !target.is_empty() && !wintrack::shell_open(target, "", "", false) {
                    log(format!("eszköz nem indult: {target}"));
                }
            }
            // laptop: a Windows saját Wi-Fi-listája, az akkumulátor, a hálózat és a Bluetooth beállításai
            ("cmd", name @ ("wifi" | "battery" | "network" | "bluetooth")) => {
                let uri = match name {
                    "wifi" => "ms-availablenetworks:",
                    "battery" => "ms-settings:batterysaver",
                    "network" => "ms-settings:network-status",
                    _ => "ms-settings:bluetooth",
                };
                self.settings_dismiss();
                wintrack::shell_open(uri, "", "", false);
            }
            // Háttérprogramok: a program ablaka vagy elindítása
            ("cmd", "tray") => {
                let path = msg["path"].as_str().unwrap_or("").to_string();
                self.tray_open(&path);
            }
            // Mostanában: egy gyakran használt program ablaka vagy elindítása
            ("cmd", "recent") => {
                let key = msg["key"].as_str().unwrap_or("").to_string();
                self.recent_open(&key);
            }
            // Terhelés: a Feladatkezelő
            ("cmd", "taskmgr") => {
                self.settings_dismiss();
                wintrack::shell_open("taskmgr.exe", "", "", false);
            }
            // Főkapcsoló: zárolás, alvás, leállítás, újraindítás
            ("cmd", "power") => {
                let action = msg["action"].as_str().unwrap_or("").to_string();
                self.power(&action);
            }
            // Időjárás és Dátum: a panel mellé nyíló részletes előrejelzés, illetve naptár
            ("weather-open", _) => self.fly_open(idx, "weather", msg),
            ("calendar-open", _) => self.fly_open(idx, "calendar", msg),
            // Események: a következő hét eseményei a panel mellett
            ("events-open", _) => self.fly_open(idx, "events", msg),
            // Jegyzet: a panel mellett nagyban nyílik meg szerkesztésre
            ("note-open", _) => self.fly_open(idx, "note", msg),
            // Jegyzet: egy teendő kipipálása (vagy vissza) a panelen
            ("task-toggle", _) => {
                let i = msg["i"].as_u64().unwrap_or(u64::MAX) as usize;
                self.note_update(
                    |n| {
                        if let Some(t) = n.tasks.get_mut(i) {
                            t.done = !t.done;
                        }
                    },
                    false,
                );
            }
            // Hangerő: a csúszka, a görgetés és a némítógomb; a százalék a hangerőkeverőt nyitja
            ("volume-set", _) => {
                let pct = msg["pct"].as_f64().unwrap_or(0.0).round().clamp(0.0, 100.0) as u8;
                if let Some(v) = self.volume_ctl() {
                    v.set(pct);
                }
                self.send_volume(true);
            }
            ("volume-mute", _) => {
                let on = msg["on"].as_bool().unwrap_or(false);
                if let Some(v) = self.volume_ctl() {
                    v.set_mute(on);
                }
                self.send_volume(true);
            }
            ("volume-open", _) => {
                self.settings_dismiss();
                wintrack::shell_open("ms-settings:apps-volume", "", "", false);
            }
            // Zene: a Spotify vezérlése
            ("media", _) => self.media_cmd(msg),
            // Aktivitás: lejárt (vagy nyugtázták) a visszaszámlálás
            ("alert", _) => self.on_alert(msg["on"].as_bool().unwrap_or(false)),
            // Claude és Codex: frissítés gomb
            ("usage-refresh", _) => {
                if let Some(tx) = &self.usage_wake {
                    let _ = tx.send(());
                }
                self.claude_fetch();
            }
            ("codex-refresh", _) => self.codex_send(true),
            _ => return false,
        }
        true
    }

    /// Az aktivitásjelző lejárt (vagy nyugtázták): a többi sáv is jelezzen.
    pub(crate) fn on_alert(&mut self, on: bool) {
        log(format!("aktivitás-figyelmeztetés: {}", if on { "be" } else { "ki" }));
        let payload = json!({ "type": "alert", "on": on });
        for b in self.bars.iter().filter(|b| b.part != "right") {
            push(&b.webview, &payload);
        }
        if on && self.cfg.activity.sound {
            unsafe {
                let _ = MessageBeep(MB_ICONEXCLAMATION);
            }
        }
    }

    /// A hangerő-vezérlő (percenként újranyitjuk, hogy az új alapértelmezett eszközt kövesse).
    pub(crate) fn volume_ctl(&mut self) -> Option<&sys::Volume> {
        if self.volume.as_ref().is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(60)) {
            self.volume = sys::Volume::open().map(|v| (Instant::now(), v));
        }
        self.volume.as_ref().map(|(_, v)| v)
    }

    /// A hangerő és a némítás a panelnek (csak változáskor, vagy ha kérik).
    pub(crate) fn send_volume(&mut self, force: bool) {
        let state = self.volume_ctl().and_then(|v| v.get());
        if state.is_none() {
            // az eszköz megszűnhetett: legközelebb újranyitjuk
            self.volume = None;
        }
        let payload = match state {
            Some((pct, muted)) => json!({ "type": "volume", "ok": true, "pct": pct, "muted": muted }),
            None => json!({ "type": "volume", "ok": false }),
        };
        let s = payload.to_string();
        if force || s != self.volume_feed.sent {
            self.volume_feed.sent = s;
            self.eval_info(&payload);
        }
    }

    /// Az akkumulátor állapota a panelnek (csak változáskor, vagy ha kérik); asztali gépen a
    /// laptop modul ebből tudja, hogy rejtve maradjon.
    pub(crate) fn send_power(&mut self, force: bool) {
        let payload = match sys::battery() {
            Some((pct, charging)) => json!({ "type": "power", "laptop": true, "pct": pct, "charging": charging }),
            None => json!({ "type": "power", "laptop": false }),
        };
        let s = payload.to_string();
        if force || s != self.power_feed.sent {
            self.power_feed.sent = s;
            self.eval_info(&payload);
        }
    }

    pub(crate) fn on_media(&mut self, payload: String) {
        if self.last_media.as_deref() == Some(payload.as_str()) {
            return;
        }
        self.eval_info(&payload);
        self.last_media = Some(payload);
    }

    pub(crate) fn media_cmd(&mut self, msg: &serde_json::Value) {
        let cmd = match msg["action"].as_str().unwrap_or("") {
            "toggle" => media::Cmd::Toggle,
            "next" => media::Cmd::Next,
            "prev" => media::Cmd::Prev,
            "seek" => media::Cmd::Seek(msg["pos"].as_f64().unwrap_or(0.0)),
            "shuffle" => media::Cmd::Shuffle,
            // a Spotify ablaka (ha nem fut, elindul)
            "open" => {
                self.settings_dismiss();
                if !wintrack::shell_open("spotify:", "", "", false) {
                    let t = self.tr(
                        "A Spotify nincs telepítve.",
                        "Spotify isn't installed.",
                        "Spotify nie jest zainstalowany.",
                        "Spotify ist nicht installiert.",
                        "Spotify no está instalado.",
                    );
                    self.toast(&t);
                }
                return;
            }
            _ => return,
        };
        if let Some(tx) = &self.media_tx {
            let _ = tx.send(cmd);
        }
    }

    /// A háttérben futó programok (értesítési terület) frissítése, ha a modul látszik. A
    /// felsorolás (registry és folyamatlista) háttérszálon fut, az eredményt az `on_tray` küldi
    /// el. `force`: a legutóbbi listát azonnal újra elküldjük (pl. a panel újratöltése után).
    pub(crate) fn send_tray(&mut self, force: bool) {
        if !self.cfg.sections.shows("tray") {
            self.tray_feed.sent.clear();
            return;
        }
        if force && !self.tray_feed.sent.is_empty() {
            self.eval_info(&self.tray_feed.sent);
        }
        if self.tray_feed.busy {
            return;
        }
        self.tray_feed.busy = true;
        std::thread::spawn(|| send(UserEvent::Tray(tray::list())));
    }

    /// A háttérprogramok a panelnek: név, ikon (a Windows által megjegyzett kép, ha nincs, a
    /// program saját ikonja) és útvonal. Elöl a Windows tálcájára kitűzöttek, utánuk név szerint.
    /// Csak változáskor küldjük.
    pub(crate) fn on_tray(&mut self, list: Vec<tray::TrayApp>) {
        self.tray_feed.busy = false;
        if !self.cfg.sections.shows("tray") {
            return;
        }
        let mut items: Vec<(bool, String, serde_json::Value)> = list
            .into_iter()
            .map(|a| {
                let name = self.program_name(&a.path);
                let icon = match a.icon {
                    Some(v) => format!("/__tray/{}?v={v:x}", a.id),
                    None => self.program_icon(&a.path),
                };
                let item = json!({ "path": a.path, "name": name, "icon": icon });
                (a.promoted, name.to_lowercase(), item)
            })
            .collect();
        items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let payload = json!({ "type": "tray", "apps": items.into_iter().map(|(_, _, v)| v).collect::<Vec<_>>() });
        let s = payload.to_string();
        if s != self.tray_feed.sent {
            self.eval_info(&s);
            self.tray_feed.sent = s;
        }
    }

    /// Kattintás egy háttérprogramra: ha van ablaka, előhozza (mint a programikonok), különben
    /// elindítja; az egypéldányos programok ilyenkor a meglévő ablakukat nyitják meg.
    fn tray_open(&mut self, path: &str) {
        if path.is_empty() || !tray::list().iter().any(|a| a.path.eq_ignore_ascii_case(path)) {
            return;
        }
        let own = self.own_hwnds();
        let mine: Vec<_> = self.tracker.snapshot(&own).into_iter().filter(|w| w.path.eq_ignore_ascii_case(path)).collect();
        if !mine.is_empty() {
            self.toggle_windows(&mine);
            return;
        }
        self.settings_dismiss();
        // Store-appot a csomagján át (a WindowsApps mappából közvetlenül nem indítható)
        let target = apps::app_for_path(&self.start_apps, path)
            .filter(|a| a.target.is_empty())
            .map_or_else(|| path.to_string(), |a| format!(r"shell:AppsFolder\{}", a.id));
        if !wintrack::shell_open(&target, "", "", false) {
            log(format!("háttérprogram nem nyílt meg: {target}"));
            let t = self.tr(
                "A program nem nyitható meg.",
                "The program could not be opened.",
                "Nie można otworzyć programu.",
                "Das Programm konnte nicht geöffnet werden.",
                "No se pudo abrir el programa.",
            );
            self.toast(&t);
        }
    }

    /// A rendszerfigyelő csak a látható modulokhoz mér (Terhelés, Hálózat).
    pub(crate) fn configure_monitor(&self) {
        monitor::configure(self.cfg.sections.shows("load"), self.cfg.sections.shows("network"), &self.cfg.network.ping_host);
    }

    /// A rendszerfigyelő mérése (load vagy net üzenet) a panelnek.
    pub(crate) fn on_monitor(&mut self, payload: &str) {
        self.eval_info(&payload);
    }

    /// A Bluetooth-eszközök töltöttségének lekérdezése háttérszálon (az eszközkezelő lassú is lehet).
    pub(crate) fn periph_poll(&mut self) {
        if self.periph_feed.busy {
            return;
        }
        self.periph_feed.busy = true;
        std::thread::spawn(|| {
            let devices: Vec<serde_json::Value> =
                periph::list().into_iter().map(|(name, pct)| json!({ "name": name, "pct": pct })).collect();
            send(UserEvent::Periph(json!({ "type": "periph", "devices": devices }).to_string()));
        });
    }

    /// A Perifériák modul adata a panelnek (csak változáskor).
    pub(crate) fn on_periph(&mut self, payload: String) {
        self.periph_feed.busy = false;
        if payload == self.periph_feed.sent {
            return;
        }
        log(format!("perifériák: {payload}"));
        self.eval_info(&payload);
        self.periph_feed.sent = payload;
    }

    /// A Codex-használat újraolvasása háttérszálon (a naplók végét olvassa, ez lassú lehet); az
    /// eredményt az `on_codex` küldi el. `force`: akkor is elküldjük, ha nem változott.
    pub(crate) fn codex_send(&mut self, force: bool) {
        self.codex_feed.next = Instant::now() + Duration::from_secs(60);
        if self.codex_feed.busy {
            return;
        }
        self.codex_feed.busy = true;
        std::thread::spawn(move || send(UserEvent::Codex(codex::read().to_string(), force)));
    }

    /// A Codex-használat a panelnek (ha változott, vagy ha kérték).
    pub(crate) fn on_codex(&mut self, payload: String, force: bool) {
        self.codex_feed.busy = false;
        if force || self.codex_feed.sent != payload {
            self.eval_info(&payload);
            self.codex_feed.sent = payload;
        }
    }
}
