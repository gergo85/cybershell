//! Megjelenés: a sötét (Cyberpunk 2077) és a világos (Ghost in the Shell) téma átadása a felületeknek.

use crate::hub::Hub;
use crate::web::push;
use serde_json::json;
use wry::WebView;

impl Hub {
    /// A most használt téma a felületeknek: színek, a séma neve és hogy látszanak-e a súgó
    /// szövegek (a title-ök).
    pub(crate) fn theme_json(&self) -> serde_json::Value {
        let t = self.cfg.active_theme();
        json!({ "main": t.main, "alt": t.alt, "hot": t.hot, "bg": t.bg, "text": t.text, "scheme": self.cfg.scheme, "tips": self.cfg.tooltips })
    }

    /// Téma- vagy sémaváltás: minden felület azonnal átszíneződik (újratöltés nélkül).
    pub(crate) fn push_theme(&mut self) {
        self.broadcast_theme();
        // a Windows és a Windows Terminal is vált (ha be van kapcsolva)
        self.sync_windows();
    }

    /// A téma (és a súgó állapota) minden felületnek, újratöltés nélkül.
    pub(crate) fn broadcast_theme(&self) {
        let payload = json!({ "type": "theme", "theme": self.theme_json() });
        let bg = self.cfg.bg_rgba();
        let mut views: Vec<&WebView> = self.bars.iter().map(|b| &b.webview).collect();
        views.extend(self.settings.as_ref().map(|w| &w.webview));
        views.extend(self.peek.as_ref().map(|w| &w.webview));
        views.extend(self.fly.as_ref().map(|w| &w.webview));
        views.extend(self.launcher.as_ref().map(|w| &w.webview));
        views.extend(self.health.as_ref().map(|w| &w.webview));
        views.extend(self.net.as_ref().map(|w| &w.webview));
        for v in views {
            let _ = v.set_background_color(bg);
            push(v, &payload);
        }
    }
}
