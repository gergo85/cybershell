#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
mod autostart;
mod config;
mod usage;
mod wintrack;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
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
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ABN_FULLSCREENAPP, ABN_POSCHANGED};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow, PostMessageW, RegisterWindowMessageW, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, SystemParametersInfoW, MB_ICONEXCLAMATION,
    GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SPI_GETWORKAREA,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_FRAMECHANGED, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WM_DISPLAYCHANGE, WM_HOTKEY, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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

#[derive(Debug, Clone)]
enum UserEvent {
    Ipc(usize, String),
    Reposition,
    ShellRestarted,
    Fullscreen(bool),
    Hotkey(i32),
    Usage(String),
    /// Fájlhúzás a bal sáv fölött: fázis ("over", "drop", "leave"), pozíció, fájlok.
    Drag(&'static str, i32, i32, Vec<String>),
    /// A claude.ai ablak üzenete (használati adat vagy belépés szükséges).
    ClaudeWeb(String),
    /// A claude.ai ablakban betöltődött egy oldal.
    ClaudeLoaded(String),
}

/// A claude.ai ablak üresjárati oldala: könnyű, de a claude.ai-hoz tartozik,
/// így innen elérhetők a saját fiókod adatai (a sütik a CyberHub saját böngészőprofiljában vannak).
const CLAUDE_IDLE_URL: &str = "https://claude.ai/robots.txt";
const CLAUDE_LOGIN_URL: &str = "https://claude.ai/login";

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
        if !["png", "jpg", "jpeg", "svg", "ico", "webp", "gif"].contains(&ext.as_str()) {
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
            ABN_FULLSCREENAPP => send(UserEvent::Fullscreen(lp.0 != 0)),
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
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

struct Bar {
    edge: Edge,
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
    /// Ha mi rejtettük el a tálcát: az eredeti állapota.
    orig_taskbar: Option<u32>,
    want_hidden_taskbar: bool,
    fullscreen: bool,
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

    fn bar(&self, edge: Edge) -> Option<&Bar> {
        self.bars.iter().find(|b| b.edge == edge)
    }

    /// Kiszámolja és lefoglalja a négy sáv helyét. `force` nélkül csak akkor nyúl
    /// hozzájuk, ha változott valami – így a rendszer értesítései nem pörgetik körbe.
    fn layout(&mut self, force: bool) {
        if self.bars.is_empty() {
            return;
        }
        let (mon, _) = appbar::primary_monitor();
        let mut base = mon;
        if !self.want_hidden_taskbar {
            if let Some((edge, rc)) = appbar::taskbar_reserved() {
                match edge {
                    Edge::Bottom => base.bottom = rc.top,
                    Edge::Top => base.top = rc.bottom,
                    Edge::Left => base.left = rc.right,
                    Edge::Right => base.right = rc.left,
                }
            }
        }
        let dpi = unsafe { GetDpiForWindow(self.bars[0].hwnd) }.max(96);
        let scale = dpi as f64 / 96.0;
        self.scale = scale;
        let px = |v: u32| ((v as f64) * scale).round() as i32;

        // A bal és a jobb panel teljes magasságú, a felső és alsó keret a kettő között fut.
        let mut plan: Vec<(usize, RECT, i32)> = Vec::new();
        let mut left_rc = RECT { left: base.left, top: base.top, right: base.left, bottom: base.bottom };
        let mut right_rc = RECT { left: base.right, top: base.top, right: base.right, bottom: base.bottom };

        for (i, edge) in EDGES.iter().enumerate() {
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
                "sáv {}: {},{} – {},{}",
                bar.edge.name(),
                bar.rect.left,
                bar.rect.top,
                bar.rect.right,
                bar.rect.bottom
            ));
        }
        if self.fullscreen {
            self.set_fullscreen(true);
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
            "munkaterület: {},{} – {},{} (DPI {dpi})",
            work.left, work.top, work.right, work.bottom
        ));
        self.suppress_until = Instant::now() + Duration::from_millis(400);
        self.save_state();
    }

    fn request_layout(&mut self) {
        if Instant::now() < self.suppress_until {
            self.relayout_pending = true;
        } else {
            self.layout(false);
        }
    }

    fn set_fullscreen(&mut self, on: bool) {
        self.fullscreen = on;
        for b in &self.bars {
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

    fn eval(&self, edge: Edge, payload: &serde_json::Value) {
        if let Some(b) = self.bar(edge) {
            let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
        }
    }

    /// Beépített ikon neve, vagy képfájl esetén a betöltési címe.
    fn icon_for(&self, icon: &str) -> (String, String) {
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

    fn send_init(&mut self, idx: usize) {
        let edge = self.bars[idx].edge;
        let c = &self.cfg;
        // A bal sáv a "left" helyű appokat kapja, a jobb panel a "right" helyűeket
        // (ezek az Indítás gomb alá kerülnek). A sorszám mindig a config szerinti marad.
        let apps: Vec<serde_json::Value> = if edge == Edge::Left || edge == Edge::Right {
            c.apps
                .iter()
                .enumerate()
                .filter(|(_, a)| (a.place.eq_ignore_ascii_case("right")) == (edge == Edge::Right))
                .map(|(i, a)| {
                    let (icon, img) = self.icon_for(&a.icon);
                    json!({
                        "id": i,
                        "name": a.name,
                        "icon": icon,
                        "img": img,
                        "color": a.color,
                        "hint": format!("{} {}", a.resolved_path(), a.match_exe.join(" ")).to_lowercase(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        // A sávok valós mérete CSS-pixelben (a kerekítés miatt eltérhet a beállítottól).
        let size = |e: Edge| -> f64 {
            self.bar(e)
                .filter(|b| b.registered)
                .map(|b| match e {
                    Edge::Left | Edge::Right => (b.rect.right - b.rect.left) as f64,
                    _ => (b.rect.bottom - b.rect.top) as f64,
                })
                .unwrap_or(0.0)
                / self.scale
        };
        let payload = json!({
            "type": "init",
            "part": edge.name(),
            "theme": c.theme,
            "geometry": { "l": size(Edge::Left), "t": size(Edge::Top), "r": size(Edge::Right), "b": size(Edge::Bottom) },
            "apps": apps,
            "activity": c.activity,
            "claudeUsage": c.claude_usage,
            "autostart": autostart::is_enabled(),
            "weather": c.weather,
        });
        self.eval(edge, &payload);
        if edge == Edge::Left {
            self.last_state.clear();
            self.poll();
        }
        if edge == Edge::Right {
            if let Some(u) = &self.last_usage {
                if let Some(b) = self.bar(Edge::Right) {
                    let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({u})"));
                }
            }
        }
    }

    /// A háttérszál eredménye (állapotsor vagy közvetlen lekérés).
    fn on_usage(&mut self, payload: String) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
            log(format!(
                "Claude-használat (háttér): {} {}",
                v["status"].as_str().unwrap_or("?"),
                v["message"].as_str().unwrap_or("")
            ));
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
        } else {
            json!({ "type": "usage", "status": "login", "plan": "" }).to_string()
        };
        if self.last_usage.as_deref() != Some(payload.as_str()) {
            if let Some(b) = self.bar(Edge::Right) {
                let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({payload})"));
            }
            self.last_usage = Some(payload);
        }
    }

    fn claude_fetch(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            c.next_fetch = Instant::now() + Duration::from_secs(5 * 60);
            let _ = c.webview.evaluate_script(CLAUDE_FETCH_JS);
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
    fn on_claude_loaded(&mut self, url: &str) {
        if url.starts_with("https://claude.ai/") {
            self.claude_fetch();
        }
    }

    fn on_claude_web(&mut self, raw: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let status = v["status"].as_str().unwrap_or("");
        if status == "skip" {
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
                        c.ok = Some((Instant::now(), payload.to_string()));
                    }
                }
                if self.claude.as_ref().is_some_and(|c| c.visible) {
                    self.claude_hide();
                }
            }
            "login" => {
                if let Some(c) = self.claude.as_mut() {
                    c.ok = None;
                }
            }
            _ => {}
        }
        self.publish_usage();
    }

    /// Az aktivitásjelző lejárt (vagy nyugtázták): a többi sáv is jelezzen.
    fn on_alert(&mut self, on: bool) {
        log(format!("aktivitás-figyelmeztetés: {}", if on { "be" } else { "ki" }));
        for e in [Edge::Left, Edge::Top, Edge::Bottom] {
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
        self.eval(Edge::Right, &json!({ "type": "toast", "text": text }));
    }

    fn set_autostart(&self, on: bool) {
        match autostart::set(on) {
            Ok(()) => log(format!("indítás a Windowszal: {}", if on { "be" } else { "ki" })),
            Err(e) => {
                log(format!("indítás a Windowszal – hiba: {e}"));
                self.toast(&e);
            }
        }
        self.eval(Edge::Right, &json!({ "type": "autostart", "on": autostart::is_enabled() }));
    }

    fn on_ipc(&mut self, idx: usize, raw: &str) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else {
            return;
        };
        let kind = msg["type"].as_str().unwrap_or("");
        if kind != "ready" && kind != "log" {
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
                let id = msg["id"].as_u64().unwrap_or(u64::MAX) as usize;
                let new_instance = msg["newInstance"].as_bool().unwrap_or(false);
                self.launch(id, new_instance);
            }
            "cmd" => match msg["name"].as_str().unwrap_or("") {
                "reload" => self.reload(),
                "config" => self.open_config(),
                "exit" => self.exit_requested = true,
                "autostart" => self.set_autostart(msg["on"].as_bool().unwrap_or(false)),
                "claude-login" => self.claude_show(),
                // a Windows saját Start menüje és keresője (Win, illetve Win+S)
                "start" => wintrack::press_keys(&[wintrack::VK_WIN]),
                "search" => wintrack::press_keys(&[wintrack::VK_WIN, b'S']),
                _ => {}
            },
            "launch-extra" => self.launch_extra(&msg),
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
    fn on_drag(&mut self, phase: &str, x: i32, y: i32, paths: Vec<String>) {
        if !paths.is_empty() {
            self.drag_paths = paths;
        }
        self.eval(Edge::Left, &json!({ "type": "drag", "phase": phase, "x": x, "y": y }));
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
            _ => self.toast("Ezzel a programmal nem nyitható meg."),
        }
    }

    fn launch(&mut self, id: usize, new_instance: bool) {
        let Some(app) = self.cfg.apps.get(id).cloned() else {
            return;
        };
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
            self.toast(&format!("Nem indítható: {}", app.name));
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
            extras.push(json!({ "key": k, "name": name, "icon": icon }));
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
            "active": active,
            "extras": extras,
            "activeExtra": active_extra,
        })
        .to_string();
        if state != self.last_state {
            self.last_state = state;
            // a jobb panel is megkapja (az ott lévő appok futás-jelzéséhez)
            for e in [Edge::Left, Edge::Right] {
                if let Some(b) = self.bar(e) {
                    let _ = b.webview.evaluate_script(&format!("window.hub&&window.hub.receive({})", self.last_state));
                }
            }
        }
    }

    fn reload(&mut self) {
        match config::load(&self.cfg_path) {
            Ok(c) => {
                self.cfg = c;
                self.matchers = self.cfg.apps.iter().map(Matcher::new).collect();
                log("konfiguráció újratöltve");
            }
            Err(e) => {
                log(&e);
                self.toast(&e);
                return;
            }
        }
        self.layout(true);
        for b in &self.bars {
            let _ = b.webview.reload();
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

    if let Ok(f) = std::fs::File::create(data_dir().join("cyberhub.log")) {
        *LOG.lock().unwrap() = Some(f);
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
    let cfg = match config::load(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            log(&e);
            let c = default_config();
            if !cfg_path.exists() {
                let _ = std::fs::write(&cfg_path, serde_json::to_string_pretty(&c).unwrap());
            }
            c
        }
    };
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

    for (idx, edge) in EDGES.iter().enumerate() {
        let window = WindowBuilder::new()
            .with_title(format!("{APP_NAME} {}", edge.name()))
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
            .build(&event_loop)
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

        let url = format!(
            "cyberhub://localhost/hub.html?part={}&l={}&t={}&r={}&b={}",
            edge.name(),
            cfg.left_width,
            cfg.frame.top,
            cfg.right_width,
            cfg.frame.bottom
        );
        let bg = (6u8, 7u8, 11u8, 255u8);
        let dir = ui_dir.clone();
        let page_edge = edge.name();
        let is_left = *edge == Edge::Left;
        // Fájlhúzás: a bal sávon az ikonokhoz kötjük, a többi sávon csak elnyeljük,
        // különben a ráejtett fájl a sáv helyén nyílna meg.
        let drag_handler = move |ev: wry::DragDropEvent| -> bool {
            if is_left {
                let to_strings = |p: Vec<PathBuf>| p.into_iter().map(|x| x.to_string_lossy().to_string()).collect();
                match ev {
                    wry::DragDropEvent::Enter { paths, position } => {
                        send(UserEvent::Drag("over", position.0, position.1, to_strings(paths)))
                    }
                    wry::DragDropEvent::Over { position } => {
                        send(UserEvent::Drag("over", position.0, position.1, Vec::new()))
                    }
                    wry::DragDropEvent::Drop { paths, position } => {
                        send(UserEvent::Drag("drop", position.0, position.1, to_strings(paths)))
                    }
                    wry::DragDropEvent::Leave => send(UserEvent::Drag("leave", 0, 0, Vec::new())),
                    _ => {}
                }
            }
            true
        };
        let webview = WebViewBuilder::new_with_web_context(&mut web_context)
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

        bars.push(Bar { edge: *edge, window, webview, hwnd, rect: RECT::default(), registered: false });
    }

    // claude.ai ablak: rejtve indul, csak belépéskor jelenik meg. A sütik a CyberHub saját
    // böngészőprofiljában maradnak, az asztali Claude apphoz nem nyúlunk.
    let claude = if cfg.claude_usage {
        let (mon, _) = appbar::primary_monitor();
        let window = WindowBuilder::new()
            .with_title("CyberHub – belépés a claude.ai-ra")
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
            Some(ClaudeWeb { window, webview, visible: false, next_fetch: Instant::now() + Duration::from_secs(300), ok: None })
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
        fullscreen: false,
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
    };
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
    event_loop.run(move |event, _target, control_flow| {
        let _keep = &web_context;
        match event {
            Event::UserEvent(ev) => match ev {
                UserEvent::Ipc(idx, msg) => hub.on_ipc(idx, &msg),
                UserEvent::Reposition => hub.request_layout(),
                UserEvent::ShellRestarted => hub.on_shell_restarted(),
                UserEvent::Fullscreen(on) => hub.set_fullscreen(on),
                UserEvent::Hotkey(HOTKEY_EXIT) => hub.exit_requested = true,
                UserEvent::Hotkey(HOTKEY_RELOAD) => hub.reload(),
                UserEvent::Hotkey(_) => {}
                UserEvent::Usage(payload) => hub.on_usage(payload),
                UserEvent::Drag(phase, x, y, paths) => hub.on_drag(phase, x, y, paths),
                UserEvent::ClaudeWeb(raw) => hub.on_claude_web(&raw),
                UserEvent::ClaudeLoaded(url) => hub.on_claude_loaded(&url),
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
        *control_flow = ControlFlow::WaitUntil(next_poll);
    });
}
