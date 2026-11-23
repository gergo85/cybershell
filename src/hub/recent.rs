//! A Mostanában modul programoldala (a felülete: ui/modules/recent.js): ugyanazok a programok,
//! mint a kereső „Gyakran használt” csoportjában. A listát a Start menü felülete számolja (a
//! használat gyakorisága és frissessége szerint, ui/parts/launcher.js: reportFrequent), és
//! változáskor elküldi; a program a Start menü azonosítóit a CyberShell adatmappájában, a
//! `recent-apps.json`-ban is megőrzi, hogy induláskor azonnal legyen mit mutatni.

use std::path::PathBuf;

use serde_json::json;

use crate::hub::Hub;
use crate::web::percent_encode;
use crate::wintrack::WinInfo;
use crate::{log, wintrack};

/// A panelnek küldött elemek száma (normál nézetben 8, kompaktban 4 látszik).
const SHOW: usize = 8;

fn file() -> PathBuf {
    crate::data_dir().join("recent-apps.json")
}

/// A megőrzött lista betöltése (induláskor).
pub(crate) fn load() -> Vec<String> {
    std::fs::read_to_string(file())
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .map(|mut v| {
            v.truncate(SHOW);
            v
        })
        .unwrap_or_default()
}

impl Hub {
    /// A Start menü felületétől kapott „Gyakran használt” lista (Start menü-azonosítók).
    pub(crate) fn on_frequent(&mut self, ids: Vec<String>) {
        let mut ids: Vec<String> = ids.into_iter().filter(|i| !i.is_empty()).collect();
        ids.dedup();
        ids.truncate(SHOW);
        if ids == self.recent_apps {
            return;
        }
        self.recent_apps = ids;
        if let Ok(t) = serde_json::to_string_pretty(&self.recent_apps)
            && let Err(e) = std::fs::write(file(), t)
        {
            log(format!("a gyakran használt programok listája nem menthető: {e}"));
        }
        self.send_recent(false);
    }

    /// A programok a panelnek (ha a modul látszik): azonosító, név, ikon. Csak változáskor.
    pub(crate) fn send_recent(&mut self, force: bool) {
        if !self.cfg.sections.shows("recent") {
            self.last_recent.clear();
            return;
        }
        // a programlista betöltése előtt nem tudjuk a neveket: addig nem küldünk
        if self.start_apps.is_empty() {
            return;
        }
        let items: Vec<serde_json::Value> = self
            .recent_apps
            .iter()
            .filter_map(|id| self.start_apps.iter().find(|a| &a.id == id))
            .map(|a| json!({ "key": a.id, "name": a.name, "icon": format!("/__appicon/{}", percent_encode(&a.id)) }))
            .collect();
        let payload = json!({ "type": "recent", "apps": items });
        let s = payload.to_string();
        if force || s != self.last_recent {
            self.last_recent = s;
            self.eval(self.info_edge(), &payload);
        }
    }

    /// Kattintás egy ikonra: ha az asztali program fut, az ablakai között vált (mint az
    /// Ikonok), különben elindul, mintha a Start menüből indították volna (ez a használati
    /// gyakoriságba is beszámít).
    pub(crate) fn recent_open(&mut self, id: &str) {
        if !self.recent_apps.iter().any(|r| r == id) {
            return;
        }
        let Some(app) = self.start_apps.iter().find(|a| a.id == id).cloned() else { return };
        self.launcher_eval(&json!({ "type": "launcher-use", "id": app.id }));
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        if !app.target.is_empty() {
            let mine: Vec<WinInfo> = wins.iter().filter(|w| w.path.eq_ignore_ascii_case(&app.target)).cloned().collect();
            if !mine.is_empty() {
                self.toggle_windows(&mine);
                return;
            }
        }
        let known: Vec<isize> = wins.iter().map(|w| w.hwnd.0 as isize).collect();
        if wintrack::shell_open(&format!("shell:AppsFolder\\{}", app.id), "", "", false) {
            self.settings_dismiss_for(None, &known);
        } else {
            log(format!("gyakran használt program nem indult: {}", app.name));
            let t = self.tr("Nem indítható", "Cannot start", "Nie można uruchomić");
            self.toast(&format!("{t}: {}", app.name));
        }
    }
}
