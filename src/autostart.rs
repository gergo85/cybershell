//! Indítás a Windowszal: a felhasználó saját „Run” bejegyzése
//! (HKCU\Software\Microsoft\Windows\CurrentVersion\Run\CyberHub).
//!
//! Az újabb Windows 11-ek az Intéző indulásakor csak azokat a Run-bejegyzéseket indítják el,
//! amelyekhez az „Indítási alkalmazások” listában (Feladatkezelő, Beállítások) is tartozik
//! engedélyező érték: HKCU\…\Explorer\StartupApproved\Run\CyberHub. Ezt a Windows csak a
//! saját felületén kapcsolt programoknál írja be, ezért a bekapcsoláskor mi állítjuk be.
//! Ha a felhasználó a Feladatkezelőben letiltja, a kapcsoló kikapcsolt állapotot mutat.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_BINARY, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegSetValueExW,
};
use windows::core::{w, PCWSTR};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APPROVED: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");
const NAME: PCWSTR = w!("CyberHub");

/// Az „Indítási alkalmazások” értéke: az első bájt páros = engedélyezve (2), páratlan = letiltva (3).
/// Hiányzó érték: a Windows-verziótól függ, ezért bekapcsoláskor mindig beírjuk.
fn approved_state() -> Option<bool> {
    let mut buf = [0u8; 12];
    let mut len = buf.len() as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            APPROVED,
            NAME,
            RRF_RT_REG_BINARY,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    };
    (r == ERROR_SUCCESS && len > 0).then(|| buf[0] % 2 == 0)
}

pub fn is_enabled() -> bool {
    let run = unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN, NAME, RRF_RT_REG_SZ, None, None, None) } == ERROR_SUCCESS;
    run && approved_state() != Some(false)
}

fn open(path: PCWSTR) -> Result<HKEY, String> {
    let mut key = HKEY::default();
    let r = unsafe {
        RegCreateKeyExW(HKEY_CURRENT_USER, path, Some(0), None, REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut key, None)
    };
    if r == ERROR_SUCCESS { Ok(key) } else { Err(format!("A beállítás nem nyitható meg ({})", r.0)) }
}

fn delete(key: HKEY) -> WIN32_ERROR {
    let r = unsafe { RegDeleteValueW(key, NAME) };
    // ha nem is volt bejegyzés, az is rendben van
    if r == ERROR_FILE_NOT_FOUND { ERROR_SUCCESS } else { r }
}

/// Be- vagy kikapcsolja. Bekapcsoláskor a most futó exe útvonalát jegyzi be.
pub fn set(enabled: bool) -> Result<(), String> {
    let run = open(RUN)?;
    let result = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string());
        match exe {
            Ok(exe) => {
                // a --autostart jelzi, hogy bejelentkezéskor indultunk: ilyenkor megvárjuk az Intézőt
                let value = format!("\"{}\" --autostart", exe.display());
                let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
                let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
                unsafe { RegSetValueExW(run, NAME, Some(0), REG_SZ, Some(bytes)) }
            }
            Err(e) => {
                unsafe {
                    let _ = RegCloseKey(run);
                }
                return Err(e);
            }
        }
    } else {
        delete(run)
    };
    unsafe {
        let _ = RegCloseKey(run);
    }
    if result != ERROR_SUCCESS {
        return Err(format!("Nem sikerült ({})", result.0));
    }
    // az „Indítási alkalmazások” engedélyezése (2, majd 11 nulla bájt), illetve törlése
    let approved = open(APPROVED)?;
    let r = if enabled {
        let mut v = [0u8; 12];
        v[0] = 2;
        unsafe { RegSetValueExW(approved, NAME, Some(0), REG_BINARY, Some(&v)) }
    } else {
        delete(approved)
    };
    unsafe {
        let _ = RegCloseKey(approved);
    }
    if r == ERROR_SUCCESS { Ok(()) } else { Err(format!("Nem sikerült engedélyezni ({})", r.0)) }
}

/// A felhasználó a Windowsban (Feladatkezelő, Beállítások › Indítási alkalmazások) letiltotta.
pub fn disabled_in_windows() -> bool {
    approved_state() == Some(false)
}
