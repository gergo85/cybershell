//! Indítás a Windowszal: a felhasználó saját „Run” bejegyzése
//! (HKCU\Software\Microsoft\Windows\CurrentVersion\Run\CyberShell).
//!
//! Az újabb Windows 11-ek az Intéző indulásakor csak azokat a Run-bejegyzéseket indítják el,
//! amelyekhez az „Indítási alkalmazások” listában (Feladatkezelő, Beállítások) is tartozik
//! engedélyező érték: HKCU\…\Explorer\StartupApproved\Run\CyberShell. Ezt a Windows csak a
//! saját felületén kapcsolt programoknál írja be, ezért a bekapcsoláskor mi állítjuk be.
//! Ha a felhasználó a Feladatkezelőben letiltja, a kapcsoló kikapcsolt állapotot mutat.
//!
//! Csomagolva (MSIX, pl. a Store-ból telepítve) a Run kulcs nem használható: a csomagolt
//! program HKCU-ba írt értékei a csomag saját registryjébe kerülnek, ezeket a Windows
//! bejelentkezéskor nem látja. Ilyenkor a csomag manifestjében bejegyzett indítási feladat
//! (StartupTask) kapcsolja a Windowszal való indulást, a lenti `task` modulban.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_BINARY, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW,
};
use windows::core::{PCWSTR, w};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APPROVED: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");
const NAME: PCWSTR = w!("CyberShell");

/// Az „Indítási alkalmazások” értéke: az első bájt páros = engedélyezve (2), páratlan = letiltva (3).
/// Hiányzó érték: a Windows-verziótól függ, ezért bekapcsoláskor mindig beírjuk.
fn approved_state() -> Option<bool> {
    let mut buf = [0u8; 12];
    let mut len = buf.len() as u32;
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, APPROVED, NAME, RRF_RT_REG_BINARY, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut len))
    };
    (r == ERROR_SUCCESS && len > 0).then(|| buf[0] % 2 == 0)
}

pub fn is_enabled() -> bool {
    if packaged() {
        return task::state().is_some_and(task::enabled);
    }
    let run = unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN, NAME, RRF_RT_REG_SZ, None, None, None) } == ERROR_SUCCESS;
    run && approved_state() != Some(false)
}

fn open(path: PCWSTR) -> Result<HKEY, String> {
    let mut key = HKEY::default();
    let r =
        unsafe { RegCreateKeyExW(HKEY_CURRENT_USER, path, Some(0), None, REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut key, None) };
    if r == ERROR_SUCCESS { Ok(key) } else { Err(format!("A beállítás nem nyitható meg ({})", r.0)) }
}

fn delete(key: HKEY) -> WIN32_ERROR {
    let r = unsafe { RegDeleteValueW(key, NAME) };
    // ha nem is volt bejegyzés, az is rendben van
    if r == ERROR_FILE_NOT_FOUND { ERROR_SUCCESS } else { r }
}

/// Be- vagy kikapcsolja. Bekapcsoláskor a most futó exe útvonalát jegyzi be.
pub fn set(enabled: bool) -> Result<(), String> {
    if packaged() {
        return task::set(enabled);
    }
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
    if packaged() {
        return task::state().is_some_and(task::blocked);
    }
    approved_state() == Some(false)
}

/// A program régi neve (v1.2-ig CyberHub). Az átnevezés után a régi „Run” bejegyzés
/// és engedélyező érték megmaradna, és bejelentkezéskor a régi exe-t is indítaná.
const LEGACY_NAME: PCWSTR = w!("CyberHub");

/// Törli a régi nevű bejegyzést. Igazat ad, ha volt ilyen, és engedélyezve volt.
pub fn remove_legacy() -> bool {
    // csomagolva a valódi Run kulcsot nem érjük el (lásd fent), és a régi név úgysem fordulhat elő
    if packaged() {
        return false;
    }
    let had = unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN, LEGACY_NAME, RRF_RT_REG_SZ, None, None, None) } == ERROR_SUCCESS;
    let mut buf = [0u8; 12];
    let mut len = buf.len() as u32;
    let approved = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, APPROVED, LEGACY_NAME, RRF_RT_REG_BINARY, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut len))
    } == ERROR_SUCCESS;
    for path in [RUN, APPROVED] {
        if let Ok(key) = open(path) {
            unsafe {
                let _ = RegDeleteValueW(key, LEGACY_NAME);
                let _ = RegCloseKey(key);
            }
        }
    }
    had && (!approved || buf[0] % 2 == 0)
}

/// Csomagolva (MSIX-ként telepítve) fut-e a program.
pub fn packaged() -> bool {
    use windows::Win32::Foundation::APPMODEL_ERROR_NO_PACKAGE;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    static PACKAGED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PACKAGED.get_or_init(|| {
        let mut len = 0u32;
        // csomag nélkül APPMODEL_ERROR_NO_PACKAGE, csomagban „kevés a hely” (a névnek)
        let r = unsafe { GetCurrentPackageFullName(&mut len, None) };
        r != APPMODEL_ERROR_NO_PACKAGE
    })
}

/// Bejelentkezéskor az indítási feladat indította-e el a programot. A StartupTask nem ad át
/// parancssori kapcsolót, ezért csomagolva ez helyettesíti a `--autostart`-ot.
pub fn launched_at_logon() -> bool {
    use windows::ApplicationModel::Activation::ActivationKind;
    use windows::ApplicationModel::AppInstance;
    packaged() && task::on_mta(|| AppInstance::GetActivatedEventArgs()?.Kind()).is_ok_and(|k| k == ActivationKind::StartupTask)
}

/// Az indítási feladat (Windows.ApplicationModel.StartupTask) kezelése csomagolt futáshoz.
mod task {
    use windows::ApplicationModel::{StartupTask, StartupTaskState};
    use windows::core::HSTRING;

    /// A feladat azonosítója: a csomag manifestjében is ugyanez kell
    /// (`<desktop:StartupTask TaskId="CyberShellStartup" Enabled="false" DisplayName="CyberShell" />`).
    const TASK_ID: &str = "CyberShellStartup";

    /// A WinRT-hívások külön, többszálú COM-os szálon futnak: a hívó (a fő szál) lehet COM
    /// nélküli vagy egyszálú, ott a várakozás az aszinkron műveletre elakadhatna.
    pub(super) fn on_mta<T: Send + 'static>(f: impl FnOnce() -> windows::core::Result<T> + Send + 'static) -> Result<T, String> {
        std::thread::spawn(move || {
            let _com = crate::wmi::ComThread::init();
            f().map_err(|e| e.message().to_string())
        })
        .join()
        .unwrap_or_else(|_| Err("a művelet szála leállt".into()))
    }

    fn get() -> windows::core::Result<StartupTask> {
        StartupTask::GetAsync(&HSTRING::from(TASK_ID))?.join()
    }

    pub(super) fn state() -> Option<StartupTaskState> {
        on_mta(|| get()?.State()).ok()
    }

    pub(super) fn enabled(s: StartupTaskState) -> bool {
        s == StartupTaskState::Enabled || s == StartupTaskState::EnabledByPolicy
    }

    /// A program maga nem kapcsolhatja be: a felhasználó (Beállítások › Alkalmazások › Indítás,
    /// Feladatkezelő) vagy a rendszergazda letiltotta.
    pub(super) fn blocked(s: StartupTaskState) -> bool {
        s == StartupTaskState::DisabledByUser || s == StartupTaskState::DisabledByPolicy
    }

    pub(super) fn set(on: bool) -> Result<(), String> {
        let s = on_mta(move || {
            let t = get()?;
            if on {
                t.RequestEnableAsync()?.join()
            } else {
                // a rendszergazda által bekapcsoltat nem lehet kikapcsolni
                if t.State()? != StartupTaskState::EnabledByPolicy {
                    t.Disable()?;
                }
                t.State()
            }
        })?;
        message(s, on).map_or(Ok(()), Err)
    }

    /// Hibaüzenet, ha a kért állapot nem jött létre (különben `None`).
    pub(super) fn message(s: StartupTaskState, wanted: bool) -> Option<String> {
        if enabled(s) == wanted {
            return None;
        }
        Some(
            match s {
                StartupTaskState::DisabledByUser => {
                    "A Windowsban letiltottad: a Beállítások › Alkalmazások › Indítás oldalon kapcsolhatod vissza."
                }
                StartupTaskState::DisabledByPolicy => "A rendszergazda letiltotta az indítást a Windowszal.",
                StartupTaskState::EnabledByPolicy => "A rendszergazda bekapcsolta, ezért nem kapcsolható ki.",
                _ => "Nem sikerült.",
            }
            .into(),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn states() {
            assert_eq!(message(StartupTaskState::Enabled, true), None);
            assert_eq!(message(StartupTaskState::Disabled, false), None);
            assert_eq!(message(StartupTaskState::DisabledByUser, false), None);
            assert!(message(StartupTaskState::DisabledByUser, true).is_some_and(|m| m.contains("Indítás")));
            assert!(message(StartupTaskState::EnabledByPolicy, false).is_some_and(|m| m.contains("rendszergazda")));
            assert!(message(StartupTaskState::Disabled, true).is_some());
            assert!(blocked(StartupTaskState::DisabledByPolicy) && !blocked(StartupTaskState::Disabled));
        }
    }
}

#[cfg(test)]
mod tests {
    /// Fejlesztéskor (cargo test) a program nincs csomagolva: a Run kulcs marad.
    #[test]
    fn not_packaged() {
        assert!(!super::packaged());
        assert!(!super::launched_at_logon());
    }

    /// Egymás utáni WinRT-hívások külön szálakon: a windows crate a WinRT-osztályok
    /// gyárát eltárolja, ennek a hívások között is érvényesnek kell maradnia. Csomag nélkül
    /// a feladat nem érhető el (hibát ad), de nem szabad összeomlania.
    #[test]
    fn repeated_calls() {
        for _ in 0..3 {
            assert!(super::task::state().is_none());
        }
    }
}
