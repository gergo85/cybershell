//! Az információs panel élő adatai: hangerő, akkumulátor, zene (Spotify) és az aktivitás-jelzés.

use crate::hub::Hub;
use crate::log;
use crate::web::push;
use crate::{media, sys, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION;

impl Hub {
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

    /// Az akkumulátor állapota az információs panelnek (csak változáskor, vagy ha kérik).
    pub(crate) fn volume_ctl(&mut self) -> Option<&sys::Volume> {
        if self.volume.as_ref().is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(60)) {
            self.volume = sys::Volume::open().map(|v| (Instant::now(), v));
        }
        self.volume.as_ref().map(|(_, v)| v)
    }

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
        if force || s != self.last_volume {
            self.last_volume = s;
            self.eval(self.info_edge(), &payload);
        }
    }

    pub(crate) fn send_power(&mut self, force: bool) {
        let payload = match sys::battery() {
            Some((pct, charging)) => json!({ "type": "power", "laptop": true, "pct": pct, "charging": charging }),
            None => json!({ "type": "power", "laptop": false }),
        };
        let s = payload.to_string();
        if force || s != self.last_power {
            self.last_power = s;
            self.eval(self.info_edge(), &payload);
        }
    }

    pub(crate) fn on_media(&mut self, payload: String) {
        if self.last_media.as_deref() == Some(payload.as_str()) {
            return;
        }
        let info = self.info_edge();
        for b in self.bars.iter().filter(|b| b.edge == info) {
            push(&b.webview, &payload);
        }
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
                    let t = self.tr("A Spotify nincs telepítve.", "Spotify isn't installed.");
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
}
