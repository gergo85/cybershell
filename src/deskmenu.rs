//! Saját asztali menü: az asztal üres területére, illetve (külön kapcsolóval) az asztali
//! ikonokra tett jobb kattintás a Windows menüje helyett a CyberShell menüjét nyitja meg,
//! ugyanazokkal a parancsokkal.
//!
//! Két háttérszál dolgozik. Az egérfigyelő (`WH_MOUSE_LL`, saját üzenetsorral) csak akkor
//! nyeli el a jobb gombot, ha az az asztal ikonlistája fölött kattint (üres helyen vagy ikonon,
//! amelyik be van kapcsolva): ezt az ikonlista saját találatvizsgálata (`LVM_HITTEST`, rövid
//! időkorláttal) dönti el. Ha a vizsgálat nem sikerül időben, a kattintás érintetlenül
//! továbbmegy, és a Windows menüje jön. Ikonon a jobb gombos húzás a Windowsé marad: ha az egér
//! lenyomva elmozdul, a lenyomást az ikon helyén újraküldjük, és a húzás onnan folytatódik.
//! A menüszál az Intéző saját menüjét kéri el (az asztal nézetének háttér-, illetve a kijelölt
//! elemekhez tartozó `IContextMenu`-ját), egy rejtett menübe tölti, és innen olvassa ki a
//! feliratokat, az almenüket, a pipákat és az ikonokat; a választott parancsot is ő futtatja.
//! Ha a menü nem készíthető el, a kattintást újraküldjük, és akkor a Windows sajátja nyílik meg.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};

use serde_json::{Value, json};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{HBITMAP, ScreenToClient};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, IServiceProvider};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    CMF_CANRENAME, CMF_EXTENDEDVERBS, CMF_NORMAL, CMIC_MASK_PTINVOKE, CMIC_MASK_SHIFT_DOWN, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX,
    CSIDL_DESKTOP, GCS_VERBA, IContextMenu, IContextMenu2, IFolderView, IShellBrowser, IShellView, IShellWindows, SID_STopLevelBrowser,
    SVGIO_BACKGROUND, SVGIO_SELECTION, SVSI_DESELECTOTHERS, SVSI_EDIT, SVSI_FOCUSED, SVSI_SELECT, SWC_DESKTOP, SWFO_NEEDDISPATCH,
    ShellWindows,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreatePopupMenu, DestroyMenu, GA_ROOT, GetAncestor, GetClassNameW, GetMenuItemCount, GetMenuItemInfoW, GetMessageW,
    GetParent, GetSystemMetrics, GetWindowThreadProcessId, HHOOK, HMENU, LLMHF_INJECTED, MENUITEMINFOW, MFS_CHECKED, MFS_DEFAULT,
    MFS_DISABLED, MFT_OWNERDRAW, MFT_RADIOCHECK, MFT_SEPARATOR, MIIM_BITMAP, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
    MSG, MSLLHOOKSTRUCT, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SM_CXDRAG, SM_CXVIRTUALSCREEN, SM_CYDRAG, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, SW_SHOWNORMAL, SendMessageTimeoutW, SetCursorPos,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL, WM_APP, WM_INITMENUPOPUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WindowFromPoint,
};
use windows::core::{Interface, PCSTR, PCWSTR, PSTR, PWSTR};

use crate::com::ComThread;
use crate::log;

/// Be van-e kapcsolva az asztal üres területén, illetve az ikonokon (a beállításokból állítható).
static ENABLED: AtomicBool = AtomicBool::new(false);
static ICONS_ON: AtomicBool = AtomicBool::new(false);
/// A fő szál legutóbbi életjele (`GetTickCount64`, ms): ha régi, nem nyelünk el kattintást.
static ALIVE: AtomicU64 = AtomicU64::new(0);
/// A jobb gomb lenyomását elnyeltük: a felengedését is el kell nyelni, és akkor nyílik a menü.
static EAT_UP: AtomicBool = AtomicBool::new(false);
/// Az elnyelt lenyomás helye (a menü itt nyílik meg).
static DOWN_X: AtomicI32 = AtomicI32::new(0);
static DOWN_Y: AtomicI32 = AtomicI32::new(0);
/// Az elnyelt lenyomás alatti ikon sorszáma az ikonlistában (-1: üres terület). Ikonnál a
/// felengedésig figyeljük, elmozdul-e az egér (akkor húzás, és a Windowsnak adjuk vissza).
static DOWN_ITEM: AtomicI32 = AtomicI32::new(-1);
/// Az egérfigyelő szálának azonosítója (a ki- és bekapcsoláskor neki szólunk).
static HOOK_TID: AtomicU32 = AtomicU32::new(0);
/// Az elnyelt kattintás jelzése a fő szálnak: hely, a Shift állapota és az ikon sorszáma
/// (-1: üres terület).
type Fire = Box<dyn Fn(i32, i32, bool, i32) + Send + Sync>;
static FIRE: OnceLock<Fire> = OnceLock::new();
static WORKER: OnceLock<Mutex<Sender<Cmd>>> = OnceLock::new();
/// A nyitott menü elemeinek ikonjai PNG-ként (a felület `/__deskicon/<menü>/<sorszám>` címen kéri).
static ICONS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Ennyi ideig várunk az ikonlista találatvizsgálatára. A Windows a lassú egérfigyelőt
/// kihagyja, többszöri túllépés után pedig csendben eltávolítja, ezért ez rövid.
const HIT_TIMEOUT_MS: u32 = 80;
/// Ha a fő szál ennél régebben jelentkezett, a kattintás a Windowsé.
const ALIVE_MS: u64 = 2000;
/// A menüparancsok azonosítóinak tartománya (`QueryContextMenu`).
const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x7FFF;
/// A menü legmélyebb almenüje (a gyári menüben kettő a legtöbb).
const MAX_DEPTH: u32 = 4;

/// Az ikonlista találatvizsgálatának adatai (`LVHITTESTINFO`).
#[repr(C)]
#[derive(Clone, Copy)]
struct HitTestInfo {
    pt: POINT,
    flags: u32,
    item: i32,
    sub_item: i32,
    group: i32,
}
const LVM_HITTEST: u32 = 0x1012;
const LVM_GETITEMSTATE: u32 = 0x102C;
const LVIS_SELECTED: usize = 2;

enum Cmd {
    /// Az első kérés lassú (a héj bővítményei ekkor töltődnek be): bekapcsoláskor előre elvégezzük.
    Warm,
    /// A menü elkészítése; `item`: az ikon sorszáma az ikonlistában (-1: az asztal háttérmenüje).
    Open {
        seq: u64,
        x: i32,
        y: i32,
        shift: bool,
        item: i32,
    },
    /// Egy almenü újraolvasása (a Küldés almenüjét a Windows késleltetve tölti fel).
    Sub {
        seq: u64,
        menu: usize,
    },
    Invoke {
        seq: u64,
        id: u32,
    },
    Release {
        seq: u64,
    },
}

/// A nyitott menü: a héj menüje, a rejtett menü, amibe betöltötte, és a futtatás adatai.
struct Current {
    seq: u64,
    cm: IContextMenu,
    menu: HMENU,
    /// Az almenük (a felület a sorszámukkal kéri újra őket).
    subs: Vec<HMENU>,
    view: IShellView,
    owner: HWND,
    pt: POINT,
    shift: bool,
    /// Az ikon sorszáma, amelyikre kattintottak (-1: háttérmenü).
    item: i32,
}

impl Drop for Current {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.menu);
        }
    }
}

fn now_ms() -> u64 {
    unsafe { GetTickCount64() }
}

/// A saját menü be- és kikapcsolása az asztal üres területén (`blank`) és az ikonokon (`icons`).
pub fn set_enabled(blank: bool, icons: bool) {
    let was_blank = ENABLED.swap(blank, Ordering::Relaxed);
    let was = ICONS_ON.swap(icons, Ordering::Relaxed) || was_blank;
    let on = blank || icons;
    if !on {
        EAT_UP.store(false, Ordering::Relaxed);
        DOWN_ITEM.store(-1, Ordering::Relaxed);
    }
    let tid = HOOK_TID.load(Ordering::Relaxed);
    if tid != 0 && was != on {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_APP, WPARAM(0), LPARAM(0));
        }
    }
    if on && !was {
        to_worker(Cmd::Warm);
    }
}

/// A fő szál életjele (minden körben hívja).
pub fn alive() {
    ALIVE.store(now_ms(), Ordering::Relaxed);
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) }.max(0) as usize;
    String::from_utf16_lossy(&buf[..n])
}

/// Mi van a pont alatt az asztalon: `Some(-1)` üres terület, `Some(n)` az ikonlista n-edik
/// ikonja; `None`, ha nem az asztal, vagy nem dönthető el (pl. az Intéző nem válaszol időben).
fn desktop_target(pt: POINT) -> Option<i32> {
    let h = unsafe { WindowFromPoint(pt) };
    if h.is_invalid() {
        return None;
    }
    let (list, view) = match class_of(h).as_str() {
        "SysListView32" => (Some(h), unsafe { GetParent(h) }.ok()?),
        // elrejtett asztali ikonoknál maga a nézet kapja a kattintást
        "SHELLDLL_DefView" => (None, h),
        _ => return None,
    };
    if class_of(view) != "SHELLDLL_DefView" {
        return None;
    }
    let root = unsafe { GetAncestor(view, GA_ROOT) };
    if !matches!(class_of(root).as_str(), "Progman" | "WorkerW") {
        return None;
    }
    match list {
        None => Some(-1),
        Some(list) => hit_item(list, pt),
    }
}

/// Melyik ikonra mutat a pont az ikonlistában (-1: egyikre sem; `None`: nem sikerült
/// megállapítani). A lista az Intéző folyamatában van, ezért a vizsgálat adatai ott, egy kis
/// lefoglalt területen utaznak.
fn hit_item(list: HWND, pt: POINT) -> Option<i32> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(list, Some(&mut pid));
        let proc = OpenProcess(PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE, false, pid).ok()?;
        let size = std::mem::size_of::<HitTestInfo>();
        let mem = VirtualAllocEx(proc, None, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        let mut result = None;
        if !mem.is_null() {
            let mut client = pt;
            let _ = ScreenToClient(list, &mut client);
            let info = HitTestInfo { pt: client, flags: 0, item: -1, sub_item: 0, group: 0 };
            if WriteProcessMemory(proc, mem, &info as *const _ as *const _, size, None).is_ok() {
                let mut res = 0usize;
                let flags = SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT;
                let ok = SendMessageTimeoutW(list, LVM_HITTEST, WPARAM(0), LPARAM(mem as isize), flags, HIT_TIMEOUT_MS, Some(&mut res));
                if ok.0 != 0 {
                    result = Some((res as u32 as i32).max(-1));
                }
            }
            let _ = VirtualFreeEx(proc, mem, 0, MEM_RELEASE);
        }
        let _ = CloseHandle(proc);
        result
    }
}

fn mouse(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32) -> INPUT {
    INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
}

/// Egérmozgatás abszolút helyre (az egész asztal koordinátáiban) a `SendInput`-nak.
fn move_to(x: i32, y: i32) -> INPUT {
    let (vx, vy) = unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN)) };
    let (vw, vh) = unsafe { (GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2), GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2)) };
    let nx = ((x - vx) as i64 * 65535 / (vw - 1) as i64) as i32;
    let ny = ((y - vy) as i64 * 65535 / (vh - 1) as i64) as i32;
    mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK, nx, ny)
}

unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pass = || unsafe { CallNextHookEx(None, code, wp, lp) };
    let msg = wp.0 as u32;
    // egérmozgásnál csak akkor van dolgunk, ha egy ikon fölött elnyelt lenyomás után vagyunk
    if code < 0 || (msg == WM_MOUSEMOVE && DOWN_ITEM.load(Ordering::Relaxed) < 0) {
        return pass();
    }
    if msg != WM_RBUTTONDOWN && msg != WM_RBUTTONUP && msg != WM_MOUSEMOVE {
        return pass();
    }
    let m = unsafe { &*(lp.0 as *const MSLLHOOKSTRUCT) };
    // a programok (és a saját újraküldésünk) kattintásait nem bántjuk
    if m.flags & LLMHF_INJECTED != 0 {
        return pass();
    }
    let (dx, dy) = (DOWN_X.load(Ordering::Relaxed), DOWN_Y.load(Ordering::Relaxed));
    if msg == WM_MOUSEMOVE {
        // jobb gombos húzás egy ikonról: a lenyomást az ikon helyén visszaadjuk a Windowsnak,
        // és a húzás onnan folytatódik (az „Áthelyezés ide / Másolás ide” menü a Windowsé)
        let (tx, ty) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
        if (m.pt.x - dx).abs() > tx || (m.pt.y - dy).abs() > ty {
            DOWN_ITEM.store(-1, Ordering::Relaxed);
            EAT_UP.store(false, Ordering::Relaxed);
            let seq = [move_to(dx, dy), mouse(MOUSEEVENTF_RIGHTDOWN, 0, 0), move_to(m.pt.x, m.pt.y)];
            unsafe {
                SendInput(&seq, std::mem::size_of::<INPUT>() as i32);
            }
            return LRESULT(1);
        }
        return pass();
    }
    if msg == WM_RBUTTONUP {
        let item = DOWN_ITEM.swap(-1, Ordering::Relaxed);
        if EAT_UP.swap(false, Ordering::Relaxed) {
            let shift = unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) } < 0;
            if let Some(f) = FIRE.get() {
                f(dx, dy, shift, item);
            }
            return LRESULT(1);
        }
        return pass();
    }
    EAT_UP.store(false, Ordering::Relaxed);
    DOWN_ITEM.store(-1, Ordering::Relaxed);
    let blank_on = ENABLED.load(Ordering::Relaxed);
    let icons_on = ICONS_ON.load(Ordering::Relaxed);
    if !(blank_on || icons_on) || now_ms().saturating_sub(ALIVE.load(Ordering::Relaxed)) > ALIVE_MS {
        return pass();
    }
    match desktop_target(m.pt) {
        Some(item) if (item < 0 && blank_on) || (item >= 0 && icons_on) => {
            DOWN_X.store(m.pt.x, Ordering::Relaxed);
            DOWN_Y.store(m.pt.y, Ordering::Relaxed);
            DOWN_ITEM.store(item, Ordering::Relaxed);
            EAT_UP.store(true, Ordering::Relaxed);
            LRESULT(1)
        }
        _ => pass(),
    }
}

/// A figyelő és a menüszál indítása. A `fire` az elnyelt kattintás helyét, a Shift állapotát
/// és az ikon sorszámát kapja (a figyelő szálán), a `ready` az elkészült menüt, illetve egy
/// újraolvasott almenü sorszámát és elemeit (a menüszálon).
pub fn start(fire: impl Fn(i32, i32, bool, i32) + Send + Sync + 'static, ready: impl Fn(u64, Option<usize>, Value) + Send + 'static) {
    if FIRE.set(Box::new(fire)).is_err() {
        return;
    }
    let (tx, rx) = channel::<Cmd>();
    let _ = WORKER.set(Mutex::new(tx));
    std::thread::spawn(move || worker(rx, ready));
    if ENABLED.load(Ordering::Relaxed) || ICONS_ON.load(Ordering::Relaxed) {
        to_worker(Cmd::Warm);
    }
    std::thread::spawn(|| unsafe {
        // az üzenetsor létrehozása, hogy a be- és kikapcsolás jelzése ne vesszen el
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        HOOK_TID.store(windows::Win32::System::Threading::GetCurrentThreadId(), Ordering::Relaxed);
        let module = GetModuleHandleW(None).ok().map(|m| m.into());
        let mut hook_handle: Option<HHOOK> = None;
        loop {
            let want = ENABLED.load(Ordering::Relaxed) || ICONS_ON.load(Ordering::Relaxed);
            if want && hook_handle.is_none() {
                match SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module, 0) {
                    Ok(h) => {
                        hook_handle = Some(h);
                        log("asztali menü: egérfigyelés elindítva");
                    }
                    Err(e) => log(format!("asztali menü: az egérfigyelés nem indítható ({e})")),
                }
            } else if !want && let Some(h) = hook_handle.take() {
                let _ = UnhookWindowsHookEx(h);
                log("asztali menü: egérfigyelés leállítva");
            }
            if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                break;
            }
        }
    });
}

fn to_worker(cmd: Cmd) {
    if let Some(w) = WORKER.get()
        && let Ok(w) = w.lock()
    {
        let _ = w.send(cmd);
    }
}

/// A menü elkészítése a kattintás helyén (a kész menüt a `ready` kapja). `item`: az ikon
/// sorszáma az ikonlistában, amelyikre kattintottak (-1: üres terület).
pub fn open(seq: u64, x: i32, y: i32, shift: bool, item: i32) {
    to_worker(Cmd::Open { seq, x, y, shift, item });
}

/// Egy almenü újraolvasása (a `ready` kapja a sorszámával).
pub fn refresh(seq: u64, menu: usize) {
    to_worker(Cmd::Sub { seq, menu });
}

/// A menü egy parancsának futtatása (az elem azonosítójával), utána a menü eldobása.
pub fn invoke(seq: u64, id: u32) {
    to_worker(Cmd::Invoke { seq, id });
}

/// A menü bezárult választás nélkül: eldobjuk.
pub fn release(seq: u64) {
    to_worker(Cmd::Release { seq });
}

/// A nyitott menü egy ikonja PNG-ként.
pub fn icon(n: usize) -> Option<Vec<u8>> {
    ICONS.lock().ok()?.get(n).cloned()
}

/// Az elnyelt kattintás újraküldése (a figyelő átengedi): így a Windows saját menüje nyílik meg.
pub fn replay(x: i32, y: i32) {
    unsafe {
        let _ = SetCursorPos(x, y);
        SendInput(&[mouse(MOUSEEVENTF_RIGHTDOWN, 0, 0), mouse(MOUSEEVENTF_RIGHTUP, 0, 0)], std::mem::size_of::<INPUT>() as i32);
    }
}

/// Az asztal ablaka (Progman vagy WorkerW, amelyikben az ikonlista van): a menü bezárása
/// után ez kapja vissza a fókuszt, ahogy a gyári menünél.
pub fn desktop_window() -> Option<HWND> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;
    use windows::core::w;
    unsafe {
        let mut after: Option<HWND> = None;
        for class in [w!("Progman"), w!("WorkerW")] {
            loop {
                let Ok(top) = FindWindowExW(None, after, class, PCWSTR::null()) else { break };
                if FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null()).is_ok() {
                    return Some(top);
                }
                after = Some(top);
            }
            after = None;
        }
    }
    None
}

/// Az asztal nézete és ablaka.
fn desktop_view() -> windows::core::Result<(IShellView, HWND)> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
        let loc = VARIANT::from(CSIDL_DESKTOP as i32);
        let mut hwnd = 0i32;
        let disp = windows.FindWindowSW(&loc, &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let browser: IShellBrowser = disp.cast::<IServiceProvider>()?.QueryService(&SID_STopLevelBrowser)?;
        Ok((browser.QueryActiveShellView()?, HWND(hwnd as isize as *mut _)))
    }
}

/// Ki van-e jelölve az ikonlista n-edik ikonja (ha nem kérdezhető le: nem).
fn item_selected(item: i32) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;
    use windows::core::w;
    let Some(top) = desktop_window() else { return false };
    unsafe {
        let Ok(view) = FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null()) else { return false };
        let Ok(list) = FindWindowExW(Some(view), None, w!("SysListView32"), PCWSTR::null()) else { return false };
        let mut res = 0usize;
        let flags = SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT;
        let ok =
            SendMessageTimeoutW(list, LVM_GETITEMSTATE, WPARAM(item as usize), LPARAM(LVIS_SELECTED as isize), flags, 500, Some(&mut res));
        ok.0 != 0 && res & LVIS_SELECTED != 0
    }
}

/// A menü elkérése: a háttérmenü (`item` < 0), vagy a kijelölt elemeké. Ikonra kattintva,
/// ahogy a Windowsban: ha az ikon nincs kijelölve, csak ő lesz kijelölve; ha igen, a
/// kijelölés marad (több elem együttes menüje). Üres helyre kattintva a kijelölés megszűnik.
fn desktop_menu(view: &IShellView, item: i32) -> windows::core::Result<IContextMenu> {
    unsafe {
        if item < 0 {
            let _ = view.SelectItem(std::ptr::null(), SVSI_DESELECTOTHERS.0 as u32);
            return view.GetItemObject(SVGIO_BACKGROUND);
        }
        let fv: IFolderView = view.cast()?;
        if item_selected(item) {
            let _ = fv.SelectItem(item, SVSI_FOCUSED.0 as u32);
        } else {
            fv.SelectItem(item, (SVSI_SELECT.0 | SVSI_DESELECTOTHERS.0 | SVSI_FOCUSED.0) as u32)?;
        }
        view.GetItemObject(SVGIO_SELECTION)
    }
}

/// A menü betöltése egy rejtett menübe (Shift lenyomásával a bővített parancsokkal együtt;
/// az elemek menüjében az Átnevezéssel). A visszatérési értéke folyamatok között nem
/// megbízható (az elemek menüje sikeres betöltésnél is hibát jelezhet), ezért azt nézzük,
/// került-e elem a menübe.
fn query(cm: &IContextMenu, shift: bool, items: bool) -> windows::core::Result<HMENU> {
    unsafe {
        let menu = CreatePopupMenu()?;
        let mut flags = CMF_NORMAL | if shift { CMF_EXTENDEDVERBS } else { 0 };
        if items {
            flags |= CMF_CANRENAME;
        }
        let hr = cm.QueryContextMenu(menu, 0, ID_FIRST, ID_LAST, flags);
        if GetMenuItemCount(Some(menu)) <= 0 {
            let _ = DestroyMenu(menu);
            hr.ok()?;
            return Err(windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL));
        }
        Ok(menu)
    }
}

/// Egy parancs belső neve (pl. „rename”). Folyamatok között csak az ANSI alak jön át, és a
/// fenntartott paraméter nem lehet üres.
fn verb_of(cm: &IContextMenu, id: u32) -> String {
    let mut buf = [0u8; 128];
    let reserved = 0u32;
    let ok = unsafe { cm.GetCommandString((id - ID_FIRST) as usize, GCS_VERBA, Some(&reserved), PSTR(buf.as_mut_ptr()), buf.len() as u32) };
    if ok.is_err() {
        return String::new();
    }
    let n = buf.iter().position(|&b| b == 0).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

/// Egy menüelem ikonja PNG-ként (a héj a legtöbb elemhez 32 bites bitképet ad; a rajzolásra
/// visszahívó és a beépített jelek nem használhatók).
fn item_icon(bmp: HBITMAP) -> Option<Vec<u8>> {
    let raw = bmp.0 as isize;
    if (-1..=11).contains(&raw) {
        return None;
    }
    let (w, h) = crate::bitmap::size(bmp);
    let h = h.abs();
    if w <= 0 || h <= 0 || w > 64 || h > 64 {
        return None;
    }
    crate::bitmap::bgra_to_png(crate::bitmap::pixels(bmp, w, h)?, w, h, true)
}

/// Amit a menü kiolvasása gyűjt: az ikonok (PNG) és az almenük (a felület ezek sorszámával
/// kéri újra őket).
#[derive(Default)]
struct Collect {
    icons: Vec<Vec<u8>>,
    subs: Vec<HMENU>,
}

/// A rejtett menü elemei a felületnek: felirat (a `&` a gyorsbetű előtt), billentyűparancs,
/// ikon, pipa vagy rádiógomb, letiltás, alapértelmezett elem és almenü. Az almenüket előbb a
/// héj tölti fel (`init`; az Új és a Küldés almenüje csak megnyitáskor készül el, a Küldésé
/// késleltetve is: ezt a felület megnyitáskor újraolvastatja, akkor már `init` nélkül).
fn read_items(menu: HMENU, cm2: Option<&IContextMenu2>, depth: u32, init: bool, got: &mut Collect) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let count = unsafe { GetMenuItemCount(Some(menu)) }.max(0) as u32;
    for pos in 0..count {
        let mut mii = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_SUBMENU | MIIM_STRING | MIIM_BITMAP,
            ..Default::default()
        };
        if unsafe { GetMenuItemInfoW(menu, pos, true, &mut mii) }.is_err() {
            continue;
        }
        if mii.fType.0 & MFT_SEPARATOR.0 != 0 {
            // elválasztó csak elemek között (az elején, a végén és duplán nem)
            if out.last().is_some_and(|l| l.get("sep").is_none()) {
                out.push(json!({ "sep": true }));
            }
            continue;
        }
        // a saját rajzolású elemeknek nincs olvasható felirata
        if mii.fType.0 & MFT_OWNERDRAW.0 != 0 || mii.cch == 0 {
            continue;
        }
        let mut buf = vec![0u16; mii.cch as usize + 1];
        mii.dwTypeData = PWSTR(buf.as_mut_ptr());
        mii.cch += 1;
        if unsafe { GetMenuItemInfoW(menu, pos, true, &mut mii) }.is_err() {
            continue;
        }
        let text = String::from_utf16_lossy(&buf[..mii.cch as usize]);
        let (label, key) = match text.split_once('\t') {
            Some((l, k)) => (l.to_string(), k.to_string()),
            None => (text, String::new()),
        };
        let state = mii.fState.0;
        let mut item = json!({
            "id": mii.wID,
            "label": label,
            "disabled": state & MFS_DISABLED.0 != 0,
        });
        if !key.is_empty() {
            item["key"] = json!(key);
        }
        if state & MFS_CHECKED.0 != 0 {
            item["check"] = json!(if mii.fType.0 & MFT_RADIOCHECK.0 != 0 { "radio" } else { "check" });
        }
        if state & MFS_DEFAULT.0 != 0 {
            item["default"] = json!(true);
        }
        if let Some(png) = item_icon(mii.hbmpItem) {
            item["icon"] = json!(got.icons.len());
            got.icons.push(png);
        }
        if !mii.hSubMenu.is_invalid() {
            if depth >= MAX_DEPTH {
                continue;
            }
            if init && let Some(c2) = cm2 {
                let _ = unsafe { c2.HandleMenuMsg(WM_INITMENUPOPUP, WPARAM(mii.hSubMenu.0 as usize), LPARAM(pos as isize)) };
            }
            item["menu"] = json!(got.subs.len());
            got.subs.push(mii.hSubMenu);
            let sub = read_items(mii.hSubMenu, cm2, depth + 1, init, got);
            if sub.is_empty() {
                item["disabled"] = json!(true);
            }
            item["sub"] = json!(sub);
        }
        out.push(item);
    }
    if out.last().is_some_and(|l| l.get("sep").is_some()) {
        out.pop();
    }
    out
}

/// A parancs futtatása. Az Átnevezést a nézettől kérjük (az ikon feliratának szerkesztése),
/// mert a menüből futtatva folyamatok között nem indul el.
fn run(c: &Current, id: u32) {
    if c.item >= 0 && verb_of(&c.cm, id) == "rename" {
        let res = c.view.cast::<IFolderView>().and_then(|fv| unsafe {
            let focused = fv.GetFocusedItem().unwrap_or(c.item);
            fv.SelectItem(focused, SVSI_EDIT.0 as u32)
        });
        match res {
            Ok(()) => log("asztali menü: átnevezés"),
            Err(e) => log(format!("asztali menü: az átnevezés nem indítható ({e})")),
        }
        return;
    }
    let offset = (id - ID_FIRST) as usize;
    let mut mask = 0x4000 /* CMIC_MASK_UNICODE */ | CMIC_MASK_PTINVOKE;
    if c.shift {
        mask |= CMIC_MASK_SHIFT_DOWN;
    }
    let ici = CMINVOKECOMMANDINFOEX {
        cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32,
        fMask: mask,
        hwnd: c.owner,
        // a parancs a sorszámával (MAKEINTRESOURCE), nem névvel
        lpVerb: PCSTR(offset as *const u8),
        lpVerbW: PCWSTR(offset as *const u16),
        nShow: SW_SHOWNORMAL.0,
        ptInvoke: c.pt,
        ..Default::default()
    };
    match unsafe { c.cm.InvokeCommand(&ici as *const _ as *const CMINVOKECOMMANDINFO) } {
        Ok(()) => log(format!("asztali menü: parancs futtatva ({id})")),
        Err(e) => log(format!("asztali menü: a parancs nem futtatható ({id}: {e})")),
    }
}

fn worker(rx: std::sync::mpsc::Receiver<Cmd>, ready: impl Fn(u64, Option<usize>, Value)) {
    // A menü az Intéző folyamatában készül el, a hívások oda futnak át: ehhez elég a többszálú
    // COM-tér, és így ennek a szálnak nem kell üzeneteket feldolgoznia.
    let _com = ComThread::init();
    let mut cur: Option<Current> = None;
    for cmd in rx {
        match cmd {
            Cmd::Warm => {
                let t = std::time::Instant::now();
                if let Ok((view, _)) = desktop_view()
                    && let Ok(cm) = unsafe { view.GetItemObject::<IContextMenu>(SVGIO_BACKGROUND) }
                    && let Ok(menu) = query(&cm, false, false)
                {
                    unsafe {
                        let _ = DestroyMenu(menu);
                    }
                    log(format!("asztali menü: előkészítve ({} ms)", t.elapsed().as_millis()));
                }
            }
            Cmd::Open { seq, x, y, shift, item } => {
                cur = None;
                let t = std::time::Instant::now();
                let built = desktop_view().and_then(|(view, owner)| {
                    let cm = desktop_menu(&view, item)?;
                    let menu = query(&cm, shift, item >= 0)?;
                    Ok((cm, view, owner, menu))
                });
                match built {
                    Ok((cm, view, owner, menu)) => {
                        let mut got = Collect::default();
                        let cm2 = cm.cast::<IContextMenu2>().ok();
                        let items = read_items(menu, cm2.as_ref(), 0, true, &mut got);
                        if let Ok(mut g) = ICONS.lock() {
                            *g = got.icons;
                        }
                        let c = Current { seq, cm, menu, subs: got.subs, view, owner, pt: POINT { x, y }, shift, item };
                        if items.is_empty() {
                            log("asztali menü: üres menü, a Windows menüje nyílik meg");
                            drop(c);
                            replay(x, y);
                            continue;
                        }
                        cur = Some(c);
                        let what = if item >= 0 { "ikon" } else { "háttér" };
                        log(format!("asztali menü ({what}): {} elem, {} ms", items.len(), t.elapsed().as_millis()));
                        ready(seq, None, json!(items));
                    }
                    Err(e) => {
                        log(format!("asztali menü: nem készíthető el ({e}), a Windows menüje nyílik meg"));
                        replay(x, y);
                    }
                }
            }
            Cmd::Sub { seq, menu } => {
                let Some(c) = cur.as_mut().filter(|c| c.seq == seq) else { continue };
                let Some(&hmenu) = c.subs.get(menu) else { continue };
                let mut got = Collect { icons: ICONS.lock().map(|g| g.clone()).unwrap_or_default(), subs: std::mem::take(&mut c.subs) };
                let cm2 = c.cm.cast::<IContextMenu2>().ok();
                let items = read_items(hmenu, cm2.as_ref(), 1, false, &mut got);
                c.subs = got.subs;
                if let Ok(mut g) = ICONS.lock() {
                    *g = got.icons;
                }
                ready(seq, Some(menu), json!(items));
            }
            Cmd::Invoke { seq, id } => {
                let Some(c) = cur.take() else { continue };
                if c.seq != seq || id < ID_FIRST {
                    cur = Some(c);
                    continue;
                }
                run(&c, id);
            }
            Cmd::Release { seq } => {
                if cur.as_ref().is_some_and(|c| c.seq == seq) {
                    cur = None;
                }
            }
        }
    }
}
