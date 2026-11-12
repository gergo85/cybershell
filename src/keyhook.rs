//! A Win billentyű a CyberShell Start menüjét, a Win+S a keresőjét nyitja meg.
//!
//! Alacsony szintű billentyűzetfigyelő, külön szálon (saját üzenetsorral), hogy a fő szál
//! elfoglaltsága sose lassítsa a gépelést. Ha a Win billentyűt önmagában nyomták le és
//! engedték fel, a felengedést elnyeljük, és helyette egy semleges „takaró” billentyűt
//! (0xE8, nincs hozzárendelve) küldünk a Win felengedése elé – így az Intéző nem nyitja meg
//! a saját Start menüjét. A Win+bármi kombinációk (Win+E, Win+L, Win+Shift+S…) érintetlenek.
//! A rendszergazdaként futó ablakok előtt a Windows nem enged figyelni: ott a gyári menü jön.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_LWIN, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Be van-e kapcsolva (a beállításokból állítható).
static ENABLED: AtomicBool = AtomicBool::new(false);
static WIN_DOWN: AtomicBool = AtomicBool::new(false);
/// A Win lenyomva tartása alatt más billentyűt is nyomtak (ez kombináció volt).
static WIN_USED: AtomicBool = AtomicBool::new(false);
/// A Win+S „S” lenyomását elnyeltük: a felengedését is el kell nyelni.
static S_EATEN: AtomicBool = AtomicBool::new(false);
static FIRE: OnceLock<Box<dyn Fn(&'static str) + Send + Sync>> = OnceLock::new();

const MASK_VK: u16 = 0xE8;

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on {
        WIN_DOWN.store(false, Ordering::Relaxed);
        S_EATEN.store(false, Ordering::Relaxed);
    }
}

fn key(vk: u16, up: bool, extended: bool) -> INPUT {
    let mut flags = if up { KEYEVENTF_KEYUP } else { Default::default() };
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pass = || unsafe { CallNextHookEx(None, code, wp, lp) };
    if code < 0 || !ENABLED.load(Ordering::Relaxed) {
        return pass();
    }
    let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
    // a saját (és más programok által) küldött billentyűket nem bántjuk
    if k.flags.0 & LLKHF_INJECTED.0 != 0 {
        return pass();
    }
    let msg = wp.0 as u32;
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    let vk = k.vkCode as u16;
    let fire = |what: &'static str| {
        if let Some(f) = FIRE.get() {
            f(what);
        }
    };

    if vk == VK_LWIN.0 || vk == VK_RWIN.0 {
        if down && !WIN_DOWN.swap(true, Ordering::Relaxed) {
            WIN_USED.store(false, Ordering::Relaxed);
        }
        if up {
            WIN_DOWN.store(false, Ordering::Relaxed);
            if !WIN_USED.load(Ordering::Relaxed) {
                // takaró billentyű, majd a Win felengedése: az Intéző kombinációnak látja
                let seq = [key(MASK_VK, false, false), key(MASK_VK, true, false), key(vk, true, true)];
                unsafe {
                    SendInput(&seq, std::mem::size_of::<INPUT>() as i32);
                }
                fire("start");
                return LRESULT(1);
            }
        }
        return pass();
    }
    if WIN_DOWN.load(Ordering::Relaxed) && down {
        WIN_USED.store(true, Ordering::Relaxed);
        if vk == b'S' as u16 {
            S_EATEN.store(true, Ordering::Relaxed);
            fire("search");
            return LRESULT(1);
        }
    }
    if up && vk == b'S' as u16 && S_EATEN.swap(false, Ordering::Relaxed) {
        return LRESULT(1);
    }
    pass()
}

/// A figyelő indítása. A `fire` a "start" vagy "search" jelzést kapja (a figyelő szálán).
pub fn start(fire: impl Fn(&'static str) + Send + Sync + 'static) {
    if FIRE.set(Box::new(fire)).is_err() {
        return;
    }
    std::thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).ok().map(|m| m.into());
        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), module, 0) {
            Ok(_) => crate::log("Win billentyű: figyelés elindítva"),
            Err(e) => {
                crate::log(format!("Win billentyű: a figyelés nem indítható ({e})"));
                return;
            }
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
}
