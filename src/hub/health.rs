//! Egészség: szünetre figyelmeztető visszaszámlálás és a piros sáv a munkaterület tetején.

use crate::appbar;
use crate::hub::Hub;
use crate::log;
use crate::web::PopupParts;
use crate::web::{cut_bottom_corners, push};
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, MB_ICONEXCLAMATION, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
};
use wry::WebView;

/// A munkaterület tetején, középen megjelenő piros figyelmeztető sáv: a szünetre figyelmeztető
/// (Egészség) és a „Megszakadt a kapcsolat” sáv (net.rs) is ilyen.
pub(crate) struct BannerWin {
    /// Az ablak, amiben a felület fut: csak életben tartjuk (eldobásakor bezárulna).
    #[expect(dead_code, reason = "csak az ablak élettartama miatt tároljuk")]
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
}

/// A figyelmeztető sáv mérete CSS-pixelben.
pub(crate) const BANNER_W: f64 = 660.0;
pub(crate) const BANNER_H: f64 = 66.0;
/// A sáv alsó sarkainak levágása CSS-pixelben (a health.css `.hb` clip-path értéke).
const BANNER_CUT: f64 = 12.0;

/// A figyelmeztető sáv megjelenítése a munkaterület tetején, középen (aktiválás nélkül, minden
/// más fölött), levágott alsó sarkokkal. `below`: a másik (épp látható) sáv alá kerül.
pub(crate) fn show_banner(hwnd: HWND, scale: f64, below: bool) {
    let work = appbar::work_area();
    let w = ((BANNER_W * scale).round() as i32).min(work.right - work.left - 24);
    let h = (BANNER_H * scale).round() as i32;
    let left = work.left + (work.right - work.left - w) / 2;
    let top = work.top + if below { h + (8.0 * scale) as i32 } else { 0 };
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_SHOWWINDOW | SWP_NOACTIVATE);
    }
    cut_bottom_corners(hwnd, w, h, (BANNER_CUT * scale).round() as i32);
}

/// Ennyi tétlenség után (zárolt gép, szünet) a visszaszámlálás elölről indul.
pub(crate) const HEALTH_IDLE: Duration = Duration::from_secs(5 * 60);

impl BannerWin {
    pub(crate) fn new((window, webview, hwnd): PopupParts) -> Self {
        BannerWin { window, webview, hwnd, visible: false }
    }
}

/// Az Egészség visszaszámlálása: mikor jár le (kikapcsolva: nincs), és mikor nézzük meg
/// legközelebb, volt-e hosszabb tétlenség.
pub(crate) struct HealthTimer {
    pub(crate) due: Option<Instant>,
    pub(crate) next_idle_check: Instant,
}

impl HealthTimer {
    pub(crate) fn new(first_check: Instant) -> Self {
        HealthTimer { due: None, next_idle_check: first_check }
    }
}

impl Hub {
    /// Látszik-e most a szünetre figyelmeztető sáv.
    pub(crate) fn health_visible(&self) -> bool {
        self.health.as_ref().is_some_and(|h| h.visible)
    }

    pub(crate) fn health_json(&self) -> serde_json::Value {
        let showing = self.health_visible();
        let remaining = self.health_timer.due.map(|d| d.saturating_duration_since(Instant::now()).as_secs());
        json!({ "enabled": self.cfg.health.enabled, "minutes": self.cfg.health.minutes, "remaining": remaining, "showing": showing })
    }

    /// Új kör: a sáv eltűnik, a visszaszámlálás elölről indul (kikapcsolva megáll).
    pub(crate) fn health_restart(&mut self) {
        self.health_timer.due = self.cfg.health.enabled.then(|| Instant::now() + Duration::from_secs(self.cfg.health.minutes as u64 * 60));
        if let Some(h) = self.health.as_mut()
            && h.visible
        {
            h.visible = false;
            unsafe {
                let _ = ShowWindow(h.hwnd, SW_HIDE);
            }
        }
    }

    /// Lejárt: a munkaterület tetején, középen megjelenik a piros sáv.
    pub(crate) fn health_show(&mut self) {
        let scale = self.scale.max(0.5);
        let minutes = self.cfg.health.minutes;
        let theme = self.theme_json();
        let Some(h) = self.health.as_mut() else { return };
        h.visible = true;
        show_banner(h.hwnd, scale, false);
        push(&h.webview, json!({ "type": "health-show", "minutes": minutes, "theme": theme }));
        log(format!("egészség: {minutes} perc letelt, szünet"));
        if self.cfg.activity.sound {
            unsafe {
                let _ = MessageBeep(MB_ICONEXCLAMATION);
            }
        }
        self.send_settings();
    }

    pub(crate) fn on_health_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        match msg["type"].as_str().unwrap_or("") {
            // a Bezárás gomb: új kör kezdődik
            "close" => {
                log("egészség: szünet nyugtázva, új kör");
                self.health_restart();
                self.send_settings();
            }
            "log" => log(format!("[szünet] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }
}
