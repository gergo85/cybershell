#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
mod autostart;
mod config;
mod peek;
mod sys;
mod usage;
mod wintrack;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::json;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Window, WindowBuilder};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{CreateMutexW, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ABN_FULLSCREENAPP, ABN_POSCHANGED};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow, PostMessageW, RegisterWindowMessageW, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, SystemParametersInfoW, MB_ICONEXCLAMATION, SW_HIDE, WM_CLOSE,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SPI_GETWORKAREA,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_FRAMECHANGED, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WM_DISPLAYCHANGE, WM_ENDSESSION, WM_HOTKEY, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::w;
use std::borrow::Cow;
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

use appbar::Edge;
use config::Config;
use wintrack::{Matcher, Tracker, WinInfo};

const POLL: Duration = Duration::from_millis(500);
const HOTKEY_EXIT: i32 = 1;
const HOTKEY_RELOAD: i32 = 2;
/// A sávok sorrendje: a két teljes magasságú oldalpanel, aztán a felső és alsó keret.
const EDGES: [Edge; 4] = [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom];
const APP_NAME: &str = "CyberHub";
const WEBSITE_URL: &str = "https://szabogergo.hu/";

#[derive(Debug, Clone)]
enum UserEvent {
    Ipc(usize, String),
    Reposition,
    ShellRestarted,
    /// Teljes képernyős program nyílt vagy zárult azon a képernyőn, ahol a sáv (hwnd) van.
    Fullscreen(isize, bool),
    Hotkey(i32),
    Usage(String),
    /// Fájlhúzás az ikonsáv fölött: sáv, fázis ("over", "drop", "leave"), pozíció, fájlok.
    Drag(usize, &'static str, i32, i32, Vec<String>),
    /// A claude.ai ablak üzenete (használati adat vagy belépés szükséges).
    ClaudeWeb(String),
    /// A claude.ai ablakban betöltődött egy oldal.
    ClaudeLoaded(String),
    /// A teljes képernyős beállítások üzenete.
    SettingsIpc(String),
    /// Az ablakelőnézet üzenete.
    PeekIpc(String),
    /// A részletes időjárás oldalpaneljének üzenete.
    FlyIpc(String),
    /// A naptárak letöltésének eredménye (háttérszálról).
    CalendarData(String),
}

/// Részletes időjárás: az információs panel mellett kinyíló oldalpanel.
struct FlyWin {
    window: Window,
    webview: WebView,
    hwnd: HWND,
    visible: bool,
    /// Mikor nyílt meg (a fókuszvesztést csak egy kis idő után figyeljük).
    shown_at: Instant,
    /// Mikor zárult be fókuszvesztés miatt (az időjárásra kattintás ne nyissa újra rögtön).
    hidden_at: Option<Instant>,
    /// Mit mutat: "weather" vagy "calendar", és melyik sáv mellett nyílt meg.
    kind: &'static str,
    bar: usize,
}

/// Az oldalpanel mérete CSS-pixelben.
const FLY_W: f64 = 380.0;
const FLY_H: f64 = 640.0;

/// A teljes képernyős beállítások ablaka.
struct SettingsWin {
    window: Window,
    webview: WebView,
    hwnd: HWND,
    visible: bool,
}

/// Az ablakelőnézet: ha egy programnak több ablaka van, az ikonja mellett
/// egymás mellett mutatja az ablakok élő kisképét.
struct PeekWin {
    window: Window,
    webview: WebView,
    hwnd: HWND,
    visible: bool,
    /// A bejegyzett élő kisképek: (DWM-azonosító, forrásablak).
    thumbs: Vec<(isize, HWND)>,
    /// A felület visszajelzésére váró kisképek: (forrásablak, cél, forrásterület).
    pending: Vec<(HWND, RECT, RECT)>,
    /// A látható ablakok (csak ezekre fogadunk el parancsot).
    hwnds: Vec<isize>,
    /// Minden új elrendezés új sorszámot kap; a felület ezzel jelez vissza.
    seq: u64,
    /// Melyik app ablakai látszanak (újrarendezéshez, ha egy ablak bezárul).
    target: serde_json::Value,
    hovered: bool,
    hide_at: Option<Instant>,
    refresh_at: Option<Instant>,
}

/// Fel vannak-e cserélve a panelek (az ikonsáv jobbra kerül). A húzáskezelő is olvassa.
static ICONS_RIGHT: AtomicBool = AtomicBool::new(false);

/// Melyik felület töltődik be az adott szélre: az ikonsáv ("left") vagy az információs panel ("right").
fn part_for(edge: Edge, swap: bool) -> &'static str {
    match (edge, swap) {
        (Edge::Left, false) | (Edge::Right, true) => "left",
        (Edge::Right, false) | (Edge::Left, true) => "right",
        (Edge::Top, _) => "top",
        (Edge::Bottom, _) => "bottom",
    }
}

fn bar_url(part: &str, edge: Edge, cfg: &Config) -> String {
    format!(
        "cyberhub://localhost/hub.html?part={part}&side={}&l={}&t={}&r={}&b={}&lang={}",
        edge.name(),
        cfg.left_width,
        cfg.frame.top,
        cfg.right_width,
        cfg.frame.bottom,
        cfg.language
    )
}

fn settings_url(cfg: &Config) -> String {
    format!("cyberhub://localhost/hub.html?part=settings&lang={}", cfg.language)
}

fn peek_url(cfg: &Config) -> String {
    format!("cyberhub://localhost/hub.html?part=peek&lang={}", cfg.language)
}

fn fly_url(cfg: &Config) -> String {
    format!("cyberhub://localhost/hub.html?part=fly&lang={}", cfg.language)
}

/// Egy már futó felület újratöltéséhez: a wry csak az első betöltésnél (`with_url`) írja át
/// a saját protokollt a WebView2 által ismert alakra, a `load_url`-nél nem. A cyberhub://
/// címre navigálás így csendben elmaradna (ezért nem működött pl. a panelcsere).
fn nav_url(url: &str) -> String {
    match url.strip_prefix("cyberhub://localhost/") {
        Some(rest) => format!("http://cyberhub.localhost/{rest}"),
        None => url.to_string(),
    }
}

/// A program verziója a beállítások számára („1.2.0” → „v1.2”).
fn version_label() -> String {
    let v = env!("CARGO_PKG_VERSION");
    format!("v{}", v.strip_suffix(".0").unwrap_or(v))
}

/// A claude.ai ablak üresjárati oldala: könnyű, de a claude.ai-hoz tartozik,
/// így innen elérhetők a saját fiókod adatai (a sütik a CyberHub saját böngészőprofiljában vannak).
const CLAUDE_IDLE_URL: &str = "https://claude.ai/robots.txt";
const CLAUDE_LOGIN_URL: &str = "https://claude.ai/login";
/// A teljes claude.ai alkalmazás: ha a munkamenet „kötése” (lásd `ClaudeWeb::revived`)
/// hosszabb kikapcsolás alatt lejárt, ennek betöltése megújítja.
const CLAUDE_APP_URL: &str = "https://claude.ai/new";

/// A claude.ai oldalon futó lekérdezés: a bejelentkezett fiók szervezete, majd annak
/// használati adatai. Ugyanazt az adatot kéri le, amit a claude.ai beállításai is mutatnak.
const CLAUDE_FETCH_JS: &str = r#"(async () => {
  const post = window.ipc ? (s) => window.ipc.postMessage(s) : (s) => window.chrome.webview.postMessage(s);
  const send = (m) => post(JSON.stringify(Object.assign({ type: 'claude-web' }, m)));
  if (location.origin !== 'https://claude.ai') return send({ status: 'skip' });
  try {
    const r = await fetch('/api/organizations', { credentials: 'include', headers: { accept: 'application/json' } });
    if (r.status === 401 || r.status === 403) return send({ status: 'login', code: r.status });
    if (!r.ok) return send({ status: 'error', code: r.status });
    const orgs = await r.json();
    if (!Array.isArray(orgs) || !orgs.length) return send({ status: 'login', code: 0 });
    const caps = (o) => o.capabilities || [];
    const org = orgs.find((o) => caps(o).some((c) => /claude_(pro|max)/.test(c)))
      || orgs.find((o) => caps(o).includes('chat')) || orgs[0];
    const u = await fetch(`/api/organizations/${org.uuid}/usage`, { credentials: 'include', headers: { accept: 'application/json' } });
    if (!u.ok) return send({ status: 'error', code: u.status });
    send({ status: 'ok', usage: await u.json(), caps: caps(org) });
  } catch (e) {
    send({ status: 'error', message: String(e).slice(0, 120) });
  }
})();"#;

/// A claude.ai belépő- és lekérdezőablak.
struct ClaudeWeb {
    window: Window,
    webview: WebView,
    visible: bool,
    next_fetch: Instant,
    /// A legutóbbi sikeres lekérés ideje és eredménye.
    ok: Option<(Instant, String)>,
    /// A belépés sütijeit ebben a futásban már tartóssá tettük-e.
    persisted: bool,
    /// A claude.ai a belépést az eszközhöz köti (`__Host-sessionBinding`, néhány percig
    /// érvényes, futás közben folyamatosan megújul). Ha a program hosszabb ideig nem futott,
    /// a kötés lejár, és a puszta lekérdezés „nincs belépve” választ kap, pedig a belépés
    /// (sessionKey) még érvényes. Ilyenkor egyszer betöltjük a teljes claude.ai oldalt,
    /// ami megújítja a kötést, és csak ha ezután sem megy, kérünk újra belépést.
    revived: bool,
    /// Mit mondott legutóbb a claude.ai: "pending" (még nem válaszolt), "ok", "login"
    /// (valóban nincs belépve) vagy "error" (nem érhető el, pl. induláskor még nincs hálózat).
    /// Belépést csak "login" esetén kérünk.
    state: &'static str,
}

/// A claude.ai munkamenet-sütijeit (amelyek a program bezárásakor elvesznének)
/// 30 napig érvényesre állítja, hogy újraindítás után se kelljen újra belépni.
/// A sütik a CyberHub saját böngészőprofiljában maradnak; értéküket nem írjuk ki.
fn persist_claude_cookies(webview: &WebView) -> usize {
    let Ok(cookies) = webview.cookies_for_url("https://claude.ai") else {
        return 0;
    };
    let mut n = 0;
    for mut c in cookies {
        let session = matches!(c.expires(), None | Some(wry::cookie::Expiration::Session));
        // a Cloudflare rövid életű sütijeit nem bántjuk
        if session && !c.name().starts_with("__cf") && !c.name().starts_with("cf_") {
            c.set_max_age(wry::cookie::time::Duration::days(30));
            if webview.set_cookie(&c).is_ok() {
                n += 1;
            }
        }
    }
    n
}

static PROXY: OnceLock<Mutex<EventLoopProxy<UserEvent>>> = OnceLock::new();
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// Pánik esetére: mit kell visszaállítani.
static CLEANUP: Mutex<Option<(Vec<isize>, Option<u32>)>> = Mutex::new(None);

fn log(msg: impl AsRef<str>) {
    if let Ok(mut g) = LOG.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{}", msg.as_ref());
        }
    }
    #[cfg(debug_assertions)]
    eprintln!("{}", msg.as_ref());
}

fn send(ev: UserEvent) {
    if let Some(p) = PROXY.get() {
        if let Ok(p) = p.lock() {
            let _ = p.send_event(ev);
        }
    }
}

fn data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(APP_NAME);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

/// A program mappája: ahol az ui\hub.html található (exe mellett vagy fölötte).
fn find_base_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let mut dir = exe.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if d.join("ui").join("hub.html").exists() {
            return d;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    exe.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A felület fájljait a cyberhub:// protokollon szolgáljuk ki (a file:// címekről
/// érkező üzeneteket a wry nem tudja feldolgozni). A /__file/ előtag alatt
/// tetszőleges helyről tölthető be ikonkép.
fn serve(ui_dir: &Path, req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = percent_decode(req.uri().path());
    // /__icon/<exe útvonala>: a program saját ikonja PNG-ként
    if let Some(exe) = path.strip_prefix("/__icon/") {
        return match wintrack::icon_png(exe) {
            Some(png) => Response::builder()
                .header(CONTENT_TYPE, "image/png")
                .header("Cache-Control", "max-age=3600")
                .body(Cow::Owned(png))
                .unwrap_or_else(|_| not_found()),
            None => not_found(),
        };
    }
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
        ui_dir.join(if rel.is_empty() { "hub.html" } else { rel })
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
    match std::fs::read(&file) {
        Ok(data) => Response::builder()
            .header(CONTENT_TYPE, mime)
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(data))
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap()
}

fn hwnd_of(window: &Window) -> HWND {
    HWND(window.hwnd() as *mut core::ffi::c_void)
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == appbar::CALLBACK_MSG {
        match wp.0 as u32 {
            ABN_POSCHANGED => send(UserEvent::Reposition),
            ABN_FULLSCREENAPP => send(UserEvent::Fullscreen(hwnd.0 as isize, lp.0 != 0)),
            _ => {}
        }
        return LRESULT(0);
    }
    if msg == WM_HOTKEY {
        send(UserEvent::Hotkey(wp.0 as i32));
        return LRESULT(0);
    }
    let tc = TASKBAR_CREATED.load(Ordering::Relaxed);
    if tc != 0 && msg == tc {
        send(UserEvent::ShellRestarted);
    }
    if msg == WM_DISPLAYCHANGE {
        send(UserEvent::Reposition);
    }
    // Kijelentkezés, leállítás: a Windows nem várja meg a szokásos kilépést, ezért
    // itt azonnal visszaadjuk a tálcát és felszabadítjuk a képernyő szélét.
    if msg == WM_ENDSESSION && wp.0 != 0 {
        emergency_cleanup();
        let _ = std::fs::remove_file(state_path());
        log("munkamenet vége: visszaállítás kész");
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

struct Bar {
    edge: Edge,
    /// Melyik képernyőn van (0 = elsődleges); képernyőnként négy sáv.
    mon: usize,
    /// A képernyő DPI-szorzója (CSS-pixel → fizikai pixel).
    scale: f64,
    /// A betöltött felület: "left" (ikonsáv), "right" (információs panel), "top", "bottom".
    part: &'static str,
    window: Window,
    webview: WebView,
    hwnd: HWND,
    rect: RECT,
    registered: bool,
}

struct Pending {
    app: usize,
    known: HashSet<isize>,
    deadline: Instant,
}

struct Hub {
    cfg: Config,
    cfg_path: PathBuf,
    matchers: Vec<Matcher>,
    bars: Vec<Bar>,
    tracker: Tracker,
    last_fg: Option<HWND>,
    last_state: String,
    pending: Option<Pending>,
    /// Hány képernyőn kellene sávcsoport (ha eltér a meglévőtől, a fő ciklus újraépíti).
    want_groups: usize,
    /// Ha mi rejtettük el a tálcát: az eredeti állapota.
    orig_taskbar: Option<u32>,
    want_hidden_taskbar: bool,
    /// Azok a képernyők, ahol épp teljes képernyős program fut.
    fullscreen: HashSet<usize>,
    scale: f64,
    suppress_until: Instant,
    relayout_pending: bool,
    exit_requested: bool,
    shut_down: bool,
    /// A legutóbbi Claude-használati adat (újratöltéskor azonnal visszaküldjük).
    last_usage: Option<String>,
    usage_wake: Option<Sender<()>>,
    /// Az éppen húzott fájlok (ha egy ikonra ejtik őket, azzal a programmal nyitjuk meg).
    drag_paths: Vec<String>,
    /// Nem kitűzött, de futó programok: megjelenítési sorrend és nevek.
    extra_order: Vec<String>,
    extra_names: HashMap<String, String>,
    claude: Option<ClaudeWeb>,
    /// A háttérszál (állapotsor, közvetlen lekérés) legutóbbi eredménye.
    thread_usage: Option<serde_json::Value>,
    settings: Option<SettingsWin>,
    /// Az appok telepítettsége (a config sorrendjében), indításkor és újratöltéskor ellenőrizve.
    available: Vec<bool>,
    /// A program indulásának ideje: a felület ebből tudja, hogy új munkamenet kezdődött-e.
    session: u64,
    /// Az előző processzoridő-mérés és a következő terhelésküldés ideje (a beállításokhoz).
    cpu_prev: Option<(u64, u64)>,
    next_sys: Instant,
    /// Bejelentkezéskor indítandó programok (sorszám) és a következő indítás ideje.
    launch_queue: std::collections::VecDeque<usize>,
    next_launch: Instant,
    peek: Option<PeekWin>,
    /// Az ablakok első észlelésének sorrendje (az előnézet ebben a sorrendben mutatja őket).
    seen: HashMap<isize, u64>,
    seen_seq: u64,
    fly: Option<FlyWin>,
    /// Az akkumulátor legutóbb elküldött állapota és a következő ellenőrzés ideje.
    last_power: String,
    next_power: Instant,
    /// A hangerő-vezérlő (percenként újranyitjuk, ha közben másik eszköz lett az alapértelmezett),
    /// és a felületnek legutóbb küldött állapot.
    volume: Option<(Instant, sys::Volume)>,
    last_volume: String,
    next_volume: Instant,
    /// A naptárak legutóbbi letöltése (ideje és tartalma), illetve folyamatban van-e.
    cal_cache: Option<(Instant, String)>,
    cal_busy: bool,
}

/// Ezek a rendszerfolyamatok ablakai ne jelenjenek meg a futó programok között.
const IGNORED_EXTRA: &[&str] = &[
    "cyberhub.exe",
    "textinputhost.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "lockapp.exe",
];

/// A claude.ai használati válaszából a panel üzenete (ugyanaz a forma, mint a háttérszálé).
fn claude_web_payload(v: &serde_json::Value) -> serde_json::Value {
    let u = &v["usage"];
    let caps: Vec<String> = v["caps"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(str::to_lowercase)).collect()).unwrap_or_default();
    let plan = if caps.iter().any(|c| c.contains("claude_max")) {
        "max"
    } else if caps.iter().any(|c| c.contains("claude_pro")) {
        "pro"
    } else {
        ""
    };
    let keys = [
        ("five_hour", "5 órás", true),
        ("seven_day", "Heti", true),
        ("seven_day_opus", "Heti · Opus", false),
        ("seven_day_sonnet", "Heti · Sonnet", false),
    ];
    let mut limits = Vec::new();
    for (key, label, always) in keys {
        let l = &u[key];
        let pct = l["utilization"].as_f64().or_else(|| l["used_percentage"].as_f64());
        if let Some(pct) = pct {
            if always || pct > 0.0 {
                limits.push(json!({ "key": key, "label": label, "pct": pct, "resetsAt": l["resets_at"] }));
            }
        }
    }
    if limits.is_empty() {
        return json!({ "type": "usage", "status": "error", "plan": plan, "message": "Ismeretlen válasz a claude.ai-tól." });
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    json!({ "type": "usage", "status": "ok", "plan": plan, "limits": limits, "fetchedAt": now, "source": "web" })
}

/// Egy nem kitűzött program csoportkulcsa: a teljes útvonal, Store-appoknál
/// (ApplicationFrameHost) az ablak maga.
fn extra_key(w: &WinInfo) -> Option<String> {
    if w.path.is_empty() || IGNORED_EXTRA.contains(&w.exe.as_str()) {
        return None;
    }
    if w.exe == "applicationframehost.exe" {
        return Some(format!("hwnd:{}", w.hwnd.0 as isize));
    }
    Some(w.path.to_lowercase())
}

impl Hub {
    fn own_hwnds(&self) -> Vec<HWND> {
        self.bars.iter().map(|b| b.hwnd).collect()
    }

    fn thickness(&self, edge: Edge) -> u32 {
        match edge {
            Edge::Left => self.cfg.left_width,
            Edge::Top => self.cfg.frame.top,
            Edge::Right => self.cfg.right_width,
            Edge::Bottom => self.cfg.frame.bottom,
        }
    }

    /// Az elsődleges képernyő adott szélén lévő sáv.
    fn bar(&self, edge: Edge) -> Option<&Bar> {
        self.bars.iter().find(|b| b.edge == edge && b.mon == 0)
    }

    /// Ugyanazon a képernyőn lévő, adott szélű sáv.
    fn bar_on(&self, mon: usize, edge: Edge) -> Option<&Bar> {
        self.bars.iter().find(|b| b.edge == edge && b.mon == mon)
    }

    /// Üzenet egyetlen sáv felületének.
    fn eval_bar(&self, idx: usize, payload: &str) {
        if let Some(b) = self.bars.get(idx) {
            let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// A felület nyelvének megfelelő szöveg.
    fn tr(&self, hu: &str, en: &str) -> String {
        if self.cfg.language == "en" { en.to_string() } else { hu.to_string() }
    }

    /// Az ikonsáv széle (felcserélt paneleknél jobbra van).
    fn icons_edge(&self) -> Edge {
        if self.cfg.swap_sides { Edge::Right } else { Edge::Left }
    }

    /// Az információs panel széle.
    fn info_edge(&self) -> Edge {
        if self.cfg.swap_sides { Edge::Left } else { Edge::Right }
    }

    fn check_available(&mut self) {
        self.available = self.cfg.apps.iter().map(sys::app_available).collect();
        for (a, ok) in self.cfg.apps.iter().zip(&self.available) {
            if !ok {
                log(format!("nincs telepítve, elrejtve: {} ({})", a.name, a.path));
            }
        }
    }

    /// Kiszámolja és lefoglalja a négy sáv helyét. `force` nélkül csak akkor nyúl
    /// hozzájuk, ha változott valami – így a rendszer értesítései nem pörgetik körbe.
    fn layout(&mut self, force: bool) {
        if self.bars.is_empty() {
            return;
        }
        let mons = appbar::monitors();
        let want = if self.cfg.all_monitors { mons.len().max(1) } else { 1 };
        if want != self.want_groups {
            log(format!("képernyők: {} (sávcsoport kell: {want})", mons.len()));
            self.want_groups = want;
        }
        let groups = self.bars.len() / EDGES.len();
        let mut plan: Vec<(usize, RECT, i32)> = Vec::new();
        for g in 0..groups {
            // a már nem létező képernyő sávjai a következő újraépítésig rejtve maradnak
            let m = match mons.get(g) {
                Some(m) => *m,
                None if g == 0 => appbar::Monitor { rect: appbar::primary_monitor().0, primary: true, dpi: 96 },
                None => {
                    for i in g * EDGES.len()..(g + 1) * EDGES.len() {
                        plan.push((i, RECT::default(), 0));
                    }
                    continue;
                }
            };
            let mut base = m.rect;
            if !self.want_hidden_taskbar {
                let tb = if m.primary { appbar::taskbar_reserved() } else { appbar::secondary_taskbar_on(m.rect) };
                if let Some((edge, rc)) = tb {
                    match edge {
                        Edge::Bottom => base.bottom = rc.top,
                        Edge::Top => base.top = rc.bottom,
                        Edge::Left => base.left = rc.right,
                        Edge::Right => base.right = rc.left,
                    }
                }
            }
            let scale = m.dpi as f64 / 96.0;
            if g == 0 {
                self.scale = scale;
            }
            let px = |v: u32| ((v as f64) * scale).round() as i32;

            // A bal és a jobb panel teljes magasságú, a felső és alsó keret a kettő között fut.
            let mut left_rc = RECT { left: base.left, top: base.top, right: base.left, bottom: base.bottom };
            let mut right_rc = RECT { left: base.right, top: base.top, right: base.right, bottom: base.bottom };
            for (k, edge) in EDGES.iter().enumerate() {
                let i = g * EDGES.len() + k;
                self.bars[i].scale = scale;
                let thick = px(self.thickness(*edge));
                let hwnd = self.bars[i].hwnd;
                let want = match edge {
                    Edge::Left => RECT { left: base.left, top: base.top, right: base.left + thick, bottom: base.bottom },
                    Edge::Right => RECT { left: base.right - thick, top: base.top, right: base.right, bottom: base.bottom },
                    Edge::Top => RECT { left: left_rc.right, top: base.top, right: right_rc.left, bottom: base.top + thick },
                    Edge::Bottom => {
                        RECT { left: left_rc.right, top: base.bottom - thick, right: right_rc.left, bottom: base.bottom }
                    }
                };
                let rc = if thick > 0 && self.bars[i].registered { appbar::query(hwnd, *edge, want, thick) } else { want };
                match edge {
                    Edge::Left => left_rc = rc,
                    Edge::Right => right_rc = rc,
                    _ => {}
                }
                plan.push((i, rc, thick));
            }
        }

        let changed = plan.iter().any(|(i, rc, _)| self.bars[*i].rect != *rc);
        if !force && !changed {
            return;
        }
        for (i, rc, thick) in plan {
            let bar = &mut self.bars[i];
            if thick <= 0 {
                if bar.registered {
                    appbar::unregister(bar.hwnd);
                    bar.registered = false;
                }
                unsafe {
                    let _ = ShowWindow(bar.hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
                }
                bar.rect = rc;
                continue;
            }
            if !bar.registered {
                bar.registered = appbar::register(bar.hwnd);
            }
            bar.rect = appbar::set(bar.hwnd, bar.edge, rc);
            log(format!(
                "sáv {} #{}: {},{} – {},{}",
                bar.edge.name(),
                bar.mon,
                bar.rect.left,
                bar.rect.top,
                bar.rect.right,
                bar.rect.bottom
            ));
        }
        for m in self.fullscreen.clone() {
            self.apply_fullscreen(m, true);
        }
        let mut work = RECT::default();
        unsafe {
            let _ = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut work as *mut RECT as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
        log(format!(
            "munkaterület: {},{} – {},{} (DPI-szorzó {:.2})",
            work.left, work.top, work.right, work.bottom, self.scale
        ));
        self.suppress_until = Instant::now() + Duration::from_millis(400);
        self.save_state();
    }

    /// Kell-e a sávcsoportokat újraépíteni (képernyő került be vagy ki).
    fn needs_rebuild(&self) -> bool {
        !self.bars.is_empty() && self.want_groups != self.bars.len() / EDGES.len()
    }

    /// A sávcsoportok számát a képernyőkhöz igazítja (a fő ciklusból, ahol ablak hozható létre).
    fn rebuild_bars(&mut self, target: &tao::event_loop::EventLoopWindowTarget<UserEvent>, web_context: &mut WebContext, ui_dir: &Path) {
        let want = self.want_groups.max(1);
        let have = self.bars.len() / EDGES.len();
        log(format!("sávcsoportok: {have} → {want}"));
        self.peek_hide();
        self.fly_hide(false);
        if want < have {
            for b in self.bars.drain(want * EDGES.len()..) {
                if b.registered {
                    appbar::unregister(b.hwnd);
                }
                unsafe {
                    let _ = ShowWindow(b.hwnd, SW_HIDE);
                }
            }
            self.fullscreen.retain(|m| *m < want);
        } else {
            for mon in have..want {
                for edge in EDGES {
                    let idx = self.bars.len();
                    let b = make_bar(target, web_context, ui_dir, &self.cfg, edge, idx, mon);
                    self.bars.push(b);
                }
            }
        }
        self.layout(true);
        for b in &self.bars {
            if b.registered {
                unsafe {
                    let _ = ShowWindow(b.hwnd, SW_SHOWNOACTIVATE);
                }
            }
        }
        self.last_state.clear();
    }

    fn request_layout(&mut self) {
        if Instant::now() < self.suppress_until {
            self.relayout_pending = true;
        } else {
            self.layout(false);
        }
    }

    /// Teljes képernyős program a sáv (hwnd) képernyőjén: ott a sávok hátrébb kerülnek.
    fn set_fullscreen(&mut self, hwnd: isize, on: bool) {
        let Some(mon) = self.bars.iter().find(|b| b.hwnd.0 as isize == hwnd).map(|b| b.mon) else { return };
        if on == self.fullscreen.contains(&mon) {
            return;
        }
        if on {
            self.fullscreen.insert(mon);
            self.peek_hide();
            self.fly_hide(false);
        } else {
            self.fullscreen.remove(&mon);
        }
        self.apply_fullscreen(mon, on);
    }

    fn apply_fullscreen(&self, mon: usize, on: bool) {
        for b in self.bars.iter().filter(|b| b.mon == mon) {
            unsafe {
                if on {
                    let _ = SetWindowPos(b.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    let _ = SetWindowPos(b.hwnd, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                } else {
                    let _ = SetWindowPos(b.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
        }
    }

    fn save_state(&self) {
        let hwnds: Vec<isize> = self.bars.iter().map(|b| b.hwnd.0 as isize).collect();
        let st = json!({ "taskbarState": self.orig_taskbar, "hwnds": hwnds });
        let _ = std::fs::write(state_path(), st.to_string());
        if let Ok(mut c) = CLEANUP.lock() {
            *c = Some((hwnds, self.orig_taskbar));
        }
    }

    /// Üzenet minden képernyő adott szélű sávjának.
    fn eval(&self, edge: Edge, payload: &serde_json::Value) {
        for b in self.bars.iter().filter(|b| b.edge == edge) {
            let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// Beépített ikon neve, vagy képfájl esetén a betöltési címe. Az "exe" a program saját ikonja.
    fn icon_for(&self, icon: &str, app_path: &str) -> (String, String) {
        if icon == "exe" {
            return (String::new(), format!("/__icon/{}", percent_encode(&app_path.replace('\\', "/"))));
        }
        let looks_like_file = icon.contains('.') || icon.contains('/') || icon.contains('\\');
        if !looks_like_file {
            return (icon.to_string(), String::new());
        }
        let p = PathBuf::from(config::expand_env(icon));
        if p.is_absolute() {
            let s = p.to_string_lossy().replace('\\', "/");
            (String::new(), format!("/__file/{}", percent_encode(&s)))
        } else {
            (String::new(), percent_encode(&icon.replace('\\', "/")))
        }
    }

    /// Az appok a felületnek: sorszám (a config szerint), név, ikon, szín.
    fn app_json(&self, i: usize, a: &config::AppEntry) -> serde_json::Value {
        let (icon, img) = self.icon_for(&a.icon, &a.resolved_path());
        json!({
            "id": i,
            "name": a.name,
            "icon": icon,
            "img": img,
            "color": a.color,
            "hint": format!("{} {}", a.resolved_path(), a.match_exe.join(" ")).to_lowercase(),
        })
    }

    fn send_init(&mut self, idx: usize) {
        let edge = self.bars[idx].edge;
        let part = self.bars[idx].part;
        let c = &self.cfg;
        // Az ikonsáv a "left" helyű appokat kapja, az információs panel a "right" helyűeket
        // (ezek az Indítás gomb alá kerülnek). A nem telepített appok kimaradnak.
        let apps: Vec<serde_json::Value> = if part == "left" || part == "right" {
            c.apps
                .iter()
                .enumerate()
                .filter(|(i, a)| a.on_right() == (part == "right") && self.available.get(*i).copied().unwrap_or(true))
                .map(|(i, a)| self.app_json(i, a))
                .collect()
        } else {
            Vec::new()
        };
        // A sávok valós mérete CSS-pixelben (a kerekítés miatt eltérhet a beállítottól).
        let mon = self.bars[idx].mon;
        let scale = self.bars[idx].scale.max(0.5);
        let size = |e: Edge| -> f64 {
            self.bar_on(mon, e)
                .filter(|b| b.registered)
                .map(|b| match e {
                    Edge::Left | Edge::Right => (b.rect.right - b.rect.left) as f64,
                    _ => (b.rect.bottom - b.rect.top) as f64,
                })
                .unwrap_or(0.0)
                / scale
        };
        let payload = json!({
            "type": "init",
            "part": edge.name(),
            "theme": c.theme,
            "geometry": { "l": size(Edge::Left), "t": size(Edge::Top), "r": size(Edge::Right), "b": size(Edge::Bottom) },
            "apps": apps,
            "activity": c.activity,
            "claudeUsage": c.claude_usage,
            "sections": c.sections,
            "session": self.session,
            "autostart": autostart::is_enabled(),
            "weather": c.weather,
            "language": c.language,
        });
        let _ = edge;
        self.eval_bar(idx, &payload.to_string());
        if part == "left" {
            self.last_state.clear();
            self.poll();
        }
        if part == "right" {
            self.send_power(true);
            self.send_volume(true);
            if let Some(u) = &self.last_usage {
                self.eval_bar(idx, u);
            }
            // újratöltés (pl. panelcsere) után is látszódjon, hogy a beállítások nyitva vannak
            if self.settings.as_ref().is_some_and(|s| s.visible) {
                self.eval_bar(idx, &json!({ "type": "settings-open", "on": true }).to_string());
            }
        }
    }

    /// A háttérszál eredménye (állapotsor vagy közvetlen lekérés).
    fn on_usage(&mut self, payload: String) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
            // csak állapotváltozáskor naplózunk, hogy ne teljen meg a napló
            let changed = self.thread_usage.as_ref().map(|t| t["status"] != v["status"] || t["message"] != v["message"]).unwrap_or(true);
            if changed {
                log(format!(
                    "Claude-használat (háttér): {} {}",
                    v["status"].as_str().unwrap_or("?"),
                    v["message"].as_str().unwrap_or("")
                ));
            }
            self.thread_usage = Some(v);
        }
        self.publish_usage();
    }

    /// A legjobb elérhető adat kiküldése a panelnek: a claude.ai-ról frissen lekért,
    /// ha nincs, a háttérszálé, ha az sincs, belépésre hívó üzenet.
    fn publish_usage(&mut self) {
        let fresh_web = self
            .claude
            .as_ref()
            .and_then(|c| c.ok.as_ref())
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(15 * 60))
            .map(|(_, p)| p.clone());
        let payload = if let Some(p) = fresh_web {
            p
        } else if let Some(t) = self.thread_usage.as_ref().filter(|t| t["status"] == "ok") {
            t.to_string()
        } else if let Some(state) = self.claude.as_ref().map(|c| c.state).filter(|s| *s != "login") {
            // a claude.ai még nem válaszolt, vagy nem érhető el: ez nem kijelentkezés
            json!({ "type": "usage", "status": if state == "error" { "offline" } else { "wait" }, "plan": "" }).to_string()
        } else {
            json!({ "type": "usage", "status": "login", "plan": "" }).to_string()
        };
        if self.last_usage.as_deref() != Some(payload.as_str()) {
            let info = self.info_edge();
            for b in self.bars.iter().filter(|b| b.edge == info) {
                let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
            }
            self.last_usage = Some(payload);
        }
    }

    fn claude_fetch(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            c.next_fetch = Instant::now() + Duration::from_secs(5 * 60);
            if c.state == "error" {
                // ha az oldal sem töltődött be (nem volt hálózat), előbb azt töltjük újra;
                // a betöltés végén magától lekérdez
                let _ = c.webview.load_url(CLAUDE_IDLE_URL);
            } else {
                let _ = c.webview.evaluate_script(CLAUDE_FETCH_JS);
            }
        }
    }

    fn claude_show(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            log("claude.ai belépőablak megnyitva");
            let _ = c.webview.load_url(CLAUDE_LOGIN_URL);
            c.window.set_visible(true);
            c.window.set_focus();
            c.visible = true;
        }
    }

    fn claude_hide(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            c.window.set_visible(false);
            c.visible = false;
            // a nehéz claude.ai alkalmazás helyett egy üres oldalon várakozunk
            let _ = c.webview.load_url(CLAUDE_IDLE_URL);
        }
    }

    /// A claude.ai ablakban betöltődött egy oldal: ha a claude.ai-on vagyunk, lekérdezünk.
    /// A teljes alkalmazás betöltése után (munkamenet megújítása) hagyunk időt az oldalnak.
    fn on_claude_loaded(&mut self, url: &str) {
        if !url.starts_with("https://claude.ai/") {
            return;
        }
        if let Some(c) = self.claude.as_mut() {
            if !c.visible && url.starts_with(CLAUDE_APP_URL) {
                c.next_fetch = Instant::now() + Duration::from_secs(6);
                return;
            }
        }
        self.claude_fetch();
    }

    fn on_claude_web(&mut self, raw: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let status = v["status"].as_str().unwrap_or("");
        if status == "skip" {
            // nem a claude.ai oldalon vagyunk (pl. hibaoldal, mert nem volt hálózat):
            // fél perc múlva újratöltjük az oldalt
            if let Some(c) = self.claude.as_mut() {
                if !c.visible && c.state != "ok" {
                    c.state = "error";
                    c.next_fetch = Instant::now() + Duration::from_secs(30);
                    self.publish_usage();
                }
            }
            return;
        }
        log(format!(
            "Claude-használat (claude.ai): {status} {}{}",
            v["code"].as_i64().map(|c| format!("HTTP {c} ")).unwrap_or_default(),
            v["message"].as_str().unwrap_or("")
        ));
        match status {
            "ok" => {
                let payload = claude_web_payload(&v);
                if payload["status"] != "ok" {
                    // ismeretlen formátum: a kulcsneveket naplózzuk (használati adat, nem titok)
                    let keys: Vec<&str> = v["usage"].as_object().map(|o| o.keys().map(|k| k.as_str()).collect()).unwrap_or_default();
                    log(format!("ismeretlen válasz, kulcsok: {keys:?}"));
                }
                if let Some(c) = self.claude.as_mut() {
                    if payload["status"] == "ok" {
                        c.state = "ok";
                        c.ok = Some((Instant::now(), payload.to_string()));
                        if !c.persisted {
                            c.persisted = true;
                            let n = persist_claude_cookies(&c.webview);
                            log(format!("claude.ai belépés megjegyezve ({n} süti)"));
                        }
                    }
                }
                if self.claude.as_ref().is_some_and(|c| c.visible) {
                    self.claude_hide();
                } else if let Some(c) = self.claude.as_mut() {
                    if c.revived {
                        c.revived = false;
                        log("claude.ai: a munkamenet megújult");
                        let _ = c.webview.load_url(CLAUDE_IDLE_URL);
                    }
                }
                // egy későbbi lejáratnál újra megpróbálhatjuk a megújítást
                if let Some(c) = self.claude.as_mut() {
                    c.revived = false;
                }
            }
            "login" => {
                if let Some(c) = self.claude.as_mut() {
                    // először megpróbáljuk megújítani a munkamenetet (lásd `revived`)
                    if !c.revived && !c.visible {
                        c.revived = true;
                        log("claude.ai: a munkamenet lejárt kötését a teljes oldal betöltésével megújítom");
                        c.next_fetch = Instant::now() + Duration::from_secs(60);
                        let _ = c.webview.load_url(CLAUDE_APP_URL);
                        return;
                    }
                    c.ok = None;
                    c.state = "login";
                }
            }
            // nem érhető el (hálózat, szerverhiba): hamarabb újrapróbáljuk
            _ => {
                if let Some(c) = self.claude.as_mut() {
                    if c.state != "ok" || c.ok.is_none() {
                        c.state = "error";
                    }
                    c.next_fetch = Instant::now() + Duration::from_secs(30);
                }
            }
        }
        self.publish_usage();
    }

    /// Az aktivitásjelző lejárt (vagy nyugtázták): a többi sáv is jelezzen.
    fn on_alert(&mut self, on: bool) {
        log(format!("aktivitás-figyelmeztetés: {}", if on { "be" } else { "ki" }));
        for e in [self.icons_edge(), Edge::Top, Edge::Bottom] {
            self.eval(e, &json!({ "type": "alert", "on": on }));
        }
        if on && self.cfg.activity.sound {
            unsafe {
                let _ = MessageBeep(MB_ICONEXCLAMATION);
            }
        }
    }

    /// Rövid értesítés a jobb panel alján (a menü fölött).
    fn toast(&self, text: &str) {
        self.eval(self.info_edge(), &json!({ "type": "toast", "text": text }));
        self.settings_eval(&json!({ "type": "toast", "text": text }));
    }

    fn set_autostart(&mut self, on: bool) {
        match autostart::set(on) {
            Ok(()) => {
                log(format!("indítás a Windowszal: {}", if on { "be" } else { "ki" }));
                // a konfigurációs fájlban is látszódjon
                self.cfg.start_with_windows = Some(on);
                self.save_config();
            }
            Err(e) => {
                log(format!("indítás a Windowszal – hiba: {e}"));
                self.toast(&e);
            }
        }
        self.eval(self.info_edge(), &json!({ "type": "autostart", "on": autostart::is_enabled() }));
        self.send_settings();
    }

    // ---- Teljes képernyős beállítások ----

    fn settings_eval(&self, payload: &serde_json::Value) {
        if let Some(s) = &self.settings {
            let _ = s.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// A beállításokhoz szükséges összes adat: kapcsolók, appok (telepítettséggel), futó programok.
    fn send_settings(&mut self) {
        if self.settings.is_none() {
            return;
        }
        let apps: Vec<serde_json::Value> = self
            .cfg
            .apps
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let mut j = self.app_json(i, a);
                j["path"] = json!(a.path);
                j["args"] = json!(a.args);
                j["rawIcon"] = json!(a.icon);
                j["place"] = json!(if a.on_right() { "right" } else { "left" });
                j["match"] = json!(a.match_exe);
                j["available"] = json!(self.available.get(i).copied().unwrap_or(true));
                j["canExeIcon"] = json!(a.resolved_path().to_lowercase().ends_with(".exe"));
                j["autostart"] = json!(a.autostart);
                j
            })
            .collect();
        // futó, de nem kitűzött programok – ezekből egy kattintással lehet ikont csinálni
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        let mut running: Vec<serde_json::Value> = Vec::new();
        for w in &wins {
            if w.path.is_empty() || self.matchers.iter().any(|m| m.matches(w)) || extra_key(w).is_none() || w.exe == "applicationframehost.exe" {
                continue;
            }
            if running.iter().any(|r| r["path"] == json!(w.path)) {
                continue;
            }
            let name = self.extra_names.entry(w.path.to_lowercase()).or_insert_with(|| wintrack::display_name(&w.path)).clone();
            running.push(json!({
                "name": name,
                "path": w.path,
                "img": format!("/__icon/{}", percent_encode(&w.path.replace('\\', "/"))),
            }));
        }
        let current_wallpaper = match sys::current_wallpaper() {
            cur if cur.is_empty() => self.cfg.wallpaper.clone(),
            cur => cur,
        };
        let payload = json!({
            "type": "settings",
            "autostart": autostart::is_enabled(),
            "swap": self.cfg.swap_sides,
            "sections": self.cfg.sections,
            "language": self.cfg.language,
            "version": version_label(),
            "apps": apps,
            "running": running,
            "theme": self.cfg.theme,
            "wallpapers": self.wallpaper_list(),
            "wallpaper": current_wallpaper,
        });
        self.settings_eval(&payload);
    }

    fn settings_show(&mut self, animate: bool) {
        self.peek_hide();
        self.fly_hide(false);
        // csak a középső munkaterületet takarja, az oldalsó panelek és a keretek látszanak
        let mon = appbar::work_area();
        if let Some(s) = self.settings.as_mut() {
            unsafe {
                let _ = SetWindowPos(
                    s.hwnd,
                    Some(HWND_TOPMOST),
                    mon.left,
                    mon.top,
                    mon.right - mon.left,
                    mon.bottom - mon.top,
                    windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
                );
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(s.hwnd);
            }
            let was_visible = s.visible;
            s.visible = true;
            if !was_visible {
                log("beállítások megnyitva");
            }
        }
        self.send_settings();
        // a felület beúszó animációval jelenik meg
        if animate {
            self.settings_eval(&json!({ "type": "show" }));
        }
        self.eval(self.info_edge(), &json!({ "type": "settings-open", "on": true }));
    }

    /// Processzor- és memóriaterhelés a beállítások kijelzőjének.
    fn send_sys(&mut self) {
        let now = sys::cpu_times();
        let cpu = match (self.cpu_prev, now) {
            (Some((pi, pt)), Some((ni, nt))) if nt > pt => {
                let total = (nt - pt) as f64;
                let idle = ni.saturating_sub(pi) as f64;
                Some(((1.0 - idle / total) * 100.0).clamp(0.0, 100.0))
            }
            _ => None,
        };
        self.cpu_prev = now;
        let (used, total) = sys::memory().unwrap_or((0, 0));
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
        self.settings_eval(&json!({
            "type": "sys",
            "cpu": cpu.map(|c| (c * 10.0).round() / 10.0),
            "memUsed": used,
            "memTotal": total,
            "cores": cores,
        }));
    }

    /// A beállítások mindig felül lévő ablaka eltakarná az onnan megnyitott
    /// programot (Intéző, böngésző, jegyzettömb), ezért ilyenkor hátrébb engedjük.
    /// A következő megnyitáskor újra felülre kerül.
    fn settings_lower(&self) {
        if let Some(s) = &self.settings {
            unsafe {
                let _ = SetWindowPos(s.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }

    /// Ha nyitva vannak a beállítások, kiúsznak (pl. egy programikonra kattintottak).
    fn settings_dismiss(&mut self) {
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_eval(&json!({ "type": "hide" }));
        }
    }

    fn settings_hide(&mut self) {
        if let Some(s) = self.settings.as_mut() {
            unsafe {
                let _ = ShowWindow(s.hwnd, SW_HIDE);
            }
            s.visible = false;
        }
        self.eval(self.info_edge(), &json!({ "type": "settings-open", "on": false }));
    }

    fn save_config(&mut self) -> bool {
        match config::save(&self.cfg_path, &self.cfg) {
            Ok(()) => true,
            Err(e) => {
                log(&e);
                self.toast(&e);
                false
            }
        }
    }

    /// A felülettől kapott app-listából Config-bejegyzések (a meglévő, nem szerkeszthető
    /// mezőket – pl. args, match – megtartjuk).
    fn apps_from_settings(&self, list: &serde_json::Value) -> Vec<config::AppEntry> {
        let mut out = Vec::new();
        for item in list.as_array().into_iter().flatten() {
            let mut a: config::AppEntry = item["id"]
                .as_u64()
                .and_then(|i| self.cfg.apps.get(i as usize).cloned())
                .unwrap_or_default();
            if let Some(s) = item["name"].as_str() {
                a.name = s.trim().to_string();
            }
            if let Some(s) = item["path"].as_str() {
                if a.path != s {
                    a.path = s.to_string();
                    a.match_exe.clear();
                }
            }
            if let Some(s) = item["args"].as_str() {
                a.args = s.to_string();
            }
            if let Some(s) = item["icon"].as_str() {
                a.icon = s.to_string();
            }
            if let Some(s) = item["color"].as_str() {
                a.color = s.to_string();
            }
            a.place = if item["place"] == "right" { "right".into() } else { String::new() };
            a.autostart = item["autostart"].as_bool().unwrap_or(false);
            if !a.name.is_empty() && !a.path.is_empty() {
                out.push(a);
            }
        }
        out
    }

    fn on_settings_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let kind = msg["type"].as_str().unwrap_or("");
        if kind != "log" {
            log(format!("beállítások: {kind} {}", msg["name"].as_str().unwrap_or("")));
        }
        match kind {
            "ready" => {
                self.send_settings();
                // nyelvváltás után a már látható felület animáció nélkül jelenik meg újra
                if self.settings.as_ref().is_some_and(|s| s.visible) {
                    self.settings_eval(&json!({ "type": "show", "instant": true }));
                }
            }
            // a felület a kiúszó animáció végén kéri az elrejtést
            "close" => self.settings_hide(),
            "language" => {
                let lang = if msg["lang"] == "en" { "en" } else { "hu" };
                if self.cfg.language != lang || self.cfg.language_auto {
                    self.cfg.language = lang.to_string();
                    self.cfg.language_auto = false;
                    log(format!("nyelv: {lang}"));
                    // A memóriában lévő beállításból töltjük újra a felületeket (nem a fájlból
                    // olvassuk vissza), így a mentés sikerétől függetlenül átvált a nyelv.
                    self.save_config();
                    self.apply_config(None);
                    if let Some(s) = &self.settings {
                        let url = settings_url(&self.cfg);
                        log(format!("beállítások újratöltése: {url}"));
                        let _ = s.webview.load_url(&nav_url(&url));
                    }
                }
            }
            "cmd" => match msg["name"].as_str().unwrap_or("") {
                // a config.json újraolvasása (kézi szerkesztés után) és az összes felület újratöltése;
                // a beállítások nyitva maradnak, egy üzenet jelzi, hogy megtörtént
                "reload" => {
                    if self.reload() {
                        let t = self.tr(
                            "A config.json újraolvasva, a felületek újratöltve.",
                            "config.json re-read, all panels reloaded.",
                        );
                        self.settings_eval(&json!({ "type": "toast", "text": t }));
                    }
                }
                "config" => {
                    self.settings_lower();
                    self.open_config();
                }
                "website" => {
                    self.settings_lower();
                    wintrack::shell_open(WEBSITE_URL, "", "", false);
                }
                "exit" => self.exit_requested = true,
                "autostart" => self.set_autostart(msg["on"].as_bool().unwrap_or(false)),
                _ => {}
            },
            "swap" => {
                self.cfg.swap_sides = msg["on"].as_bool().unwrap_or(false);
                if self.save_config() {
                    self.reload();
                }
                self.send_settings();
            }
            "sections" => {
                if let Ok(mut s) = serde_json::from_value::<config::Sections>(msg["sections"].clone()) {
                    s.normalize();
                    self.cfg.sections = s;
                    if self.save_config() {
                        self.eval(self.info_edge(), &json!({ "type": "sections", "sections": self.cfg.sections }));
                    }
                }
            }
            "save-apps" => {
                let apps = self.apps_from_settings(&msg["apps"]);
                log(format!("ikonok mentve: {} db", apps.len()));
                self.cfg.apps = apps;
                if self.save_config() {
                    self.reload();
                    let t = self.tr("Az ikonok elmentve.", "Icons saved.");
                    self.toast(&t);
                }
                self.send_settings();
            }
            "browse" => {
                let owner = self.settings.as_ref().map(|s| s.hwnd).unwrap_or_default();
                if let Some(p) = sys::pick_program(owner) {
                    let is_exe = p.path.to_lowercase().ends_with(".exe");
                    self.settings_eval(&json!({
                        "type": "picked",
                        "app": {
                            "name": p.name,
                            "path": p.path,
                            "args": p.args,
                            "icon": if is_exe { "exe" } else { "" },
                            "img": if is_exe { format!("/__icon/{}", percent_encode(&p.path.replace('\\', "/"))) } else { String::new() },
                            "canExeIcon": is_exe,
                        }
                    }));
                }
            }
            "wall-set" => {
                let p = msg["path"].as_str().unwrap_or("").to_string();
                self.apply_wallpaper(&p);
            }
            "wall-add" => {
                let owner = self.settings.as_ref().map(|s| s.hwnd).unwrap_or_default();
                if let Some(file) = sys::pick_image(owner) {
                    match self.import_wallpaper(&file) {
                        Ok(p) => self.apply_wallpaper(&p),
                        Err(e) => {
                            log(format!("háttérkép másolása sikertelen: {e}"));
                            self.toast(&e);
                        }
                    }
                }
            }
            "wall-folder" => {
                let d = self.wallpaper_dir();
                let _ = std::fs::create_dir_all(&d);
                self.settings_lower();
                wintrack::shell_open(&d.to_string_lossy(), "", "", false);
            }
            "log" => log(format!("[beállítások] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    // ---- Ablakelőnézet (több ablakos programoknál) ----

    fn peek_eval(&self, payload: &serde_json::Value) {
        if let Some(p) = &self.peek {
            let _ = p.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// Az ikonsáv egy több ablakos program fölött időzik: elrendezzük a kisképeket,
    /// és elküldjük a felületnek. Megjelenni akkor fog, amikor a felület kirajzolta.
    fn peek_open(&mut self, idx: usize, msg: &serde_json::Value) {
        if self.peek.is_none() {
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale, bar_mon)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale, b.mon)) else { return };
        if self.fullscreen.contains(&bar_mon) || self.settings.as_ref().is_some_and(|s| s.visible) {
            self.peek_hide();
            return;
        }
        let (mut mine, _) = self.target_windows(msg);
        mine.sort_by_key(|w| self.seen.get(&(w.hwnd.0 as isize)).copied().unwrap_or(u64::MAX));
        if mine.len() < 2 {
            self.peek_hide();
            return;
        }
        let icons_left = bar_edge == Edge::Left;
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);

        // hány kártya fér el egymás mellett, illetve egymás alatt
        let card_h = peek::TITLE_H + peek::THUMB_H;
        let avail_w = (work.right - work.left) as f64 / scale - 24.0;
        let avail_h = (work.bottom - work.top) as f64 / scale - 16.0;
        let max_cols = (((avail_w - 2.0 * peek::PAD + peek::GAP) / (peek::CARD_W + peek::GAP)).floor() as usize).max(1);
        let max_rows = (((avail_h - 2.0 * peek::PAD - peek::HEADER + peek::GAP) / (card_h + peek::GAP)).floor() as usize).max(1);
        let cols = mine.len().min(max_cols);
        mine.truncate(cols * max_rows);
        let rows = mine.len().div_ceil(cols);
        let w_css = 2.0 * peek::PAD + cols as f64 * peek::CARD_W + (cols - 1) as f64 * peek::GAP;
        let h_css = 2.0 * peek::PAD + peek::HEADER + rows as f64 * card_h + (rows - 1) as f64 * peek::GAP;
        let (w, h) = ((w_css * scale).round() as i32, (h_css * scale).round() as i32);

        // az ikon magasságában, a munkaterületen belül
        let icon_y = msg["y"].as_f64().unwrap_or(0.0);
        let cy = bar_rect.top + (icon_y * scale).round() as i32;
        let top = (cy - h / 2).clamp(work.top + 6, (work.bottom - h - 6).max(work.top));
        let left = if icons_left { bar_rect.right } else { bar_rect.left - w };

        let fg = self.last_fg;
        let mut items = Vec::new();
        let mut pending = Vec::new();
        for (i, win) in mine.iter().enumerate() {
            let (col, row) = (i % cols, i / cols);
            let card = peek::Rectf {
                x: peek::PAD + col as f64 * (peek::CARD_W + peek::GAP),
                y: peek::PAD + peek::HEADER + row as f64 * (card_h + peek::GAP),
                w: peek::CARD_W,
                h: card_h,
            };
            let boxr = peek::Rectf {
                x: card.x + peek::INSET,
                y: card.y + peek::TITLE_H,
                w: peek::CARD_W - 2.0 * peek::INSET,
                h: peek::THUMB_H - peek::INSET,
            };
            // kis méretre tett ablakról a Windows nem ad élő képet: ott az ikon látszik
            let src = if win.iconic { None } else { peek::visible_source(win.hwnd) };
            let thumb = src.map(|s| peek::fit(boxr, &s));
            if let (Some(s), Some(t)) = (src, thumb) {
                pending.push((win.hwnd, t.to_phys(scale), s));
            }
            let icon = if win.path.is_empty() {
                String::new()
            } else {
                format!("/__icon/{}", percent_encode(&win.path.replace('\\', "/")))
            };
            items.push(json!({
                "hwnd": win.hwnd.0 as isize,
                "title": if win.title_raw.trim().is_empty() { wintrack::display_name(&win.path) } else { win.title_raw.clone() },
                "icon": icon,
                "iconic": win.iconic,
                "active": Some(win.hwnd) == fg && !win.iconic,
                "card": card.json(),
                "box": boxr.json(),
                "thumb": thumb.map(|t| t.json()),
            }));
        }
        let name = if let Some(id) = msg["id"].as_u64() {
            self.cfg.apps.get(id as usize).map(|a| a.name.clone()).unwrap_or_default()
        } else {
            let key = msg["key"].as_str().unwrap_or("").to_string();
            match self.extra_names.get(&key) {
                Some(n) => n.clone(),
                None => mine.first().map(|w| wintrack::display_name(&w.path)).unwrap_or_default(),
            }
        };
        let hwnds: Vec<isize> = mine.iter().map(|w| w.hwnd.0 as isize).collect();
        let mut target = json!({ "y": icon_y, "bar": idx });
        if !msg["id"].is_null() {
            target["id"] = msg["id"].clone();
        }
        if !msg["key"].is_null() {
            target["key"] = msg["key"].clone();
        }

        let Some(pk) = self.peek.as_mut() else { return };
        for (id, _) in pk.thumbs.drain(..) {
            peek::drop_thumb(id);
        }
        pk.seq += 1;
        pk.pending = pending;
        pk.hwnds = hwnds;
        pk.target = target;
        pk.hide_at = None;
        pk.refresh_at = None;
        let seq = pk.seq;
        unsafe {
            let _ = SetWindowPos(pk.hwnd, Some(HWND_TOPMOST), left, top, w, h, SWP_NOACTIVATE);
        }
        let payload = json!({
            "type": "peek",
            "seq": seq,
            "name": name,
            "count": items.len(),
            "items": items,
            "theme": self.cfg.theme,
        });
        self.peek_eval(&payload);
    }

    /// A felület kirajzolta az előnézetet: bejegyezzük a kisképeket és megjelenítjük.
    fn peek_ready(&mut self, seq: u64) {
        let Some(pk) = self.peek.as_mut() else { return };
        if seq != pk.seq {
            return;
        }
        for (src, rect, source) in pk.pending.drain(..) {
            if let Some(id) = peek::show_thumb(pk.hwnd, src, rect, source) {
                pk.thumbs.push((id, src));
            }
        }
        if !pk.visible {
            pk.visible = true;
            unsafe {
                let _ = ShowWindow(pk.hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(pk.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            self.eval(self.icons_edge(), &json!({ "type": "peek-state", "open": true }));
        }
    }

    fn peek_hide(&mut self) {
        let Some(pk) = self.peek.as_mut() else { return };
        pk.seq += 1;
        pk.pending.clear();
        pk.hwnds.clear();
        pk.hide_at = None;
        pk.refresh_at = None;
        pk.hovered = false;
        for (id, _) in pk.thumbs.drain(..) {
            peek::drop_thumb(id);
        }
        let was_visible = pk.visible;
        pk.visible = false;
        if was_visible {
            unsafe {
                let _ = ShowWindow(pk.hwnd, SW_HIDE);
            }
            self.eval(self.icons_edge(), &json!({ "type": "peek-state", "open": false }));
        }
        self.peek_eval(&json!({ "type": "peek-clear" }));
    }

    /// Az egér elhagyta az ikont vagy az előnézetet: kis késleltetéssel bezárjuk
    /// (közben átérhet az ikonról az előnézetre és vissza).
    fn peek_leave(&mut self, from_icon: bool) {
        let Some(pk) = self.peek.as_mut() else { return };
        if from_icon && pk.hovered {
            return;
        }
        if !from_icon {
            pk.hovered = false;
        }
        if pk.visible {
            pk.hide_at = Some(Instant::now() + Duration::from_millis(if from_icon { 320 } else { 260 }));
        } else {
            // még meg sem jelent: a folyamatban lévő elrendezést eldobjuk
            pk.seq += 1;
            pk.pending.clear();
        }
    }

    fn on_peek_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let hwnd = msg["hwnd"].as_i64().map(|h| h as isize);
        let known = |pk: &PeekWin, h: Option<isize>| {
            h.filter(|h| pk.hwnds.contains(h)).map(|h| HWND(h as *mut core::ffi::c_void))
        };
        match msg["type"].as_str().unwrap_or("") {
            "peek-ready" => self.peek_ready(msg["seq"].as_u64().unwrap_or(0)),
            "peek-enter" => {
                if let Some(pk) = self.peek.as_mut() {
                    pk.hovered = true;
                    pk.hide_at = None;
                }
            }
            "peek-leave" => self.peek_leave(false),
            "peek-activate" => {
                let Some(h) = self.peek.as_ref().and_then(|pk| known(pk, hwnd)) else { return };
                self.peek_hide();
                wintrack::activate(h);
                self.last_fg = Some(h);
                self.last_state.clear();
                self.poll();
            }
            "peek-close" => {
                let Some(pk) = self.peek.as_mut() else { return };
                let Some(h) = known(pk, hwnd) else { return };
                log("előnézet: ablak bezárása");
                if let Some(pos) = pk.thumbs.iter().position(|(_, src)| *src == h) {
                    let (id, _) = pk.thumbs.remove(pos);
                    peek::drop_thumb(id);
                }
                pk.hwnds.retain(|x| *x != h.0 as isize);
                pk.refresh_at = Some(Instant::now() + Duration::from_millis(450));
                unsafe {
                    let _ = PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
            "log" => log(format!("[előnézet] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    /// Időzített teendők a fő ciklusban (az előnézet bezárása, újrarendezése).
    fn tick(&mut self) {
        let now = Instant::now();
        // a részletes időjárás bezárul, ha máshova kattintanak (elveszti a fókuszt)
        if let Some(f) = &self.fly {
            if f.visible && now.duration_since(f.shown_at) > Duration::from_millis(400) {
                let fg = unsafe { GetForegroundWindow() };
                if fg != f.hwnd {
                    self.fly_hide(true);
                }
            }
        }
        if now >= self.next_power {
            self.next_power = now + Duration::from_secs(20);
            self.send_power(false);
        }
        // a hangerőt a billentyűzet vagy más program is állíthatja: gyakran ránézünk
        if now >= self.next_volume {
            self.next_volume = now + Duration::from_millis(600);
            self.send_volume(false);
        }
        let Some(pk) = self.peek.as_ref() else { return };
        if pk.hide_at.is_some_and(|t| now >= t) {
            self.peek_hide();
        } else if pk.refresh_at.is_some_and(|t| now >= t) {
            let target = pk.target.clone();
            if let Some(pk) = self.peek.as_mut() {
                pk.refresh_at = None;
            }
            let idx = target["bar"].as_u64().unwrap_or(0) as usize;
            self.peek_open(idx, &target);
        }
    }

    /// A legközelebbi időzített teendő (ennyit várhat a fő ciklus).
    fn next_deadline(&self) -> Option<Instant> {
        let fly = self.fly.as_ref().filter(|f| f.visible).map(|_| Instant::now() + Duration::from_millis(150));
        let pk = self.peek.as_ref();
        [fly, pk.and_then(|p| p.hide_at), pk.and_then(|p| p.refresh_at)].into_iter().flatten().min()
    }

    // ---- Laptop: akkumulátor ----

    /// Az akkumulátor állapota az információs panelnek (csak változáskor, vagy ha kérik).
    fn volume_ctl(&mut self) -> Option<&sys::Volume> {
        if self.volume.as_ref().is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(60)) {
            self.volume = sys::Volume::open().map(|v| (Instant::now(), v));
        }
        self.volume.as_ref().map(|(_, v)| v)
    }

    fn send_volume(&mut self, force: bool) {
        let state = self.volume_ctl().and_then(|v| v.get());
        if state.is_none() {
            // az eszköz megszűnhetett: legközelebb újranyitjuk
            self.volume = None;
        }
        let payload = match state {
            Some((pct, muted)) => json!({ "type": "volume", "ok": true, "pct": pct, "muted": muted }),
            None => json!({ "type": "volume", "ok": false }),
        };
        let s = payload.to_string();
        if force || s != self.last_volume {
            self.last_volume = s;
            self.eval(self.info_edge(), &payload);
        }
    }

    fn send_power(&mut self, force: bool) {
        let payload = match sys::battery() {
            Some((pct, charging)) => json!({ "type": "power", "laptop": true, "pct": pct, "charging": charging }),
            None => json!({ "type": "power", "laptop": false }),
        };
        let s = payload.to_string();
        if force || s != self.last_power {
            self.last_power = s;
            self.eval(self.info_edge(), &payload);
        }
    }

    // ---- Részletes időjárás (oldalpanel) ----

    fn fly_eval(&self, payload: &serde_json::Value) {
        if let Some(f) = &self.fly {
            let _ = f.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// Az időjárásra vagy a dátumra kattintottak: a kérő panel mellett, a szekció magasságában
    /// kinyílik az oldalpanel. Ha ugyanaz már nyitva van, bezárul.
    fn fly_open(&mut self, idx: usize, kind: &'static str, msg: &serde_json::Value) {
        let Some(f) = self.fly.as_ref() else { return };
        if f.visible && f.kind == kind && f.bar == idx {
            self.fly_hide(false);
            return;
        }
        // épp most zárult be, mert erre a szekcióra kattintva elvesztette a fókuszt: ez zárás volt
        if !f.visible && f.kind == kind && f.hidden_at.is_some_and(|t| t.elapsed() < Duration::from_millis(450)) {
            return;
        }
        let Some((bar_rect, bar_edge, bar_scale, bar_mon)) = self.bars.get(idx).map(|b| (b.rect, b.edge, b.scale, b.mon)) else { return };
        if self.fullscreen.contains(&bar_mon) {
            return;
        }
        // nyitott beállítások: azok kiúsznak, az oldalpanel pedig megnyílik
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_eval(&json!({ "type": "hide" }));
        }
        let scale = bar_scale.max(0.5);
        let work = appbar::work_area_of(bar_rect);
        let w = (FLY_W * scale).round() as i32;
        let h = ((FLY_H * scale).round() as i32).min(work.bottom - work.top - 12);
        // az alja a szekció aljához igazodik, a munkaterületen belül
        let sec_bottom = bar_rect.top + (msg["bottom"].as_f64().unwrap_or(0.0) * scale).round() as i32;
        let top = (sec_bottom - h).clamp(work.top + 6, (work.bottom - h - 6).max(work.top));
        let left = if bar_edge == Edge::Right { bar_rect.left - w } else { bar_rect.right };
        self.peek_hide();
        let Some(f) = self.fly.as_mut() else { return };
        f.visible = true;
        f.shown_at = Instant::now();
        f.hidden_at = None;
        f.kind = kind;
        f.bar = idx;
        unsafe {
            let _ = SetWindowPos(f.hwnd, Some(HWND_TOPMOST), left, top, w, h, windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW);
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(f.hwnd);
        }
        log(format!("oldalpanel megnyitva: {kind}"));
        let payload = if kind == "calendar" {
            json!({ "type": "calendar-open", "feeds": self.calendar_feed_list(), "theme": self.cfg.theme, "language": self.cfg.language })
        } else {
            json!({ "type": "weather-open", "weather": self.cfg.weather, "theme": self.cfg.theme, "language": self.cfg.language })
        };
        self.fly_eval(&payload);
        self.eval(self.info_edge(), &json!({ "type": "fly-state", "kind": kind, "open": true }));
    }

    fn fly_hide(&mut self, lost_focus: bool) {
        let Some(f) = self.fly.as_mut() else { return };
        if !f.visible {
            return;
        }
        f.visible = false;
        f.hidden_at = lost_focus.then(Instant::now);
        unsafe {
            let _ = ShowWindow(f.hwnd, SW_HIDE);
        }
        self.eval(self.info_edge(), &json!({ "type": "fly-state", "open": false }));
    }

    // ---- Naptár (Google Naptár és más iCal-naptárak) ----

    /// A csatolt naptárak a felületnek: név és szín (a titkos címet nem küldjük ki).
    fn calendar_feed_list(&self) -> serde_json::Value {
        json!(self.cfg.calendar.feeds.iter().map(|f| json!({ "name": f.name, "color": f.color })).collect::<Vec<_>>())
    }

    fn send_calendar_feeds(&self) {
        self.fly_eval(&json!({ "type": "cal-feeds", "feeds": self.calendar_feed_list() }));
    }

    /// A naptárak letöltése háttérszálon (tíz percig a legutóbbit adjuk vissza).
    fn calendar_fetch(&mut self, force: bool) {
        if let Some((at, data)) = &self.cal_cache {
            if !force && at.elapsed() < Duration::from_secs(10 * 60) {
                self.fly_eval(&json!({ "type": "cal-data", "feeds": serde_json::from_str::<serde_json::Value>(data).unwrap_or_default() }));
                return;
            }
        }
        if self.cfg.calendar.feeds.is_empty() {
            self.cal_cache = None;
            self.fly_eval(&json!({ "type": "cal-data", "feeds": [] }));
            return;
        }
        if self.cal_busy {
            return;
        }
        self.cal_busy = true;
        let feeds: Vec<(String, String, String)> =
            self.cfg.calendar.feeds.iter().map(|f| (f.name.clone(), f.url.clone(), f.color.clone())).collect();
        std::thread::spawn(move || {
            let agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(20)))
                .build()
                .new_agent();
            let out: Vec<serde_json::Value> = feeds
                .iter()
                .map(|(name, url, color)| {
                    let res = agent.get(url).header("User-Agent", "CyberHub").call();
                    match res {
                        Ok(mut r) => match r.body_mut().with_config().limit(20 * 1024 * 1024).read_to_string() {
                            Ok(text) if text.contains("BEGIN:VCALENDAR") => {
                                json!({ "name": name, "color": color, "ok": true, "ics": text })
                            }
                            Ok(_) => json!({ "name": name, "color": color, "ok": false, "error": "not-ics" }),
                            Err(_) => json!({ "name": name, "color": color, "ok": false, "error": "read" }),
                        },
                        Err(ureq::Error::StatusCode(code)) => {
                            json!({ "name": name, "color": color, "ok": false, "error": format!("HTTP {code}") })
                        }
                        Err(_) => json!({ "name": name, "color": color, "ok": false, "error": "offline" }),
                    }
                })
                .collect();
            send(UserEvent::CalendarData(serde_json::Value::Array(out).to_string()));
        });
    }

    fn on_calendar_data(&mut self, raw: String) {
        self.cal_busy = false;
        let feeds: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        let summary: Vec<String> = feeds
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| format!("{}: {}", f["name"].as_str().unwrap_or("?"), if f["ok"] == true { "ok" } else { f["error"].as_str().unwrap_or("hiba") }))
            .collect();
        log(format!("naptár letöltve – {}", summary.join(", ")));
        self.cal_cache = Some((Instant::now(), raw));
        self.fly_eval(&json!({ "type": "cal-data", "feeds": feeds }));
    }

    /// Új naptár csatolása (Google Naptár: „Titkos cím iCal formátumban”).
    fn calendar_add(&mut self, msg: &serde_json::Value) {
        let mut url = msg["url"].as_str().unwrap_or("").trim().to_string();
        if let Some(rest) = url.strip_prefix("webcal://") {
            url = format!("https://{rest}");
        }
        if !url.starts_with("https://") || url.len() > 2000 {
            self.fly_eval(&json!({ "type": "cal-error", "error": "url" }));
            return;
        }
        if self.cfg.calendar.feeds.iter().any(|f| f.url == url) {
            self.fly_eval(&json!({ "type": "cal-error", "error": "dup" }));
            return;
        }
        const COLORS: [&str; 6] = ["#00e1ff", "#f3e600", "#ff2a6d", "#7ee787", "#b26bff", "#ffb02e"];
        let name = msg["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(40).collect()).unwrap_or_else(|| {
            if url.contains("google.com") { "Google Naptár".to_string() } else { self.tr("Naptár", "Calendar") }
        });
        let color = COLORS[self.cfg.calendar.feeds.len() % COLORS.len()].to_string();
        log(format!("naptár csatolva: {name}"));
        self.cfg.calendar.feeds.push(config::CalendarFeed { name, url, color });
        self.save_config();
        self.cal_cache = None;
        self.send_calendar_feeds();
        self.calendar_fetch(true);
    }

    fn on_fly_ipc(&mut self, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        match msg["type"].as_str().unwrap_or("") {
            "close" => self.fly_hide(false),
            "cal-fetch" => self.calendar_fetch(msg["force"].as_bool().unwrap_or(false)),
            "cal-add" => self.calendar_add(&msg),
            "cal-remove" => {
                let i = msg["i"].as_u64().unwrap_or(u64::MAX) as usize;
                if i < self.cfg.calendar.feeds.len() {
                    let f = self.cfg.calendar.feeds.remove(i);
                    log(format!("naptár eltávolítva: {}", f.name));
                    self.save_config();
                    self.cal_cache = None;
                    self.send_calendar_feeds();
                    self.calendar_fetch(true);
                }
            }
            // a Google Naptár beállításai (itt található a titkos iCal-cím)
            "cal-help" => {
                wintrack::shell_open("https://calendar.google.com/calendar/r/settings", "", "", false);
                self.fly_hide(false);
            }
            "log" => log(format!("[időjárás] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    // ---- Háttérkép ----

    fn wallpaper_dir(&self) -> PathBuf {
        self.cfg_path.parent().map(|p| p.join("wallpapers")).unwrap_or_else(|| PathBuf::from("wallpapers"))
    }

    fn is_wallpaper_file(p: &Path) -> bool {
        p.is_file()
            && p.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .is_some_and(|e| sys::WALLPAPER_EXTS.contains(&e.as_str()))
    }

    /// A háttérképmappa képei, név szerint rendezve.
    fn wallpaper_list(&self) -> Vec<serde_json::Value> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.wallpaper_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| Self::is_wallpaper_file(p))
            .collect();
        files.sort();
        files
            .iter()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                json!({ "path": s, "url": format!("/__file/{}", percent_encode(&s.replace('\\', "/"))) })
            })
            .collect()
    }

    /// Egy tetszőleges helyről választott képet bemásol a háttérképmappába.
    fn import_wallpaper(&self, file: &str) -> Result<String, String> {
        let src = PathBuf::from(file);
        let dir = self.wallpaper_dir();
        if src.parent().is_some_and(|p| p.eq(&dir)) {
            return Ok(file.to_string());
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "kep".into());
        let ext = src.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_else(|| "jpg".into());
        let mut dest = dir.join(format!("{stem}.{ext}"));
        let mut n = 2;
        while dest.exists() {
            dest = dir.join(format!("{stem}-{n}.{ext}"));
            n += 1;
        }
        std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    }

    fn apply_wallpaper(&mut self, path: &str) {
        if !Self::is_wallpaper_file(Path::new(path)) {
            let t = self.tr("A kép nem található.", "Image not found.");
            self.toast(&t);
            return;
        }
        match sys::set_wallpaper(path) {
            Ok(()) => {
                log(format!("háttérkép: {path}"));
                if self.cfg.wallpaper != path {
                    self.cfg.wallpaper = path.to_string();
                    self.save_config();
                }
                let t = self.tr("Háttérkép beállítva.", "Wallpaper set.");
                self.toast(&t);
            }
            Err(e) => {
                log(format!("háttérkép hiba: {e}"));
                let t = self.tr("A háttérkép nem állítható be", "Couldn't set the wallpaper");
                self.toast(&format!("{t}: {e}"));
            }
        }
        self.send_settings();
    }

    /// Főkapcsoló: zárolás, alvás, leállítás, újraindítás.
    fn power(&mut self, action: &str) {
        log(format!("főkapcsoló: {action}"));
        if action == "shutdown" || action == "restart" {
            // a munkamenet véget ér: előbb mindent visszaállítunk
            self.shutdown();
        }
        match sys::power(action) {
            Ok(()) => {
                if action == "shutdown" || action == "restart" {
                    self.exit_requested = true;
                }
            }
            Err(e) => {
                log(format!("főkapcsoló hiba: {e}"));
                self.toast(&e);
            }
        }
    }

    fn on_ipc(&mut self, idx: usize, raw: &str) {
        // egy közben megszűnt képernyő sávjától jövő üzenet
        if idx >= self.bars.len() {
            return;
        }
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else {
            return;
        };
        let kind = msg["type"].as_str().unwrap_or("");
        if !matches!(kind, "ready" | "log" | "peek" | "peek-leave") {
            let detail = msg["name"]
                .as_str()
                .map(str::to_string)
                .or_else(|| msg["id"].as_u64().map(|i| i.to_string()))
                .or_else(|| msg["on"].as_bool().map(|b| b.to_string()))
                .unwrap_or_default();
            log(format!("parancs ({}): {kind} {detail}", self.bars[idx].edge.name()));
        }
        match kind {
            "ready" => {
                log(format!("felület betöltve: {}", self.bars[idx].edge.name()));
                self.send_init(idx);
            }
            "launch" => {
                self.settings_dismiss();
                let id = msg["id"].as_u64().unwrap_or(u64::MAX) as usize;
                let new_instance = msg["newInstance"].as_bool().unwrap_or(false);
                self.launch(id, new_instance);
            }
            "cmd" => match msg["name"].as_str().unwrap_or("") {
                "reload" => {
                    self.reload();
                }
                "config" => self.open_config(),
                "exit" => self.exit_requested = true,
                "autostart" => self.set_autostart(msg["on"].as_bool().unwrap_or(false)),
                "claude-login" => self.claude_show(),
                // a MENÜ gomb nyitja és zárja a beállításokat (záráskor a felület animál)
                "settings" => {
                    if self.settings.as_ref().is_some_and(|s| s.visible) {
                        self.settings_eval(&json!({ "type": "hide" }));
                    } else {
                        self.settings_show(true);
                    }
                }
                "power" => {
                    let action = msg["action"].as_str().unwrap_or("").to_string();
                    self.power(&action);
                }
                // a Windows saját Start menüje és keresője (Win, illetve Win+S)
                "start" => wintrack::press_keys(&[wintrack::VK_WIN]),
                "search" => wintrack::press_keys(&[wintrack::VK_WIN, b'S']),
                // laptop: a Windows saját Wi-Fi-listája, illetve az akkumulátor beállításai
                "wifi" => {
                    wintrack::shell_open("ms-availablenetworks:", "", "", false);
                }
                "battery" => {
                    wintrack::shell_open("ms-settings:batterysaver", "", "", false);
                }
                _ => {}
            },
            "weather-open" => self.fly_open(idx, "weather", &msg),
            "calendar-open" => self.fly_open(idx, "calendar", &msg),
            "launch-extra" => {
                self.settings_dismiss();
                self.launch_extra(&msg);
            }
            // hangerő: a jobb panel csúszkája, görgetése és némítógombja
            "volume-set" => {
                let pct = msg["pct"].as_f64().unwrap_or(0.0).round().clamp(0.0, 100.0) as u8;
                if let Some(v) = self.volume_ctl() {
                    v.set(pct);
                }
                self.send_volume(true);
            }
            "volume-mute" => {
                let on = msg["on"].as_bool().unwrap_or(false);
                if let Some(v) = self.volume_ctl() {
                    v.set_mute(on);
                }
                self.send_volume(true);
            }
            // a Windows hangerőkeverője (programonkénti hangerő)
            "volume-open" => {
                self.settings_dismiss();
                wintrack::shell_open("ms-settings:apps-volume", "", "", false);
            }
            "peek" => self.peek_open(idx, &msg),
            "peek-leave" => self.peek_leave(true),
            "drag-hover" => self.drag_hover(&msg),
            "drop-open" => self.drop_open(&msg),
            "alert" => self.on_alert(msg["on"].as_bool().unwrap_or(false)),
            "usage-refresh" => {
                if let Some(tx) = &self.usage_wake {
                    let _ = tx.send(());
                }
                self.claude_fetch();
            }
            "log" => log(format!("[ui] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    /// Egy program ablakai közti váltás, ahogy a tálca is csinálja: ha nem aktív,
    /// előhozza; ha aktív és egy ablaka van, kis méretre teszi; ha több, a következőre vált.
    fn toggle_windows(&mut self, mine: &[WinInfo]) {
        if mine.is_empty() {
            return;
        }
        let active = mine.iter().find(|w| Some(w.hwnd) == self.last_fg && !w.iconic);
        match active {
            Some(cur) if mine.len() > 1 => {
                let next = mine.iter().rev().find(|w| w.hwnd != cur.hwnd).unwrap();
                wintrack::activate(next.hwnd);
                self.last_fg = Some(next.hwnd);
            }
            Some(cur) => {
                wintrack::minimize(cur.hwnd);
                self.last_fg = None;
            }
            None => {
                wintrack::activate(mine[0].hwnd);
                self.last_fg = Some(mine[0].hwnd);
            }
        }
        self.last_state.clear();
        self.poll();
    }

    /// A felülettől kapott cél (kitűzött app sorszáma vagy futó program kulcsa) ablakai.
    fn target_windows(&mut self, msg: &serde_json::Value) -> (Vec<WinInfo>, Option<String>) {
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        if let Some(id) = msg["id"].as_u64().map(|i| i as usize) {
            let Some(m) = self.matchers.get(id) else { return (Vec::new(), None) };
            let mine = wins.iter().filter(|w| m.matches(w)).cloned().collect();
            let exe = self.cfg.apps.get(id).map(|a| a.resolved_path()).filter(|p| p.to_lowercase().ends_with(".exe"));
            return (mine, exe);
        }
        if let Some(key) = msg["key"].as_str() {
            let mine: Vec<WinInfo> = wins.iter().filter(|w| extra_key(w).as_deref() == Some(key)).cloned().collect();
            let exe = mine.first().map(|w| w.path.clone()).filter(|p| p.to_lowercase().ends_with(".exe"));
            return (mine, exe);
        }
        (Vec::new(), None)
    }

    fn launch_extra(&mut self, msg: &serde_json::Value) {
        self.peek_hide();
        let new_instance = msg["newInstance"].as_bool().unwrap_or(false);
        let (mine, exe) = self.target_windows(msg);
        if new_instance {
            if let Some(exe) = exe {
                wintrack::shell_open(&exe, "", "", false);
            }
            return;
        }
        self.toggle_windows(&mine);
    }

    /// Fájlhúzás a bal sáv fölött: továbbítjuk a felületnek, az dönti el, melyik ikon fölött van.
    fn on_drag(&mut self, idx: usize, phase: &str, x: i32, y: i32, paths: Vec<String>) {
        if self.peek.as_ref().is_some_and(|p| p.visible) {
            self.peek_hide();
        }
        if !paths.is_empty() {
            self.drag_paths = paths;
        }
        self.eval_bar(idx, &json!({ "type": "drag", "phase": phase, "x": x, "y": y }).to_string());
    }

    /// Húzás közben egy ikon fölött időzött a kurzor: az ablakát a többi fölé hozzuk.
    fn drag_hover(&mut self, msg: &serde_json::Value) {
        let (mine, _) = self.target_windows(msg);
        if let Some(w) = mine.first() {
            log(format!("húzás: előtérbe hozva – {}", w.title_raw));
            wintrack::raise_for_drop(w.hwnd);
        }
    }

    /// Fájlok ejtése egy ikonra: megnyitjuk őket azzal a programmal.
    fn drop_open(&mut self, msg: &serde_json::Value) {
        let paths = std::mem::take(&mut self.drag_paths);
        let (_, exe) = self.target_windows(msg);
        match exe {
            Some(exe) if !paths.is_empty() => {
                let ok = wintrack::open_with(&exe, &paths);
                log(format!("ejtés: {} fájl megnyitása – {exe} -> {}", paths.len(), if ok { "ok" } else { "HIBA" }));
            }
            _ => {
                let t = self.tr("Ezzel a programmal nem nyitható meg.", "This program can't open it.");
                self.toast(&t);
            }
        }
    }

    fn launch(&mut self, id: usize, new_instance: bool) {
        let Some(app) = self.cfg.apps.get(id).cloned() else {
            return;
        };
        self.peek_hide();
        let own = self.own_hwnds();
        let wins = self.tracker.snapshot(&own);
        let m = &self.matchers[id];
        let mine: Vec<WinInfo> = wins.iter().filter(|w| m.matches(w)).cloned().collect();

        if !new_instance && !mine.is_empty() {
            self.toggle_windows(&mine);
            return;
        }

        let path = app.resolved_path();
        let ok = wintrack::shell_open(&path, &config::expand_env(&app.args), &config::expand_env(&app.cwd), app.maximize());
        log(format!("indítás: {} ({path}) -> {}", app.name, if ok { "ok" } else { "HIBA" }));
        if ok {
            if app.maximize() {
                self.pending = Some(Pending {
                    app: id,
                    known: wins.iter().map(|w| w.hwnd.0 as isize).collect(),
                    deadline: Instant::now() + Duration::from_secs(20),
                });
            }
        } else {
            let t = self.tr("Nem indítható", "Cannot start");
            self.toast(&format!("{t}: {}", app.name));
        }
    }

    fn poll(&mut self) {
        if self.bars.is_empty() {
            return;
        }
        let own = self.own_hwnds();
        let fg = unsafe { GetForegroundWindow() };
        if !fg.is_invalid() && !own.contains(&fg) {
            self.last_fg = Some(wintrack::root_owner(fg));
        }
        let wins = self.tracker.snapshot(&own);
        for w in &wins {
            let k = w.hwnd.0 as isize;
            if !self.seen.contains_key(&k) {
                self.seen_seq += 1;
                self.seen.insert(k, self.seen_seq);
            }
        }
        if self.seen.len() > 400 {
            let live: HashSet<isize> = wins.iter().map(|w| w.hwnd.0 as isize).collect();
            self.seen.retain(|k, _| live.contains(k));
        }

        if let Some(p) = &self.pending {
            if Instant::now() > p.deadline {
                self.pending = None;
            } else if let Some(w) = wins.iter().find(|w| {
                !p.known.contains(&(w.hwnd.0 as isize)) && w.maximizable && !w.iconic && self.matchers[p.app].matches(w)
            }) {
                wintrack::maximize(w.hwnd);
                self.pending = None;
            }
        }

        let running: Vec<usize> = self
            .matchers
            .iter()
            .enumerate()
            .filter(|(_, m)| wins.iter().any(|w| m.matches(w)))
            .map(|(i, _)| i)
            .collect();
        // több ablakos programok: ezek fölött az ikon előnézetet mutat
        let multi: Vec<usize> = self
            .matchers
            .iter()
            .enumerate()
            .filter(|(_, m)| wins.iter().filter(|w| m.matches(w)).count() >= 2)
            .map(|(i, _)| i)
            .collect();
        let fg_win = self.last_fg.and_then(|h| wins.iter().find(|w| w.hwnd == h)).filter(|w| !w.iconic);
        let active = fg_win.and_then(|w| self.matchers.iter().position(|m| m.matches(w)));

        // Nem kitűzött, de futó programok, a megjelenésük sorrendjében.
        let mut present: Vec<(String, &WinInfo)> = Vec::new();
        for w in &wins {
            if self.matchers.iter().any(|m| m.matches(w)) {
                continue;
            }
            if let Some(k) = extra_key(w) {
                if !present.iter().any(|(pk, _)| *pk == k) {
                    present.push((k, w));
                }
            }
        }
        self.extra_order.retain(|k| present.iter().any(|(pk, _)| pk == k));
        for (k, _) in &present {
            if !self.extra_order.contains(k) {
                self.extra_order.push(k.clone());
            }
        }
        let mut extras = Vec::new();
        for k in &self.extra_order.clone() {
            let Some((_, w)) = present.iter().find(|(pk, _)| pk == k) else { continue };
            let (name, icon) = if k.starts_with("hwnd:") {
                (w.title_raw.clone(), String::new())
            } else {
                let name = self.extra_names.entry(k.clone()).or_insert_with(|| wintrack::display_name(&w.path)).clone();
                (name, format!("/__icon/{}", percent_encode(&w.path.replace('\\', "/"))))
            };
            let n = wins.iter().filter(|x| extra_key(x).as_deref() == Some(k.as_str())).count();
            extras.push(json!({ "key": k, "name": name, "icon": icon, "n": n }));
        }
        let active_extra = fg_win.filter(|_| active.is_none()).and_then(extra_key);

        // A tálca időnként visszajön (pl. Start menü után) – ilyenkor újra elrejtjük,
        // de nem, amíg a Start menü vagy a keresés van előtérben.
        if self.want_hidden_taskbar && appbar::taskbar_visible() {
            let cls = if fg.is_invalid() { String::new() } else { wintrack::class_name(fg) };
            if cls != "Windows.UI.Core.CoreWindow" && cls != "Shell_TrayWnd" && cls != "XamlExplorerHostIslandWindow" {
                appbar::rehide_taskbar();
            }
        }

        // bejelentkezéskor a „géppel induló” programok egyenként, kis szünetekkel indulnak
        if !self.launch_queue.is_empty() && Instant::now() >= self.next_launch {
            if let Some(id) = self.launch_queue.pop_front() {
                let running = self.matchers.get(id).is_some_and(|m| wins.iter().any(|w| m.matches(w)));
                if !running {
                    log(format!("automatikus programindítás: {}", self.cfg.apps.get(id).map(|a| a.name.as_str()).unwrap_or("?")));
                    self.launch(id, false);
                }
            }
            self.next_launch = Instant::now() + Duration::from_millis(1500);
        }

        // nyitott beállításoknál másodpercenként processzor- és memóriaterhelés
        if self.settings.as_ref().is_some_and(|s| s.visible) && Instant::now() >= self.next_sys {
            self.next_sys = Instant::now() + Duration::from_secs(1);
            self.send_sys();
        }

        // a claude.ai használati adatok ötpercenkénti frissítése (belépés közben nem)
        if self.claude.as_ref().is_some_and(|c| !c.visible && Instant::now() >= c.next_fetch) {
            self.claude_fetch();
        }

        if self.relayout_pending && Instant::now() >= self.suppress_until {
            self.relayout_pending = false;
            self.layout(false);
        }

        let state = json!({
            "type": "state",
            "running": running,
            "multi": multi,
            "active": active,
            "extras": extras,
            "activeExtra": active_extra,
        })
        .to_string();
        if state != self.last_state {
            self.last_state = state;
            // a jobb panel is megkapja (az ott lévő appok futás-jelzéséhez)
            for b in self.bars.iter().filter(|b| matches!(b.edge, Edge::Left | Edge::Right)) {
                let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({})", self.last_state));
            }
        }
    }

    fn reload(&mut self) -> bool {
        let old_wallpaper = self.cfg.wallpaper.clone();
        match config::load(&self.cfg_path) {
            Ok(c) => {
                self.cfg = c;
                log("konfiguráció újratöltve");
            }
            Err(e) => {
                log(&e);
                self.toast(&e);
                return false;
            }
        }
        self.apply_config(Some(old_wallpaper));
        true
    }

    /// A (frissen betöltött vagy memóriában módosított) beállítás alkalmazása: elrendezés,
    /// a felületek újratöltése az aktuális nyelvvel és panelhelyzettel.
    fn apply_config(&mut self, old_wallpaper: Option<String>) {
        self.peek_hide();
        self.fly_hide(false);
        self.matchers = self.cfg.apps.iter().map(Matcher::new).collect();
        self.check_available();
        ICONS_RIGHT.store(self.cfg.swap_sides, Ordering::Relaxed);
        self.layout(true);
        // ha a panelek helye változott, a két oldalsó ablak a másik felületet tölti be
        let swap = self.cfg.swap_sides;
        // a címben a nyelv és a panel helye is benne van, ezért mindig újratöltjük a címet
        for b in &mut self.bars {
            b.part = part_for(b.edge, swap);
            let url = bar_url(b.part, b.edge, &self.cfg);
            log(format!("felület újratöltése: {url}"));
            let _ = b.webview.load_url(&nav_url(&url));
        }
        if let Some(p) = &self.peek {
            let _ = p.webview.load_url(&nav_url(&peek_url(&self.cfg)));
        }
        if let Some(f) = &self.fly {
            let _ = f.webview.load_url(&nav_url(&fly_url(&self.cfg)));
        }
        // ha a háttérképet a fájlban írták át, beállítjuk
        if old_wallpaper.is_some_and(|w| w != self.cfg.wallpaper) && !self.cfg.wallpaper.is_empty() {
            let w = self.cfg.wallpaper.clone();
            match sys::set_wallpaper(&w) {
                Ok(()) => log(format!("háttérkép a konfigurációból: {w}")),
                Err(e) => log(format!("háttérkép hiba: {e}")),
            }
        }
        self.last_usage = None;
        self.last_state.clear();
        // nyitott beállításoknál (pl. panelcsere után) a munkaterület változhatott
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_show(false);
        }
    }

    fn open_config(&self) {
        let p = self.cfg_path.to_string_lossy().to_string();
        if !wintrack::shell_open(&p, "", "", false) {
            wintrack::shell_open("notepad.exe", &format!("\"{p}\""), "", false);
        }
    }

    fn on_shell_restarted(&mut self) {
        log("az Intéző újraindult – sávok újraregisztrálása");
        for b in &mut self.bars {
            b.registered = false;
        }
        if self.want_hidden_taskbar {
            appbar::hide_taskbar();
        }
        self.layout(true);
    }

    fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        for b in &self.bars {
            if b.registered {
                appbar::unregister(b.hwnd);
            }
        }
        if let Some(b) = self.bar(Edge::Left) {
            unsafe {
                let _ = UnregisterHotKey(Some(b.hwnd), HOTKEY_EXIT);
                let _ = UnregisterHotKey(Some(b.hwnd), HOTKEY_RELOAD);
            }
        }
        if let Some(s) = self.orig_taskbar {
            appbar::restore_taskbar(s);
        }
        let _ = std::fs::remove_file(state_path());
        if let Ok(mut c) = CLEANUP.lock() {
            *c = None;
        }
        log("kilépés, minden visszaállítva");
    }
}

/// Egy sáv (ablak és felület) létrehozása az adott képernyőhöz. A helyét a `layout` adja meg.
fn make_bar(
    target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
    web_context: &mut WebContext,
    ui_dir: &Path,
    cfg: &Config,
    edge: Edge,
    idx: usize,
    mon: usize,
) -> Bar {
    let window = WindowBuilder::new()
        // Az elsődleges képernyő bal sávja „CyberHub left” (erre keres a --quit és az őrfolyamat),
        // a többi képernyőé sorszámot kap.
        .with_title(if mon == 0 { format!("{APP_NAME} {}", edge.name()) } else { format!("{APP_NAME} {} {mon}", edge.name()) })
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_visible(false)
        .with_focused(false)
        .with_skip_taskbar(true)
        .with_undecorated_shadow(false)
        .with_drag_and_drop(false)
        .with_position(PhysicalPosition::new(0, 0))
        .with_inner_size(PhysicalSize::new(64u32, 64u32))
        .build(target)
        .expect("ablak létrehozása");
    let hwnd = hwnd_of(&window);
    unsafe {
        // Keret nélküli, eszköz típusú ablak: nem látszik az Alt+Tabban, nincs lekerekítve.
        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize);
        let pref = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const _,
            std::mem::size_of_val(&pref) as u32,
        );
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 64, 64, SWP_NOACTIVATE | SWP_FRAMECHANGED);
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 0x4E48, 0);
    }

    let part = part_for(edge, cfg.swap_sides);
    let url = bar_url(part, edge, cfg);
    let bg = (6u8, 7u8, 11u8, 255u8);
    let dir = ui_dir.to_path_buf();
    let page_edge = edge.name();
    let this_edge = edge;
    // Fájlhúzás: az ikonsávon az ikonokhoz kötjük, a többi sávon csak elnyeljük,
    // különben a ráejtett fájl a sáv helyén nyílna meg. (A panelek felcserélhetők.)
    let drag_handler = move |ev: wry::DragDropEvent| -> bool {
        let icons_here = part_for(this_edge, ICONS_RIGHT.load(Ordering::Relaxed)) == "left";
        if icons_here {
            let to_strings = |p: Vec<PathBuf>| p.into_iter().map(|x| x.to_string_lossy().to_string()).collect();
            match ev {
                wry::DragDropEvent::Enter { paths, position } => {
                    send(UserEvent::Drag(idx, "over", position.0, position.1, to_strings(paths)))
                }
                wry::DragDropEvent::Over { position } => {
                    send(UserEvent::Drag(idx, "over", position.0, position.1, Vec::new()))
                }
                wry::DragDropEvent::Drop { paths, position } => {
                    send(UserEvent::Drag(idx, "drop", position.0, position.1, to_strings(paths)))
                }
                wry::DragDropEvent::Leave => send(UserEvent::Drag(idx, "leave", 0, 0, Vec::new())),
                _ => {}
            }
        }
        true
    };
    let webview = WebViewBuilder::new_with_web_context(web_context)
        .with_custom_protocol("cyberhub".into(), move |_id, req| serve(&dir, &req))
        .with_on_page_load_handler(move |ev, url| {
            if matches!(ev, wry::PageLoadEvent::Finished) {
                log(format!("oldal betöltve ({page_edge}): {url}"));
            }
        })
        .with_url(url)
        .with_background_color(bg)
        .with_devtools(cfg!(debug_assertions))
        .with_default_context_menus(false)
        .with_browser_accelerator_keys(false)
        .with_ipc_handler(move |req: Request<String>| send(UserEvent::Ipc(idx, req.body().clone())))
        .with_drag_drop_handler(drag_handler)
        .build(&window)
        .expect("WebView2 létrehozása");

    Bar { edge, mon, scale: 1.0, part, window, webview, hwnd, rect: RECT::default(), registered: false }
}

/// Ha egy korábbi futás nem tudott rendesen kilépni, itt rendbe tesszük a képernyőt.
fn recover_stale() -> bool {
    let p = state_path();
    let Ok(text) = std::fs::read_to_string(&p) else {
        return false;
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = v["hwnds"].as_array() {
            for h in arr.iter().filter_map(|x| x.as_i64()) {
                appbar::unregister(HWND(h as isize as *mut core::ffi::c_void));
            }
        }
        if let Some(s) = v["taskbarState"].as_u64() {
            appbar::restore_taskbar(s as u32);
        }
    }
    let _ = std::fs::remove_file(&p);
    log("előző futás maradványai eltávolítva");
    true
}

/// Őrfolyamat: megvárja, amíg a fő példány kilép – akár összeomlás vagy a
/// Feladatkezelőből leállítás miatt –, és ha az nem tudott rendet rakni, rendet rak.
fn guard(pid: u32) {
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(h) => {
                WaitForSingleObject(h, INFINITE);
                let _ = CloseHandle(h);
            }
            Err(_) => return,
        }
    }
    if let Ok(f) = std::fs::OpenOptions::new().append(true).open(data_dir().join("cyberhub.log")) {
        *LOG.lock().unwrap() = Some(f);
    }
    std::thread::sleep(Duration::from_millis(300));
    let running = || unsafe { FindWindowW(None, w!("CyberHub left")) }.map(|h| !h.is_invalid()).unwrap_or(false);
    if running() {
        return;
    }
    if state_path().exists() {
        log("őr: a program nem lépett ki rendesen, visszaállítás");
        recover_stale();
    }
    // Néhány másodpercig figyeljük, hogy a tálca tényleg látható maradjon.
    for _ in 0..25 {
        if running() {
            return;
        }
        if !appbar::taskbar_visible() {
            log("őr: a tálca rejtve maradt, megjelenítés");
            appbar::show_taskbar();
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn emergency_cleanup() {
    if let Ok(c) = CLEANUP.lock() {
        if let Some((hwnds, tb)) = c.as_ref() {
            for h in hwnds {
                appbar::unregister(HWND(*h as *mut core::ffi::c_void));
            }
            if let Some(s) = tb {
                appbar::restore_taskbar(*s);
            }
        }
    }
}

fn default_config() -> Config {
    let mut c = Config::default();
    c.resolve_language();
    c.apps = vec![config::AppEntry {
        name: "Fájlok".into(),
        path: "explorer.exe".into(),
        icon: "folder".into(),
        match_title: String::new(),
        maximize: Some(false),
        ..Default::default()
    }];
    c
}

/// Claude Code állapotsor-parancsa: a Claude Code a standard bemeneten átadja az
/// aktuális adatokat (köztük az 5 órás és heti korlátot). Ezeket elmentjük a
/// panelnek, és egy rövid sort írunk vissza, amit a Claude Code megjelenít.
fn statusline() {
    use std::io::Read;
    let mut input = String::new();
    let read = std::io::stdin().read_to_string(&mut input);
    let v: serde_json::Value =
        serde_json::from_str(input.trim_start_matches('\u{feff}')).unwrap_or(serde_json::Value::Null);
    if std::env::var("CYBERHUB_DEBUG").is_ok() {
        let _ = std::fs::write(
            data_dir().join("statusline-debug.txt"),
            format!("read={:?} len={} rate_limits={}", read.map_err(|e| e.kind()), input.len(), v["rate_limits"].is_object()),
        );
    }
    let rl = &v["rate_limits"];
    if rl.is_object() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let out = json!({ "five_hour": rl["five_hour"], "seven_day": rl["seven_day"], "writtenAt": now });
        let path = usage::limits_file();
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, out.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(m) = v["model"]["display_name"].as_str() {
        parts.push(m.to_string());
    }
    if let Some(p) = rl["five_hour"]["used_percentage"].as_f64() {
        parts.push(format!("5h {}%", p.round()));
    }
    if let Some(p) = rl["seven_day"]["used_percentage"].as_f64() {
        parts.push(format!("hét {}%", p.round()));
    }
    print!("{}", parts.join(" · "));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();

    if flag("--statusline") {
        statusline();
        return;
    }

    // --quit: a futó példányt szabályosan leállítja (ugyanaz, mint a Ctrl+Alt+Q).
    if flag("--quit") {
        unsafe {
            if let Ok(h) = FindWindowW(None, w!("CyberHub left")) {
                let _ = PostMessageW(Some(h), WM_HOTKEY, WPARAM(HOTKEY_EXIT as usize), LPARAM(0));
            }
        }
        return;
    }

    if flag("--restore") {
        recover_stale();
        return;
    }

    // --usage-check: egyszer lekéri a Claude-használatot, és csak az állapotot írja ki.
    if flag("--usage-check") {
        let v = usage::fetch();
        let summary = json!({ "status": v["status"], "message": v["message"], "plan": v["plan"], "limits": v["limits"] });
        let _ = std::fs::write(data_dir().join("usage-check.txt"), summary.to_string());
        return;
    }

    if let Some(pid) = value("--guard").and_then(|s| s.parse::<u32>().ok()) {
        guard(pid);
        return;
    }

    // Egyszerre csak egy példány futhat.
    unsafe {
        let _m = CreateMutexW(None, true, w!("Local\\CyberHub.SingleInstance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        std::mem::forget(_m);
    }

    // Az előző futás naplóját megtartjuk (cyberhub.prev.log), hogy egy sikertelen
    // indítás nyoma ne vesszen el.
    // Ha a fájl épp foglalt (pl. rendszerinduláskor a vírusirtó vizsgálja), néhányszor
    // újrapróbáljuk, végső esetben időbélyeges nevű naplóba írunk – napló mindig legyen.
    let log_path = data_dir().join("cyberhub.log");
    for attempt in 0..10 {
        let _ = std::fs::rename(&log_path, data_dir().join("cyberhub.prev.log"));
        if let Ok(f) = std::fs::File::create(&log_path) {
            *LOG.lock().unwrap() = Some(f);
            break;
        }
        if attempt == 9 {
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            if let Ok(f) = std::fs::File::create(data_dir().join(format!("cyberhub-{ts}.log"))) {
                *LOG.lock().unwrap() = Some(f);
            }
        } else {
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    let started_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    log(format!("indulás (unix idő: {started_at}), paraméterek: {:?}", &args[1..]));

    // Bejelentkezéskori indulás: megvárjuk, amíg az Intéző és a tálca elkészül,
    // különben a képernyősávok lefoglalása és a tálca elrejtése nem sikerülne.
    if flag("--autostart") {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(90) {
            if appbar::taskbar_hwnd().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        std::thread::sleep(Duration::from_secs(4));
        log(format!("automatikus indítás: az Intéző kész ({} mp után)", t0.elapsed().as_secs()));
    }

    // Őrfolyamat: ha ez a példány bármi módon leáll, visszaadja a tálcát.
    if let Ok(exe) = std::env::current_exe() {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let _ = std::process::Command::new(exe)
            .args(["--guard", &std::process::id().to_string()])
            .creation_flags(DETACHED_PROCESS)
            .spawn();
    }

    recover_stale();
    std::panic::set_hook(Box::new(|info| {
        log(format!("PÁNIK: {info}"));
        emergency_cleanup();
    }));

    let base_dir = find_base_dir();
    let ui_dir = base_dir.join("ui");
    let cfg_path = value("--config").map(PathBuf::from).unwrap_or_else(|| base_dir.join("config.json"));
    let mut loaded_ok = true;
    let mut cfg = match config::load(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            log(&e);
            loaded_ok = false;
            let c = default_config();
            if !cfg_path.exists() {
                let _ = config::save(&cfg_path, &c);
            }
            c
        }
    };
    // „Indulás a géppel”: a konfigurációs fájl az irányadó. Ha még nincs benne,
    // a Windows jelenlegi beállítását vesszük át. Bekapcsolt állapotban a bejegyzést
    // mindig frissítjük (aktuális exe-útvonal, az Intézőre váró indítás).
    let registered = autostart::is_enabled();
    let mut autostart_changed = false;
    match cfg.start_with_windows {
        None => cfg.start_with_windows = Some(registered),
        // a Windowsban (Feladatkezelő) kikapcsolták: ezt tiszteletben tartjuk
        Some(true) if autostart::disabled_in_windows() => {
            log("indítás a Windowszal: a Windowsban letiltották, kikapcsolva");
            cfg.start_with_windows = Some(false);
            autostart_changed = true;
        }
        Some(want) => {
            if want || registered {
                if let Err(e) = autostart::set(want) {
                    log(format!("indítás a Windowszal – hiba: {e}"));
                }
            }
        }
    }
    // A fájlba minden opció kerüljön bele (a hiányzókat alapértékkel kiegészítjük).
    if loaded_ok && (autostart_changed || config::needs_rewrite(&cfg_path, &cfg)) {
        match config::save(&cfg_path, &cfg) {
            Ok(()) => log("konfigurációs fájl kiegészítve az összes opcióval"),
            Err(e) => log(e),
        }
    }
    let hide_taskbar = cfg.hide_taskbar && !flag("--keep-taskbar");
    let exit_after = value("--exit-after").and_then(|s| s.parse::<u64>().ok()).map(Duration::from_secs);
    log(format!("mappa: {}", base_dir.display()));
    log(format!("konfiguráció: {} ({} app)", cfg_path.display(), cfg.apps.len()));

    // Az indításkor előtérben lévő ablak lesz az első „aktív” program.
    let initial_fg = {
        let fg = unsafe { GetForegroundWindow() };
        (!fg.is_invalid()).then(|| wintrack::root_owner(fg))
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // A fájlhúzáshoz (OLE drag & drop) kell; az ablakkezelő ezt csak a saját
    // húzáskezelőjével együtt indítaná, amit kikapcsoltunk.
    unsafe {
        let _ = OleInitialize(None);
    }
    let _ = PROXY.set(Mutex::new(event_loop.create_proxy()));
    TASKBAR_CREATED.store(unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) }, Ordering::Relaxed);

    let mut web_context = WebContext::new(Some(data_dir().join("WebView2")));
    let mut bars: Vec<Bar> = Vec::new();

    // képernyőnként négy sáv; elöl az elsődleges képernyőé
    let groups = if cfg.all_monitors { appbar::monitors().len().max(1) } else { 1 };
    for mon in 0..groups {
        for edge in EDGES {
            let idx = bars.len();
            bars.push(make_bar(&event_loop, &mut web_context, &ui_dir, &cfg, edge, idx, mon));
        }
    }

    // Teljes képernyős beállítások: rejtve indul, a MENÜ gombra jelenik meg.
    let settings = {
        let window = WindowBuilder::new()
            .with_title("CyberHub – beállítások")
            .with_decorations(false)
            .with_visible(false)
            .with_always_on_top(true)
            .with_skip_taskbar(true)
            .with_undecorated_shadow(false)
            .with_drag_and_drop(false)
            .with_inner_size(PhysicalSize::new(800u32, 600u32))
            .build(&event_loop)
            .ok();
        window.and_then(|window| {
            let hwnd = hwnd_of(&window);
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize);
                let pref = DWMWCP_DONOTROUND;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of_val(&pref) as u32,
                );
            }
            let dir = ui_dir.clone();
            let webview = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_custom_protocol("cyberhub".into(), move |_id, req| serve(&dir, &req))
                .with_url(settings_url(&cfg))
                .with_background_color((6, 7, 11, 255))
                .with_devtools(cfg!(debug_assertions))
                .with_default_context_menus(false)
                .with_browser_accelerator_keys(false)
                .with_ipc_handler(|req: Request<String>| send(UserEvent::SettingsIpc(req.body().clone())))
                .with_drag_drop_handler(|_| true)
                .build(&window)
                .map_err(|e| log(format!("beállítások ablak nem hozható létre: {e}")))
                .ok()?;
            Some(SettingsWin { window, webview, hwnd, visible: false })
        })
    };
    // Ablakelőnézet: rejtve indul, a több ablakos programok ikonja fölött jelenik meg.
    let peek_win = {
        let window = WindowBuilder::new()
            .with_title("CyberHub – ablakelőnézet")
            .with_decorations(false)
            .with_visible(false)
            .with_focused(false)
            .with_always_on_top(true)
            .with_skip_taskbar(true)
            .with_undecorated_shadow(false)
            .with_drag_and_drop(false)
            .with_inner_size(PhysicalSize::new(480u32, 240u32))
            .build(&event_loop)
            .ok();
        window.and_then(|window| {
            let hwnd = hwnd_of(&window);
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize);
                let pref = DWMWCP_DONOTROUND;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of_val(&pref) as u32,
                );
            }
            let dir = ui_dir.clone();
            let webview = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_custom_protocol("cyberhub".into(), move |_id, req| serve(&dir, &req))
                .with_url(peek_url(&cfg))
                .with_background_color((6, 7, 11, 255))
                .with_devtools(cfg!(debug_assertions))
                .with_default_context_menus(false)
                .with_browser_accelerator_keys(false)
                .with_ipc_handler(|req: Request<String>| send(UserEvent::PeekIpc(req.body().clone())))
                .with_drag_drop_handler(|_| true)
                .build(&window)
                .map_err(|e| log(format!("előnézet ablak nem hozható létre: {e}")))
                .ok()?;
            Some(PeekWin {
                window,
                webview,
                hwnd,
                visible: false,
                thumbs: Vec::new(),
                pending: Vec::new(),
                hwnds: Vec::new(),
                seq: 0,
                target: serde_json::Value::Null,
                hovered: false,
                hide_at: None,
                refresh_at: None,
            })
        })
    };
    // Részletes időjárás: rejtve indul, az időjárásra kattintva nyílik ki a panel mellett.
    let fly_win = {
        let window = WindowBuilder::new()
            .with_title("CyberHub – oldalpanel")
            .with_decorations(false)
            .with_visible(false)
            .with_always_on_top(true)
            .with_skip_taskbar(true)
            .with_undecorated_shadow(false)
            .with_drag_and_drop(false)
            .with_inner_size(PhysicalSize::new(380u32, 560u32))
            .build(&event_loop)
            .ok();
        window.and_then(|window| {
            let hwnd = hwnd_of(&window);
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize);
                let pref = DWMWCP_DONOTROUND;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of_val(&pref) as u32,
                );
            }
            let dir = ui_dir.clone();
            let webview = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_custom_protocol("cyberhub".into(), move |_id, req| serve(&dir, &req))
                .with_url(fly_url(&cfg))
                .with_background_color((6, 7, 11, 255))
                .with_devtools(cfg!(debug_assertions))
                .with_default_context_menus(false)
                .with_browser_accelerator_keys(false)
                .with_ipc_handler(|req: Request<String>| send(UserEvent::FlyIpc(req.body().clone())))
                .with_drag_drop_handler(|_| true)
                .build(&window)
                .map_err(|e| log(format!("oldalpanel nem hozható létre: {e}")))
                .ok()?;
            Some(FlyWin { window, webview, hwnd, visible: false, shown_at: Instant::now(), hidden_at: None, kind: "weather", bar: 0 })
        })
    };
    ICONS_RIGHT.store(cfg.swap_sides, Ordering::Relaxed);

    // claude.ai ablak: rejtve indul, csak belépéskor jelenik meg. A sütik a CyberHub saját
    // böngészőprofiljában maradnak, az asztali Claude apphoz nem nyúlunk.
    let claude = if cfg.claude_usage {
        let (mon, _) = appbar::primary_monitor();
        let window = WindowBuilder::new()
            .with_title(if cfg.language == "en" { "CyberHub – sign in to claude.ai" } else { "CyberHub – belépés a claude.ai-ra" })
            // a tálcán az exe saját ikonja látszódjon (assets/cyberhub.rc, 1-es azonosító)
            .with_window_icon({
                use tao::platform::windows::IconExtWindows;
                tao::window::Icon::from_resource(1, None).ok()
            })
            .with_visible(false)
            .with_inner_size(tao::dpi::LogicalSize::new(480.0, 760.0))
            .with_position(PhysicalPosition::new((mon.left + mon.right) / 2 - 300, (mon.top + mon.bottom) / 2 - 470))
            .build(&event_loop)
            .ok();
        window.and_then(|window| {
            let webview = WebViewBuilder::new_with_web_context(&mut web_context)
                .with_url(CLAUDE_IDLE_URL)
                .with_background_color((6, 7, 11, 255))
                .with_ipc_handler(|req: Request<String>| send(UserEvent::ClaudeWeb(req.body().clone())))
                // A Google-belépés felugró ablakban zajlik; alapból ezt a komponens letiltaná,
                // és a claude.ai „There was an error logging you in” hibát adna.
                .with_new_window_req_handler(|url: String, _features| {
                    log(format!("claude.ai felugró ablak: {}", url.split('?').next().unwrap_or("")));
                    if url.starts_with("https://") { wry::NewWindowResponse::Allow } else { wry::NewWindowResponse::Deny }
                })
                .with_on_page_load_handler(|ev, url| {
                    if matches!(ev, wry::PageLoadEvent::Finished) {
                        send(UserEvent::ClaudeLoaded(url));
                    }
                })
                .build(&window)
                .map_err(|e| log(format!("claude.ai ablak nem hozható létre: {e}")))
                .ok()?;
            Some(ClaudeWeb {
                window,
                webview,
                visible: false,
                next_fetch: Instant::now() + Duration::from_secs(300),
                ok: None,
                persisted: false,
                revived: false,
                state: "pending",
            })
        })
    } else {
        None
    };

    let orig_taskbar = if hide_taskbar {
        let s = appbar::taskbar_state();
        appbar::hide_taskbar();
        Some(s)
    } else {
        None
    };

    // Claude-használat lekérése háttérszálon.
    let usage_wake = if cfg.claude_usage {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        usage::spawn(rx, |v| send(UserEvent::Usage(v.to_string())));
        Some(tx)
    } else {
        None
    };

    let matchers = cfg.apps.iter().map(Matcher::new).collect();
    let mut hub = Hub {
        cfg,
        cfg_path,
        matchers,
        bars,
        tracker: Tracker::default(),
        last_fg: initial_fg,
        last_state: String::new(),
        pending: None,
        orig_taskbar,
        want_hidden_taskbar: hide_taskbar,
        fullscreen: HashSet::new(),
        want_groups: 0,
        scale: 1.0,
        suppress_until: Instant::now(),
        relayout_pending: false,
        exit_requested: false,
        shut_down: false,
        last_usage: None,
        usage_wake,
        drag_paths: Vec::new(),
        extra_order: Vec::new(),
        extra_names: HashMap::new(),
        claude,
        thread_usage: None,
        settings,
        available: Vec::new(),
        session: started_at,
        cpu_prev: None,
        next_sys: Instant::now(),
        launch_queue: std::collections::VecDeque::new(),
        next_launch: Instant::now(),
        peek: peek_win,
        seen: HashMap::new(),
        seen_seq: 0,
        fly: fly_win,
        last_power: String::new(),
        next_power: Instant::now(),
        volume: None,
        last_volume: String::new(),
        next_volume: Instant::now(),
        cal_cache: None,
        cal_busy: false,
    };
    hub.check_available();
    // Bejelentkezéskori indulásnál a „géppel induló” programokat is elindítjuk.
    if flag("--autostart") {
        hub.launch_queue = hub
            .cfg
            .apps
            .iter()
            .enumerate()
            .filter(|(i, a)| a.autostart && hub.available.get(*i).copied().unwrap_or(true))
            .map(|(i, _)| i)
            .collect();
        hub.next_launch = Instant::now() + Duration::from_secs(5);
        if !hub.launch_queue.is_empty() {
            log(format!("bejelentkezéskor indítandó programok: {}", hub.launch_queue.len()));
        }
    }
    hub.save_state();
    hub.layout(true);
    for b in &hub.bars {
        if b.registered {
            unsafe {
                let _ = ShowWindow(b.hwnd, SW_SHOWNOACTIVATE);
            }
        }
        let _ = &b.window;
    }
    if let Some(left) = hub.bar(Edge::Left) {
        unsafe {
            let mods = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
            if RegisterHotKey(Some(left.hwnd), HOTKEY_EXIT, mods, 'Q' as u32).is_err() {
                log("a Ctrl+Alt+Q gyorsbillentyű foglalt");
            }
            let _ = RegisterHotKey(Some(left.hwnd), HOTKEY_RELOAD, mods, 'R' as u32);
        }
    }

    let started = Instant::now();
    let mut next_poll = Instant::now();
    event_loop.run(move |event, target, control_flow| {
        match event {
            Event::UserEvent(ev) => match ev {
                UserEvent::Ipc(idx, msg) => hub.on_ipc(idx, &msg),
                UserEvent::Reposition => hub.request_layout(),
                UserEvent::ShellRestarted => hub.on_shell_restarted(),
                UserEvent::Fullscreen(h, on) => hub.set_fullscreen(h, on),
                UserEvent::Hotkey(HOTKEY_EXIT) => hub.exit_requested = true,
                UserEvent::Hotkey(HOTKEY_RELOAD) => {
                    hub.reload();
                }
                UserEvent::Hotkey(_) => {}
                UserEvent::Usage(payload) => hub.on_usage(payload),
                UserEvent::Drag(idx, phase, x, y, paths) => hub.on_drag(idx, phase, x, y, paths),
                UserEvent::ClaudeWeb(raw) => hub.on_claude_web(&raw),
                UserEvent::ClaudeLoaded(url) => hub.on_claude_loaded(&url),
                UserEvent::SettingsIpc(raw) => hub.on_settings_ipc(&raw),
                UserEvent::PeekIpc(raw) => hub.on_peek_ipc(&raw),
                UserEvent::FlyIpc(raw) => hub.on_fly_ipc(&raw),
                UserEvent::CalendarData(raw) => hub.on_calendar_data(raw),
            },
            // a claude.ai ablak bezárása csak elrejti
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. }
                if hub.claude.as_ref().is_some_and(|c| c.window.id() == window_id) =>
            {
                hub.claude_hide();
            }
            Event::WindowEvent { event: WindowEvent::ScaleFactorChanged { .. }, .. } => {
                hub.relayout_pending = true;
            }
            Event::LoopDestroyed => {
                hub.shutdown();
                return;
            }
            _ => {}
        }
        let _keep_settings = hub.settings.as_ref().map(|s| &s.window);
        let _keep_peek = hub.peek.as_ref().map(|p| &p.window);
        let _keep_fly = hub.fly.as_ref().map(|f| &f.window);
        hub.tick();
        // képernyő került be vagy ki: a sávcsoportokat itt (ablak létrehozására alkalmas helyen) igazítjuk
        if hub.needs_rebuild() {
            hub.rebuild_bars(target, &mut web_context, &ui_dir);
        }

        let now = Instant::now();
        if now >= next_poll {
            hub.poll();
            next_poll = now + POLL;
        }
        if hub.exit_requested || exit_after.is_some_and(|d| now.duration_since(started) >= d) {
            hub.shutdown();
            *control_flow = ControlFlow::Exit;
            return;
        }
        let wake = hub.next_deadline().map_or(next_poll, |d| d.min(next_poll));
        *control_flow = ControlFlow::WaitUntil(wake);
    });
}
