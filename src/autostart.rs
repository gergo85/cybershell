//! Indítás a Windowszal: a felhasználó saját „Run” bejegyzése
//! (HKCU\Software\Microsoft\Windows\CurrentVersion\Run\CyberHub).

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
};
use windows::core::w;

pub fn is_enabled() -> bool {
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            w!("CyberHub"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        ) == ERROR_SUCCESS
    }
}

/// Be- vagy kikapcsolja. Bekapcsoláskor a most futó exe útvonalát jegyzi be.
pub fn set(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut key = HKEY::default();
        let r = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            Some(0),
            KEY_SET_VALUE,
            &mut key,
        );
        if r != ERROR_SUCCESS {
            return Err(format!("A beállítás nem nyitható meg ({})", r.0));
        }
        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            // a --autostart jelzi, hogy bejelentkezéskor indultunk: ilyenkor megvárjuk az Intézőt
            let value = format!("\"{}\" --autostart", exe.display());
            let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            RegSetValueExW(key, w!("CyberHub"), Some(0), REG_SZ, Some(bytes))
        } else {
            let r = RegDeleteValueW(key, w!("CyberHub"));
            // ha nem is volt bejegyzés, az is rendben van
            if r == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND { ERROR_SUCCESS } else { r }
        };
        let _ = RegCloseKey(key);
        if result == ERROR_SUCCESS { Ok(()) } else { Err(format!("Nem sikerült ({})", result.0)) }
    }
}
