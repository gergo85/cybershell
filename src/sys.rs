//! Rendszerfunkciók: főkapcsoló (zárolás, alvás, leállítás, újraindítás),
//! telepítettség-ellenőrzés, programválasztó ablak és parancsikonok feloldása.

use std::os::windows::process::CommandExt;
use std::path::Path;

use windows::Win32::Foundation::{FILETIME, HWND};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Storage::FileSystem::{SearchPathW, WIN32_FIND_DATAW};
use windows::Win32::System::Com::{CLSCTX_ALL, CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile, STGM_READ};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::System::Shutdown::{EWX_LOGOFF, ExitWindowsEx, LockWorkStation, SHTDN_REASON_FLAG_PLANNED};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    DWPOS_FILL, DesktopWallpaper, FOS_FILEMUSTEXIST, FileOpenDialog, IDesktopWallpaper, IFileOpenDialog, IShellItem, IShellLinkW,
    SHCreateItemFromParsingName, SIGDN_FILESYSPATH, ShellLink,
};
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
use windows::core::{HSTRING, Interface, PCWSTR, w};

use crate::config::AppEntry;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---- Rendszerterhelés ----

fn ft(f: FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

/// A processzoridők pillanatnyi állása: (üresjárat, összes). A kettő különbségéből
/// két mérés között kiszámolható a terhelés.
pub fn cpu_times() -> Option<(u64, u64)> {
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

/// A hely (szélesség, hosszúság), ahol a nap kel és nyugszik: az Időjárás szekció
/// települése, ahogy a felület megtalálta. Újratöltés után is megmarad.
static GEO: std::sync::Mutex<Option<(f64, f64)>> = std::sync::Mutex::new(None);

pub fn set_geo(lat: f64, lon: f64) -> bool {
    let Ok(mut g) = GEO.lock() else { return false };
    let changed = *g != Some((lat, lon));
    *g = Some((lat, lon));
    changed
}

/// Éjszaka van-e (napnyugta és napkelte között). A napállást a NOAA közelítő képletével
/// számoljuk, világidőben, így az időzóna nem számít. Ha a hely még nem ismert (`fallback`
/// sem), helyi idő szerint 19 és 7 óra között számít éjszakának.
pub fn is_night(fallback: Option<(f64, f64)>) -> bool {
    let geo = GEO.lock().ok().and_then(|g| *g).or(fallback);
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let Some((lat, lon)) = geo else {
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        return t.wHour >= 19 || t.wHour < 7;
    };
    let now_min = (secs % 86400) as f64 / 60.0;
    let doy = day_of_year(secs / 86400) as f64;
    let g = 2.0 * std::f64::consts::PI / 365.0 * (doy - 1.0);
    let eqtime = 229.18 * (0.000075 + 0.001868 * g.cos() - 0.032077 * g.sin() - 0.014615 * (2.0 * g).cos() - 0.040849 * (2.0 * g).sin());
    let decl = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2.0 * g).cos() + 0.000907 * (2.0 * g).sin()
        - 0.002697 * (3.0 * g).cos()
        + 0.00148 * (3.0 * g).sin();
    let phi = lat.to_radians();
    let cos_ha = 90.833f64.to_radians().cos() / (phi.cos() * decl.cos()) - phi.tan() * decl.tan();
    if cos_ha > 1.0 {
        return true; // sarki éjszaka
    }
    if cos_ha < -1.0 {
        return false; // éjféli nap
    }
    let ha = cos_ha.acos().to_degrees();
    let rise = (720.0 - 4.0 * (lon + ha) - eqtime).rem_euclid(1440.0);
    let set = (720.0 - 4.0 * (lon - ha) - eqtime).rem_euclid(1440.0);
    // nappal: napkeltétől napnyugtáig (világidőben az éjfél a kettő közé is eshet)
    let day = if rise < set { now_min >= rise && now_min < set } else { now_min >= rise || now_min < set };
    !day
}

/// Az év napja (1–366) a Unix-időből számolt napok alapján (polgári naptár).
fn day_of_year(days: u64) -> u32 {
    let mut y = 1970u64;
    let mut d = days;
    loop {
        let len = if (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400) { 366 } else { 365 };
        if d < len {
            return d as u32 + 1;
        }
        d -= len;
        y += 1;
    }
}

/// Ennyi ideje nem nyúltak a billentyűzethez és az egérhez.
pub fn idle_time() -> std::time::Duration {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        if !GetLastInputInfo(&mut info).as_bool() {
            return std::time::Duration::ZERO;
        }
        let now = GetTickCount();
        std::time::Duration::from_millis(now.wrapping_sub(info.dwTime) as u64)
    }
}

/// Főkapcsoló-műveletek.
pub fn power(action: &str) -> Result<(), String> {
    match action {
        "lock" => unsafe { LockWorkStation().map_err(|e| e.to_string()) },
        "sleep" => {
            // alvó állapot (nem hibernálás)
            if unsafe { SetSuspendState(false, false, false) } { Ok(()) } else { Err("Az alvó állapot nem érhető el.".into()) }
        }
        // kijelentkezés: a Windows saját hívásával (mint a Start menü Kijelentkezés parancsa)
        "signout" => unsafe { ExitWindowsEx(EWX_LOGOFF, SHTDN_REASON_FLAG_PLANNED).map_err(|e| e.to_string()) },
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

/// Egy exe telepített helye a Windows „App Paths” nyilvántartásából (a telepítők ide írják be,
/// hogy a Futtatás ablak és a Start menü név szerint megtalálja).
fn app_paths(exe: &str) -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let key = HSTRING::from(format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{exe}"));
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let mut buf = [0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let r = unsafe { RegGetValueW(root, &key, None, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut size)) };
        if r.is_ok() {
            let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let p = String::from_utf16_lossy(&buf[..n]).trim_matches('"').to_string();
            if Path::new(&p).is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Egy (ezen a gépen) nem található program megkeresése, ha máshova telepítették: más
/// mappába, más verziószámmal (pl. „Photoshop 2025” a „2026” helyett), Store-app helyett
/// asztali változatként vagy fordítva. Sorrendben: a Windows App Paths nyilvántartása az exe
/// neve szerint, a Start menü parancsikonjai ugyanarra az exe-re, végül a Start menüben
/// pontosan ugyanilyen nevű program. Steam-játékot nem keres.
pub fn locate_app(app: &AppEntry, start: &[crate::apps::StartApp]) -> Option<String> {
    let p = app.resolved_path();
    let lower = p.to_lowercase();
    if lower.contains("://") {
        return None;
    }
    let mut exes: Vec<String> = Vec::new();
    if !lower.starts_with("shell:")
        && lower.ends_with(".exe")
        && let Some(f) = Path::new(&p).file_name()
    {
        exes.push(f.to_string_lossy().to_lowercase());
    }
    for m in &app.match_exe {
        let m = m.to_lowercase();
        if m.ends_with(".exe") && !exes.contains(&m) {
            exes.push(m);
        }
    }
    for e in &exes {
        if let Some(found) = app_paths(e) {
            return Some(found);
        }
    }
    let file_of = |t: &str| Path::new(t).file_name().map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
    for e in &exes {
        if let Some(a) = start.iter().find(|a| !a.target.is_empty() && file_of(&a.target) == *e && Path::new(&a.target).is_file()) {
            return Some(a.target.clone());
        }
    }
    let name = app.name.trim().to_lowercase();
    let hits: Vec<&crate::apps::StartApp> = start.iter().filter(|a| a.name.trim().to_lowercase() == name).collect();
    if let [a] = hits.as_slice() {
        return Some(if a.target.is_empty() { format!(r"shell:AppsFolder\{}", a.id) } else { a.target.clone() });
    }
    None
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
    let file = pick_file(owner, w!("Program hozzáadása a CyberShellhez"), w!("Programok és parancsikonok"), w!("*.exe;*.lnk"))?;
    picked_from_file(&file)
}

/// Fájlválasztó ablak képekhez (háttérkép hozzáadása).
pub fn pick_image(owner: HWND) -> Option<String> {
    pick_file(owner, w!("Háttérkép hozzáadása a CyberShellhez"), w!("Képek"), w!("*.jpg;*.jpeg;*.png;*.bmp"))
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
        let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).map_err(|e| e.message().to_string())?;
        let _ = dw.SetPosition(DWPOS_FILL);
        dw.SetWallpaper(PCWSTR::null(), &HSTRING::from(path)).map_err(|e| e.message().to_string())
    }
}

/// Fájlok áthelyezése a Lomtárba (visszaállíthatók), kérdés és előrehaladás-ablak nélkül.
pub fn recycle(files: &[std::path::PathBuf]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW, SHFileOperationW,
    };
    // dupla nullával lezárt lista
    let mut from: Vec<u16> = Vec::new();
    for f in files {
        from.extend(f.as_os_str().encode_wide());
        from.push(0);
    }
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r != 0 { Err(format!("hibakód {r}")) } else { Ok(()) }
}

/// A jelenlegi háttérkép útvonala (üres, ha nincs, vagy egyszínű a háttér). Előbb a
/// beállításhoz is használt IDesktopWallpaper-től kérdezzük (ez a beállítás után azonnal az új
/// képet adja; a régi SPI-lekérdezés egy ideig még az előzőt mutatta), aztán a régi módon.
pub fn current_wallpaper() -> String {
    let fresh = unsafe {
        CoCreateInstance::<_, IDesktopWallpaper>(&DesktopWallpaper, None, CLSCTX_ALL).ok().and_then(|dw| {
            let p = dw.GetWallpaper(PCWSTR::null()).ok()?;
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as *const _));
            s
        })
    };
    if let Some(p) = fresh.filter(|p| !p.is_empty()) {
        return p;
    }
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
pub struct Volume(IAudioEndpointVolume);

impl Volume {
    /// A mostani alapértelmezett eszköz. Ha az eszköz közben megváltozik (pl. fejhallgató),
    /// a hívó egy idő után újat kér.
    pub fn open() -> Option<Volume> {
        use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
        use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
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
    let id = unsafe { GetUserDefaultUILanguage() };
    // az elsődleges nyelv az alsó 10 bit; a magyar: LANG_HUNGARIAN = 0x0E
    if id & 0x3ff == 0x0e { "hu" } else { "en" }
}
