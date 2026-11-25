#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
mod apps;
mod autostart;
mod bitmap;
mod clipboard;
mod codex;
mod com;
mod config;
mod deskmenu;
/// A beágyazott felület és gyári háttérképek (a build.rs készíti).
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}
mod hardware;
mod hub;
mod keyhook;
mod lifecycle;
mod media;
mod monitor;
mod neon;
mod notes;
mod peek;
mod periph;
mod reg;
mod sys;
mod tray;
mod usage;
mod web;
mod winsync;
mod wintrack;
mod wmi;

use crate::appbar::Edge;
use crate::hub::claude::make_claude_window;
use crate::hub::deskmenu::DeskWin;
use crate::hub::flyout::FlyWin;
use crate::hub::health::{BANNER_H, BANNER_W, BannerWin};
use crate::hub::layout::Bar;
use crate::hub::layout::EDGES;
use crate::hub::layout::ICONS_EDGE;
use crate::hub::layout::edge_index;
use crate::hub::layout::icons_edge_of;
use crate::hub::layout::make_bar;
use crate::hub::preview::PeekWin;
use crate::hub::settings::SettingsWin;
use crate::hub::start::LaunchWin;
use crate::hub::{Hub, Popups, Startup};
use crate::lifecycle::emergency_cleanup;
use crate::lifecycle::guard;
use crate::lifecycle::recover_stale;
use crate::lifecycle::statusline;
use crate::web::desk_url;
use crate::web::fly_url;
use crate::web::health_url;
use crate::web::launcher_url;
use crate::web::make_popup;
use crate::web::net_url;
use crate::web::peek_url;
use crate::web::settings_url;
use serde_json::json;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey};
use windows::Win32::UI::Shell::{ABN_FULLSCREENAPP, ABN_POSCHANGED, DefSubclassProc};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow, PostMessageW, RegisterWindowMessageW, SW_SHOWNOACTIVATE, ShowWindow, WM_DISPLAYCHANGE, WM_ENDSESSION,
    WM_HOTKEY, WM_SETTINGCHANGE,
};
use windows::core::{PCWSTR, w};
use wry::WebContext;

const POLL: Duration = Duration::from_millis(500);

const HOTKEY_EXIT: i32 = 1;

const HOTKEY_RELOAD: i32 = 2;

const APP_NAME: &str = "CyberShell";

/// A CyberShell oldala a Microsoft Store-ban (a beállítások bal alsó sarkában lévő gomb).
const STORE_URL: &str = "https://apps.microsoft.com/detail/9nw7gcgdgr71";

const GITHUB_URL: &str = "https://github.com/gergo85/cybershell";

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
    /// Az oldalpanel (időjárás, naptár, Események, Notesz) üzenete.
    FlyIpc(String),
    /// A naptárak letöltésének eredménye (háttérszálról).
    CalendarData(String),
    /// A Win billentyű (vagy Win+S): "start" vagy "search".
    WinKey(&'static str),
    /// A Start menü és kereső üzenete.
    LauncherIpc(String),
    /// A telepített programok felsorolása elkészült (háttérszálról).
    AppsReady(Vec<apps::StartApp>),
    /// A Spotify lejátszásának állapota (háttérszálról).
    Media(String),
    /// A rendszerfigyelő mérése (Terhelés, Hálózat modul).
    Monitor(String),
    /// A Bluetooth-eszközök töltöttsége (Perifériák modul, háttérszálról).
    Periph(String),
    /// A háttérprogramok felsorolása elkészült (háttérszálról).
    Tray(Vec<tray::TrayApp>),
    /// A Codex-használat kiolvasva (háttérszálról): a panel üzenete, és hogy akkor is el kell-e küldeni, ha nem változott.
    Codex(String, bool),
    /// A szünetre figyelmeztető sáv üzenete.
    HealthIpc(String),
    /// A Windows színbeállításai (sötét/világos mód, kiemelőszín) megváltoztak.
    SystemTheme,
    /// A „Megszakadt a kapcsolat” sáv üzenete.
    NetIpc(String),
    /// A gép alkatrészeinek lekérdezése elkészült (háttérszálról).
    Hardware(Option<serde_json::Value>),
    /// Új elem a vágólap-előzményekben (a figyelő szálról).
    Clipboard,
    /// Jobb kattintás az asztalon (a figyelő szálról): hely, a Shift állapota és az ikon
    /// sorszáma az ikonlistában (-1: üres terület).
    DeskClick(i32, i32, bool, i32),
    /// Az asztali menü elemei elkészültek (a menüszálról): a kérés sorszáma, az újraolvasott
    /// almenü sorszáma (`None`: az egész menü) és az elemek.
    DeskItems(u64, Option<usize>, serde_json::Value),
    /// Az asztali menü üzenete.
    DeskIpc(String),
}

static PROXY: OnceLock<Mutex<EventLoopProxy<UserEvent>>> = OnceLock::new();

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Pánik esetére: mit kell visszaállítani.
static CLEANUP: Mutex<Option<(Vec<isize>, Option<u32>)>> = Mutex::new(None);

fn log(msg: impl AsRef<str>) {
    if let Ok(mut g) = LOG.lock()
        && let Some(f) = g.as_mut()
    {
        let _ = writeln!(f, "{}", msg.as_ref());
    }
    #[cfg(debug_assertions)]
    eprintln!("{}", msg.as_ref());
}

/// A mostani idő Unix-időben, ezredmásodpercben (a felületnek és a mentett adatoknak).
fn unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// A mostani idő Unix-időben, másodpercben.
fn unix_secs() -> u64 {
    unix_ms() / 1000
}

fn send(ev: UserEvent) {
    if let Some(p) = PROXY.get()
        && let Ok(p) = p.lock()
    {
        let _ = p.send_event(ev);
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

/// A program verziója a beállítások számára („1.4.0” → „v1.4”).
fn version_label() -> String {
    let v = env!("CARGO_PKG_VERSION");
    format!("v{}", v.strip_suffix(".0").unwrap_or(v))
}

unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
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
    // a Windows sötét/világos módja vagy kiemelőszíne megváltozott
    if msg == WM_SETTINGCHANGE && lp.0 != 0 {
        let what = unsafe { PCWSTR(lp.0 as *const u16).to_string() }.unwrap_or_default();
        if what == "ImmersiveColorSet" {
            send(UserEvent::SystemTheme);
        }
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
            if let Ok(h) = FindWindowW(None, w!("CyberShell left")) {
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
        let _m = CreateMutexW(None, true, w!("Local\\CyberShell.SingleInstance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        std::mem::forget(_m);
    }

    // Az előző futás naplóját megtartjuk (cybershell.prev.log), hogy egy sikertelen
    // indítás nyoma ne vesszen el.
    // Ha a fájl épp foglalt (pl. rendszerinduláskor a vírusirtó vizsgálja), néhányszor
    // újrapróbáljuk, végső esetben időbélyeges nevű naplóba írunk – napló mindig legyen.
    let log_path = data_dir().join("cybershell.log");
    for attempt in 0..10 {
        let _ = std::fs::rename(&log_path, data_dir().join("cybershell.prev.log"));
        if let Ok(f) = std::fs::File::create(&log_path) {
            *LOG.lock().unwrap() = Some(f);
            break;
        }
        if attempt == 9 {
            let ts = crate::unix_secs();
            if let Ok(f) = std::fs::File::create(data_dir().join(format!("cybershell-{ts}.log"))) {
                *LOG.lock().unwrap() = Some(f);
            }
        } else {
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    let started_at = crate::unix_secs();
    log(format!("indulás (unix idő: {started_at}), paraméterek: {:?}, verzió: {}", &args[1..], version_label()));

    // Bejelentkezéskori indulás (a Run kulcs a --autostart kapcsolóval indít, csomagolva az
    // indítási feladat kapcsoló nélkül): megvárjuk, amíg az Intéző és a tálca elkészül, különben
    // a képernyősávok lefoglalása és a tálca elrejtése nem sikerülne.
    let at_logon = flag("--autostart") || autostart::launched_at_logon();
    if at_logon {
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
        let _ = std::process::Command::new(exe).args(["--guard", &std::process::id().to_string()]).creation_flags(DETACHED_PROCESS).spawn();
    }

    recover_stale();
    std::panic::set_hook(Box::new(|info| {
        log(format!("PÁNIK: {info}"));
        emergency_cleanup();
    }));

    let base_dir = find_base_dir();
    let ui_dir = base_dir.join("ui");
    if !ui_dir.join("hub.html").exists() {
        log("nincs ui mappa az exe mellett: a beágyazott felület töltődik be");
    }
    let (cfg, cfg_path) = lifecycle::prepare_config(&base_dir, value("--config").map(PathBuf::from));
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

    // A felugró ablakok: mind rejtve indul. A beállítások a MENÜ gombra, az ablakelőnézet a több
    // ablakos programok ikonja fölött, az oldalpanel (időjárás, naptár) a szekcióra kattintva, a
    // Start menü és a kereső a gombokra vagy a Win billentyűre, a szünet-sáv az Egészség lejártakor.
    let mut popup = |what: &str, url: String, size: (u32, u32), focusable: bool, ipc: fn(String) -> UserEvent| {
        make_popup(&event_loop, &mut web_context, &ui_dir, &cfg, what, url, size, focusable, ipc)
    };
    let banner = (BANNER_W as u32, BANNER_H as u32);
    let popups = Popups {
        settings: popup("beállítások", settings_url(&cfg), (800, 600), true, UserEvent::SettingsIpc).map(SettingsWin::new),
        peek: popup("ablakelőnézet", peek_url(&cfg), (480, 240), false, UserEvent::PeekIpc).map(PeekWin::new),
        fly: popup("oldalpanel", fly_url(&cfg), (380, 560), true, UserEvent::FlyIpc).map(FlyWin::new),
        launcher: popup("Start menü", launcher_url(&cfg), (700, 780), true, UserEvent::LauncherIpc).map(LaunchWin::new),
        health: popup("szünet", health_url(&cfg), banner, false, UserEvent::HealthIpc).map(BannerWin::new),
        net: popup("kapcsolat", net_url(&cfg), banner, false, UserEvent::NetIpc).map(BannerWin::new),
        // az asztali menü: a kattintás képernyőjére igazodik (lásd hub/deskmenu.rs)
        desk: popup("asztali menü", desk_url(&cfg), (320, 400), true, UserEvent::DeskIpc).map(DeskWin::new),
        // claude.ai ablak: rejtve indul, csak belépéskor jelenik meg
        claude: if cfg.claude_usage { make_claude_window(&event_loop, &mut web_context, &cfg) } else { None },
    };
    ICONS_EDGE.store(edge_index(icons_edge_of(&cfg)), Ordering::Relaxed);

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

    let startup = Startup { foreground: initial_fg, orig_taskbar, hide_taskbar, usage_wake, session: started_at };
    let mut hub = Hub::new(cfg, cfg_path, bars, popups, startup);
    // A telepített programok listája (a Start menünek) háttérben készül el.
    hub.apps_refresh(true);
    hub.health_restart();
    // a Windows (mód, kiemelőszín) és a Windows Terminal igazítása a témához
    hub.sync_windows();
    // Win billentyű: saját Start menü (Win) és kereső (Win+S)
    keyhook::set_enabled(hub.cfg.start_menu.win_key);
    clipboard::set_enabled(hub.cfg.start_menu.clipboard);
    clipboard::start(|| send(UserEvent::Clipboard));
    keyhook::start(|what| send(UserEvent::WinKey(what)));
    // saját asztali menü: jobb kattintás az asztal üres területén
    deskmenu::set_enabled(hub.cfg.desktop_menu, hub.cfg.desktop_icon_menu);
    deskmenu::start(
        |x, y, shift, item| send(UserEvent::DeskClick(x, y, shift, item)),
        |seq, sub, items| send(UserEvent::DeskItems(seq, sub, items)),
    );
    // Terhelés és Hálózat: a rendszerfigyelő háttérszál (csak a látható moduloknak mér)
    monitor::spawn(|v| send(UserEvent::Monitor(v.to_string())));
    hub.configure_monitor();
    // Zene: a Spotify lejátszásának figyelése háttérszálon
    {
        let (tx, rx) = std::sync::mpsc::channel::<media::Cmd>();
        media::spawn(rx, |v| send(UserEvent::Media(v.to_string())));
        hub.media_tx = Some(tx);
    }
    hub.check_available();
    // Bejelentkezéskori indulásnál a „géppel induló” programokat is elindítjuk.
    if at_logon {
        hub.logon_launch.queue = hub
            .cfg
            .apps
            .iter()
            .enumerate()
            .filter(|(i, a)| a.autostart && hub.available.get(*i).copied().unwrap_or(true))
            .map(|(i, _)| i)
            .collect();
        hub.logon_launch.next = Instant::now() + Duration::from_secs(5);
        if !hub.logon_launch.queue.is_empty() {
            log(format!("bejelentkezéskor indítandó programok: {}", hub.logon_launch.queue.len()));
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
                UserEvent::WinKey(what) => hub.launcher_toggle(what, None),
                UserEvent::LauncherIpc(raw) => hub.on_launcher_ipc(&raw),
                UserEvent::AppsReady(list) => hub.on_apps_ready(list),
                UserEvent::Media(payload) => hub.on_media(payload),
                UserEvent::Monitor(payload) => hub.on_monitor(&payload),
                UserEvent::Periph(payload) => hub.on_periph(payload),
                UserEvent::Tray(list) => hub.on_tray(list),
                UserEvent::Codex(payload, force) => hub.on_codex(payload, force),
                UserEvent::HealthIpc(raw) => hub.on_health_ipc(&raw),
                UserEvent::Clipboard => hub.send_clips(),
                UserEvent::DeskClick(x, y, shift, item) => hub.on_desk_click(x, y, shift, item),
                UserEvent::DeskItems(seq, sub, items) => hub.on_desk_items(seq, sub, items),
                UserEvent::DeskIpc(raw) => hub.on_desk_ipc(&raw),
                UserEvent::SystemTheme => hub.on_system_theme(),
                UserEvent::Hardware(v) => hub.on_hardware(v),
                UserEvent::NetIpc(raw) => hub.on_net_ipc(&raw),
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
        // életjel az asztali menü figyelőjének (ha a fő szál elakad, nem nyel el kattintást)
        deskmenu::alive();
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
