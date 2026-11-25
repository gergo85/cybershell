//! Saját asztali menü: a felugró ablak, amiben a gyári asztali menü parancsai a CyberShell
//! stílusában jelennek meg (a figyelő és a héj menüje: deskmenu.rs).
//!
//! Az ablak a kattintás képernyőjét teljesen lefedi, de csak a menü (és a nyitott almenük)
//! területe látszik belőle és kap kattintást: a többit az ablak régiója kivágja. Így a menü
//! és az almenük helyét a felület maga számolja ki, és a képernyő szélén meg tud fordulni.

use crate::deskmenu;
use crate::hub::Hub;
use crate::log;
use crate::web::PopupParts;
use crate::web::push;
use crate::wintrack;
use serde_json::json;
use std::time::{Duration, Instant};
use tao::window::Window;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, DeleteObject, GetMonitorInfoW, HGDIOBJ, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, RGN_OR,
    SetWindowRgn,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos,
    ShowWindow,
};
use wry::WebView;

/// Az asztali menü felugró ablaka.
pub(crate) struct DeskWin {
    /// Az ablak, amiben a felület fut: csak életben tartjuk (eldobásakor bezárulna).
    #[expect(dead_code, reason = "csak az ablak élettartama miatt tároljuk")]
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) visible: bool,
    /// Mikor jelent meg (a fókuszvesztést csak egy kis idő után figyeljük).
    pub(crate) shown_at: Instant,
    /// Megjelenés óta megkapta-e már a fókuszt (attól kezdve zárul be, ha máshova kattintanak).
    pub(crate) focused: bool,
    /// A legutóbb kért menü sorszáma (a régebbi kérések válaszát eldobjuk).
    pub(crate) seq: u64,
    /// A kattintás helye és képernyője (a menü itt nyílik meg; ha nem sikerül, ide küldjük
    /// vissza a kattintást a Windowsnak).
    pub(crate) at: POINT,
    pub(crate) mon: RECT,
}

impl DeskWin {
    pub(crate) fn new((window, webview, hwnd): PopupParts) -> Self {
        DeskWin {
            window,
            webview,
            hwnd,
            visible: false,
            shown_at: Instant::now(),
            focused: false,
            seq: 0,
            at: POINT::default(),
            mon: RECT::default(),
        }
    }
}

fn monitor_at(pt: POINT) -> RECT {
    unsafe {
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        mi.rcMonitor
    }
}

impl Hub {
    /// Jobb kattintás az asztalon (a figyelő elnyelte): a menü elkészítése; `item` az ikon
    /// sorszáma az ikonlistában (-1: üres terület). Ha a saját menü nem jelenhet meg, a kattintás
    /// visszamegy a Windowsnak.
    pub(crate) fn on_desk_click(&mut self, x: i32, y: i32, shift: bool, item: i32) {
        let on = if item >= 0 { self.cfg.desktop_icon_menu } else { self.cfg.desktop_menu };
        if !on || self.desk.is_none() {
            deskmenu::replay(x, y);
            return;
        }
        self.desk_hide(false);
        self.peek_hide();
        self.fly_hide(false);
        self.launcher_hide(false);
        let Some(d) = self.desk.as_mut() else { return };
        d.seq += 1;
        d.at = POINT { x, y };
        d.mon = monitor_at(d.at);
        let (seq, mon) = (d.seq, d.mon);
        // Az ablak előre a kattintás képernyőjére kerül (rejtve), hogy a felület már az ottani
        // méretarányban rajzoljon. Kétszer: ha az első áthelyezés DPI-váltással jár, a Windows
        // átméretezi az ablakot, a második visszaállítja a képernyő méretére.
        for _ in 0..2 {
            unsafe {
                let _ =
                    SetWindowPos(d.hwnd, Some(HWND_TOPMOST), mon.left, mon.top, mon.right - mon.left, mon.bottom - mon.top, SWP_NOACTIVATE);
            }
        }
        deskmenu::open(seq, x, y, shift, item);
    }

    /// A menü elemei elkészültek (a menüszálról): a felület kirajzolja, és visszaküldi a
    /// látható területeket (`desk-rects`), akkor jelenik meg az ablak.
    /// `sub`: egy újraolvasott almenü sorszáma (a felület a nyitott almenüt frissíti vele).
    pub(crate) fn on_desk_items(&mut self, seq: u64, sub: Option<usize>, items: serde_json::Value) {
        let theme = self.theme_json();
        let Some(d) = self.desk.as_ref() else { return };
        if seq != d.seq {
            deskmenu::release(seq);
            return;
        }
        if let Some(menu) = sub {
            push(&d.webview, json!({ "type": "desk-sub", "seq": seq, "menu": menu, "items": items }));
            return;
        }
        let payload = json!({
            "type": "desk-open",
            "seq": seq,
            // a kattintás helye az ablakon belül, fizikai pixelben (a felület osztja le a saját arányával)
            "x": d.at.x - d.mon.left,
            "y": d.at.y - d.mon.top,
            "items": items,
            "theme": theme,
        });
        push(&d.webview, &payload);
    }

    /// A felület üzenete: a látható területek, a választott parancs vagy a bezárás.
    pub(crate) fn on_desk_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let seq = msg["seq"].as_u64().unwrap_or(0);
        let current = self.desk.as_ref().is_some_and(|d| d.seq == seq);
        match msg["type"].as_str().unwrap_or("") {
            "desk-rects" if current => self.desk_region(&msg),
            "desk-invoke" if current => {
                let id = msg["id"].as_u64().unwrap_or(0) as u32;
                // a parancs az asztalon fut (pl. az új mappa átnevezése): az kapja a fókuszt,
                // és az általa nyitott ablakok (Személyre szabás) előre jöhetnek
                self.desk_close(true, false);
                unsafe {
                    let _ = AllowSetForegroundWindow(ASFW_ANY);
                }
                deskmenu::invoke(seq, id);
            }
            "desk-close" if current => self.desk_hide(true),
            // egy almenü megnyílt: újraolvassuk (a Küldés almenüje késleltetve töltődik fel)
            "desk-sub" if current => deskmenu::refresh(seq, msg["menu"].as_u64().unwrap_or(0) as usize),
            // a felület hibája: a Windows menüje nyílik meg helyette
            "desk-fail" if current => {
                let at = self.desk.as_ref().map(|d| d.at).unwrap_or_default();
                log(format!("asztali menü: a felület nem rajzolta ki ({})", msg["error"].as_str().unwrap_or("")));
                self.desk_close(false, true);
                deskmenu::replay(at.x, at.y);
            }
            "log" => log(format!("felület: {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    /// Az ablak látható része a felület szerint (a menü és a nyitott almenük téglalapjai,
    /// CSS-pixelben a felület méretarányával); az első üzenetre jelenik meg az ablak.
    fn desk_region(&mut self, msg: &serde_json::Value) {
        let Some(d) = self.desk.as_mut() else { return };
        let dpr = msg["dpr"].as_f64().unwrap_or(1.0).max(0.5);
        let px = |v: &serde_json::Value| (v.as_f64().unwrap_or(0.0) * dpr).round() as i32;
        let rects = msg["rects"].as_array().cloned().unwrap_or_default();
        if rects.is_empty() {
            return;
        }
        unsafe {
            let rgn = CreateRectRgn(0, 0, 0, 0);
            for r in &rects {
                let (x, y) = (px(&r[0]), px(&r[1]));
                let part = CreateRectRgn(x, y, x + px(&r[2]), y + px(&r[3]));
                CombineRgn(Some(rgn), Some(rgn), Some(part), RGN_OR);
                let _ = DeleteObject(HGDIOBJ(part.0));
            }
            // a régiót ettől kezdve a rendszer kezeli
            SetWindowRgn(d.hwnd, Some(rgn), true);
        }
        if !d.visible {
            d.visible = true;
            d.shown_at = Instant::now();
            d.focused = false;
            let (hwnd, mon) = (d.hwnd, d.mon);
            unsafe {
                let flags = SWP_SHOWWINDOW;
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), mon.left, mon.top, mon.right - mon.left, mon.bottom - mon.top, flags);
            }
            wintrack::activate(hwnd);
        }
    }

    /// Bezárás választás nélkül (Esc, máshova kattintás). `refocus`: az asztal kapja vissza a
    /// fókuszt (Esc után; máshova kattintáskor az kapja, ahova kattintottak).
    pub(crate) fn desk_hide(&mut self, refocus: bool) {
        self.desk_close(refocus, true);
    }

    fn desk_close(&mut self, refocus: bool, release: bool) {
        let Some(d) = self.desk.as_mut() else { return };
        if !d.visible {
            return;
        }
        d.visible = false;
        let (hwnd, seq) = (d.hwnd, d.seq);
        if refocus && let Some(desk) = deskmenu::desktop_window() {
            unsafe {
                let _ = SetForegroundWindow(desk);
            }
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        push(&d.webview, json!({ "type": "desk-clear" }));
        if release {
            deskmenu::release(seq);
        }
    }

    /// A menü bezárul, ha máshova kattintanak (elveszti a fókuszt); a `tick` hívja.
    pub(crate) fn desk_watch_focus(&mut self, fg: HWND, now: Instant) {
        let Some(d) = self.desk.as_mut() else { return };
        if !d.visible {
            return;
        }
        if fg == d.hwnd {
            d.focused = true;
        }
        if (d.focused || now.duration_since(d.shown_at) > Duration::from_millis(400)) && fg != d.hwnd {
            self.desk_hide(false);
        }
    }

    /// A beállítás ki- vagy bekapcsolása.
    /// A beállítás ki- vagy bekapcsolása: az asztal üres területén (`icons` = false), illetve az
    /// ikonokon.
    pub(crate) fn set_desktop_menu(&mut self, icons: bool, on: bool) {
        if icons {
            self.cfg.desktop_icon_menu = on;
        } else {
            self.cfg.desktop_menu = on;
        }
        deskmenu::set_enabled(self.cfg.desktop_menu, self.cfg.desktop_icon_menu);
        if !on {
            self.desk_hide(false);
        }
        let what = if icons { "az ikonokon" } else { "az asztalon" };
        log(format!("saját asztali menü {what}: {}", if on { "be" } else { "ki" }));
    }
}
