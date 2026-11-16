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

// ---- Rendszerkomponensek (a beállítások Rendszer fülének) ----

/// A gép alkatrészei: alaplap (BIOS, foglalatok), processzor, memóriamodulok, videókártya,
/// Windows-változat. A Windows WMI-adatbázisából (PowerShell), ezért lassú (1–2 mp):
/// háttérszálon hívd.
pub fn hardware() -> Option<serde_json::Value> {
    const SCRIPT: &str = r#"$ErrorActionPreference='SilentlyContinue';
$c=Get-CimInstance Win32_Processor|select -First 1 Name,NumberOfCores,NumberOfLogicalProcessors,MaxClockSpeed;
$b=Get-CimInstance Win32_BaseBoard|select -First 1 Manufacturer,Product;
$bi=Get-CimInstance Win32_BIOS|select -First 1 SMBIOSBIOSVersion,@{n='date';e={if($_.ReleaseDate){$_.ReleaseDate.ToString('yyyy-MM-dd')}}};
$ma=Get-CimInstance Win32_PhysicalMemoryArray|?{$_.Use -eq 3}|select -First 1 MemoryDevices,MaxCapacityEx;
$m=@(Get-CimInstance Win32_PhysicalMemory|select Capacity,SMBIOSMemoryType,Speed,ConfiguredClockSpeed,Manufacturer,PartNumber,DeviceLocator,BankLabel);
$g=@(Get-CimInstance Win32_VideoController|select Name,AdapterRAM);
$gm=@(Get-ItemProperty 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*'|?{$_.'HardwareInformation.qwMemorySize'}|select DriverDesc,@{n='mem';e={$_.'HardwareInformation.qwMemorySize'}});
$o=Get-CimInstance Win32_OperatingSystem|select Caption,BuildNumber;
$d=@(Get-PhysicalDisk|select DeviceId,FriendlyName,MediaType,BusType,Size);
$p=@(Get-Partition|?{$_.DriveLetter}|select DiskNumber,@{n='L';e={[string]$_.DriveLetter}});
$v=@(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3'|select DeviceID,Size,FreeSpace,VolumeName);
[Console]::OutputEncoding=[Text.Encoding]::UTF8;
@{cpu=$c;board=$b;bios=$bi;slots=$ma;mem=$m;gpu=$g;gpumem=$gm;os=$o;disks=$d;parts=$p;vols=$v}|ConvertTo-Json -Depth 3 -Compress"#;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let raw: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(hardware_summary(&raw))
}

/// Ismert videókártyák adatai, amit a Windows nem ad meg: memóriatípus, buszszélesség (bit),
/// a memória sebessége (Gbps) és a grafikus processzor boost-órajele (MHz). Az asztali
/// változatokra; a hosszabb (pontosabb) név előrébb, hogy pl. az „RX 6600 XT” ne „RX 6600”
/// legyen.
const GPU_SPECS: &[(&str, &str, u32, f32, u32)] = &[
    // AMD
    ("RX 9070 XT", "GDDR6", 256, 20.0, 2970),
    ("RX 9070", "GDDR6", 256, 20.0, 2520),
    ("RX 9060 XT", "GDDR6", 128, 20.0, 3130),
    ("RX 7900 XTX", "GDDR6", 384, 20.0, 2500),
    ("RX 7900 GRE", "GDDR6", 256, 18.0, 2245),
    ("RX 7900 XT", "GDDR6", 320, 20.0, 2400),
    ("RX 7800 XT", "GDDR6", 256, 19.5, 2430),
    ("RX 7700 XT", "GDDR6", 192, 18.0, 2544),
    ("RX 7600 XT", "GDDR6", 128, 18.0, 2755),
    ("RX 7600", "GDDR6", 128, 18.0, 2655),
    ("RX 6950 XT", "GDDR6", 256, 18.0, 2310),
    ("RX 6900 XT", "GDDR6", 256, 16.0, 2250),
    ("RX 6800 XT", "GDDR6", 256, 16.0, 2250),
    ("RX 6800", "GDDR6", 256, 16.0, 2105),
    ("RX 6750 XT", "GDDR6", 192, 18.0, 2600),
    ("RX 6700 XT", "GDDR6", 192, 16.0, 2581),
    ("RX 6700", "GDDR6", 160, 16.0, 2450),
    ("RX 6650 XT", "GDDR6", 128, 17.5, 2635),
    ("RX 6600 XT", "GDDR6", 128, 16.0, 2589),
    ("RX 6600", "GDDR6", 128, 14.0, 2491),
    ("RX 6500 XT", "GDDR6", 64, 18.0, 2815),
    ("RX 6400", "GDDR6", 64, 16.0, 2321),
    ("RX 5700 XT", "GDDR6", 256, 14.0, 1905),
    ("RX 5700", "GDDR6", 256, 14.0, 1725),
    ("RX 5600 XT", "GDDR6", 192, 14.0, 1560),
    ("RX 5500 XT", "GDDR6", 128, 14.0, 1845),
    ("RX 590", "GDDR5", 256, 8.0, 1545),
    ("RX 580", "GDDR5", 256, 8.0, 1340),
    ("RX 570", "GDDR5", 256, 7.0, 1244),
    // NVIDIA
    ("RTX 5090", "GDDR7", 512, 28.0, 2407),
    ("RTX 5080", "GDDR7", 256, 30.0, 2617),
    ("RTX 5070 TI", "GDDR7", 256, 28.0, 2452),
    ("RTX 5070", "GDDR7", 192, 28.0, 2512),
    ("RTX 5060 TI", "GDDR7", 128, 28.0, 2572),
    ("RTX 5060", "GDDR7", 128, 28.0, 2497),
    ("RTX 4090", "GDDR6X", 384, 21.0, 2520),
    ("RTX 4080 SUPER", "GDDR6X", 256, 23.0, 2550),
    ("RTX 4080", "GDDR6X", 256, 22.4, 2505),
    ("RTX 4070 TI SUPER", "GDDR6X", 256, 21.0, 2610),
    ("RTX 4070 TI", "GDDR6X", 192, 21.0, 2610),
    ("RTX 4070 SUPER", "GDDR6X", 192, 21.0, 2475),
    ("RTX 4070", "GDDR6X", 192, 21.0, 2475),
    ("RTX 4060 TI", "GDDR6", 128, 18.0, 2535),
    ("RTX 4060", "GDDR6", 128, 17.0, 2460),
    ("RTX 3090 TI", "GDDR6X", 384, 21.0, 1860),
    ("RTX 3090", "GDDR6X", 384, 19.5, 1695),
    ("RTX 3080 TI", "GDDR6X", 384, 19.0, 1665),
    ("RTX 3080", "GDDR6X", 320, 19.0, 1710),
    ("RTX 3070 TI", "GDDR6X", 256, 19.0, 1770),
    ("RTX 3070", "GDDR6", 256, 14.0, 1725),
    ("RTX 3060 TI", "GDDR6", 256, 14.0, 1665),
    ("RTX 3060", "GDDR6", 192, 15.0, 1777),
    ("RTX 3050", "GDDR6", 128, 14.0, 1777),
    ("RTX 2080 TI", "GDDR6", 352, 14.0, 1545),
    ("RTX 2080 SUPER", "GDDR6", 256, 15.5, 1815),
    ("RTX 2080", "GDDR6", 256, 14.0, 1710),
    ("RTX 2070 SUPER", "GDDR6", 256, 14.0, 1770),
    ("RTX 2070", "GDDR6", 256, 14.0, 1620),
    ("RTX 2060 SUPER", "GDDR6", 256, 14.0, 1650),
    ("RTX 2060", "GDDR6", 192, 14.0, 1680),
    ("GTX 1660 TI", "GDDR6", 192, 12.0, 1770),
    ("GTX 1660 SUPER", "GDDR6", 192, 14.0, 1785),
    ("GTX 1660", "GDDR5", 192, 8.0, 1785),
    ("GTX 1080 TI", "GDDR5X", 352, 11.0, 1582),
    ("GTX 1080", "GDDR5X", 256, 10.0, 1733),
    ("GTX 1070", "GDDR5", 256, 8.0, 1683),
    ("GTX 1060", "GDDR5", 192, 8.0, 1708),
    // Intel
    ("ARC B580", "GDDR6", 192, 19.0, 2670),
    ("ARC B570", "GDDR6", 160, 19.0, 2500),
    ("ARC A770", "GDDR6", 256, 17.5, 2400),
    ("ARC A750", "GDDR6", 256, 16.0, 2400),
    ("ARC A580", "GDDR6", 256, 16.0, 2000),
    ("ARC A380", "GDDR6", 96, 15.5, 2000),
];

/// Egy videókártya ismert adatai a neve alapján (laptopos változatnál nincs: ott mások).
fn gpu_spec(name: &str) -> Option<(&'static str, u32, f32, u32)> {
    let up = name.to_uppercase();
    if up.contains("LAPTOP") || up.contains("MOBILE") || up.contains("MAX-Q") {
        return None;
    }
    let mut specs: Vec<&(&str, &str, u32, f32, u32)> = GPU_SPECS.iter().collect();
    specs.sort_by_key(|s| std::cmp::Reverse(s.0.len()));
    specs
        .into_iter()
        .find(|s| {
            up.match_indices(s.0).any(|(i, _)| {
                let after = &up[i + s.0.len()..];
                // a név itt véget ér (pl. „RX 6600” után nem „0” jön)
                !after.starts_with(|c: char| c.is_ascii_alphanumeric())
            })
        })
        .map(|s| (s.1, s.2, s.3, s.4))
}

/// Az alaplap lapkakészlete a típusnevéből (pl. „B450M Pro4” → B450, „X670E Taichi” → X670E).
fn chipset_of(product: &str) -> Option<String> {
    product.split(|c: char| !c.is_ascii_alphanumeric()).find_map(|tok| {
        let t = tok.to_uppercase();
        let b = t.as_bytes();
        if b.len() < 4 || !b"ABXHZQ".contains(&b[0]) || !b[1..4].iter().all(u8::is_ascii_digit) {
            return None;
        }
        match &t[4..] {
            "" | "M" => Some(t[..4].to_string()),
            "E" | "EM" => Some(format!("{}E", &t[..4])),
            _ => None,
        }
    })
}

fn hardware_summary(raw: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;
    let s = |v: &serde_json::Value| v.as_str().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
    let list = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        match v {
            serde_json::Value::Array(a) => a.clone(),
            serde_json::Value::Null => Vec::new(),
            o => vec![o.clone()],
        }
    };
    let gb = |b: f64| {
        let g = b / 1_073_741_824.0;
        if g >= 10.0 || (g - g.round()).abs() < 0.05 { format!("{:.0} GB", g) } else { format!("{:.1} GB", g) }
    };
    let c = &raw["cpu"];
    let cpu_name = s(&c["Name"]);
    let cpu = json!({
        "name": cpu_name,
        "cores": c["NumberOfCores"],
        "threads": c["NumberOfLogicalProcessors"],
        "mhz": c["MaxClockSpeed"],
    });
    // alaplap: gyártó és típus, lapkakészlet, BIOS, memóriafoglalatok
    let product = s(&raw["board"]["Product"]);
    let vendor = if cpu_name.to_uppercase().contains("INTEL") {
        "Intel"
    } else if cpu_name.to_uppercase().contains("AMD") {
        "AMD"
    } else {
        ""
    };
    let chipset = chipset_of(&product).map(|c| format!("{vendor} {c}").trim().to_string());
    let slots_total = raw["slots"]["MemoryDevices"].as_u64().unwrap_or(0);
    let max_mem = raw["slots"]["MaxCapacityEx"].as_f64().map(|kb| gb(kb * 1024.0)).unwrap_or_default();
    let board = json!({
        "name": format!("{} {}", s(&raw["board"]["Manufacturer"]), product).trim(),
        "chipset": chipset,
        "bios": s(&raw["bios"]["SMBIOSBIOSVersion"]),
        "biosDate": s(&raw["bios"]["date"]),
        "slots": slots_total,
        "maxMem": max_mem,
    });
    // memória: modulonként (méret, típus, sebesség, gyártó, típusszám, foglalat)
    let kind = |t: u64| match t {
        20 => "DDR",
        21 | 22 => "DDR2",
        24 => "DDR3",
        26 => "DDR4",
        27 => "LPDDR",
        28 => "LPDDR2",
        29 => "LPDDR3",
        30 => "LPDDR4",
        34 => "DDR5",
        35 => "LPDDR5",
        _ => "",
    };
    let mut total = 0f64;
    let mut modules = Vec::new();
    for m in list(&raw["mem"]) {
        let cap = m["Capacity"].as_f64().unwrap_or(0.0);
        total += cap;
        let maker = s(&m["Manufacturer"]);
        let maker = if maker.eq_ignore_ascii_case("unknown") || maker.starts_with("0000") { String::new() } else { maker };
        modules.push(json!({
            "size": gb(cap),
            "type": kind(m["SMBIOSMemoryType"].as_u64().unwrap_or(0)),
            "speed": m["Speed"].as_u64().unwrap_or(0),
            "configured": m["ConfiguredClockSpeed"].as_u64().unwrap_or(0),
            "maker": maker,
            "part": s(&m["PartNumber"]),
            "slot": s(&m["DeviceLocator"]),
        }));
    }
    let types: Vec<&str> = modules.iter().filter_map(|m| m["type"].as_str()).filter(|t| !t.is_empty()).collect();
    let mem_type = if !types.is_empty() && types.iter().all(|t| *t == types[0]) { types[0].to_string() } else { String::new() };
    // a modulok a leglassabb sebességén járnak együtt
    let run = modules
        .iter()
        .filter_map(|m| m["configured"].as_u64().filter(|v| *v > 0).or(m["speed"].as_u64()))
        .filter(|v| *v > 0)
        .min()
        .unwrap_or(0);
    let same = modules.windows(2).all(|w| w[0]["size"] == w[1]["size"] && w[0]["maker"] == w[1]["maker"] && w[0]["part"] == w[1]["part"]);
    // videókártyák: a pontos memóriaméret a registryből (a WMI 4 GB-nál levág), a többi a táblából
    let vram: Vec<(String, f64)> = list(&raw["gpumem"]).iter().map(|g| (s(&g["DriverDesc"]), g["mem"].as_f64().unwrap_or(0.0))).collect();
    let gpus: Vec<serde_json::Value> = list(&raw["gpu"])
        .iter()
        .map(|g| {
            let name = s(&g["Name"]);
            let mem = vram.iter().find(|(n, _)| *n == name).map(|(_, m)| *m).or(g["AdapterRAM"].as_f64()).unwrap_or(0.0);
            let spec = gpu_spec(&name);
            json!({
                "name": name,
                "vram": if mem > 0.0 { gb(mem) } else { String::new() },
                "memType": spec.map(|x| x.0),
                "bus": spec.map(|x| x.1),
                "gbps": spec.map(|x| x.2),
                "boost": spec.map(|x| x.3),
            })
        })
        .filter(|g| !g["name"].as_str().unwrap_or("").is_empty())
        .collect();
    let os = format!("{} ({})", s(&raw["os"]["Caption"]).trim_start_matches("Microsoft "), s(&raw["os"]["BuildNumber"]));
    // háttértárak: a gyártói (tízes) méret, és a rajtuk lévő meghajtók szabad helye (mint az Intézőben)
    let dec = |b: f64| {
        if b >= 1e12 { format!("{} TB", format!("{:.1}", b / 1e12).trim_end_matches(".0")) } else { format!("{:.0} GB", b / 1e9) }
    };
    let vols = list(&raw["vols"]);
    let disks: Vec<serde_json::Value> = list(&raw["disks"])
        .iter()
        .map(|d| {
            let id = d["DeviceId"].as_str().map(str::to_string).or_else(|| d["DeviceId"].as_u64().map(|n| n.to_string())).unwrap_or_default();
            let letters: Vec<String> = list(&raw["parts"])
                .iter()
                .filter(|p| p["DiskNumber"].as_u64().map(|n| n.to_string()) == Some(id.clone()))
                .filter_map(|p| p["L"].as_str().map(|l| format!("{l}:")))
                .collect();
            let volumes: Vec<serde_json::Value> = letters
                .iter()
                .filter_map(|l| vols.iter().find(|v| v["DeviceID"].as_str() == Some(l.as_str())))
                .map(|v| {
                    json!({
                        "letter": s(&v["DeviceID"]),
                        "label": s(&v["VolumeName"]),
                        "free": gb(v["FreeSpace"].as_f64().unwrap_or(0.0)),
                        "total": gb(v["Size"].as_f64().unwrap_or(0.0)),
                        "pct": v["Size"].as_f64().filter(|t| *t > 0.0).map(|t| (100.0 * (1.0 - v["FreeSpace"].as_f64().unwrap_or(0.0) / t)).round()),
                    })
                })
                .collect();
            let text = |v: &serde_json::Value| match v {
                serde_json::Value::String(x) if x != "Unspecified" => x.clone(),
                _ => String::new(),
            };
            json!({
                "name": s(&d["FriendlyName"]),
                "media": text(&d["MediaType"]),
                "bus": text(&d["BusType"]),
                "size": dec(d["Size"].as_f64().unwrap_or(0.0)),
                "volumes": volumes,
            })
        })
        .collect();
    json!({
        "board": board,
        "cpu": cpu,
        "mem": {
            "total": if total > 0.0 { gb(total) } else { String::new() },
            "type": mem_type,
            "speed": run,
            "slots": slots_total,
            "same": same,
            "modules": modules,
        },
        "gpus": gpus,
        "disks": disks,
        "os": os,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_names() {
        assert_eq!(gpu_spec("AMD Radeon RX 6600").map(|s| s.1), Some(128));
        assert_eq!(gpu_spec("AMD Radeon RX 6600 XT").map(|s| s.3), Some(2589));
        assert_eq!(gpu_spec("NVIDIA GeForce RTX 4070 Ti SUPER").map(|s| s.1), Some(256));
        assert_eq!(gpu_spec("NVIDIA GeForce RTX 4070 Laptop GPU"), None);
        assert_eq!(gpu_spec("Microsoft Basic Display Adapter"), None);
    }

    #[test]
    fn chipsets() {
        assert_eq!(chipset_of("B450M Pro4").as_deref(), Some("B450"));
        assert_eq!(chipset_of("X670E Taichi").as_deref(), Some("X670E"));
        assert_eq!(chipset_of("ROG STRIX Z790-E GAMING WIFI").as_deref(), Some("Z790"));
        assert_eq!(chipset_of("PRIME"), None);
    }
}

// ---- Internetkapcsolat ----

/// Van-e internetkapcsolat (a Windows hálózatkezelője szerint, IPv4 vagy IPv6).
/// `None`, ha nem kérdezhető le. A COM-ot a fő szál már elindította.
pub fn internet_ok() -> Option<bool> {
    use windows::Win32::Networking::NetworkListManager::{
        INetworkListManager, NLM_CONNECTIVITY_IPV4_INTERNET, NLM_CONNECTIVITY_IPV6_INTERNET, NetworkListManager,
    };
    unsafe {
        let nlm: INetworkListManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).ok()?;
        let c = nlm.GetConnectivity().ok()?;
        Some(c.0 & (NLM_CONNECTIVITY_IPV4_INTERNET.0 | NLM_CONNECTIVITY_IPV6_INTERNET.0) != 0)
    }
}
