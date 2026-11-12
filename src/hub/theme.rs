//! Megjelenés: a sötét (Cyberpunk 2077) és a világos (Ghost in the Shell) téma átadása a felületeknek.

use crate::hub::Hub;
use crate::web::push;
use serde_json::json;
use wry::WebView;

impl Hub {
    /// A most használt téma a felületeknek: színek és a séma neve.
    pub(crate) fn theme_json(&self) -> serde_json::Value {
        let t = self.cfg.active_theme();
        json!({ "main": t.main, "alt": t.alt, "hot": t.hot, "bg": t.bg, "text": t.text, "scheme": self.cfg.scheme })
    }

    /// Téma- vagy sémaváltás: minden felület azonnal átszíneződik (újratöltés nélkül).
    pub(crate) fn push_theme(&mut self) {
        let payload = json!({ "type": "theme", "theme": self.theme_json() });
        let bg = self.cfg.bg_rgba();
        let mut views: Vec<&WebView> = self.bars.iter().map(|b| &b.webview).collect();
        views.extend(self.settings.as_ref().map(|w| &w.webview));
        views.extend(self.peek.as_ref().map(|w| &w.webview));
        views.extend(self.fly.as_ref().map(|w| &w.webview));
        views.extend(self.launcher.as_ref().map(|w| &w.webview));
        views.extend(self.health.as_ref().map(|w| &w.webview));
        for v in views {
            let _ = v.set_background_color(bg);
            push(v, &payload);
        }
    }
}
