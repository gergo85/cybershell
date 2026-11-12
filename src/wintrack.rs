//! Futó programok ablakainak figyelése, indítás és előtérbe hozás,
//! valamint a nem kitűzött programok ikonja és neve.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, HGDIOBJ, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_MENU, keybd_event};
use windows::Win32::UI::Shell::{SHDefExtractIconW, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, EnumWindows, GA_ROOTOWNER, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetAncestor, GetClassNameW, GetIconInfo, GetWindow,
    GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, HICON, HWND_NOTOPMOST, HWND_TOPMOST, ICONINFO, IsIconic, IsWindowVisible,
    SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOWMAXIMIZED, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetWindowPos, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX,
};
use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR};

use crate::config::AppEntry;

#[derive(Clone, Debug)]
pub struct WinInfo {
    pub hwnd: HWND,
    /// A program teljes elérési útja (eredeti kis- és nagybetűkkel).
    pub path: String,
    /// Az exe neve kisbetűvel (pl. "chrome.exe").
    pub exe: String,
    /// Az ablak címe kisbetűvel (párosításhoz).
    pub title: String,
    /// Az ablak címe eredeti formájában (megjelenítéshez).
    pub title_raw: String,
    pub iconic: bool,
    pub maximizable: bool,
}

/// Egy app párosítási szabályai előre feldolgozva.
pub struct Matcher {
    exes: Vec<String>,
    title: String,
}

impl Matcher {
    pub fn new(app: &AppEntry) -> Self {
        Matcher { exes: app.exe_names(), title: app.match_title.to_lowercase() }
    }
    pub fn matches(&self, w: &WinInfo) -> bool {
        (!w.exe.is_empty() && self.exes.contains(&w.exe)) || (!self.title.is_empty() && w.title.contains(&self.title))
    }
}

const IGNORED_CLASSES: &[&str] = &["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

unsafe extern "system" fn collect(hwnd: HWND, lp: LPARAM) -> BOOL {
    let v = unsafe { &mut *(lp.0 as *mut Vec<HWND>) };
    v.push(hwnd);
    BOOL(1)
}

pub fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn title_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn path_of_pid(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Olyan ablak-e, amilyet a tálca is mutatna.
fn is_app_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        if GetWindow(hwnd, GW_OWNER).map(|o| !o.is_invalid()).unwrap_or(false) {
            return false;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 && ex & WS_EX_APPWINDOW.0 == 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0 {
            return false;
        }
    }
    let class = class_name(hwnd);
    !IGNORED_CLASSES.contains(&class.as_str())
}

#[derive(Default)]
pub struct Tracker {
    pid_cache: HashMap<u32, String>,
    polls: u32,
}

impl Tracker {
    /// Az összes látható alkalmazásablak z-sorrendben (legfelső elöl).
    pub fn snapshot(&mut self, own: &[HWND]) -> Vec<WinInfo> {
        self.polls += 1;
        if self.polls.is_multiple_of(120) {
            self.pid_cache.clear();
        }
        let mut all: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut all as *mut Vec<HWND> as isize));
        }
        let mut out = Vec::new();
        for hwnd in all {
            if own.contains(&hwnd) || !is_app_window(hwnd) {
                continue;
            }
            let title_raw = title_of(hwnd);
            if title_raw.is_empty() {
                continue;
            }
            let mut pid = 0u32;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
            }
            let path = match self.pid_cache.get(&pid) {
                Some(p) => p.clone(),
                None => {
                    let p = path_of_pid(pid).unwrap_or_default();
                    self.pid_cache.insert(pid, p.clone());
                    p
                }
            };
            let exe = Path::new(&path).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
            out.push(WinInfo {
                hwnd,
                path,
                exe,
                title: title_raw.to_lowercase(),
                title_raw,
                iconic: unsafe { IsIconic(hwnd) }.as_bool(),
                maximizable: style & WS_MAXIMIZEBOX.0 != 0,
            });
        }
        out
    }
}

pub fn root_owner(hwnd: HWND) -> HWND {
    let r = unsafe { GetAncestor(hwnd, GA_ROOTOWNER) };
    if r.is_invalid() { hwnd } else { r }
}

pub fn activate(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            // A Windows néha megtagadja az előtérbe hozást; egy Alt-lenyomás feloldja a zárat.
            keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_EXTENDEDKEY, 0);
            keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Fájlhúzás közben: az ablakot a többi fölé hozza, hogy rá lehessen ejteni a
/// fájlt. Billentyűt nem szimulál, mert az Alt megváltoztatná a húzás módját.
pub fn raise_for_drop(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
        let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        let _ = SetForegroundWindow(hwnd);
    }
}

pub const VK_WIN: u8 = 0x5B;
pub const VK_CONTROL: u8 = 0x11;

/// Billentyűkombináció lenyomása (sorban le, fordított sorrendben fel): a Windows saját
/// Start menüjéhez (Win) és keresőjéhez (Win+S), ha a saját menü ki van kapcsolva.
pub fn press_keys(keys: &[u8]) {
    let flags = |vk: u8| if vk == VK_WIN { KEYEVENTF_EXTENDEDKEY } else { Default::default() };
    unsafe {
        for &vk in keys {
            keybd_event(vk, 0, flags(vk), 0);
        }
        for &vk in keys.iter().rev() {
            keybd_event(vk, 0, flags(vk) | KEYEVENTF_KEYUP, 0);
        }
    }
}

pub fn minimize(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
}

pub fn maximize(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_MAXIMIZE);
    }
}

fn opt(s: &str) -> Option<HSTRING> {
    if s.trim().is_empty() { None } else { Some(HSTRING::from(s)) }
}

fn pcwstr(h: &Option<HSTRING>) -> PCWSTR {
    match h {
        Some(s) => PCWSTR(s.as_ptr()),
        None => PCWSTR::null(),
    }
}

/// Indítás a Windows héjon keresztül (exe, mappa, URI, Store-app egyaránt).
pub fn shell_open(file: &str, args: &str, cwd: &str, maximized: bool) -> bool {
    let file = HSTRING::from(file);
    let args = opt(args);
    let cwd = opt(cwd);
    let show = if maximized { SW_SHOWMAXIMIZED } else { SW_SHOWNORMAL };
    let r = unsafe { ShellExecuteW(None, &HSTRING::from("open"), &file, pcwstr(&args), pcwstr(&cwd), show) };
    r.0 as isize > 32
}

/// Fájlok megnyitása egy adott programmal (az ikonra ejtett fájlokhoz).
pub fn open_with(exe: &str, files: &[String]) -> bool {
    let args = files.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(" ");
    shell_open(exe, &args, "", false)
}

// ---- Nem kitűzött programok neve és ikonja ----

/// A program megjelenítendő neve: a fájl leírása (pl. „Jegyzettömb”), ha nincs,
/// a fájlnév kiterjesztés nélkül. Az Electronra és hasonló keretrendszerekre épülő
/// programoknál (pl. GitHub Desktop, Discord) a leírás gyakran csak a keretrendszer
/// neve („Electron”) – ilyenkor a termék neve (ProductName) a valódi név.
pub fn display_name(path: &str) -> String {
    let info = version_strings(path, &["FileDescription", "ProductName"]);
    let usable = |s: &Option<String>| s.clone().filter(|t| !is_framework_name(t));
    usable(&info[0]).or_else(|| usable(&info[1])).unwrap_or_else(|| {
        let stem = Path::new(path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut c = stem.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => stem,
        }
    })
}

/// Keretrendszer- vagy futtatókörnyezet-név, ami nem a program saját neve.
fn is_framework_name(name: &str) -> bool {
    const GENERIC: [&str; 9] =
        ["electron", "nw.js", "node.js", "node", "cef", "chromium", "chromium embedded framework", "tauri app", "webview2"];
    let n = name.trim().to_lowercase();
    GENERIC.contains(&n.as_str())
}

/// A fájl verzióadatainak szöveges mezői (pl. FileDescription, ProductName) a megadott sorrendben.
fn version_strings(path: &str, keys: &[&str]) -> Vec<Option<String>> {
    let none = || keys.iter().map(|_| None).collect();
    unsafe {
        let file = HSTRING::from(path);
        let size = GetFileVersionInfoSizeW(&file, None);
        if size == 0 {
            return none();
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(&file, None, size, data.as_mut_ptr() as *mut _).is_err() {
            return none();
        }
        // nyelv és kódlap a fordítási táblából
        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let mut lang = (0x0409u16, 0x04B0u16);
        if VerQueryValueW(data.as_ptr() as *const _, &HSTRING::from("\\VarFileInfo\\Translation"), &mut ptr, &mut len).as_bool() && len >= 4
        {
            let t = std::slice::from_raw_parts(ptr as *const u16, 2);
            lang = (t[0], t[1]);
        }
        keys.iter()
            .map(|k| {
                let key = format!("\\StringFileInfo\\{:04x}{:04x}\\{k}", lang.0, lang.1);
                let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
                let mut len = 0u32;
                if !VerQueryValueW(data.as_ptr() as *const _, &HSTRING::from(key), &mut ptr, &mut len).as_bool() || len == 0 {
                    return None;
                }
                let s = std::slice::from_raw_parts(ptr as *const u16, len as usize);
                let text = String::from_utf16_lossy(s).trim_end_matches('\0').trim().to_string();
                (!text.is_empty()).then_some(text)
            })
            .collect()
    }
}

static ICON_CACHE: Mutex<Option<HashMap<String, Option<Vec<u8>>>>> = Mutex::new(None);

/// A program ikonja PNG-ként (48 px), gyorsítótárazva.
pub fn icon_png(path: &str) -> Option<Vec<u8>> {
    let key = path.to_lowercase();
    if let Ok(g) = ICON_CACHE.lock()
        && let Some(hit) = g.as_ref().and_then(|m| m.get(&key))
    {
        return hit.clone();
    }
    let png = extract_icon(path);
    if let Ok(mut g) = ICON_CACHE.lock() {
        g.get_or_insert_with(HashMap::new).insert(key, png.clone());
    }
    png
}

fn extract_icon(path: &str) -> Option<Vec<u8>> {
    unsafe {
        let mut icon = HICON::default();
        let size = 48u32;
        let hr = SHDefExtractIconW(&HSTRING::from(path), 0, 0, Some(&mut icon), None, size | (16 << 16));
        if hr.is_err() || icon.is_invalid() {
            return None;
        }
        let png = hicon_to_png(icon);
        let _ = DestroyIcon(icon);
        png
    }
}

unsafe fn hicon_to_png(icon: HICON) -> Option<Vec<u8>> {
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(icon, &mut info).ok()?;
        let color = info.hbmColor;
        let mask = info.hbmMask;
        let cleanup = || {
            if !color.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(color.0));
            }
            if !mask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(mask.0));
            }
        };
        if color.is_invalid() {
            cleanup();
            return None;
        }
        let mut bm = BITMAP::default();
        GetObjectW(HGDIOBJ(color.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 256 || h > 256 {
            cleanup();
            return None;
        }
        let mut bi = BITMAPINFO {
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
        let mut px = vec![0u8; (w * h * 4) as usize];
        let hdc = GetDC(None);
        let lines = GetDIBits(hdc, color, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        ReleaseDC(None, hdc);
        cleanup();
        if lines == 0 {
            return None;
        }
        // BGRA → RGBA; régi ikonoknál nincs alfa, ott teljesen átlátszatlan
        let has_alpha = px.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
        for p in px.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            }
        }
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().ok()?;
            writer.write_image_data(&px).ok()?;
        }
        Some(out)
    }
}

/// Megnyitás adott művelettel (pl. "runas": futtatás rendszergazdaként).
pub fn shell_verb(verb: &str, file: &str, args: &str) -> bool {
    let args = opt(args);
    let r = unsafe { ShellExecuteW(None, &HSTRING::from(verb), &HSTRING::from(file), pcwstr(&args), PCWSTR::null(), SW_SHOWNORMAL) };
    r.0 as isize > 32
}
