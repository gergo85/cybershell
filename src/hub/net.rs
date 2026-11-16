//! Internetkapcsolat: ha megszakad, a munkaterület tetején, középen piros sáv jelenik meg
//! („Megszakadt a kapcsolat”) a hálózati beállításokra vivő gombbal; visszatéréskor eltűnik.
//! Itt van a Codex-használat frissítése is (a panel Codex szekciójának).

use crate::hub::Hub;
use crate::hub::health::{HEALTH_H, HEALTH_W};
use crate::web::push;
use crate::{appbar, codex, log, sys, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use windows::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow};

/// Ennyi egymást követő „nincs internet” mérés után jelzünk (3 mp-enként mérünk), hogy egy
/// pillanatnyi kimaradás (pl. Wi-Fi-váltás) ne villantsa fel a sávot.
const OFFLINE_AFTER: u8 = 2;

impl Hub {
    /// Időzített mérés a fő ciklusból.
    pub(crate) fn net_check(&mut self) {
        let Some(ok) = sys::internet_ok() else { return };
        if ok {
            self.net_misses = 0;
            if self.net_offline {
                self.net_offline = false;
                self.net_dismissed = false;
                log("internetkapcsolat helyreállt");
                self.net_hide();
            }
            return;
        }
        self.net_misses = self.net_misses.saturating_add(1);
        if self.net_misses >= OFFLINE_AFTER && !self.net_offline {
            self.net_offline = true;
            log("internetkapcsolat megszakadt");
            self.net_show();
        }
    }

    /// A piros sáv megjelenítése (ha épp a szünet-sáv is látszik, alá kerül).
    pub(crate) fn net_show(&mut self) {
        if self.net_dismissed {
            return;
        }
        let work = appbar::work_area();
        let scale = self.scale.max(0.5);
        let theme = self.theme_json();
        let below_health = self.health.as_ref().is_some_and(|h| h.visible);
        let Some(n) = self.net.as_mut() else { return };
        let w = ((HEALTH_W * scale).round() as i32).min(work.right - work.left - 24);
        let h = (HEALTH_H * scale).round() as i32;
        let left = work.left + (work.right - work.left - w) / 2;
        let top = work.top + if below_health { h + (8.0 * scale) as i32 } else { 0 };
        n.visible = true;
        unsafe {
            let _ = SetWindowPos(n.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_SHOWWINDOW | SWP_NOACTIVATE);
        }
        push(&n.webview, json!({ "type": "net-show", "theme": theme }));
    }

    pub(crate) fn net_hide(&mut self) {
        if let Some(n) = self.net.as_mut()
            && n.visible
        {
            n.visible = false;
            unsafe {
                let _ = ShowWindow(n.hwnd, SW_HIDE);
            }
        }
    }

    pub(crate) fn on_net_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        match msg["type"].as_str().unwrap_or("") {
            // a Windows hálózati beállításai (a sáv látva marad, amíg nincs kapcsolat)
            "net-open" => {
                self.settings_dismiss();
                wintrack::shell_open("ms-settings:network-status", "", "", false);
            }
            // a sáv elrejtése a következő kapcsolatváltásig
            "close" => {
                self.net_dismissed = true;
                self.net_hide();
            }
            "log" => log(format!("[kapcsolat] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    /// A Codex-használat újraolvasása és elküldése az információs panelnek (ha változott).
    pub(crate) fn codex_send(&mut self, force: bool) {
        self.next_codex = Instant::now() + Duration::from_secs(60);
        let payload = codex::read().to_string();
        if force || self.last_codex.as_deref() != Some(payload.as_str()) {
            self.eval(self.info_edge(), &serde_json::from_str(&payload).unwrap_or_default());
            self.last_codex = Some(payload);
        }
    }
}
