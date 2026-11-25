//! Ablakelőnézet: több ablakos programok ikonja fölött az ablakok élő kisképei.

use crate::hub::Hub;
use crate::hub::layout::is_vertical;
use crate::hub::layout::popup_pos;
use crate::log;
use crate::web::PopupParts;
use crate::web::push;
use crate::{appbar, peek, wintrack};
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, PostMessageW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow, WM_CLOSE,
};
use wry::WebView;

/// Az ablakelőnézet: ha egy programnak több ablaka van, az ikonja mellett
/// egymás mellett mutatja az ablakok élő kisképét.
pub(crate) struct PeekWin {
    /// Az ablak, amiben a felület fut: csak életben tartjuk (eldobásakor bezárulna).
    #[expect(dead_code, reason = "csak az ablak élettartama miatt tároljuk")]
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
    /// A bejegyzett élő kisképek: (DWM-azonosító, forrásablak).
    pub(crate) thumbs: Vec<(isize, HWND)>,
    /// A felület visszajelzésére váró kisképek: (forrásablak, cél, forrásterület).
    pub(crate) pending: Vec<(HWND, RECT, RECT)>,
    /// A látható ablakok (csak ezekre fogadunk el parancsot).
    pub(crate) hwnds: Vec<isize>,
    /// Minden új elrendezés új sorszámot kap; a felület ezzel jelez vissza.
    pub(crate) seq: u64,
    /// Melyik app ablakai látszanak (újrarendezéshez, ha egy ablak bezárul).
    pub(crate) target: serde_json::Value,
    pub(crate) hovered: bool,
    pub(crate) hide_at: Option<Instant>,
    pub(crate) refresh_at: Option<Instant>,
    /// Mikor kérték a megnyitást (a megjelenés idejét naplózzuk).
    pub(crate) asked_at: Option<Instant>,
    /// Az a program-ablak, amelyik épp a kártyája fölötti időzés miatt van előtérben.
    pub(crate) shown: Option<peek::Shown>,
    /// Kis méretből előhozott ablakok, amelyeknek az animációit (a kicsinyítésük lefutása után)
    /// vissza kell kapcsolni: (ablak, mikor).
    pub(crate) fx_restore: Vec<(isize, Instant)>,
}

impl PeekWin {
    pub(crate) fn new((window, webview, hwnd): PopupParts) -> Self {
        PeekWin {
            window,
            webview,
            hwnd,
            visible: false,
            thumbs: Vec::new(),
            pending: Vec::new(),
            hwnds: Vec::new(),
            seq: 0,
            target: serde_json::Value::Null,
            hovered: false,
            hide_at: None,
            refresh_at: None,
            asked_at: None,
            shown: None,
            fx_restore: Vec::new(),
        }
    }

    /// Az elengedett ablak animációinak visszakapcsolása kicsit később: ha azonnal tennénk,
    /// a kicsinyítése még animálva futhatna le.
    fn release(&mut self, s: peek::Shown) {
        if s.iconic {
            self.fx_restore.retain(|(h, _)| *h != s.hwnd);
            self.fx_restore.push((s.hwnd, Instant::now() + Duration::from_millis(600)));
        }
    }
}

impl Hub {
    /// Egy program memóriahasználata kiírva (pl. „412 MB”, „1,3 GB”) az előnézet és a
    /// menü fejlécébe; az összes folyamatát összeadva, mint a Feladatkezelő.
    fn memory_label(&self, path: &str) -> Option<String> {
        let bytes = wintrack::memory_of_path(path)?;
        let mb = bytes as f64 / (1024.0 * 1024.0);
        let s = if mb < 1000.0 {
            format!("{} MB", mb.round().max(1.0))
        } else {
            let gb = format!("{:.1}", mb / 1024.0);
            let gb = if self.cfg.language == "en" { gb } else { gb.replace('.', ",") };
            format!("{gb} GB")
        };
        Some(s)
    }

    /// Az előnézet kártyája fölött időzve előtérbe hozott ablakot visszateszi a helyére.
    fn peek_unshow(&mut self) {
        let Some(pk) = self.peek.as_mut() else { return };
        if let Some(s) = pk.shown.take() {
            peek::unraise(s);
            pk.release(s);
        }
    }

    /// Az elengedett ablakok animációinak visszakapcsolása, ha eljött az ideje (az épp
    /// előtérben lévőé majd az elengedése után).
    pub(crate) fn peek_fx_tick(&mut self, now: Instant) {
        let Some(pk) = self.peek.as_mut() else { return };
        let shown = pk.shown.map(|s| s.hwnd);
        pk.fx_restore.retain(|&(h, at)| {
            if Some(h) == shown {
                return false;
            }
            if now < at {
                return true;
            }
            peek::set_transitions(HWND(h as *mut core::ffi::c_void), true);
            false
        });
    }

    pub(crate) fn peek_eval(&self, payload: &serde_json::Value) {
        if let Some(p) = &self.peek {
            push(&p.webview, payload);
        }
    }

    pub(crate) fn peek_visible(&self) -> bool {
        self.peek.as_ref().is_some_and(|p| p.visible)
    }

    /// Látszik az előnézet, és az egérmutató fölötte van (a felület egéreseményeitől függetlenül).
    pub(crate) fn peek_under_cursor(&self) -> bool {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
        let Some(pk) = self.peek.as_ref().filter(|p| p.visible) else { return false };
        let (mut pt, mut r) = (POINT::default(), RECT::default());
        unsafe {
            if GetCursorPos(&mut pt).is_err() || GetWindowRect(pk.hwnd, &mut r).is_err() {
                return false;
            }
        }
        pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom
    }

    /// Az ikonsáv egy több ablakos program fölött időzik: elrendezzük a kisképeket,
    /// és elküldjük a felületnek. Megjelenni akkor fog, amikor a felület kirajzolta.
    pub(crate) fn peek_open(&mut self, idx: usize, msg: &serde_json::Value) {
        if self.peek.is_none() {
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale, bar_mon)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale, b.mon)) else { return };
        if self.fullscreen.contains(&bar_mon) || self.settings_visible() {
            self.peek_hide();
            return;
        }
        let (mut mine, _) = self.target_windows(msg);
        mine.sort_by_key(|w| self.seen.get(&(w.hwnd.0 as isize)).copied().unwrap_or(u64::MAX));
        if mine.len() < 2 {
            self.peek_hide();
            return;
        }
        self.peek_unshow();
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);

        // hány kártya fér el egymás mellett, illetve egymás alatt
        let card_h = peek::TITLE_H + peek::THUMB_H;
        let avail_w = (work.right - work.left) as f64 / scale - 24.0;
        let avail_h = (work.bottom - work.top) as f64 / scale - 16.0;
        let max_cols = (((avail_w - 2.0 * peek::PAD + peek::GAP) / (peek::CARD_W + peek::GAP)).floor() as usize).max(1);
        let max_rows = (((avail_h - 2.0 * peek::PAD - peek::HEADER + peek::GAP) / (card_h + peek::GAP)).floor() as usize).max(1);
        let cols = mine.len().min(max_cols);
        mine.truncate(cols * max_rows);
        let rows = mine.len().div_ceil(cols);
        let w_css = 2.0 * peek::PAD + cols as f64 * peek::CARD_W + (cols - 1) as f64 * peek::GAP;
        let h_css = 2.0 * peek::PAD + peek::HEADER + rows as f64 * card_h + (rows - 1) as f64 * peek::GAP;
        let (w, h) = ((w_css * scale).round() as i32, (h_css * scale).round() as i32);

        // az ikonhoz igazítva, a munkaterületen belül
        let icon_y = msg["y"].as_f64().unwrap_or(0.0);
        let along = if is_vertical(bar_edge) { icon_y } else { msg["x"].as_f64().unwrap_or(0.0) };
        let (left, top) = popup_pos(bar_rect, bar_edge, w, h, (along * scale).round() as i32, work);

        let fg = self.last_fg;
        let mut items = Vec::new();
        let mut pending = Vec::new();
        for (i, win) in mine.iter().enumerate() {
            let (col, row) = (i % cols, i / cols);
            let card = peek::Rectf {
                x: peek::PAD + col as f64 * (peek::CARD_W + peek::GAP),
                y: peek::PAD + peek::HEADER + row as f64 * (card_h + peek::GAP),
                w: peek::CARD_W,
                h: card_h,
            };
            let boxr = peek::Rectf {
                x: card.x + peek::INSET,
                y: card.y + peek::TITLE_H,
                w: peek::CARD_W - 2.0 * peek::INSET,
                h: peek::THUMB_H - peek::INSET,
            };
            // kis méretre tett ablakról a Windows nem ad élő képet: ott az ikon látszik
            let src = if win.iconic { None } else { peek::visible_source(win.hwnd) };
            let thumb = src.map(|s| peek::fit(boxr, &s));
            if let (Some(s), Some(t)) = (src, thumb) {
                pending.push((win.hwnd, t.to_phys(scale), s));
            }
            let icon = if win.path.is_empty() { String::new() } else { self.program_icon(&win.path) };
            items.push(json!({
                "hwnd": win.hwnd.0 as isize,
                "title": if win.title_raw.trim().is_empty() { wintrack::display_name(&win.path) } else { win.title_raw.clone() },
                "icon": icon,
                "iconic": win.iconic,
                "active": Some(win.hwnd) == fg && !win.iconic,
                "card": card.json(),
                "box": boxr.json(),
                "thumb": thumb.map(|t| t.json()),
            }));
        }
        let name = if let Some(id) = msg["id"].as_u64() {
            self.cfg.apps.get(id as usize).map(|a| a.name.clone()).unwrap_or_default()
        } else {
            let key = msg["key"].as_str().unwrap_or("").to_string();
            match self.extra_names.get(&key) {
                Some(n) => n.clone(),
                None => mine.first().map(|w| w.path.clone()).map(|p| self.program_name(&p)).unwrap_or_default(),
            }
        };
        let mem = mine.first().and_then(|w| self.memory_label(&w.path));
        let hwnds: Vec<isize> = mine.iter().map(|w| w.hwnd.0 as isize).collect();
        let mut target = json!({ "y": icon_y, "x": msg["x"], "bar": idx });
        if !msg["id"].is_null() {
            target["id"] = msg["id"].clone();
        }
        if !msg["key"].is_null() {
            target["key"] = msg["key"].clone();
        }

        let Some(pk) = self.peek.as_mut() else { return };
        for (id, _) in pk.thumbs.drain(..) {
            peek::drop_thumb(id);
        }
        pk.seq += 1;
        pk.asked_at = Some(Instant::now());
        pk.pending = pending;
        pk.hwnds = hwnds;
        pk.target = target;
        pk.hide_at = None;
        pk.refresh_at = None;
        let seq = pk.seq;
        unsafe {
            let _ = SetWindowPos(pk.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_NOACTIVATE);
        }
        let payload = json!({
            "type": "peek",
            "seq": seq,
            "name": name,
            "mem": mem,
            "count": items.len(),
            "items": items,
            "theme": self.theme_json(),
        });
        self.peek_eval(&payload);
    }

    /// Jobb kattintás egy ikonon: kis menü a program ablakához (Teljes képernyőre – ha még
    /// nincs úgy –, Új ablak nyitása, Ablak bezárása), az ablakelőnézet felugró ablakában, annak
    /// stílusában.
    /// Csak akkor jelenik meg, ha a programnak pontosan egy ablaka van nyitva.
    pub(crate) fn app_menu(&mut self, idx: usize, msg: &serde_json::Value) {
        use windows::Win32::UI::WindowsAndMessaging::IsZoomed;
        if self.peek.is_none() {
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale)) else { return };
        let (mine, exe) = self.target_windows(msg);
        if mine.len() != 1 {
            self.peek_hide();
            return;
        }
        let win = mine[0].clone();
        let zoomed = !win.iconic && unsafe { IsZoomed(win.hwnd).as_bool() };
        let mut items = Vec::new();
        if !zoomed {
            items.push(json!({ "cmd": "max", "icon": "maximize", "label": self.tr("Teljes képernyőre", "Maximize", "Maksymalizuj", "Maximieren", "Maximizar") }));
        }
        // új példány: a kitűzött program a saját beállításaival indul, a nem kitűzött az exe-jéből
        // (ha nem exe, pl. Store-app, nincs mit újra elindítani)
        if msg["id"].is_u64() || exe.is_some() {
            items
                .push(json!({ "cmd": "new", "icon": "plus", "label": self.tr("Új ablak nyitása", "Open new window", "Otwórz nowe okno", "Neues Fenster öffnen", "Abrir ventana nueva") }));
        }
        items.push(
            json!({ "cmd": "close", "icon": "close", "label": self.tr("Ablak bezárása", "Close window", "Zamknij okno", "Fenster schließen", "Cerrar ventana"), "danger": true }),
        );
        let name = match msg["id"].as_u64() {
            Some(id) => self.cfg.apps.get(id as usize).map(|a| a.name.clone()).unwrap_or_default(),
            None => self.program_name(&win.path),
        };
        let mem = self.memory_label(&win.path);

        // méret: fejléc, gombok (32 px), köztük 4 px, alul 10 px
        let n = items.len() as f64;
        let (w_css, h_css) = (264.0, 38.0 + n * 32.0 + (n - 1.0) * 4.0 + 10.0);
        let scale = bar_scale.max(0.5);
        let (w, h) = ((w_css * scale).round() as i32, (h_css * scale).round() as i32);
        let work = appbar::work_area_of(bar_rect);
        let along = if is_vertical(bar_edge) { msg["y"].as_f64().unwrap_or(0.0) } else { msg["x"].as_f64().unwrap_or(0.0) };
        let (left, top) = popup_pos(bar_rect, bar_edge, w, h, (along * scale).round() as i32, work);

        let Some(pk) = self.peek.as_mut() else { return };
        for (id, _) in pk.thumbs.drain(..) {
            peek::drop_thumb(id);
        }
        pk.seq += 1;
        pk.asked_at = Some(Instant::now());
        pk.pending.clear();
        pk.hwnds = vec![win.hwnd.0 as isize];
        // a menü célja (az Új ablak nyitásához): kitűzött app sorszáma vagy futó program kulcsa
        pk.target = json!({ "menu": true, "id": msg["id"], "key": msg["key"] });
        pk.hide_at = None;
        pk.refresh_at = None;
        let seq = pk.seq;
        unsafe {
            let _ = SetWindowPos(pk.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_NOACTIVATE);
        }
        let payload = json!({
            "type": "menu",
            "seq": seq,
            "name": name,
            "mem": mem,
            "hwnd": win.hwnd.0 as isize,
            "items": items,
            "theme": self.theme_json(),
        });
        self.peek_eval(&payload);
    }

    /// A felület kirajzolta az előnézetet: bejegyezzük a kisképeket és megjelenítjük.
    pub(crate) fn peek_ready(&mut self, seq: u64) {
        let Some(pk) = self.peek.as_mut() else { return };
        if seq != pk.seq {
            return;
        }
        for (src, rect, source) in pk.pending.drain(..) {
            if let Some(id) = peek::show_thumb(pk.hwnd, src, rect, source) {
                pk.thumbs.push((id, src));
            }
        }
        if let Some(t) = pk.asked_at.take() {
            log(format!("előnézet: {} ms alatt kirajzolva", t.elapsed().as_millis()));
        }
        if !pk.visible {
            pk.visible = true;
            unsafe {
                let _ = ShowWindow(pk.hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(pk.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            self.eval(self.icons_edge(), &json!({ "type": "peek-state", "open": true }));
        }
    }

    pub(crate) fn peek_hide(&mut self) {
        self.peek_unshow();
        let Some(pk) = self.peek.as_mut() else { return };
        pk.seq += 1;
        pk.pending.clear();
        pk.hwnds.clear();
        pk.hide_at = None;
        pk.refresh_at = None;
        pk.hovered = false;
        for (id, _) in pk.thumbs.drain(..) {
            peek::drop_thumb(id);
        }
        let was_visible = pk.visible;
        pk.visible = false;
        if was_visible {
            unsafe {
                let _ = ShowWindow(pk.hwnd, SW_HIDE);
            }
            self.eval(self.icons_edge(), &json!({ "type": "peek-state", "open": false }));
        }
        self.peek_eval(&json!({ "type": "peek-clear" }));
    }

    /// Az egér elhagyta az ikont vagy az előnézetet: kis késleltetéssel bezárjuk
    /// (közben átérhet az ikonról az előnézetre és vissza).
    pub(crate) fn peek_leave(&mut self, from_icon: bool) {
        let Some(pk) = self.peek.as_mut() else { return };
        if from_icon && pk.hovered {
            return;
        }
        if !from_icon {
            pk.hovered = false;
        }
        if pk.visible {
            pk.hide_at = Some(Instant::now() + Duration::from_millis(if from_icon { 320 } else { 260 }));
        } else {
            // még meg sem jelent: a folyamatban lévő elrendezést eldobjuk
            pk.seq += 1;
            pk.pending.clear();
        }
    }

    pub(crate) fn on_peek_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let hwnd = msg["hwnd"].as_i64().map(|h| h as isize);
        let known = |pk: &PeekWin, h: Option<isize>| h.filter(|h| pk.hwnds.contains(h)).map(|h| HWND(h as *mut core::ffi::c_void));
        match msg["type"].as_str().unwrap_or("") {
            "peek-ready" => self.peek_ready(msg["seq"].as_u64().unwrap_or(0)),
            "peek-enter" => {
                if let Some(pk) = self.peek.as_mut() {
                    pk.hovered = true;
                    pk.hide_at = None;
                }
            }
            "peek-leave" => self.peek_leave(false),
            // egy kártya fölött időzik az egér: az ablaka előtérbe kerül (mint a Windows
            // tálcáján); a kártyáról lemenve visszakerül a helyére
            "peek-hover" => {
                let h = self.peek.as_ref().and_then(|pk| known(pk, hwnd));
                if self.peek.as_ref().is_some_and(|pk| pk.shown.map(|s| s.hwnd) == h.map(|h| h.0 as isize)) {
                    return;
                }
                self.peek_unshow();
                if let (Some(h), Some(pk)) = (h, self.peek.as_mut()) {
                    pk.shown = Some(peek::raise(h));
                }
            }
            "peek-activate" => {
                let Some(h) = self.peek.as_ref().and_then(|pk| known(pk, hwnd)) else { return };
                // a kiválasztott ablak előtérben marad
                if let Some(pk) = self.peek.as_mut()
                    && let Some(s) = pk.shown.take()
                {
                    pk.release(s);
                }
                self.peek_hide();
                wintrack::activate(h);
                self.last_fg = Some(h);
                self.last_state.clear();
                self.poll();
            }
            "peek-close" => {
                let Some(pk) = self.peek.as_mut() else { return };
                let Some(h) = known(pk, hwnd) else { return };
                log("előnézet: ablak bezárása");
                if let Some(pos) = pk.thumbs.iter().position(|(_, src)| *src == h) {
                    let (id, _) = pk.thumbs.remove(pos);
                    peek::drop_thumb(id);
                }
                pk.hwnds.retain(|x| *x != h.0 as isize);
                if pk.shown.is_some_and(|s| s.hwnd == h.0 as isize) {
                    pk.shown = None;
                }
                pk.refresh_at = Some(Instant::now() + Duration::from_millis(450));
                unsafe {
                    let _ = PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
            // az előnézet alján: a program összes ablakának bezárása
            "peek-close-all" => {
                let Some(pk) = self.peek.as_mut() else { return };
                pk.shown = None;
                let all: Vec<isize> = pk.hwnds.clone();
                log(format!("előnézet: összes ablak bezárása ({} db)", all.len()));
                self.peek_hide();
                for h in all {
                    unsafe {
                        let _ = PostMessageW(Some(HWND(h as *mut core::ffi::c_void)), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
                self.last_state.clear();
            }
            // az ikon menüje: Teljes képernyőre, Új ablak nyitása, Ablak bezárása
            "menu-pick" => {
                let Some(h) = self.peek.as_ref().and_then(|pk| known(pk, hwnd)) else { return };
                let target = self.peek.as_ref().map(|pk| pk.target.clone()).unwrap_or_default();
                self.peek_hide();
                match msg["cmd"].as_str().unwrap_or("") {
                    "max" => {
                        log("ikonmenü: teljes képernyőre");
                        wintrack::maximize(h);
                        wintrack::activate(h);
                        self.last_fg = Some(h);
                    }
                    "new" => {
                        log("ikonmenü: új ablak nyitása");
                        if let Some(id) = target["id"].as_u64() {
                            self.launch(id as usize, true);
                        } else if target["key"].is_string() {
                            self.launch_extra(&json!({ "key": target["key"], "newInstance": true }));
                        }
                    }
                    "close" => {
                        log("ikonmenü: ablak bezárása");
                        unsafe {
                            let _ = PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0));
                        }
                    }
                    _ => {}
                }
                self.last_state.clear();
                self.poll();
            }
            "log" => log(format!("[előnézet] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }
}
