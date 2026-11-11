//! Rendszerfunkciók: főkapcsoló (zárolás, alvás, leállítás, újraindítás),
//! telepítettség-ellenőrzés, programválasztó ablak és parancsikonok feloldása.

use std::os::windows::process::CommandExt;
use std::path::Path;

use windows::Win32::Foundation::HWND;
use windows::Win32::Storage::FileSystem::{SearchPathW, WIN32_FIND_DATAW};
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, IPersistFile, CLSCTX_ALL, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    DesktopWallpaper, FileOpenDialog, IDesktopWallpaper, IFileOpenDialog, IShellItem, IShellLinkW,
    SHCreateItemFromParsingName, ShellLink, DWPOS_FILL, FOS_FILEMUSTEXIST, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
use windows::core::{w, HSTRING, Interface, PCWSTR};

use crate::config::AppEntry;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---- Rendszerterhelés ----

fn ft(f: windows::Win32::Foundation::FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

/// A processzoridők pillanatnyi állása: (üresjárat, összes). A kettő különbségéből
/// két mérés között kiszámolható a terhelés.
pub fn cpu_times() -> Option<(u64, u64)> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    // a kernelidő az üresjáratot is tartalmazza
    Some((ft(idle), ft(kernel) + ft(user)))
}

/// Memória: (használt bájt, összes bájt).
pub fn memory() -> Option<(u64, u64)> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
    Some((m.ullTotalPhys.saturating_sub(m.ullAvailPhys), m.ullTotalPhys))
}

/// Akkumulátor: None, ha a gépben nincs (asztali gép). Egyébként (töltöttség %, tölt-e).
pub fn battery() -> Option<(Option<u8>, bool)> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    // 128: nincs akkumulátor, 255: ismeretlen állapot
    if s.BatteryFlag == 255 || s.BatteryFlag & 128 != 0 {
        return None;
    }
    let pct = (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent);
    // 8: töltés folyamatban; hálózaton, teli akkumulátornál is „töltőn” jelezzük
    let charging = s.BatteryFlag & 8 != 0 || s.ACLineStatus == 1;
    Some((pct, charging))
}

/// Főkapcsoló-műveletek.
pub fn power(action: &str) -> Result<(), String> {
    match action {
        "lock" => unsafe { LockWorkStation().map_err(|e| e.to_string()) },
        "sleep" => {
            // alvó állapot (nem hibernálás)
            if unsafe { SetSuspendState(false, false, false) } { Ok(()) } else { Err("Az alvó állapot nem érhető el.".into()) }
        }
        "shutdown" | "restart" => {
            let flag = if action == "shutdown" { "/s" } else { "/r" };
            std::process::Command::new("shutdown.exe")
                .args([flag, "/t", "0"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Ismeretlen művelet: {action}")),
    }
}

/// Telepítve van-e még a program. Ami nem ellenőrizhető (pl. ismeretlen URI), azt elérhetőnek vesszük.
pub fn app_available(app: &AppEntry) -> bool {
    let p = app.resolved_path();
    let lower = p.to_lowercase();
    if lower.starts_with("shell:appsfolder\\") {
        // Store-app: a héj csak akkor találja meg, ha telepítve van
        return unsafe { SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(p.as_str()), None) }.is_ok();
    }
    if let Some(id) = lower.strip_prefix("steam://rungameid/") {
        return steam_installed(id.trim_matches('/'));
    }
    if lower.contains("://") || lower.ends_with(':') {
        return true;
    }
    let path = Path::new(&p);
    if path.is_absolute() {
        return path.exists();
    }
    // csak fájlnév (pl. explorer.exe, wt.exe): keresés a PATH-ban
    let mut buf = [0u16; 512];
    unsafe { SearchPathW(None, &HSTRING::from(p.as_str()), None, Some(&mut buf), None) > 0 }
}

fn steam_installed(id: &str) -> bool {
    let key = HSTRING::from(format!("Software\\Valve\\Steam\\Apps\\{id}"));
    let mut val: u32 = 0;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            w!("Installed"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut val as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    r.is_ok() && val == 1
}

/// A programválasztóval kiválasztott program.
pub struct Picked {
    pub name: String,
    pub path: String,
    pub args: String,
}

/// Fájlválasztó ablak programokhoz (.exe és parancsikon).
pub fn pick_program(owner: HWND) -> Option<Picked> {
    let file = pick_file(owner, w!("Program hozzáadása a CyberHubhoz"), w!("Programok és parancsikonok"), w!("*.exe;*.lnk"))?;
    picked_from_file(&file)
}

/// Fájlválasztó ablak képekhez (háttérkép hozzáadása).
pub fn pick_image(owner: HWND) -> Option<String> {
    pick_file(owner, w!("Háttérkép hozzáadása a CyberHubhoz"), w!("Képek"), w!("*.jpg;*.jpeg;*.png;*.bmp"))
}

fn pick_file(owner: HWND, title: PCWSTR, filter_name: PCWSTR, filter: PCWSTR) -> Option<String> {
    unsafe {
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let filters = [
            COMDLG_FILTERSPEC { pszName: filter_name, pszSpec: filter },
            COMDLG_FILTERSPEC { pszName: w!("Minden fájl"), pszSpec: w!("*.*") },
        ];
        let _ = dlg.SetFileTypes(&filters);
        let _ = dlg.SetTitle(title);
        if let Ok(opts) = dlg.GetOptions() {
            let _ = dlg.SetOptions(opts | FOS_FILEMUSTEXIST);
        }
        dlg.Show(Some(owner)).ok()?;
        let item = dlg.GetResult().ok()?;
        let pw = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let file = pw.to_string().unwrap_or_default();
        CoTaskMemFree(Some(pw.0 as *const _));
        (!file.is_empty()).then_some(file)
    }
}

// ---- Háttérkép ----

/// A háttérképnek használható kiterjesztések.
pub const WALLPAPER_EXTS: &[&str] = &["jpg", "jpeg", "png", "bmp"];

/// Beállítja a Windows háttérképét minden monitoron, kitöltő elrendezéssel.
pub fn set_wallpaper(path: &str) -> Result<(), String> {
    unsafe {
        let dw: IDesktopWallpaper =
            CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).map_err(|e| e.message().to_string())?;
        let _ = dw.SetPosition(DWPOS_FILL);
        dw.SetWallpaper(PCWSTR::null(), &HSTRING::from(path)).map_err(|e| e.message().to_string())
    }
}

/// A jelenlegi háttérkép útvonala (üres, ha nincs, vagy egyszínű a háttér).
pub fn current_wallpaper() -> String {
    let mut buf = [0u16; 1024];
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr() as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    if ok.is_err() {
        return String::new();
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

/// Egy fájlból (exe vagy parancsikon) összeállítja a program adatait.
pub fn picked_from_file(file: &str) -> Option<Picked> {
    if file.is_empty() {
        return None;
    }
    let stem = Path::new(file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if file.to_lowercase().ends_with(".lnk") {
        if let Some((target, args)) = resolve_lnk(file) {
            return Some(Picked { name: stem, path: target, args });
        }
        // ha a parancsikon célja nem fájl (pl. Store-app), magát a parancsikont indítjuk
        return Some(Picked { name: stem, path: file.to_string(), args: String::new() });
    }
    Some(Picked { name: crate::wintrack::display_name(file), path: file.to_string(), args: String::new() })
}

fn resolve_lnk(file: &str) -> Option<(String, String)> {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        persist.Load(&HSTRING::from(file), STGM_READ).ok()?;
        let mut target = [0u16; 1024];
        let mut fd = WIN32_FIND_DATAW::default();
        link.GetPath(&mut target, &mut fd, 0).ok()?;
        let target = String::from_utf16_lossy(&target).trim_end_matches('\0').to_string();
        if target.is_empty() {
            return None;
        }
        let mut args = [0u16; 1024];
        let _ = link.GetArguments(&mut args);
        let args = String::from_utf16_lossy(&args).trim_end_matches('\0').to_string();
        Some((target, args))
    }
}

// ---- Hangerő (a Windows alapértelmezett lejátszóeszköze) ----

/// Az alapértelmezett lejátszóeszköz hangerő-vezérlője (Core Audio).
pub struct Volume(windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume);

impl Volume {
    /// A mostani alapértelmezett eszköz. Ha az eszköz közben megváltozik (pl. fejhallgató),
    /// a hívó egy idő után újat kér.
    pub fn open() -> Option<Volume> {
        use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            dev.Activate(CLSCTX_ALL, None).ok().map(Volume)
        }
    }

    /// Hangerő százalékban és a némítás.
    pub fn get(&self) -> Option<(u8, bool)> {
        unsafe {
            let v = self.0.GetMasterVolumeLevelScalar().ok()?;
            let m = self.0.GetMute().ok()?.as_bool();
            Some(((v * 100.0).round().clamp(0.0, 100.0) as u8, m))
        }
    }

    pub fn set(&self, pct: u8) -> bool {
        unsafe {
            // hangerő állításakor a némítást is feloldjuk (ahogy a Windows saját csúszkája)
            let _ = self.0.SetMute(false, std::ptr::null());
            self.0.SetMasterVolumeLevelScalar(pct.min(100) as f32 / 100.0, std::ptr::null()).is_ok()
        }
    }

    pub fn set_mute(&self, on: bool) -> bool {
        unsafe { self.0.SetMute(on, std::ptr::null()).is_ok() }
    }
}

/// A Windows felületének nyelve: magyar esetén "hu", minden más esetben "en".
pub fn system_language() -> &'static str {
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    // az elsődleges nyelv az alsó 10 bit; a magyar: LANG_HUNGARIAN = 0x0E
    if id & 0x3ff == 0x0e { "hu" } else { "en" }
}
