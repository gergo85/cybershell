//! Képernyősávok lefoglalása (AppBar API) és a Windows tálca kezelése.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint};
use windows::Win32::UI::Shell::{
    ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_GETTASKBARPOS, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, ABM_SETSTATE,
    ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, HWND_TOPMOST, IsWindowVisible, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SetWindowPos, ShowWindow, WM_APP,
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
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOACTIVATE);
        }
        r
    }
}

/// Az asztal munkaterülete: a képernyő a lefoglalt sávok (és a tálca) nélkül.
pub fn work_area() -> RECT {
    use windows::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
    let mut r = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut RECT as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    r
}

/// Egy képernyő: teljes terület, munkaterület, elsődleges-e, DPI.
#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub rect: RECT,
    pub primary: bool,
    pub dpi: u32,
}

unsafe extern "system" fn enum_monitor(
    mon: windows::Win32::Graphics::Gdi::HMONITOR,
    _hdc: windows::Win32::Graphics::Gdi::HDC,
    _rc: *mut RECT,
    data: LPARAM,
) -> windows::core::BOOL {
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    let list = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(mon, &mut mi) }.as_bool() {
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
        list.push(Monitor { rect: mi.rcMonitor, primary: mi.dwFlags & 1 != 0, dpi: dx.max(96) });
    }
    true.into()
}

/// A képernyők: elöl az elsődleges, utána balról jobbra, fentről lefelé.
pub fn monitors() -> Vec<Monitor> {
    use windows::Win32::Graphics::Gdi::EnumDisplayMonitors;
    let mut list: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_monitor), LPARAM(&mut list as *mut Vec<Monitor> as isize));
    }
    list.sort_by_key(|m| (!m.primary, m.rect.left, m.rect.top));
    list
}

/// Annak a képernyőnek a munkaterülete, amelyiken a megadott terület (pl. egy sáv) van.
pub fn work_area_of(rc: RECT) -> RECT {
    use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromRect};
    unsafe {
        let mon = MonitorFromRect(&rc, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        mi.rcWork
    }
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

/// A további képernyők tálcái (ha a Windows minden képernyőn mutatja a tálcát).
pub fn secondary_taskbars() -> Vec<HWND> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;
    let mut out = Vec::new();
    let mut prev: Option<HWND> = None;
    while let Ok(h) = unsafe { FindWindowExW(None, prev, w!("Shell_SecondaryTrayWnd"), None) } {
        if h.is_invalid() || out.contains(&h) {
            break;
        }
        out.push(h);
        prev = Some(h);
        if out.len() > 16 {
            break;
        }
    }
    out
}

fn all_taskbars() -> Vec<HWND> {
    let mut v: Vec<HWND> = taskbar_hwnd().into_iter().collect();
    v.extend(secondary_taskbars());
    v
}

/// Egy további képernyő tálcájának helye és éle, ha látható és nem automatikusan rejtett.
pub fn secondary_taskbar_on(mon: RECT) -> Option<(Edge, RECT)> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    if taskbar_state() & ABS_AUTOHIDE != 0 {
        return None;
    }
    for h in secondary_taskbars() {
        if !unsafe { IsWindowVisible(h) }.as_bool() {
            continue;
        }
        let mut r = RECT::default();
        if unsafe { GetWindowRect(h, &mut r) }.is_err() {
            continue;
        }
        let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        if cx < mon.left || cx >= mon.right || cy < mon.top || cy >= mon.bottom {
            continue;
        }
        let edge = if r.right - r.left >= r.bottom - r.top {
            if r.top <= mon.top { Edge::Top } else { Edge::Bottom }
        } else if r.left <= mon.left {
            Edge::Left
        } else {
            Edge::Right
        };
        return Some((edge, r));
    }
    None
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
    for tb in all_taskbars() {
        unsafe {
            let _ = ShowWindow(tb, SW_HIDE);
        }
    }
}

pub fn taskbar_visible() -> bool {
    all_taskbars().iter().any(|tb| unsafe { IsWindowVisible(*tb) }.as_bool())
}

pub fn show_taskbar() {
    for tb in all_taskbars() {
        unsafe {
            let _ = ShowWindow(tb, SW_SHOW);
        }
    }
}

pub fn rehide_taskbar() {
    for tb in all_taskbars() {
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
    // a további képernyők tálcái
    for tb in secondary_taskbars() {
        unsafe {
            let _ = ShowWindow(tb, SW_SHOW);
        }
    }
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
