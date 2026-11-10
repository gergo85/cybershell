//! Képernyősávok lefoglalása (AppBar API) és a Windows tálca kezelése.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_GETTASKBARPOS, ABM_NEW,
    ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsWindowVisible, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SW_HIDE, SW_SHOW, WM_APP,
};
use windows::core::w;

pub const CALLBACK_MSG: u32 = WM_APP + 0x42;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Top,
    Right,
    Bottom,
}

impl Edge {
    fn abe(self) -> u32 {
        match self {
            Edge::Left => ABE_LEFT,
            Edge::Top => ABE_TOP,
            Edge::Right => ABE_RIGHT,
            Edge::Bottom => ABE_BOTTOM,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Edge::Left => "left",
            Edge::Top => "top",
            Edge::Right => "right",
            Edge::Bottom => "bottom",
        }
    }
}

fn data(hwnd: HWND) -> APPBARDATA {
    APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, hWnd: hwnd, ..Default::default() }
}

pub fn register(hwnd: HWND) -> bool {
    let mut d = data(hwnd);
    d.uCallbackMessage = CALLBACK_MSG;
    unsafe { SHAppBarMessage(ABM_NEW, &mut d) != 0 }
}

pub fn unregister(hwnd: HWND) {
    let mut d = data(hwnd);
    unsafe {
        SHAppBarMessage(ABM_REMOVE, &mut d);
    }
}

/// Megkérdezi a rendszert, hova kerülhet a sáv, és a kért vastagságra igazítja.
/// A sávval párhuzamos kiterjedést (pl. a bal sáv magasságát) a kért értéken tartjuk.
pub fn query(hwnd: HWND, edge: Edge, want: RECT, thick: i32) -> RECT {
    let mut d = data(hwnd);
    d.uEdge = edge.abe();
    d.rc = want;
    unsafe {
        SHAppBarMessage(ABM_QUERYPOS, &mut d);
    }
    let mut rc = d.rc;
    match edge {
        Edge::Left => {
            rc.right = rc.left + thick;
            rc.top = want.top;
            rc.bottom = want.bottom;
        }
        Edge::Right => {
            rc.left = rc.right - thick;
            rc.top = want.top;
            rc.bottom = want.bottom;
        }
        Edge::Top => {
            rc.bottom = rc.top + thick;
            rc.left = want.left;
            rc.right = want.right;
        }
        Edge::Bottom => {
            rc.top = rc.bottom - thick;
            rc.left = want.left;
            rc.right = want.right;
        }
    }
    rc
}

/// Véglegesíti a helyet, és odamozgatja az ablakot.
pub fn set(hwnd: HWND, edge: Edge, rc: RECT) -> RECT {
    let mut d = data(hwnd);
    d.uEdge = edge.abe();
    d.rc = rc;
    unsafe {
        SHAppBarMessage(ABM_SETPOS, &mut d);
    }
    {
        // A rendszer a már bejegyzett merőleges sávokhoz (felső/alsó keret) is igazítaná
        // a hosszát – újratöltéskor így rövidülne meg az oldalsó panel. A sávval párhuzamos
        // kiterjedést ezért a kért értéken tartjuk, csak a szél felőli helyet vesszük át.
        let mut r = d.rc;
        match edge {
            Edge::Left | Edge::Right => {
                r.top = rc.top;
                r.bottom = rc.bottom;
            }
            Edge::Top | Edge::Bottom => {
                r.left = rc.left;
                r.right = rc.right;
            }
        }
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOACTIVATE,
            );
        }
        r
    }
}

/// Az asztal munkaterülete: a képernyő a lefoglalt sávok (és a tálca) nélkül.
pub fn work_area() -> RECT {
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
    let mut r = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut RECT as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    r
}

pub fn primary_monitor() -> (RECT, RECT) {
    unsafe {
        let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        (mi.rcMonitor, mi.rcWork)
    }
}

// ---- Tálca ----

pub fn taskbar_hwnd() -> Option<HWND> {
    unsafe { FindWindowW(w!("Shell_TrayWnd"), None).ok().filter(|h| !h.is_invalid()) }
}

pub fn taskbar_state() -> u32 {
    let mut d = data(taskbar_hwnd().unwrap_or_default());
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut d) as u32 }
}

pub fn set_taskbar_state(state: u32) {
    let mut d = data(taskbar_hwnd().unwrap_or_default());
    d.lParam = LPARAM(state as isize);
    unsafe {
        SHAppBarMessage(ABM_SETSTATE, &mut d);
    }
}

/// A tálca helye és éle, ha látható és nem automatikusan rejtett.
pub fn taskbar_reserved() -> Option<(Edge, RECT)> {
    if taskbar_state() & ABS_AUTOHIDE != 0 {
        return None;
    }
    let tb = taskbar_hwnd()?;
    if !unsafe { IsWindowVisible(tb) }.as_bool() {
        return None;
    }
    let mut d = data(tb);
    if unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut d) } == 0 {
        return None;
    }
    let edge = match d.uEdge {
        ABE_LEFT => Edge::Left,
        ABE_TOP => Edge::Top,
        ABE_RIGHT => Edge::Right,
        _ => Edge::Bottom,
    };
    Some((edge, d.rc))
}

pub fn hide_taskbar() {
    set_taskbar_state(taskbar_state() | ABS_AUTOHIDE);
    if let Some(tb) = taskbar_hwnd() {
        unsafe {
            let _ = ShowWindow(tb, SW_HIDE);
        }
    }
}

pub fn taskbar_visible() -> bool {
    taskbar_hwnd().map(|tb| unsafe { IsWindowVisible(tb) }.as_bool()).unwrap_or(false)
}

pub fn show_taskbar() {
    if let Some(tb) = taskbar_hwnd() {
        unsafe {
            let _ = ShowWindow(tb, SW_SHOW);
        }
    }
}

pub fn rehide_taskbar() {
    if let Some(tb) = taskbar_hwnd() {
        unsafe {
            let _ = ShowWindow(tb, SW_HIDE);
        }
    }
}

/// Visszaállítja a tálcát. Előbb az automatikus elrejtést kapcsoljuk vissza, és csak
/// utána jelenítjük meg: fordított sorrendben az állapotváltás újra elrejti.
/// A megjelenítést ellenőrizzük, és szükség esetén megismételjük.
pub fn restore_taskbar(original_state: u32) {
    set_taskbar_state(original_state);
    let Some(tb) = taskbar_hwnd() else {
        return;
    };
    for _ in 0..10 {
        unsafe {
            let _ = ShowWindow(tb, SW_SHOW);
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        if unsafe { IsWindowVisible(tb) }.as_bool() {
            // még egy kis várakozás után is ellenőrizzük, mert a héj késve reagálhat
            std::thread::sleep(std::time::Duration::from_millis(300));
            if unsafe { IsWindowVisible(tb) }.as_bool() {
                return;
            }
        }
    }
}
