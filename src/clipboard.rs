//! Vágólap-előzmények: az utoljára másolt szövegek a Start menü keresőjéhez.
//!
//! Külön szálon egy csak üzenetekre szolgáló (láthatatlan) ablak figyeli a vágólap
//! változását (AddClipboardFormatListener). Csak szöveget tárolunk, legfeljebb 30-at, és
//! csak a memóriában: a program bezárásával az előzmények elvesznek. Amit a másoló
//! program rejtettnek jelöl (a jelszókezelők így tesznek), azt nem tároljuk – ugyanazokat
//! a jelzéseket vesszük figyelembe, mint a Windows saját vágólap-előzményei (Win+V).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner, IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId, HWND_MESSAGE, MSG, RegisterClassW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WNDCLASSW,
};
use windows::core::{HSTRING, w};

/// Egy tárolt vágólap-elem.
#[derive(Clone, Debug)]
pub struct Clip {
    pub id: u64,
    pub text: String,
    /// Mikor másolták (Unix-idő, ezredmásodperc).
    pub at: u64,
    /// A másoló program exe-je (ha kideríthető).
    pub source: String,
}

const MAX_ITEMS: usize = 30;
/// Ennél hosszabb szöveget nem tárolunk (pl. egy teljes fájl tartalma).
const MAX_CHARS: usize = 200_000;
const CF_UNICODETEXT: u32 = 13;

static HISTORY: Mutex<Vec<Clip>> = Mutex::new(Vec::new());
static ENABLED: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NOTIFY: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Be- és kikapcsolás (a beállításokból). Kikapcsoláskor az előzmények törlődnek.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on {
        clear();
    }
}

/// A figyelés indítása; `notify` minden új elem után hívódik (a figyelő szálon).
pub fn start(notify: impl Fn() + Send + Sync + 'static) {
    if NOTIFY.set(Box::new(notify)).is_err() {
        return;
    }
    std::thread::spawn(|| unsafe {
        let Ok(module) = GetModuleHandleW(None) else { return };
        let class = w!("CyberShellClipboard");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: module.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(module.into()),
            None,
        ) else {
            return;
        };
        if AddClipboardFormatListener(hwnd).is_err() {
            crate::log("vágólap-figyelés: HIBA, nem indult el");
            return;
        }
        crate::log("vágólap-figyelés elindult");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        if ENABLED.load(Ordering::Relaxed)
            && capture()
            && let Some(f) = NOTIFY.get()
        {
            f();
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn format_id(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) }
}

fn has_format(f: u32) -> bool {
    f != 0 && unsafe { IsClipboardFormatAvailable(f) }.is_ok()
}

/// A vágólap megnyitása; ha épp egy másik program tartja nyitva, néhányszor újrapróbáljuk.
fn open() -> bool {
    for _ in 0..8 {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    false
}

/// A program kérte-e, hogy ez a tartalom ne kerüljön előzményekbe.
fn excluded() -> bool {
    if has_format(format_id("ExcludeClipboardContentFromMonitorProcessing")) || has_format(format_id("Clipboard Viewer Ignore")) {
        return true;
    }
    // „CanIncludeInClipboardHistory” = 0: nem kerülhet előzményekbe
    let f = format_id("CanIncludeInClipboardHistory");
    has_format(f) && read_global(f, |p, n| n >= 4 && unsafe { *(p as *const u32) } == 0).unwrap_or(false)
}

/// Egy vágólap-formátum adatának olvasása (a vágólapnak nyitva kell lennie).
fn read_global<T>(format: u32, f: impl FnOnce(*const u8, usize) -> T) -> Option<T> {
    unsafe {
        let h = GetClipboardData(format).ok()?;
        let g = HGLOBAL(h.0);
        let p = GlobalLock(g) as *const u8;
        if p.is_null() {
            return None;
        }
        let r = f(p, GlobalSize(g));
        let _ = GlobalUnlock(g);
        Some(r)
    }
}

/// Az aktuális vágólap-tartalom felvétele az előzmények elejére. Igaz, ha változott a lista.
fn capture() -> bool {
    if !has_format(CF_UNICODETEXT) {
        return false;
    }
    let source = unsafe {
        GetClipboardOwner()
            .ok()
            .filter(|h| !h.is_invalid())
            .map(|h| {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, Some(&mut pid));
                pid
            })
            .filter(|&pid| pid != 0 && pid != std::process::id())
            .and_then(crate::wintrack::path_of_pid)
            .unwrap_or_default()
    };
    if !open() {
        return false;
    }
    let text = if excluded() {
        None
    } else {
        read_global(CF_UNICODETEXT, |p, n| {
            let units = unsafe { std::slice::from_raw_parts(p as *const u16, n / 2) };
            let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
            String::from_utf16_lossy(&units[..end])
        })
    };
    unsafe {
        let _ = CloseClipboard();
    }
    let Some(text) = text.filter(|t| !t.trim().is_empty() && t.chars().count() <= MAX_CHARS) else { return false };
    let Ok(mut h) = HISTORY.lock() else { return false };
    // ugyanaz a szöveg újra: előre kerül (a forrása megmarad, ha most nem ismert)
    let mut source = source;
    if let Some(i) = h.iter().position(|c| c.text == text) {
        let old = h.remove(i);
        if source.is_empty() {
            source = old.source;
        }
    }
    h.insert(0, Clip { id: NEXT_ID.fetch_add(1, Ordering::Relaxed), text, at: crate::unix_ms(), source });
    h.truncate(MAX_ITEMS);
    true
}

/// Az előzmények, a legutóbbi elöl.
pub fn list() -> Vec<Clip> {
    HISTORY.lock().map(|h| h.clone()).unwrap_or_default()
}

pub fn remove(id: u64) {
    if let Ok(mut h) = HISTORY.lock() {
        h.retain(|c| c.id != id);
    }
}

pub fn clear() {
    if let Ok(mut h) = HISTORY.lock() {
        h.clear();
    }
}

/// Egy elem visszatétele a vágólapra. A figyelő ezt is észleli, és az elem előre kerül.
pub fn copy(id: u64) -> bool {
    let Some(text) = HISTORY.lock().ok().and_then(|h| h.iter().find(|c| c.id == id).map(|c| c.text.clone())) else { return false };
    set_text(&text)
}

/// Szöveg a vágólapra (Unicode).
pub fn set_text(text: &str) -> bool {
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    if !open() {
        return false;
    }
    let ok = unsafe {
        let _ = EmptyClipboard();
        match GlobalAlloc(GMEM_MOVEABLE, units.len() * 2) {
            Ok(g) => {
                let p = GlobalLock(g) as *mut u16;
                if p.is_null() {
                    false
                } else {
                    std::ptr::copy_nonoverlapping(units.as_ptr(), p, units.len());
                    let _ = GlobalUnlock(g);
                    // sikeres átadás után a memória a vágólapé
                    SetClipboardData(CF_UNICODETEXT, Some(HANDLE(g.0))).is_ok()
                }
            }
            Err(_) => false,
        }
    };
    unsafe {
        let _ = CloseClipboard();
    }
    ok
}
