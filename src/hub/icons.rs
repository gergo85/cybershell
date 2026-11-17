//! Az Ikonok viselkedése: indítás, váltás egy program ablakai között, a nem kitűzött futó
//! programok és a fájlhúzás.

use crate::hub::Hub;
use crate::hub::Pending;
use crate::hub::extra_key;
use crate::log;
use crate::wintrack::WinInfo;
use crate::{config, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};

impl Hub {
    /// Egy program ablakai közti váltás, ahogy a tálca is csinálja: ha nem aktív,
    /// előhozza; ha aktív és egy ablaka van, kis méretre teszi; ha több, a következőre vált.
    /// Nyitott beállítások mellett csak akkor zárja be azokat, ha az előhozott ablak ugyanazon
    /// a képernyőn van (kis méretre tételnél nyitva maradnak).
    pub(crate) fn toggle_windows(&mut self, mine: &[WinInfo]) {
        if mine.is_empty() {
            return;
        }
        let active = mine.iter().find(|w| Some(w.hwnd) == self.last_fg && !w.iconic);
        let shown = match active {
            Some(cur) if mine.len() > 1 => {
                let next = mine.iter().rev().find(|w| w.hwnd != cur.hwnd).unwrap();
                wintrack::activate(next.hwnd);
                self.last_fg = Some(next.hwnd);
                Some(next.hwnd)
            }
            Some(cur) => {
                wintrack::minimize(cur.hwnd);
                self.last_fg = None;
                None
            }
            None => {
                wintrack::activate(mine[0].hwnd);
                self.last_fg = Some(mine[0].hwnd);
                Some(mine[0].hwnd)
            }
        };
        if let Some(h) = shown {
            self.settings_dismiss_for(Some(h), &[]);
        }
        self.last_state.clear();
        self.poll();
    }

    /// A felülettől kapott cél (kitűzött app sorszáma vagy futó program kulcsa) ablakai.
    pub(crate) fn target_windows(&mut self, msg: &serde_json::Value) -> (Vec<WinInfo>, Option<String>) {
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        if let Some(id) = msg["id"].as_u64().map(|i| i as usize) {
            let Some(m) = self.matchers.get(id) else { return (Vec::new(), None) };
            let mine = wins.iter().filter(|w| m.matches(w)).cloned().collect();
            let exe = self.cfg.apps.get(id).map(|a| a.resolved_path()).filter(|p| p.to_lowercase().ends_with(".exe"));
            return (mine, exe);
        }
        if let Some(key) = msg["key"].as_str() {
            let mine: Vec<WinInfo> = wins.iter().filter(|w| extra_key(w).as_deref() == Some(key)).cloned().collect();
            let exe = mine.first().map(|w| w.path.clone()).filter(|p| p.to_lowercase().ends_with(".exe"));
            return (mine, exe);
        }
        (Vec::new(), None)
    }

    pub(crate) fn launch_extra(&mut self, msg: &serde_json::Value) {
        self.peek_hide();
        let new_instance = msg["newInstance"].as_bool().unwrap_or(false);
        let (mine, exe) = self.target_windows(msg);
        if new_instance {
            if let Some(exe) = exe {
                let own = self.own_hwnds();
                let known: Vec<isize> = self.tracker.snapshot(&own).iter().map(|w| w.hwnd.0 as isize).collect();
                if wintrack::shell_open(&exe, "", "", false) {
                    self.settings_dismiss_for(None, &known);
                }
            }
            return;
        }
        self.toggle_windows(&mine);
    }

    /// Fájlhúzás a bal sáv fölött: továbbítjuk a felületnek, az dönti el, melyik ikon fölött van.
    pub(crate) fn on_drag(&mut self, idx: usize, phase: &str, x: i32, y: i32, paths: Vec<String>) {
        if self.peek.as_ref().is_some_and(|p| p.visible) {
            self.peek_hide();
        }
        if !paths.is_empty() {
            self.drag_paths = paths;
        }
        self.eval_bar(idx, &json!({ "type": "drag", "phase": phase, "x": x, "y": y }).to_string());
    }

    /// Húzás közben egy ikon fölött időzött a kurzor: az ablakát a többi fölé hozzuk.
    pub(crate) fn drag_hover(&mut self, msg: &serde_json::Value) {
        let (mine, _) = self.target_windows(msg);
        if let Some(w) = mine.first() {
            log(format!("húzás: előtérbe hozva – {}", w.title_raw));
            wintrack::raise_for_drop(w.hwnd);
        }
    }

    /// Fájlok ejtése egy ikonra: megnyitjuk őket azzal a programmal.
    pub(crate) fn drop_open(&mut self, msg: &serde_json::Value) {
        let paths = std::mem::take(&mut self.drag_paths);
        let (_, exe) = self.target_windows(msg);
        match exe {
            Some(exe) if !paths.is_empty() => {
                let ok = wintrack::open_with(&exe, &paths);
                log(format!("ejtés: {} fájl megnyitása – {exe} -> {}", paths.len(), if ok { "ok" } else { "HIBA" }));
            }
            _ => {
                let t =
                    self.tr("Ezzel a programmal nem nyitható meg.", "This program can't open it.", "Ten program nie może tego otworzyć.");
                self.toast(&t);
            }
        }
    }

    pub(crate) fn launch(&mut self, id: usize, new_instance: bool) {
        let Some(app) = self.cfg.apps.get(id).cloned() else {
            return;
        };
        self.peek_hide();
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        let m = &self.matchers[id];
        let mine: Vec<WinInfo> = wins.iter().filter(|w| m.matches(w)).cloned().collect();

        if !new_instance && !mine.is_empty() {
            self.toggle_windows(&mine);
            return;
        }

        let path = app.resolved_path();
        let ok = wintrack::shell_open(&path, &config::expand_env(&app.args), &config::expand_env(&app.cwd), app.maximize());
        log(format!("indítás: {} ({path}) -> {}", app.name, if ok { "ok" } else { "HIBA" }));
        if ok {
            let known: Vec<isize> = wins.iter().map(|w| w.hwnd.0 as isize).collect();
            self.settings_dismiss_for(None, &known);
            if app.maximize() {
                self.pending = Some(Pending {
                    app: id,
                    known: wins.iter().map(|w| w.hwnd.0 as isize).collect(),
                    deadline: Instant::now() + Duration::from_secs(20),
                });
            }
        } else {
            let t = self.tr("Nem indítható", "Cannot start", "Nie można uruchomić");
            self.toast(&format!("{t}: {}", app.name));
        }
    }
}
