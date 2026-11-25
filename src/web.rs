//! A felületek kiszolgálása (saját protokoll, ikonok, albumborító), a felületek címei,
//! a felugró ablakok közös váza és az üzenetküldés a felületeknek.

use crate::APP_NAME;
use crate::UserEvent;
use crate::config::Config;
use crate::log;
use crate::send;
use crate::{apps, media, tray, wintrack};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tao::dpi::PhysicalSize;
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Window, WindowBuilder};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, SetWindowLongPtrW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

/// A felület címébe kerülő beállítások, hogy már az első kirajzolás jó legyen: kikapcsolt súgó
/// szövegeknél a jelzés (title nélkül jelenik meg), valamint a dátum formátuma és a 12 órás óra.
pub(crate) fn ui_params(cfg: &Config) -> String {
    let mut s = String::new();
    if !cfg.tooltips {
        s.push_str("&tips=0");
    }
    if cfg.date_time.date_format != "auto" {
        s.push_str(&format!("&df={}", cfg.date_time.date_format));
    }
    if cfg.date_time.hour12 {
        s.push_str("&h12=1");
    }
    s
}

/// Egy felugró ablak felületének címe: a `part` (és ami utána jön) a hub.js-nek szól, mögötte
/// a nyelv, a séma és a többi kezdeti beállítás.
fn popup_url(part: &str, cfg: &Config) -> String {
    format!("cybershell://localhost/hub.html?part={part}&lang={}&scheme={}{}", cfg.language, cfg.scheme, ui_params(cfg))
}

pub(crate) fn settings_url(cfg: &Config) -> String {
    popup_url("settings", cfg)
}

pub(crate) fn peek_url(cfg: &Config) -> String {
    popup_url("peek", cfg)
}

pub(crate) fn fly_url(cfg: &Config) -> String {
    popup_url("fly", cfg)
}

pub(crate) fn health_url(cfg: &Config) -> String {
    popup_url("health", cfg)
}

/// A „Megszakadt a kapcsolat” sáv (a szünet-sáv felülete, más tartalommal).
pub(crate) fn net_url(cfg: &Config) -> String {
    popup_url("health&kind=net", cfg)
}

pub(crate) fn launcher_url(cfg: &Config) -> String {
    popup_url("launcher", cfg)
}

/// A saját asztali menü.
pub(crate) fn desk_url(cfg: &Config) -> String {
    popup_url("desk", cfg)
}

/// Egy már futó felület újratöltéséhez: a wry csak az első betöltésnél (`with_url`) írja át
/// a saját protokollt a WebView2 által ismert alakra, a `load_url`-nél nem. A cybershell://
/// címre navigálás így csendben elmaradna (ezért nem működött pl. a panelcsere).
pub(crate) fn nav_url(url: &str) -> String {
    match url.strip_prefix("cybershell://localhost/") {
        Some(rest) => format!("http://cybershell.localhost/{rest}"),
        None => url.to_string(),
    }
}

pub(crate) fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(crate) fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Egy program saját (az exe-jéből kinyert) ikonjának címe a felületnek (lásd `serve`).
pub(crate) fn exe_icon_url(path: &str) -> String {
    format!("/__icon/{}", percent_encode(&path.replace('\\', "/")))
}

/// A Start menü egy programjának ikoncíme, a programazonosító szerint.
pub(crate) fn app_icon_url(id: &str) -> String {
    format!("/__appicon/{}", percent_encode(id))
}

/// Egy tetszőleges helyen lévő kép (saját ikon, háttérkép) címe a felületnek.
pub(crate) fn file_url(path: &str) -> String {
    format!("/__file/{}", percent_encode(&path.replace('\\', "/")))
}

/// A felület fájljait a cybershell:// protokollon szolgáljuk ki (a file:// címekről
/// érkező üzeneteket a wry nem tudja feldolgozni). A /__file/ előtag alatt
/// tetszőleges helyről tölthető be ikonkép.
pub(crate) fn serve(ui_dir: &Path, req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = percent_decode(req.uri().path());
    let png = |data: Option<Vec<u8>>| data.map(|d| ("image/png", d));
    // /__appicon/<azonosító>: a Start menü egy programjának ikonja
    if let Some(id) = path.strip_prefix("/__appicon/") {
        return image_response(png(apps::icon_png(id)), "max-age=86400");
    }
    // /__userpic: a Windows-fiók profilképe (a Start menü aljára)
    if path == "/__userpic" {
        let pic = apps::user_picture().and_then(|p| {
            let data = std::fs::read(&p).ok()?;
            let is_png = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
            Some((if is_png { "image/png" } else { "image/jpeg" }, data))
        });
        return image_response(pic, "no-cache");
    }
    // /__tray/<azonosító>: egy háttérprogram ikonjának képe az értesítési területről
    if let Some(id) = path.strip_prefix("/__tray/") {
        return image_response(png(tray::icon_png(id)), "max-age=86400");
    }
    // /__deskicon/<menü>/<sorszám>: az asztali menü egy elemének ikonja (a menü sorszáma csak a
    // gyorsítótár ellen van a címben)
    if let Some(rest) = path.strip_prefix("/__deskicon/") {
        let n = rest.rsplit('/').next().and_then(|n| n.parse::<usize>().ok());
        return image_response(png(n.and_then(crate::deskmenu::icon)), "no-store");
    }
    // /__art/<sorszám>: a Spotifyban szóló szám albumborítója
    if path.starts_with("/__art/") {
        return image_response(media::art(), "max-age=86400");
    }
    // /__icon/<exe útvonala>: a program saját ikonja PNG-ként
    if let Some(exe) = path.strip_prefix("/__icon/") {
        return image_response(png(wintrack::icon_png(exe)), "max-age=3600");
    }
    let mut embedded_rel: Option<String> = None;
    let file = if let Some(abs) = path.strip_prefix("/__file/") {
        let p = PathBuf::from(abs);
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !["png", "jpg", "jpeg", "svg", "ico", "webp", "gif", "bmp"].contains(&ext.as_str()) {
            return not_found();
        }
        p
    } else {
        let rel = path.trim_start_matches('/');
        if rel.split(['/', '\\']).any(|seg| seg == "..") {
            return not_found();
        }
        let rel = if rel.is_empty() { "hub.html" } else { rel };
        embedded_rel = Some(rel.replace('\\', "/"));
        ui_dir.join(rel)
    };
    let mime = match file.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    };
    // Elsőként a lemezről (az ui mappa szabadon átírható); ha nincs ott, a beágyazott példányból.
    let data = match std::fs::read(&file) {
        Ok(d) => Cow::Owned(d),
        Err(_) => match embedded_rel.and_then(|r| crate::embedded::UI_FILES.iter().find(|(n, _)| *n == r)) {
            Some((_, d)) => Cow::Borrowed(*d),
            None => return not_found(),
        },
    };
    Response::builder().header(CONTENT_TYPE, mime).header("Cache-Control", "no-store").body(data).unwrap_or_else(|_| not_found())
}

/// Menet közben előállított kép (típus és tartalom) válaszként, a megadott gyorsítótárazással;
/// ha nincs ilyen kép, 404.
fn image_response(image: Option<(impl AsRef<str>, Vec<u8>)>, cache: &str) -> Response<Cow<'static, [u8]>> {
    match image {
        Some((mime, data)) => Response::builder()
            .header(CONTENT_TYPE, mime.as_ref())
            .header("Cache-Control", cache)
            .body(Cow::Owned(data))
            .unwrap_or_else(|_| not_found()),
        None => not_found(),
    }
}

pub(crate) fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap()
}

pub(crate) fn hwnd_of(window: &Window) -> HWND {
    HWND(window.hwnd() as *mut core::ffi::c_void)
}

/// Keret nélküli, mindig felül lévő eszközablak: nem látszik a tálcán és az Alt+Tabban,
/// a sarkai nincsenek lekerekítve (a sávok és a felugró ablakok közös stílusa).
pub(crate) fn tool_window_style(hwnd: HWND) {
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize);
        let pref = DWMWCP_DONOTROUND;
        let _ =
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, std::mem::size_of_val(&pref) as u32);
    }
}

/// Az ablak alsó két sarkának levágása (`cut` képpontos ferde éllel), ahogy a felület
/// `clip-path`-ja rajzolja: így a levágott sarokban az alatta lévő látszik, nem az ablak
/// háttérszíne (a piros figyelmeztető sávoknál).
pub(crate) fn cut_bottom_corners(hwnd: HWND, w: i32, h: i32, cut: i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{CreatePolygonRgn, SetWindowRgn, WINDING};
    let c = cut.clamp(0, h.min(w / 2));
    let pts = [
        POINT { x: 0, y: 0 },
        POINT { x: w, y: 0 },
        POINT { x: w, y: h - c },
        POINT { x: w - c, y: h },
        POINT { x: c, y: h },
        POINT { x: 0, y: h - c },
    ];
    unsafe {
        let rgn = CreatePolygonRgn(&pts, WINDING);
        // a régiót ettől kezdve a rendszer kezeli (nem kell felszabadítani)
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(hwnd, Some(rgn), true);
        }
    }
}

/// A saját felületek (sávok, felugró ablakok) közös WebView-beállításai: a cybershell://
/// protokoll, a kezdőcím és a téma háttérszíne, böngészős helyi menü és gyorsbillentyűk nélkül
/// (a fejlesztői eszközök csak a fejlesztői buildben érhetők el).
pub(crate) fn ui_webview<'a>(web_context: &'a mut WebContext, ui_dir: &Path, cfg: &Config, url: String) -> WebViewBuilder<'a> {
    let dir = ui_dir.to_path_buf();
    WebViewBuilder::new_with_web_context(web_context)
        .with_custom_protocol("cybershell".into(), move |_id, req| serve(&dir, &req))
        .with_url(url)
        .with_background_color(cfg.bg_rgba())
        .with_devtools(cfg!(debug_assertions))
        .with_default_context_menus(false)
        .with_browser_accelerator_keys(false)
}

/// Egy felugró ablak részei: az ablak, a benne futó felület és az ablak azonosítója.
pub(crate) type PopupParts = (Window, WebView, HWND);

/// Rejtett felugró ablak a saját felülettel. `focusable`: kaphat-e billentyűzetet
/// (az ablakelőnézet és a szünet-sáv nem veszi el a fókuszt). Az `ipc` a felület üzeneteit
/// alakítja eseménnyé.
#[allow(clippy::too_many_arguments)]
pub(crate) fn make_popup(
    target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
    web_context: &mut WebContext,
    ui_dir: &Path,
    cfg: &Config,
    what: &str,
    url: String,
    size: (u32, u32),
    focusable: bool,
    ipc: fn(String) -> UserEvent,
) -> Option<PopupParts> {
    let window = WindowBuilder::new()
        .with_title(format!("{APP_NAME} – {what}"))
        .with_decorations(false)
        .with_visible(false)
        .with_focused(focusable)
        .with_always_on_top(true)
        .with_skip_taskbar(true)
        .with_undecorated_shadow(false)
        .with_drag_and_drop(false)
        .with_inner_size(PhysicalSize::new(size.0, size.1))
        .build(target)
        .map_err(|e| log(format!("{what}: az ablak nem hozható létre ({e})")))
        .ok()?;
    let hwnd = hwnd_of(&window);
    tool_window_style(hwnd);
    let webview = ui_webview(web_context, ui_dir, cfg, url)
        .with_ipc_handler(move |req: Request<String>| send(ipc(req.body().clone())))
        .with_drag_drop_handler(|_| true)
        .build(&window)
        .map_err(|e| log(format!("{what}: a felület nem hozható létre ({e})")))
        .ok()?;
    Some((window, webview, hwnd))
}

/// Üzenet egy felületnek (a hub.js `window.hub.receive` függvénye kapja).
pub(crate) fn push(webview: &WebView, payload: impl std::fmt::Display) {
    let _ = webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
}
