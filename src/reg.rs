//! A registry olvasása és írása (a felhasználó és a gép beállításai). Minden függvény a kulcs
//! útvonalát és az érték nevét kapja; az üres név a kulcs alapértékét jelenti. Hiányzó kulcs,
//! érték vagy eltérő típus esetén `None` (illetve `false`).

use windows::Win32::System::Registry::{
    HKEY, KEY_READ, REG_BINARY, REG_DWORD, REG_ROUTINE_FLAGS, REG_VALUE_TYPE, RRF_RT_ANY, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
};
use windows::core::{HSTRING, PWSTR};

/// Egy érték nyers tartalma a megadott típus(ok)ra szűkítve.
fn read(root: HKEY, key: &str, value: &str, types: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let (k, v) = (HSTRING::from(key), HSTRING::from(value));
    let mut len = 0u32;
    unsafe { RegGetValueW(root, &k, &v, types, None, None, Some(&mut len)) }.ok().ok()?;
    let mut data = vec![0u8; len as usize];
    if len > 0 {
        unsafe { RegGetValueW(root, &k, &v, types, None, Some(data.as_mut_ptr() as *mut _), Some(&mut len)) }.ok().ok()?;
        data.truncate(len as usize);
    }
    Some(data)
}

/// Szöveges érték (REG_SZ, a környezeti változók feloldásával), a szélein lévő szóközök nélkül;
/// az üres szöveg is `None`.
pub fn get_string(root: HKEY, key: &str, value: &str) -> Option<String> {
    let data = read(root, key, value, RRF_RT_REG_SZ)?;
    let wide: Vec<u16> = data.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).take_while(|&c| c != 0).collect();
    Some(String::from_utf16_lossy(&wide).trim().to_string()).filter(|s| !s.is_empty())
}

/// 32 bites szám (REG_DWORD).
pub fn get_dword(root: HKEY, key: &str, value: &str) -> Option<u32> {
    let data = read(root, key, value, RRF_RT_REG_DWORD)?;
    Some(u32::from_le_bytes(*data.first_chunk::<4>()?))
}

/// Bináris érték (REG_BINARY).
pub fn get_binary(root: HKEY, key: &str, value: &str) -> Option<Vec<u8>> {
    read(root, key, value, RRF_RT_REG_BINARY)
}

/// Bármilyen típusú érték nyers tartalma (pl. egy REG_QWORD vagy REG_BINARY szám).
pub fn get_raw(root: HKEY, key: &str, value: &str) -> Option<Vec<u8>> {
    read(root, key, value, RRF_RT_ANY)
}

fn write(root: HKEY, key: &str, value: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> bool {
    let (k, v) = (HSTRING::from(key), HSTRING::from(value));
    unsafe { RegSetKeyValueW(root, &k, &v, kind.0, Some(data.as_ptr() as *const _), data.len() as u32) }.is_ok()
}

/// 32 bites szám írása (a kulcs, ha nincs, létrejön).
pub fn set_dword(root: HKEY, key: &str, value: &str, v: u32) -> bool {
    write(root, key, value, REG_DWORD, &v.to_le_bytes())
}

/// Bináris érték írása (a kulcs, ha nincs, létrejön).
pub fn set_binary(root: HKEY, key: &str, value: &str, data: &[u8]) -> bool {
    write(root, key, value, REG_BINARY, data)
}

/// Egy kulcs közvetlen alkulcsainak neve.
pub fn subkeys(root: HKEY, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut h = HKEY::default();
    if unsafe { RegOpenKeyExW(root, &HSTRING::from(key), None, KEY_READ, &mut h) }.is_err() {
        return out;
    }
    for i in 0.. {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        if unsafe { RegEnumKeyExW(h, i, Some(PWSTR(buf.as_mut_ptr())), &mut len, None, None, None, None) }.is_err() {
            break;
        }
        out.push(String::from_utf16_lossy(&buf[..len as usize]));
    }
    unsafe {
        let _ = RegCloseKey(h);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    /// Élő olvasás ezen a gépen (csak olvas): a Windows-változat és a felhasználói kulcsok.
    #[test]
    fn live_read() {
        let nt = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        assert!(get_string(HKEY_LOCAL_MACHINE, nt, "ProductName").is_some_and(|s| s.contains("Windows")));
        assert!(get_dword(HKEY_LOCAL_MACHINE, nt, "CurrentMajorVersionNumber").is_some_and(|v| v >= 10));
        assert!(get_string(HKEY_LOCAL_MACHINE, nt, "nincs-ilyen-ertek").is_none());
        assert!(get_dword(HKEY_LOCAL_MACHINE, nt, "ProductName").is_none(), "eltérő típus");
        assert!(subkeys(HKEY_CURRENT_USER, "Software").iter().any(|k| k.eq_ignore_ascii_case("Microsoft")));
    }
}
