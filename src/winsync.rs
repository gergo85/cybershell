//! Igazodás a Windowshoz: a Windows sötét/világos módja és kiemelőszíne, a Windows Terminal
//! színsémája és a neonkeret más programok ablakain (Windows 11, DWM ablakkeret-szín).
//!
//! Az első bekapcsoláskor az eredeti értékeket egy mentésbe (`winsync-backup.json` a
//! CyberShell adatmappájában) tesszük, kikapcsoláskor ezekből állítjuk vissza őket.
//! A registry írása és a Windows értesítése háttérszálon fut (a körüzenet lassú lehet).

use crate::config::{Theme, WindowsSync};
use crate::{log, reg};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DwmSetWindowAttribute};
use windows::Win32::System::Registry::HKEY_CURRENT_USER;
use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE};
use windows::core::HSTRING;

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
const DWM: &str = r"Software\Microsoft\Windows\DWM";
const DESKTOP: &str = r"Control Panel\Desktop";

/// A Windows Terminalban létrehozott színsémák neve.
pub const TERM_DARK: &str = "CyberShell Dark";
pub const TERM_LIGHT: &str = "CyberShell Light";

/// Egyszerre csak egy szinkron fusson (a mentésfájlt és a registryt is írják).
static LOCK: Mutex<()> = Mutex::new(());

// ───────────── registry ─────────────

fn get_dword(key: &str, name: &str) -> Option<u32> {
    reg::get_dword(HKEY_CURRENT_USER, key, name)
}

fn set_dword(key: &str, name: &str, v: u32) -> bool {
    reg::set_dword(HKEY_CURRENT_USER, key, name, v)
}

fn get_binary(key: &str, name: &str) -> Option<Vec<u8>> {
    reg::get_binary(HKEY_CURRENT_USER, key, name)
}

fn set_binary(key: &str, name: &str, data: &[u8]) -> bool {
    reg::set_binary(HKEY_CURRENT_USER, key, name, data)
}

/// Szól a futó programoknak (Intéző, Beállítások, Terminal…), hogy a színek megváltoztak.
fn broadcast_colors() {
    let what = HSTRING::from("ImmersiveColorSet");
    unsafe {
        let _ =
            SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(what.as_ptr() as isize), SMTO_ABORTIFHUNG, 300, None);
    }
}

/// A Windows alkalmazás-módja: igaz, ha világos (None, ha nem olvasható).
pub fn windows_light() -> Option<bool> {
    get_dword(PERSONALIZE, "AppsUseLightTheme").map(|v| v != 0)
}

// ───────────── színek ─────────────

type Rgb = (u8, u8, u8);

pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.trim().trim_start_matches('#');
    if h.len() < 6 {
        return None;
    }
    let c = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some((c(0)?, c(2)?, c(4)?))
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// `a` és `b` keveréke: t = 0 → a, t = 1 → b.
fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round().clamp(0.0, 255.0) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

const WHITE: Rgb = (255, 255, 255);
const BLACK: Rgb = (0, 0, 0);

/// COLORREF (0x00BBGGRR) a DWM-nek és a registrynek.
pub fn colorref(c: Rgb) -> u32 {
    c.0 as u32 | (c.1 as u32) << 8 | (c.2 as u32) << 16
}

// ───────────── neonkeret ─────────────

/// A Windows saját keretszíne (visszaállításhoz).
pub const BORDER_DEFAULT: u32 = 0xFFFF_FFFF;

/// Egy ablak keretének színe (Windows 11-en; régebbi Windowson nem csinál semmit).
pub fn set_border(hwnd: HWND, color: u32) -> bool {
    unsafe { DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &color as *const u32 as *const _, 4).is_ok() }
}

/// Az aktív és a többi ablak keretszíne a téma fő színéből (sötét témában a háttér felé
/// tompítva, világosban a fő szín halványabb változata).
pub fn border_colors(t: &Theme) -> (u32, u32) {
    let main = parse_hex(&t.main).unwrap_or((243, 230, 0));
    let bg = parse_hex(&t.bg).unwrap_or((6, 7, 11));
    (colorref(main), colorref(mix(main, bg, 0.62)))
}

// ───────────── mentés (az eredeti Windows-beállítások) ─────────────

fn backup_path() -> PathBuf {
    crate::data_dir().join("winsync-backup.json")
}

fn load_backup() -> Value {
    std::fs::read_to_string(backup_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}))
}

fn save_backup(v: &Value) {
    let empty = v.as_object().is_none_or(|o| o.is_empty());
    if empty {
        let _ = std::fs::remove_file(backup_path());
    } else if let Ok(t) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(backup_path(), t);
    }
}

fn to_hex_bytes(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn from_hex_bytes(s: &str) -> Vec<u8> {
    (0..s.len() / 2).filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()).collect()
}

// ───────────── sötét/világos mód ─────────────

fn apply_mode(light: bool, backup: &mut Value) -> bool {
    if backup.get("mode").is_none() {
        backup["mode"] = json!({
            "apps": get_dword(PERSONALIZE, "AppsUseLightTheme"),
            "system": get_dword(PERSONALIZE, "SystemUsesLightTheme"),
        });
    }
    let want = light as u32;
    let mut changed = false;
    for name in ["AppsUseLightTheme", "SystemUsesLightTheme"] {
        if get_dword(PERSONALIZE, name) != Some(want) && set_dword(PERSONALIZE, name, want) {
            changed = true;
        }
    }
    if changed {
        log(format!("Windows mód: {}", if light { "világos" } else { "sötét" }));
    }
    changed
}

fn restore_mode(backup: &mut Value) -> bool {
    let Some(m) = backup.get("mode").cloned() else { return false };
    let mut changed = false;
    for (k, name) in [("apps", "AppsUseLightTheme"), ("system", "SystemUsesLightTheme")] {
        if let Some(v) = m[k].as_u64()
            && get_dword(PERSONALIZE, name) != Some(v as u32)
        {
            changed |= set_dword(PERSONALIZE, name, v as u32);
        }
    }
    backup.as_object_mut().map(|o| o.remove("mode"));
    log("Windows mód: az eredeti visszaállítva");
    changed
}

// ───────────── kiemelőszín ─────────────

/// A Windows kiemelőszín-palettája (8 × RGBA): három világosabb árnyalat, maga a szín,
/// három sötétebb, a nyolcadik a meglévőből marad.
fn accent_palette(c: Rgb, old: Option<&[u8]>) -> Vec<u8> {
    let shades =
        [mix(c, WHITE, 0.55), mix(c, WHITE, 0.35), mix(c, WHITE, 0.15), c, mix(c, BLACK, 0.2), mix(c, BLACK, 0.4), mix(c, BLACK, 0.6)];
    let mut out = Vec::with_capacity(32);
    for s in shades {
        out.extend_from_slice(&[s.0, s.1, s.2, 0]);
    }
    match old {
        Some(o) if o.len() >= 32 => out.extend_from_slice(&o[28..32]),
        _ => out.extend_from_slice(&[0x88, 0x88, 0x88, 0]),
    }
    out
}

fn apply_accent(c: Rgb, backup: &mut Value) -> bool {
    let palette_old = get_binary(ACCENT, "AccentPalette");
    if backup.get("accent").is_none() {
        backup["accent"] = json!({
            "palette": palette_old.as_deref().map(to_hex_bytes),
            "menu": get_dword(ACCENT, "AccentColorMenu"),
            "start": get_dword(ACCENT, "StartColorMenu"),
            "dwmAccent": get_dword(DWM, "AccentColor"),
            "colorization": get_dword(DWM, "ColorizationColor"),
            "afterglow": get_dword(DWM, "ColorizationAfterglow"),
            "auto": get_dword(DESKTOP, "AutoColorization"),
        });
    }
    let abgr = |x: Rgb| 0xFF00_0000 | colorref(x);
    let menu = abgr(c);
    if get_dword(ACCENT, "AccentColorMenu") == Some(menu) && get_dword(DESKTOP, "AutoColorization").unwrap_or(0) == 0 {
        return false;
    }
    let palette = accent_palette(c, palette_old.as_deref());
    let argb = 0xC400_0000 | (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32;
    set_binary(ACCENT, "AccentPalette", &palette);
    set_dword(ACCENT, "AccentColorMenu", menu);
    set_dword(ACCENT, "StartColorMenu", abgr(mix(c, BLACK, 0.2)));
    set_dword(DWM, "AccentColor", menu);
    set_dword(DWM, "ColorizationColor", argb);
    set_dword(DWM, "ColorizationAfterglow", argb);
    // a háttérképből választott automatikus szín felülírná
    set_dword(DESKTOP, "AutoColorization", 0);
    log(format!("Windows kiemelőszín: {}", hex(c)));
    true
}

fn restore_accent(backup: &mut Value) -> bool {
    let Some(a) = backup.get("accent").cloned() else { return false };
    if let Some(p) = a["palette"].as_str() {
        set_binary(ACCENT, "AccentPalette", &from_hex_bytes(p));
    }
    for (k, key, name) in [
        ("menu", ACCENT, "AccentColorMenu"),
        ("start", ACCENT, "StartColorMenu"),
        ("dwmAccent", DWM, "AccentColor"),
        ("colorization", DWM, "ColorizationColor"),
        ("afterglow", DWM, "ColorizationAfterglow"),
        ("auto", DESKTOP, "AutoColorization"),
    ] {
        if let Some(v) = a[k].as_u64() {
            set_dword(key, name, v as u32);
        }
    }
    backup.as_object_mut().map(|o| o.remove("accent"));
    log("Windows kiemelőszín: az eredeti visszaállítva");
    true
}

// ───────────── Windows Terminal ─────────────

/// A Windows Terminal beállításfájljai (Store-os, Preview és telepítő nélküli változat).
fn terminal_files() -> Vec<PathBuf> {
    let Ok(local) = std::env::var("LOCALAPPDATA") else { return Vec::new() };
    let local = PathBuf::from(local);
    [
        local.join(r"Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"),
        local.join(r"Packages\Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe\LocalState\settings.json"),
        local.join(r"Microsoft\Windows Terminal\settings.json"),
    ]
    .into_iter()
    .filter(|p| p.is_file())
    .collect()
}

/// A Terminal beállításai JSON-kommentet és záró vesszőt is tartalmazhatnak: ezeket
/// kiszedjük (a szövegeken belül nem nyúlunk semmihez).
fn strip_jsonc(src: &str) -> String {
    let b: Vec<char> = src.trim_start_matches('\u{feff}').chars().collect();
    let mut out = String::with_capacity(b.len());
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            out.push(c);
            if c == '\\' && i + 1 < b.len() {
                out.push(b[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_str = true;
            out.push(c);
            i += 1;
        } else if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && b.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == ',' {
            // záró vessző: a következő nem üres karakter ] vagy }
            let next = b[i + 1..].iter().find(|x| !x.is_whitespace());
            if !matches!(next, Some(']') | Some('}')) {
                out.push(c);
            }
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Terminál-színséma a CyberShell egyik témájából.
fn term_scheme(name: &str, t: &Theme, light: bool) -> Value {
    let p = |s: &str, d: Rgb| parse_hex(s).unwrap_or(d);
    let bg = p(&t.bg, if light { (227, 233, 236) } else { (6, 7, 11) });
    let fg = p(&t.text, if light { (24, 38, 45) } else { (201, 204, 214) });
    let main = p(&t.main, (243, 230, 0));
    let alt = p(&t.alt, (0, 225, 255));
    let hot = p(&t.hot, (255, 42, 109));
    let (green, blue, purple) =
        if light { ((31, 138, 76), (44, 98, 166), (122, 79, 201)) } else { ((57, 255, 156), (77, 124, 255), (178, 107, 255)) };
    let (yellow, cyan) = if light { ((160, 120, 0), main) } else { (main, alt) };
    // a „fényes” változat sötét témában világosabb, világosban sötétebb
    let bright = |c: Rgb| if light { mix(c, BLACK, 0.25) } else { mix(c, WHITE, 0.3) };
    let (black, white, bright_black, bright_white) = if light {
        (fg, mix(fg, bg, 0.55), mix(fg, bg, 0.35), mix(fg, bg, 0.75))
    } else {
        (mix(bg, WHITE, 0.12), fg, mix(fg, bg, 0.5), WHITE)
    };
    json!({
        "name": name,
        "background": hex(bg),
        "foreground": hex(fg),
        "cursorColor": hex(main),
        "selectionBackground": hex(mix(alt, bg, if light { 0.6 } else { 0.55 })),
        "black": hex(black),
        "red": hex(hot),
        "green": hex(green),
        "yellow": hex(yellow),
        "blue": hex(blue),
        "purple": hex(purple),
        "cyan": hex(cyan),
        "white": hex(white),
        "brightBlack": hex(bright_black),
        "brightRed": hex(bright(hot)),
        "brightGreen": hex(bright(green)),
        "brightYellow": hex(bright(yellow)),
        "brightBlue": hex(bright(blue)),
        "brightPurple": hex(bright(purple)),
        "brightCyan": hex(bright(cyan)),
        "brightWhite": hex(bright_white),
    })
}

/// A Terminal saját formázásával (4 szóköz) írt JSON.
fn pretty4(v: &Value) -> Option<String> {
    use serde::Serialize;
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser).ok()?;
    String::from_utf8(buf).ok()
}

fn read_terminal(path: &PathBuf) -> Option<(String, Value)> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&strip_jsonc(&text)) {
        Ok(v) if v.is_object() => Some((text, v)),
        _ => {
            log(format!("Windows Terminal: a beállításfájl nem értelmezhető: {}", path.display()));
            None
        }
    }
}

fn write_terminal(path: &PathBuf, old_text: &str, v: &Value) -> bool {
    let Some(text) = pretty4(v) else { return false };
    let norm = |s: &str| s.trim_start_matches('\u{feff}').replace("\r\n", "\n").trim().to_string();
    if norm(&text) == norm(old_text) {
        return false;
    }
    let tmp = path.with_extension("json.cybershell-tmp");
    if std::fs::write(&tmp, text).is_err() || std::fs::rename(&tmp, path).is_err() {
        log(format!("Windows Terminal: nem írható: {}", path.display()));
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    true
}

fn apply_terminal(dark: &Theme, light: &Theme, light_active: bool, backup: &mut Value) {
    for path in terminal_files() {
        let Some((text, mut v)) = read_terminal(&path) else { continue };
        let key = path.to_string_lossy().to_string();
        if !v["profiles"].is_object() {
            continue;
        }
        if !v["profiles"]["defaults"].is_object() {
            v["profiles"]["defaults"] = json!({});
        }
        if backup["terminal"].get(&key).is_none() {
            backup["terminal"][&key] = v["profiles"]["defaults"].get("colorScheme").cloned().unwrap_or(Value::Null);
        }
        if !v["schemes"].is_array() {
            v["schemes"] = json!([]);
        }
        let schemes = v["schemes"].as_array_mut().expect("tömb");
        for s in [term_scheme(TERM_DARK, dark, false), term_scheme(TERM_LIGHT, light, true)] {
            match schemes.iter_mut().find(|x| x["name"] == s["name"]) {
                Some(x) => *x = s,
                None => schemes.push(s),
            }
        }
        v["profiles"]["defaults"]["colorScheme"] = json!(if light_active { TERM_LIGHT } else { TERM_DARK });
        if write_terminal(&path, &text, &v) {
            log(format!("Windows Terminal: {} színséma", if light_active { TERM_LIGHT } else { TERM_DARK }));
        }
    }
}

fn restore_terminal(backup: &mut Value) {
    let Some(saved) = backup.get("terminal").and_then(|t| t.as_object()).cloned() else { return };
    for (key, prev) in saved {
        let path = PathBuf::from(&key);
        let Some((text, mut v)) = read_terminal(&path) else { continue };
        if let Some(d) = v["profiles"]["defaults"].as_object_mut() {
            match prev {
                Value::Null => {
                    d.remove("colorScheme");
                }
                p => {
                    d.insert("colorScheme".into(), p);
                }
            }
        }
        write_terminal(&path, &text, &v);
    }
    backup.as_object_mut().map(|o| o.remove("terminal"));
    log("Windows Terminal: az eredeti színséma visszaállítva");
}

/// Van-e Windows Terminal a gépen (a beállításokban ettől függ a kapcsoló felirata).
pub fn terminal_installed() -> bool {
    !terminal_files().is_empty()
}

// ───────────── összefogva ─────────────

/// A kért állapot átvezetése a Windowsra, háttérszálon. `follow_windows`: a CyberShell épp a
/// Windows módját követi, ilyenkor a módot nem írjuk vissza.
pub fn apply(sync: WindowsSync, follow_windows: bool, light: bool, dark_theme: Theme, light_theme: Theme) {
    std::thread::spawn(move || {
        let _g = LOCK.lock();
        let mut backup = load_backup();
        let mut colors_changed = false;
        if sync.dark_mode {
            if !follow_windows {
                colors_changed |= apply_mode(light, &mut backup);
            }
        } else {
            colors_changed |= restore_mode(&mut backup);
        }
        if sync.accent {
            let t = if light { &light_theme } else { &dark_theme };
            let pick = match sync.accent_color.as_str() {
                "main" => &t.main,
                "alt" => &t.alt,
                _ => &t.hot,
            };
            if let Some(c) = parse_hex(pick) {
                colors_changed |= apply_accent(c, &mut backup);
            }
        } else {
            colors_changed |= restore_accent(&mut backup);
        }
        if sync.terminal {
            if !backup["terminal"].is_object() {
                backup["terminal"] = json!({});
            }
            apply_terminal(&dark_theme, &light_theme, light, &mut backup);
        } else {
            restore_terminal(&mut backup);
        }
        save_backup(&backup);
        if colors_changed {
            broadcast_colors();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonc() {
        let s = "{\n // megjegyzés\n \"a\": \"x//y\", /* blokk */ \"b\": [1, 2,],\n}";
        let v: Value = serde_json::from_str(&strip_jsonc(s)).unwrap();
        assert_eq!(v["a"], "x//y");
        assert_eq!(v["b"], json!([1, 2]));
    }

    #[test]
    fn palette() {
        let p = accent_palette((243, 230, 0), None);
        assert_eq!(p.len(), 32);
        assert_eq!(&p[12..16], &[243, 230, 0, 0]);
    }
}
