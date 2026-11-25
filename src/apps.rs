//! A Start menü és a kereső adatai: a telepített programok (a Windows „Alkalmazások”
//! mappája, benne a Store-appok is), a legutóbbi fájlok és a programok ikonja.
//!
//! A lista ugyanaz, amit a Windows saját Start menüje „Minden alkalmazás” néven mutat:
//! a `shell:AppsFolder` elemei. Mindegyiknek van egy azonosítója (a mappán belüli
//! értelmezési neve), ezzel indítható: `shell:AppsFolder\<azonosító>`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use windows::Win32::Foundation::{PROPERTYKEY, SIZE};
use windows::Win32::Graphics::Gdi::{DeleteObject, HBITMAP, HGDIOBJ};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, FOLDERID_Recent, IEnumShellItems, IShellItem, IShellItem2, IShellItemImageFactory,
    KF_FLAG_DEFAULT, SHCreateItemFromParsingName, SHGetKnownFolderItem, SHGetKnownFolderPath, SIGDN_NORMALDISPLAY,
    SIGDN_PARENTRELATIVEPARSING, SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT,
};
use windows::core::{GUID, HSTRING, PWSTR};

use crate::bitmap;

/// Egy telepített program a Start menüben.
#[derive(Clone, Debug)]
pub struct StartApp {
    /// Azonosító: a `shell:AppsFolder` mappán belüli név (Store-appnál az AUMID).
    pub id: String,
    pub name: String,
    /// Asztali programnál a parancsikon célja (exe), Store-appnál üres.
    pub target: String,
}

/// A legutóbb felsorolt programok (a felület ikonkéréseihez is).
static APPS: Mutex<Vec<StartApp>> = Mutex::new(Vec::new());
/// Az ikonok PNG-ként, azonosító szerint (None: nincs ikon).
static ICONS: Mutex<Option<HashMap<String, Option<Vec<u8>>>>> = Mutex::new(None);

/// System.Link.TargetParsingPath: a parancsikon céljának teljes útvonala.
const PKEY_LINK_TARGET: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25), pid: 2 };

fn take_pwstr(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = unsafe { p.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
    s
}

/// Ezek nem programok (súgó, weboldal, eltávolító, dokumentáció): a Windows 11 Start
/// menüje is elrejti őket.
fn is_junk(name: &str, id: &str, target: &str) -> bool {
    let n = name.to_lowercase();
    let t = if target.is_empty() { id.to_lowercase() } else { target.to_lowercase() };
    const WORDS: &[&str] =
        &["uninstall", "eltávolít", "readme", "read me", "release notes", "license", "licence", "help ", "website", "documentation"];
    const EXTS: &[&str] = &[".url", ".chm", ".txt", ".pdf", ".htm", ".html", ".rtf", ".md", ".log", ".ini"];
    WORDS.iter().any(|w| n.contains(w) || n.ends_with(w.trim()))
        || EXTS.iter().any(|e| t.ends_with(e))
        || t.starts_with("http://")
        || t.starts_with("https://")
        || t.contains("\\unins") && t.ends_with(".exe")
}

/// A Windows része-e (és nem telepített program): a Windows mappájában lévő eszközök (Parancssor,
/// PowerShell, Karaktertábla…), a felügyeleti konzolok (.msc), a héj elemei (Fájlkezelő,
/// Vezérlőpult, Futtatás), a beépített rendszercsomagok (Gépház, Windows biztonság, Első
/// lépések…) és a mappára vagy más nem futtatható célra mutató parancsikonok. A Start menü
/// „Minden alkalmazás” szűrője ez alapján választja szét a telepített programokat.
pub fn is_system(id: &str, target: &str) -> bool {
    let t = target.to_lowercase();
    if t.is_empty() {
        // Store-app (csomagnév!belépési pont): a Windows saját csomagjai rendszerelemek
        let family = id.split('!').next().unwrap_or("").to_lowercase();
        return !id.contains('!')
            || family.starts_with("windows.")
            || family.starts_with("microsoftwindows.")
            || family.starts_with("microsoft.sechealthui");
    }
    if t.starts_with("steam://") {
        return false;
    }
    if t.ends_with(".exe") {
        return t.contains(r"\windows\");
    }
    // .msc, ::{CLSID}, mappa és minden más nem futtatható cél
    true
}

/// A Windows „Alkalmazások” mappájának elemei, név szerint rendezve.
/// A hívó szálon COM-nak inicializálva kell lennie.
pub fn enumerate() -> Vec<StartApp> {
    let mut out: Vec<StartApp> = Vec::new();
    unsafe {
        let Ok(folder) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) else {
            return out;
        };
        let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else {
            return out;
        };
        loop {
            let mut batch: [Option<IShellItem>; 32] = Default::default();
            let mut fetched = 0u32;
            if items.Next(&mut batch, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            for item in batch.iter().take(fetched as usize).flatten() {
                let name = item.GetDisplayName(SIGDN_NORMALDISPLAY).map(take_pwstr).unwrap_or_default();
                let id = item.GetDisplayName(SIGDN_PARENTRELATIVEPARSING).map(take_pwstr).unwrap_or_default();
                if name.trim().is_empty() || id.is_empty() {
                    continue;
                }
                let target =
                    item.cast::<IShellItem2>().ok().and_then(|i2| i2.GetString(&PKEY_LINK_TARGET).ok()).map(take_pwstr).unwrap_or_default();
                if is_junk(&name, &id, &target) {
                    continue;
                }
                // ugyanaz a program többször (pl. két parancsikon ugyanarra): egyszer kell
                if out.iter().any(|a| a.name == name && (a.id == id || (!target.is_empty() && a.target.eq_ignore_ascii_case(&target)))) {
                    continue;
                }
                out.push(StartApp { id, name, target });
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

use windows::core::Interface;

/// Háttérszálon felsorolja a programokat, és előre elkészíti az ikonjukat.
/// `done` a kész listát kapja (még az ikonok előtt, hogy a menü azonnal használható legyen).
pub fn refresh(done: impl FnOnce(Vec<StartApp>) + Send + 'static) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let list = enumerate();
        if let Ok(mut g) = APPS.lock() {
            *g = list.clone();
        }
        let ids: Vec<String> = list.iter().map(|a| a.id.clone()).collect();
        done(list);
        for id in ids {
            let cached = ICONS.lock().ok().is_some_and(|g| g.as_ref().is_some_and(|m| m.contains_key(&id)));
            if !cached {
                let png = extract(&id);
                if let Ok(mut g) = ICONS.lock() {
                    g.get_or_insert_with(HashMap::new).insert(id, png);
                }
            }
        }
    });
}

/// Egy program ikonja PNG-ként (a felület `/__appicon/<azonosító>` címen kéri).
pub fn icon_png(id: &str) -> Option<Vec<u8>> {
    if let Ok(g) = ICONS.lock()
        && let Some(hit) = g.as_ref().and_then(|m| m.get(id))
    {
        return hit.clone();
    }
    let png = extract(id);
    if let Ok(mut g) = ICONS.lock() {
        g.get_or_insert_with(HashMap::new).insert(id.to_string(), png.clone());
    }
    png
}

/// Egy futó program neve a Start menü listájából (a Windows honosított neve, pl. „Jegyzettömb”
/// a Notepad.exe-hez). Asztali programnál a parancsikon célja egyezik, Store-appnál a csomag:
/// a `WindowsApps\<Név>_<verzió>_<arch>__<kiadó>\…` útvonalból a `<Név>_<kiadó>!` azonosító-előtag.
pub fn name_for_path(list: &[StartApp], path: &str) -> Option<String> {
    app_for_path(list, path).map(|a| a.name.clone())
}

/// Egy futó program bejegyzése a Start menü listájában (lásd `name_for_path`).
pub fn app_for_path<'a>(list: &'a [StartApp], path: &str) -> Option<&'a StartApp> {
    if let Some(a) = list.iter().find(|a| !a.target.is_empty() && a.target.eq_ignore_ascii_case(path)) {
        return Some(a);
    }
    let lower = path.to_lowercase();
    let rest = &path[lower.find("\\windowsapps\\")? + "\\windowsapps\\".len()..];
    let dir = rest.split('\\').next()?;
    let (name, publisher) = (dir.split('_').next()?, dir.rsplit_once("__")?.1);
    let family = format!("{name}_{publisher}!").to_lowercase();
    let hits: Vec<&StartApp> = list.iter().filter(|a| a.id.to_lowercase().starts_with(&family)).collect();
    // több belépési pontú csomagnál (pl. Office) nem találgatunk
    (hits.len() == 1).then(|| hits[0])
}

/// A program ismert-e (csak ismert azonosítót indítunk el).
pub fn find(id: &str) -> Option<StartApp> {
    APPS.lock().ok().and_then(|g| g.iter().find(|a| a.id == id).cloned())
}

fn extract(id: &str) -> Option<Vec<u8>> {
    unsafe {
        let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(format!("shell:AppsFolder\\{id}")), None).ok()?;
        let factory: IShellItemImageFactory = item.cast().ok()?;
        let bmp = factory.GetImage(SIZE { cx: 48, cy: 48 }, SIIGBF_RESIZETOFIT | SIIGBF_ICONONLY).ok()?;
        let png = hbitmap_to_png(bmp);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        png
    }
}

/// A héjtól kapott (előszorzott alfájú) 32 bites kép PNG-ként.
fn hbitmap_to_png(bmp: HBITMAP) -> Option<Vec<u8>> {
    let (w, h) = bitmap::size(bmp);
    let h = h.abs();
    if w <= 0 || h <= 0 || w > 512 || h > 512 {
        return None;
    }
    bitmap::bgra_to_png(bitmap::pixels(bmp, w, h)?, w, h, true)
}

/// A Windows „Legutóbbi elemek” mappája.
fn recent_dir() -> Option<PathBuf> {
    unsafe { SHGetKnownFolderPath(&FOLDERID_Recent, KF_FLAG_DEFAULT, None) }.map(take_pwstr).map(PathBuf::from).ok()
}

/// Egy legutóbbi fájl levétele a listáról: csak a parancsikonja törlődik a „Legutóbbi elemek”
/// mappából (ahogy a Windows „Eltávolítás a listából” parancsa), maga a fájl megmarad.
/// Csak ebben a mappában lévő .lnk fájlt törlünk.
pub fn remove_recent(path: &str) -> bool {
    let Some(dir) = recent_dir() else { return false };
    let p = PathBuf::from(path);
    let inside = p.parent().is_some_and(|d| d.as_os_str().eq_ignore_ascii_case(dir.as_os_str()));
    let lnk = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk"));
    inside && lnk && std::fs::remove_file(&p).is_ok()
}

/// Az összes legutóbbi fájl levétele a listáról (a mappák parancsikonjai maradnak).
/// Visszaadja, hányat vett le.
pub fn clear_recent() -> usize {
    recent_files(usize::MAX).iter().filter(|(_, p)| remove_recent(p)).count()
}

/// A legutóbb megnyitott fájlok (a Windows „Legutóbbi elemek” mappájából), a legújabb elöl.
/// Név és a parancsikon útvonala (azt nyitjuk meg).
pub fn recent_files(limit: usize) -> Vec<(String, String)> {
    let Some(dir) = recent_dir() else { return Vec::new() };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files
        .into_iter()
        .take(limit)
        .filter_map(|(_, p)| {
            let name = p.file_stem()?.to_string_lossy().to_string();
            // a mappák parancsikonjának nincs kiterjesztése: azokat kihagyjuk
            std::path::Path::new(&name).extension()?;
            Some((name, p.to_string_lossy().to_string()))
        })
        .collect()
}

/// Egy szöveges érték a HKLM alól (üres vagy hiányzó érték: None).
fn hklm_string(key: &str, value: &str) -> Option<String> {
    crate::reg::get_string(windows::Win32::System::Registry::HKEY_LOCAL_MACHINE, key, value)
}

/// A bejelentkezett felhasználó egy adata a bejelentkező képernyő nyilvántartásából
/// (LogonUI\SessionData), pl. a teljes neve vagy a biztonsági azonosítója (SID).
fn logon_value(value: &str) -> Option<String> {
    let user = std::env::var("USERNAME").unwrap_or_default().to_lowercase();
    if user.is_empty() {
        return None;
    }
    (0..32).find_map(|i| {
        let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\LogonUI\SessionData\{i}");
        let sam = hklm_string(&key, "LoggedOnSAMUser")?.to_lowercase();
        if !sam.ends_with(&format!("\\{user}")) {
            return None;
        }
        hklm_string(&key, value)
    })
}

/// A bejelentkezett felhasználó teljes neve (a Start menü aljára): ugyanaz, amit a Windows a
/// bejelentkező képernyőn mutat. Ha nem található, a felhasználónév.
pub fn user_name() -> String {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| logon_value("LoggedOnDisplayName").unwrap_or_else(|| std::env::var("USERNAME").unwrap_or_default())).clone()
}

/// A Windows-fiók profilképe (96 pontos, a Windows ennyit tárol több méretben is): a fájl
/// útvonala, ha a felhasználó beállított képet. A Start menü a neve mellett mutatja.
pub fn user_picture() -> Option<PathBuf> {
    let sid = logon_value("LoggedOnUserSID")?;
    let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AccountPicture\Users\{sid}");
    ["Image96", "Image192", "Image64"].iter().find_map(|v| hklm_string(&key, v).map(PathBuf::from).filter(|p| p.is_file()))
}

#[cfg(test)]
mod tests {
    use super::is_system;

    #[test]
    fn installed_or_system() {
        // telepített programok
        assert!(!is_system("Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"));
        assert!(!is_system("ai.artcraft.app", r"C:\Users\x\AppData\Local\ArtCraft\artcraft.exe"));
        assert!(!is_system("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify", ""));
        assert!(!is_system("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App", ""));
        assert!(!is_system("steam://rungameid/1091500", "steam://rungameid/1091500"));
        // a Windows részei
        assert!(is_system(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\cmd.exe", r"C:\WINDOWS\system32\cmd.exe"));
        assert!(is_system("Microsoft.AutoGenerated.{X}", r"C:\WINDOWS\system32\services.msc"));
        assert!(is_system("Microsoft.Windows.Explorer", "::{52205FD8-5DFB-447D-801A-D0B52F2E83E1}"));
        assert!(is_system("windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel", ""));
        assert!(is_system("MicrosoftWindows.Client.CBS_cw5n1h2txyewy!WindowsBackup", ""));
        assert!(is_system("Microsoft.SecHealthUI_8wekyb3d8bbwe!SecHealthUI", ""));
        assert!(is_system(r"C:\xampp7\htdocs", r"C:\xampp7\htdocs"));
    }
}
