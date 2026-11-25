//! Neonkeret-réteg az aktív ablak körül.
//!
//! A Windows 11 saját ablakkerete (DWM) teljes méretű ablakon nem látszik, és csak 1 px
//! vastag. Ezért az előtérben lévő program köré egy saját, átlátszó, kattintásokat átengedő
//! ablakot teszünk, ami a téma fő színével fénylő keretet rajzol: normál ablaknál kívül, a
//! kerete mentén, teljes méretűnél belül, a szélein. Az ablak mozgását és átméretezését a
//! Windows ablakeseményeiből követjük, így húzás közben sem marad le.
//! Minden hívás a fő szálon fut (az eseménykezelő is oda érkezik).

use std::sync::Mutex;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, ReleaseDC, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GW_HWNDPREV, GetWindow, GetWindowRect, HWND_TOP, IsIconic, IsWindow, IsWindowVisible, IsZoomed,
    RegisterClassW, SW_HIDE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow,
    WINDOW_EX_STYLE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

/// A fénylő vonal vastagsága, a körülötte lévő fény szélessége (képpont), és a vonal, illetve
/// a fény legnagyobb átlátszatlansága (0–255): vékony, visszafogott keret.
const LINE: i32 = 1;
const GLOW: i32 = 4;
const LINE_ALPHA: u32 = 190;
const GLOW_ALPHA: f64 = 70.0;

const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_SHOW: u32 = 0x8002;
const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;
const WINEVENT_OUTOFCONTEXT: u32 = 0;
const WINEVENT_SKIPOWNPROCESS: u32 = 2;

/// A legutóbb rajzolt réteg: szélesség, magasság, szín, belül-e a keret.
type Drawn = (i32, i32, (u8, u8, u8), bool);

#[derive(Default)]
struct State {
    /// A saját rétegablak (0: még nincs).
    overlay: isize,
    /// A követett ablak (0: nincs).
    target: isize,
    color: (u8, u8, u8),
    /// A legutóbb rajzolt méret, szín és elhelyezés (ha nem változott, csak mozgatunk).
    drawn: Option<Drawn>,
    shown: bool,
    hooked: bool,
}

static STATE: Mutex<State> = Mutex::new(State { overlay: 0, target: 0, color: (0, 0, 0), drawn: None, shown: false, hooked: false });

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn create_overlay() -> isize {
    unsafe {
        let inst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("CyberShellNeon");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: inst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let ex: WINDOW_EX_STYLE = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        CreateWindowExW(ex, class, w!(""), WS_POPUP, 0, 0, 1, 1, None, None, Some(inst.into()), None).map(|h| h.0 as isize).unwrap_or(0)
    }
}

unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, _event: u32, hwnd: HWND, id_object: i32, _child: i32, _thread: u32, _time: u32) {
    // csak a követett ablak saját (nem gyermekobjektum) eseményei
    if id_object != 0 {
        return;
    }
    let target = STATE.lock().map(|s| s.target).unwrap_or(0);
    if target != 0 && hwnd.0 as isize == target {
        refresh();
    }
}

fn hook() {
    unsafe {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_event), 0, 0, flags);
        SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_event), 0, 0, flags);
    }
}

/// Melyik ablak kapja a keretet (`None`: senki), és milyen színnel. Gyakran hívható (a fő
/// ciklus minden körében): újrarajzolni csak méret- vagy színváltozáskor kell, a réteget
/// viszont mindig visszateszi közvetlenül a követett ablak fölé (a z-sorrend változásáról
/// nem jön esemény).
pub fn set_target(target: Option<HWND>, color: (u8, u8, u8)) {
    {
        let Ok(mut s) = STATE.lock() else { return };
        let t = target.map(|h| h.0 as isize).unwrap_or(0);
        if t != 0 && s.overlay == 0 {
            s.overlay = create_overlay();
        }
        if t != 0 && !s.hooked {
            s.hooked = true;
            drop(s);
            hook();
            s = match STATE.lock() {
                Ok(s) => s,
                Err(_) => return,
            };
        }
        s.target = t;
        s.color = color;
    }
    refresh();
}

fn hwnd(v: isize) -> HWND {
    HWND(v as *mut core::ffi::c_void)
}

/// A keret igazítása a követett ablakhoz (helyzet, méret, láthatóság, sorrend).
pub fn refresh() {
    let Ok(mut s) = STATE.lock() else { return };
    if s.overlay == 0 {
        return;
    }
    let ov = hwnd(s.overlay);
    let Some((rect, inset)) = (s.target != 0).then(|| frame_of(hwnd(s.target))).flatten() else {
        if s.shown {
            unsafe {
                let _ = ShowWindow(ov, SW_HIDE);
            }
            s.shown = false;
        }
        return;
    };
    let g = if inset { 0 } else { GLOW };
    let (x, y) = (rect.left - g, rect.top - g);
    let (w, h) = (rect.right - rect.left + 2 * g, rect.bottom - rect.top + 2 * g);
    let key = (w, h, s.color, inset);
    if s.drawn != Some(key) {
        draw(ov, x, y, w, h, g, s.color);
        s.drawn = Some(key);
    }
    // közvetlenül a követett ablak fölé (a fölötte lévő ablakok alá) kerül
    let target = hwnd(s.target);
    let above = unsafe { GetWindow(target, GW_HWNDPREV) }.ok().filter(|a| *a != ov);
    unsafe {
        let _ = SetWindowPos(ov, Some(above.unwrap_or(HWND_TOP)), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    s.shown = true;
}

/// A követett ablak látható kerete, és hogy belül kell-e rajzolni (teljes méretű ablak).
/// `None`, ha nem kell keret (elrejtett, kis méretre tett, másik asztalon lévő, teljes
/// képernyős ablak).
fn frame_of(target: HWND) -> Option<(RECT, bool)> {
    unsafe {
        if !IsWindow(Some(target)).as_bool() || !IsWindowVisible(target).as_bool() || IsIconic(target).as_bool() {
            return None;
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(target, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        if cloaked != 0 {
            return None;
        }
        let mut r = RECT::default();
        if DwmGetWindowAttribute(target, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut RECT as *mut _, std::mem::size_of::<RECT>() as u32)
            .is_err()
        {
            GetWindowRect(target, &mut r).ok()?;
        }
        if r.right - r.left < 40 || r.bottom - r.top < 40 {
            return None;
        }
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mon = MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST);
        let _ = GetMonitorInfoW(mon, &mut mi);
        let m = mi.rcMonitor;
        // teljes képernyős program (játék, videó): nincs keret
        if r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom {
            return None;
        }
        // teljes méretű ablak, vagy a munkaterületet kitöltő: a keret belül, hogy látsszon
        let w = mi.rcWork;
        let fills = r.left <= w.left && r.top <= w.top && r.right >= w.right && r.bottom >= w.bottom;
        Some((r, IsZoomed(target).as_bool() || fills))
    }
}

/// A keret kirajzolása: `line_at` képpontnyira a réteg szélétől egy telt vonal, körülötte
/// kifelé és befelé halványuló fény.
fn draw(ov: HWND, x: i32, y: i32, w: i32, h: i32, line_at: i32, (r, g, b): (u8, u8, u8)) {
    if w <= 0 || h <= 0 {
        return;
    }
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(Some(screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            return;
        };
        let old = SelectObject(mem, bmp.into());
        let px = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
        // a széltől mért távolság szerinti átlátszatlanság (előre kiszámolva)
        let band = line_at + LINE + GLOW;
        let alpha: Vec<u32> = (0..band)
            .map(|e| {
                let d = e - line_at;
                if (0..LINE).contains(&d) {
                    LINE_ALPHA
                } else {
                    let dist = if d < 0 { -d } else { d - LINE + 1 } as f64;
                    let t = (1.0 - dist / (GLOW as f64 + 1.0)).max(0.0);
                    (GLOW_ALPHA * t * t).round() as u32
                }
            })
            .collect();
        let color = |a: u32| -> u32 {
            let m = |c: u8| (c as u32 * a + 127) / 255;
            (a << 24) | (m(r) << 16) | (m(g) << 8) | m(b)
        };
        let lut: Vec<u32> = alpha.iter().map(|a| color(*a)).collect();
        for yy in 0..h {
            let ey = yy.min(h - 1 - yy);
            let row = &mut px[(yy * w) as usize..((yy + 1) * w) as usize];
            if ey < band {
                for (xx, p) in row.iter_mut().enumerate() {
                    let xx = xx as i32;
                    let e = ey.min(xx).min(w - 1 - xx);
                    *p = lut[e as usize];
                }
            } else {
                for xx in (0..band.min(w)).chain((w - band).max(0)..w) {
                    let e = xx.min(w - 1 - xx);
                    if e < band {
                        row[xx as usize] = lut[e as usize];
                    }
                }
            }
        }
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        let _ = UpdateLayeredWindow(
            ov,
            Some(screen),
            Some(&POINT { x, y }),
            Some(&SIZE { cx: w, cy: h }),
            Some(mem),
            Some(&POINT { x: 0, y: 0 }),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
    }
}
