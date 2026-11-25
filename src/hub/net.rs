//! Internetkapcsolat: ha megszakad, a munkaterület tetején, középen piros sáv jelenik meg
//! („Megszakadt a kapcsolat”) a hálózati beállításokra vivő gombbal. Addig látszik, amíg vissza
//! nem jön a kapcsolat, vagy be nem zárják: máshova kattintva sem tűnik el (3 mp-enként újra
//! előre kerül).

use crate::hub::Hub;
use crate::hub::health::show_banner;
use crate::web::push;
use crate::{log, sys, wintrack};
use serde_json::json;
use std::time::Instant;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
};

/// Ennyi egymást követő „nincs internet” mérés után jelzünk (3 mp-enként mérünk), hogy egy
/// pillanatnyi kimaradás (pl. Wi-Fi-váltás) ne villantsa fel a sávot.
const OFFLINE_AFTER: u8 = 2;
/// Ennyi egymást követő sikeres mérés után tűnik el a sáv (egy téves „van internet” jelzés miatt ne).
const ONLINE_AFTER: u8 = 2;

/// A kapcsolatfigyelés állapota.
pub(crate) struct NetWatch {
    /// Az egymást követő sikertelen, illetve sikeres mérések száma.
    pub(crate) misses: u8,
    pub(crate) oks: u8,
    /// Épp nincs kapcsolat (a sáv látszik, ha nem zárták be).
    pub(crate) offline: bool,
    /// A sávot bezárták: a következő kapcsolatváltásig rejtve marad.
    pub(crate) dismissed: bool,
    /// A következő mérés ideje.
    pub(crate) next_check: Instant,
}

impl NetWatch {
    pub(crate) fn new(first: Instant) -> Self {
        NetWatch { misses: 0, oks: 0, offline: false, dismissed: false, next_check: first }
    }
}

impl Hub {
    /// Időzített mérés a fő ciklusból.
    pub(crate) fn net_check(&mut self) {
        let Some(ok) = sys::internet_ok() else { return };
        if ok {
            self.net_watch.misses = 0;
            self.net_watch.oks = self.net_watch.oks.saturating_add(1);
            if self.net_watch.offline && self.net_watch.oks >= ONLINE_AFTER {
                self.net_watch.offline = false;
                self.net_watch.dismissed = false;
                log("internetkapcsolat helyreállt");
                self.net_hide();
            } else if self.net_watch.offline {
                self.net_keep();
            }
            return;
        }
        self.net_watch.oks = 0;
        self.net_watch.misses = self.net_watch.misses.saturating_add(1);
        if self.net_watch.misses >= OFFLINE_AFTER && !self.net_watch.offline {
            self.net_watch.offline = true;
            log("internetkapcsolat megszakadt");
            self.net_show();
        } else if self.net_watch.offline {
            self.net_keep();
        }
    }

    /// Amíg nincs kapcsolat és nem zárták be: a sáv látható és minden más fölött marad (ha
    /// közben valami eltakarta vagy elrejtette, visszajön), aktiválás és újraanimálás nélkül.
    fn net_keep(&mut self) {
        if self.net_watch.dismissed {
            return;
        }
        let Some(n) = self.net.as_mut() else { return };
        n.visible = true;
        unsafe {
            let _ = SetWindowPos(n.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_SHOWWINDOW | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
        }
    }

    /// A piros sáv megjelenítése (ha épp a szünet-sáv is látszik, alá kerül).
    pub(crate) fn net_show(&mut self) {
        if self.net_watch.dismissed {
            return;
        }
        let scale = self.scale.max(0.5);
        let theme = self.theme_json();
        let below_health = self.health_visible();
        let Some(n) = self.net.as_mut() else { return };
        n.visible = true;
        show_banner(n.hwnd, scale, below_health);
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
                self.net_watch.dismissed = true;
                self.net_hide();
            }
            "log" => log(format!("[kapcsolat] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }
}
