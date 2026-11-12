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
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, HBITMAP, HGDIOBJ, ReleaseDC,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, FOLDERID_Recent, IEnumShellItems, IShellItem, IShellItem2, IShellItemImageFactory,
    KF_FLAG_DEFAULT, SHCreateItemFromParsingName, SHGetKnownFolderItem, SHGetKnownFolderPath, SIGDN_NORMALDISPLAY,
    SIGDN_PARENTRELATIVEPARSING, SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT,
};
use windows::core::{GUID, HSTRING, PWSTR};

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
    if let Some(a) = list.iter().find(|a| !a.target.is_empty() && a.target.eq_ignore_ascii_case(path)) {
        return Some(a.name.clone());
    }
    let lower = path.to_lowercase();
    let rest = &path[lower.find("\\windowsapps\\")? + "\\windowsapps\\".len()..];
    let dir = rest.split('\\').next()?;
    let (name, publisher) = (dir.split('_').next()?, dir.rsplit_once("__")?.1);
    let family = format!("{name}_{publisher}!").to_lowercase();
    let hits: Vec<&StartApp> = list.iter().filter(|a| a.id.to_lowercase().starts_with(&family)).collect();
    // több belépési pontú csomagnál (pl. Office) nem találgatunk
    (hits.len() == 1).then(|| hits[0].name.clone())
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
unsafe fn hbitmap_to_png(bmp: HBITMAP) -> Option<Vec<u8>> {
    unsafe {
        let mut bm = BITMAP::default();
        GetObjectW(HGDIOBJ(bmp.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
        let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
        if w <= 0 || h <= 0 || w > 512 || h > 512 {
            return None;
        }
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (w * h * 4) as usize];
        let dc = GetDC(None);
        let lines = GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        if lines == 0 {
            return None;
        }
        let has_alpha = px.chunks(4).any(|p| p[3] != 0);
        for p in px.chunks_mut(4) {
            p.swap(0, 2); // BGRA → RGBA
            if !has_alpha {
                p[3] = 255;
            } else if p[3] > 0 && p[3] < 255 {
                // előszorzott alfa → egyenes alfa
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().ok()?;
            wr.write_image_data(&px).ok()?;
        }
        Some(out)
    }
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

/// A bejelentkezett felhasználó teljes neve (a Start menü aljára): ugyanaz, amit a Windows a
/// bejelentkező képernyőn mutat. Ha nem található, a felhasználónév.
pub fn user_name() -> String {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        let user = std::env::var("USERNAME").unwrap_or_default();
        let read = |key: &str, value: &str| -> Option<String> {
            let mut buf = [0u16; 260];
            let mut len = (buf.len() * 2) as u32;
            let r = unsafe {
                RegGetValueW(
                    HKEY_LOCAL_MACHINE,
                    &HSTRING::from(key),
                    &HSTRING::from(value),
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut len),
                )
            };
            if r.is_err() {
                return None;
            }
            let n = (len as usize / 2).saturating_sub(1).min(buf.len());
            Some(String::from_utf16_lossy(&buf[..n]).trim().to_string())
        };
        for i in 0..32 {
            let key = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\LogonUI\\SessionData\\{i}");
            let sam = read(&key, "LoggedOnSAMUser").unwrap_or_default();
            if !user.is_empty()
                && sam.to_lowercase().ends_with(&format!("\\{}", user.to_lowercase()))
                && let Some(name) = read(&key, "LoggedOnDisplayName").filter(|n| !n.is_empty())
            {
                return name;
            }
        }
        user
    })
    .clone()
}
