//! Egészség: szünetre figyelmeztető visszaszámlálás és a piros sáv a munkaterület tetején.

use crate::appbar;
use crate::hub::Hub;
use crate::log;
use crate::web::push;
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, MB_ICONEXCLAMATION, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
};
use wry::WebView;

/// Egészség: a munkaterület tetején, középen megjelenő piros sáv (szünetre figyelmeztet).
pub(crate) struct HealthWin {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
}

/// A figyelmeztető sáv mérete CSS-pixelben.
pub(crate) const HEALTH_W: f64 = 660.0;
pub(crate) const HEALTH_H: f64 = 66.0;

/// Ennyi tétlenség után (zárolt gép, szünet) a visszaszámlálás elölről indul.
pub(crate) const HEALTH_IDLE: Duration = Duration::from_secs(5 * 60);

impl Hub {
    pub(crate) fn health_json(&self) -> serde_json::Value {
        let showing = self.health.as_ref().is_some_and(|h| h.visible);
        let remaining = self.health_due.map(|d| d.saturating_duration_since(Instant::now()).as_secs());
        json!({ "enabled": self.cfg.health.enabled, "minutes": self.cfg.health.minutes, "remaining": remaining, "showing": showing })
    }

    /// Új kör: a sáv eltűnik, a visszaszámlálás elölről indul (kikapcsolva megáll).
    pub(crate) fn health_restart(&mut self) {
        self.health_due = self.cfg.health.enabled.then(|| Instant::now() + Duration::from_secs(self.cfg.health.minutes as u64 * 60));
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
        let work = appbar::work_area();
        let scale = self.scale.max(0.5);
        let minutes = self.cfg.health.minutes;
        let theme = self.theme_json();
        let Some(h) = self.health.as_mut() else { return };
        let w = ((HEALTH_W * scale).round() as i32).min(work.right - work.left - 24);
        let hh = (HEALTH_H * scale).round() as i32;
        let left = work.left + (work.right - work.left - w) / 2;
        h.visible = true;
        unsafe {
            let _ = SetWindowPos(h.hwnd, Some(HWND_TOPMOST), left, work.top, w, hh, SWP_SHOWWINDOW | SWP_NOACTIVATE);
        }
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
