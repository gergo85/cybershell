//! A program központi állapota (`Hub`): a sávok, a felugró ablakok és a háttérszálak összefogása.
//! A metódusok témák szerint az alábbi almodulokban vannak; itt az alapok: felületüzenetek,
//! a sávok indulása, időzítés, a sávok parancsai, a futó programok figyelése és az újratöltés.

pub(crate) mod claude;
pub(crate) mod deskmenu;
pub(crate) mod flyout;
pub(crate) mod health;
pub(crate) mod icons;
pub(crate) mod layout;
pub(crate) mod net;
pub(crate) mod panel;
pub(crate) mod preview;
pub(crate) mod recent;
pub(crate) mod settings;
pub(crate) mod start;
pub(crate) mod sync;
pub(crate) mod theme;

use crate::CLEANUP;
use crate::HOTKEY_EXIT;
use crate::HOTKEY_RELOAD;
use crate::appbar::Edge;
use crate::config::Config;
use crate::hub::claude::ClaudeWeb;
use crate::hub::deskmenu::DeskWin;
use crate::hub::flyout::CalendarCache;
use crate::hub::flyout::FlyWin;
use crate::hub::health::HEALTH_IDLE;
use crate::hub::health::{BannerWin, HealthTimer};
use crate::hub::icons::LogonLaunch;
use crate::hub::layout::Bar;
use crate::hub::layout::ICONS_EDGE;
use crate::hub::layout::bar_url;
use crate::hub::layout::edge_index;
use crate::hub::layout::insets_of;
use crate::hub::layout::part_for;
use crate::hub::net::NetWatch;
use crate::hub::panel::Feed;
use crate::hub::preview::PeekWin;
use crate::hub::settings::SettingsWin;
use crate::hub::start::LaunchWin;
use crate::log;
use crate::state_path;
use crate::web::desk_url;
use crate::web::fly_url;
use crate::web::health_url;
use crate::web::launcher_url;
use crate::web::nav_url;
use crate::web::net_url;
use crate::web::peek_url;
use crate::web::push;
use crate::web::{app_icon_url, exe_icon_url, file_url, percent_encode};
use crate::wintrack::Matcher;
use crate::wintrack::Tracker;
use crate::wintrack::WinInfo;
use crate::{appbar, apps, autostart, clipboard, config, keyhook, media, sys, wintrack};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

/// Egy indítás utáni figyelés: az indításkor már meglévő ablakok azonosítói, és meddig várunk
/// a program első új ablakára.
pub(crate) struct WindowWatch {
    known: HashSet<isize>,
    deadline: Instant,
}

impl WindowWatch {
    pub(crate) fn new(known: HashSet<isize>, wait: Duration) -> Self {
        WindowWatch { known, deadline: Instant::now() + wait }
    }

    pub(crate) fn expired(&self) -> bool {
        Instant::now() > self.deadline
    }

    /// Az első új (az indításkor még nem létező), nem kis méretre tett ablak.
    pub(crate) fn first_new<'a>(&self, wins: &'a [WinInfo]) -> Option<&'a WinInfo> {
        self.first_new_where(wins, |_| true)
    }

    /// Mint a `first_new`, de csak az `ok` feltételnek megfelelő ablakok közül.
    pub(crate) fn first_new_where<'a>(&self, wins: &'a [WinInfo], ok: impl Fn(&WinInfo) -> bool) -> Option<&'a WinInfo> {
        wins.iter().find(|w| !self.known.contains(&(w.hwnd.0 as isize)) && !w.iconic && ok(w))
    }
}

/// Az ablakok azonosítói (az indítás előtt meglévők a `WindowWatch`-nak).
pub(crate) fn window_ids(wins: &[WinInfo]) -> HashSet<isize> {
    wins.iter().map(|w| w.hwnd.0 as isize).collect()
}

/// Teljes méretre nyitandó, most indított program (a config sorszáma) és az első új ablakának figyelése.
pub(crate) struct PendingMaximize {
    pub(crate) app: usize,
    pub(crate) watch: WindowWatch,
}

/// A program teljes állapota. Csak a fő szálon él (a háttérszálak `UserEvent`-tel jelentkeznek).
/// A mezők témák szerint szakaszokba rendezve; a hozzájuk tartozó metódusok a `hub/` almoduljaiban.
pub(crate) struct Hub {
    // ── beállítás ──
    pub(crate) cfg: Config,
    pub(crate) cfg_path: PathBuf,
    /// Az Ikonok programjainak ablakpárosítása (a config sorrendjében).
    pub(crate) matchers: Vec<Matcher>,
    /// Az appok telepítettsége (a config sorrendjében), indításkor és újratöltéskor ellenőrizve.
    pub(crate) available: Vec<bool>,
    /// A program indulásának ideje: a felület ebből tudja, hogy új munkamenet kezdődött-e.
    pub(crate) session: u64,

    // ── életciklus ──
    pub(crate) exit_requested: bool,
    pub(crate) shut_down: bool,

    // ── sávok és képernyők (hub/layout.rs) ──
    pub(crate) bars: Vec<Bar>,
    /// Hány képernyőn kellene sávcsoport (ha eltér a meglévőtől, a fő ciklus újraépíti).
    pub(crate) want_groups: usize,
    /// Az elsődleges képernyő DPI-szorzója.
    pub(crate) scale: f64,
    /// Az elsődleges képernyő munkaterülete a sávok nélkül (a legutóbbi elrendezés szerint).
    pub(crate) work: RECT,
    /// Azok a képernyők, ahol épp teljes képernyős program fut.
    pub(crate) fullscreen: HashSet<usize>,
    /// Egy elrendezés után ennyi ideig nem rendezünk újra (a saját változásunk visszhangja ellen),
    /// közben csak megjegyezzük, hogy kell.
    pub(crate) suppress_until: Instant,
    pub(crate) relayout_pending: bool,
    /// Ha mi rejtettük el a tálcát: az eredeti állapota.
    pub(crate) orig_taskbar: Option<u32>,
    pub(crate) want_hidden_taskbar: bool,

    // ── futó programok és ablakok (hub/icons.rs, `poll`) ──
    pub(crate) tracker: Tracker,
    /// A legutóbb előtérben lévő programablak.
    pub(crate) last_fg: Option<HWND>,
    /// Képernyőnként (a sávcsoport sorszáma szerint) az ott legutóbb előtérbe került ablak: az
    /// Ikonok ezt jelölik aktívnak az adott képernyőn.
    pub(crate) fg_by_mon: HashMap<usize, HWND>,
    /// Bezárt felugró ablak: utána a képernyője legfelső programja kapja vissza a fókuszt
    /// (`refocus_later`), legkorábban a megadott időpontban.
    pub(crate) refocus: Option<(HWND, Instant)>,
    /// Az Ikonoknak legutóbb küldött állapot (képernyőnként egy-egy JSON, sorban).
    pub(crate) last_state: String,
    /// Nem kitűzött, de futó programok: megjelenítési sorrend és nevek.
    pub(crate) extra_order: Vec<String>,
    pub(crate) extra_names: HashMap<String, String>,
    /// Az ablakok első észlelésének sorrendje (az előnézet ebben a sorrendben mutatja őket).
    pub(crate) seen: HashMap<isize, u64>,
    pub(crate) seen_seq: u64,
    /// Az éppen húzott fájlok (ha egy ikonra ejtik őket, azzal a programmal nyitjuk meg).
    pub(crate) drag_paths: Vec<String>,

    // ── indítások figyelése ──
    /// Teljes méretre nyitandó, most indított program.
    pub(crate) pending_maximize: Option<PendingMaximize>,
    /// Nyitott beállítások mellett indított program: az első új ablakáig figyeljük, és csak
    /// akkor zárjuk be a beállításokat, ha az ablak ugyanazon a képernyőn jelenik meg.
    pub(crate) settings_watch: Option<WindowWatch>,
    /// A Start menüből vagy a keresőből indított program: az első új ablakát előre hozzuk, ha
    /// a Windows a többi mögött nyitotta meg.
    pub(crate) launch_watch: Option<WindowWatch>,
    /// Bejelentkezéskor a „géppel induló” programok egyenkénti indítása.
    pub(crate) logon_launch: LogonLaunch,

    // ── felugró ablakok (mind rejtve indul; amelyik nem jött létre, az `None`) ──
    pub(crate) settings: Option<SettingsWin>,
    pub(crate) peek: Option<PeekWin>,
    pub(crate) fly: Option<FlyWin>,
    pub(crate) launcher: Option<LaunchWin>,
    pub(crate) health: Option<BannerWin>,
    /// A „Megszakadt a kapcsolat” sáv.
    pub(crate) net: Option<BannerWin>,
    /// A saját asztali menü.
    pub(crate) desk: Option<DeskWin>,
    pub(crate) claude: Option<ClaudeWeb>,
    /// Melyik képernyő információs paneljén nyitották meg a beállításokat, illetve a Start
    /// menüt / keresőt: a gomb csak ott világít.
    pub(crate) settings_bar: Option<usize>,
    pub(crate) launcher_bar: Option<usize>,
    /// Sávonként az Indítás és a Keresés gomb közepe a sáv mentén (CSS-pixel).
    pub(crate) quick_along: HashMap<(usize, &'static str), f64>,

    // ── Start menü (hub/start.rs) ──
    /// A telepített programok, a felsorolás ideje, és hogy folyamatban van-e.
    pub(crate) start_apps: Vec<apps::StartApp>,
    pub(crate) apps_at: Option<Instant>,
    pub(crate) apps_busy: bool,
    /// A kereső „Gyakran használt” programjai (Start menü-azonosítók): a Mostanában modul is ezt mutatja.
    pub(crate) recent_apps: Vec<String>,

    // ── az információs panel moduljainak adatai (hub/panel.rs) ──
    // A felületnek legutóbb küldött üzenetet mindegyiknél megtartjuk: változatlan üzenetet nem
    // küldünk újra, a panel újratöltésekor (`send_init`) pedig ezt kapja vissza. Az időzítve
    // frissülő adatoknál ez egy `Feed` (benne a következő frissítés ideje is); az eseményre
    // érkezőknél (Claude, Spotify, Mostanában) egy `last_*` mező.
    pub(crate) power_feed: Feed,
    /// A hangerő-vezérlő (percenként újranyitjuk, ha közben másik eszköz lett az alapértelmezett).
    pub(crate) volume: Option<(Instant, sys::Volume)>,
    pub(crate) volume_feed: Feed,
    pub(crate) tray_feed: Feed,
    pub(crate) codex_feed: Feed,
    pub(crate) periph_feed: Feed,
    pub(crate) last_recent: String,
    /// Parancsok a Spotify-szálnak.
    pub(crate) media_tx: Option<Sender<media::Cmd>>,
    pub(crate) last_media: Option<String>,
    /// A Claude-használat: a háttérszál ébresztője és legutóbbi eredménye, és a panelnek küldött üzenet.
    pub(crate) usage_wake: Option<Sender<()>>,
    pub(crate) thread_usage: Option<serde_json::Value>,
    pub(crate) last_usage: Option<String>,
    /// A csatolt naptárak letöltése (az oldalpanelnek és az Események modulnak).
    pub(crate) calendar: CalendarCache,

    // ── időzített figyelések (`tick`) ──
    /// Az Egészség visszaszámlálása.
    pub(crate) health_timer: HealthTimer,
    /// Az internetkapcsolat figyelése.
    pub(crate) net_watch: NetWatch,
    /// A megjelenés („auto”) következő ellenőrzése.
    pub(crate) next_scheme_check: Instant,

    // ── a beállítások ablak adatai (hub/settings.rs) ──
    /// Az előző processzoridő-mérés és a következő terhelésküldés ideje.
    pub(crate) cpu_prev: Option<(u64, u64)>,
    pub(crate) next_sys: Instant,
    /// A gép alkatrészei (az első megnyitáskor kérdezzük le), és hogy folyamatban van-e.
    pub(crate) hardware: Option<serde_json::Value>,
    pub(crate) hardware_busy: bool,
    /// Sikertelen lekérdezés után legkorábban ekkor próbáljuk újra (ne minden beállításváltáskor).
    pub(crate) hardware_retry: Instant,

    // ── neonkeret (hub/sync.rs) ──
    /// Az ablakok, amelyek keretét átszíneztük (és a beállított szín).
    pub(crate) borders: HashMap<isize, u32>,
    /// A neonkeret-réteg által követett ablak (az utolsó előtérbe került programablak).
    pub(crate) neon_target: Option<isize>,
}

/// Ezek a rendszerfolyamatok ablakai ne jelenjenek meg a futó programok között.
pub(crate) const IGNORED_EXTRA: &[&str] =
    &["cybershell.exe", "textinputhost.exe", "shellexperiencehost.exe", "searchhost.exe", "startmenuexperiencehost.exe", "lockapp.exe"];

/// Egy nem kitűzött program csoportkulcsa: a teljes útvonal, Store-appoknál
/// (ApplicationFrameHost) az ablak maga.
pub(crate) fn extra_key(w: &WinInfo) -> Option<String> {
    if w.path.is_empty() || IGNORED_EXTRA.contains(&w.exe.as_str()) {
        return None;
    }
    if w.exe == "applicationframehost.exe" {
        return Some(format!("hwnd:{}", w.hwnd.0 as isize));
    }
    Some(w.path.to_lowercase())
}

/// A felugró ablakok (mind rejtve indul; amelyik nem jött létre, az `None`).
pub(crate) struct Popups {
    pub(crate) settings: Option<SettingsWin>,
    pub(crate) peek: Option<PeekWin>,
    pub(crate) fly: Option<FlyWin>,
    pub(crate) launcher: Option<LaunchWin>,
    pub(crate) health: Option<BannerWin>,
    pub(crate) net: Option<BannerWin>,
    pub(crate) desk: Option<DeskWin>,
    pub(crate) claude: Option<ClaudeWeb>,
}

/// Az indulás körülményei (a main.rs állítja össze, a `Hub::new` kapja meg).
pub(crate) struct Startup {
    /// Az indításkor előtérben lévő ablak: ez lesz az első „aktív” program.
    pub(crate) foreground: Option<HWND>,
    /// Ha induláskor mi rejtettük el a tálcát: az eredeti állapota.
    pub(crate) orig_taskbar: Option<u32>,
    /// Rejtve kell-e tartani a tálcát (a beállítás szerint, ha a --keep-taskbar nem tiltja).
    pub(crate) hide_taskbar: bool,
    /// A Claude-használatot lekérdező háttérszál ébresztője (ha a funkció be van kapcsolva).
    pub(crate) usage_wake: Option<Sender<()>>,
    /// Az indulás Unix-ideje (másodperc).
    pub(crate) session: u64,
}

impl Hub {
    /// Az induló állapot: a beállítás, a sávok, a felugró ablakok és az indulás körülményei;
    /// minden más alapállapotban (üres gyorsítótárak, azonnal esedékes időzítők).
    pub(crate) fn new(cfg: Config, cfg_path: PathBuf, bars: Vec<Bar>, popups: Popups, start: Startup) -> Hub {
        let now = Instant::now();
        let Popups { settings, peek, fly, launcher, health, net, desk, claude } = popups;
        Hub {
            // beállítás
            matchers: cfg.apps.iter().map(Matcher::new).collect(),
            cfg,
            cfg_path,
            available: Vec::new(),
            session: start.session,
            // életciklus
            exit_requested: false,
            shut_down: false,
            // sávok és képernyők
            bars,
            want_groups: 0,
            scale: 1.0,
            work: RECT::default(),
            fullscreen: HashSet::new(),
            suppress_until: now,
            relayout_pending: false,
            orig_taskbar: start.orig_taskbar,
            want_hidden_taskbar: start.hide_taskbar,
            // futó programok és ablakok
            tracker: Tracker::default(),
            last_fg: start.foreground,
            fg_by_mon: HashMap::new(),
            refocus: None,
            last_state: String::new(),
            extra_order: Vec::new(),
            extra_names: HashMap::new(),
            seen: HashMap::new(),
            seen_seq: 0,
            drag_paths: Vec::new(),
            // indítások figyelése
            pending_maximize: None,
            settings_watch: None,
            launch_watch: None,
            logon_launch: LogonLaunch::new(now),
            // felugró ablakok
            settings,
            peek,
            fly,
            launcher,
            health,
            net,
            desk,
            claude,
            settings_bar: None,
            launcher_bar: None,
            quick_along: HashMap::new(),
            // Start menü
            start_apps: Vec::new(),
            apps_at: None,
            apps_busy: false,
            recent_apps: recent::load(),
            // az információs panel moduljainak adatai (néhány kicsit később indul, hogy az
            // indulást ne lassítsa)
            power_feed: Feed::new(now),
            volume: None,
            volume_feed: Feed::new(now),
            tray_feed: Feed::new(now),
            codex_feed: Feed::new(now),
            periph_feed: Feed::new(now + Duration::from_secs(3)),
            last_recent: String::new(),
            media_tx: None,
            last_media: None,
            usage_wake: start.usage_wake,
            thread_usage: None,
            last_usage: None,
            calendar: CalendarCache::new(now + Duration::from_secs(5)),
            // időzített figyelések
            health_timer: HealthTimer::new(now),
            net_watch: NetWatch::new(now + Duration::from_secs(5)),
            next_scheme_check: now,
            // a beállítások ablak adatai
            cpu_prev: None,
            next_sys: now,
            hardware: None,
            hardware_busy: false,
            hardware_retry: now,
            // neonkeret
            borders: HashMap::new(),
            neon_target: None,
        }
    }

    /// Egy nem kitűzött program megjelenítendő neve: a Start menü (honosított) neve, ha ott
    /// megtalálható, különben a fájl leírása. Csak a programlista betöltése után tároljuk el,
    /// hogy az induláskor kapott angol leírás ne ragadjon meg.
    pub(crate) fn program_name(&mut self, path: &str) -> String {
        let key = path.to_lowercase();
        if let Some(n) = self.extra_names.get(&key) {
            return n.clone();
        }
        let name = apps::name_for_path(&self.start_apps, path).unwrap_or_else(|| wintrack::display_name(path));
        if !self.start_apps.is_empty() {
            self.extra_names.insert(key, name.clone());
        }
        name
    }

    /// Üzenet egyetlen sáv felületének (JSON-érték vagy már kész JSON-szöveg).
    pub(crate) fn eval_bar(&self, idx: usize, payload: &(impl Display + ?Sized)) {
        if let Some(b) = self.bars.get(idx) {
            push(&b.webview, payload);
        }
    }

    /// Üzenet minden képernyő adott szélű sávjának.
    pub(crate) fn eval(&self, edge: Edge, payload: &(impl Display + ?Sized)) {
        for b in self.bars.iter().filter(|b| b.edge == edge) {
            push(&b.webview, payload);
        }
    }

    /// Üzenet minden képernyő információs paneljének.
    pub(crate) fn eval_info(&self, payload: &(impl Display + ?Sized)) {
        self.eval(self.info_edge(), payload);
    }

    /// Nyitott állapot (MENÜ, Start, Időjárás, Dátum) csak annak a képernyőnek az információs
    /// paneljén, ahol rákattintottak (ha nem ismert, az elsődlegesén); a többin kikapcsolva.
    pub(crate) fn eval_info_at(&self, idx: Option<usize>, on: &serde_json::Value, off: &serde_json::Value) {
        let info = self.info_edge();
        let target = idx
            .filter(|i| self.bars.get(*i).is_some_and(|b| b.edge == info))
            .or_else(|| self.bars.iter().position(|b| b.edge == info && b.mon == 0));
        for (i, b) in self.bars.iter().enumerate().filter(|(_, b)| b.edge == info) {
            push(&b.webview, if Some(i) == target { on } else { off });
        }
    }

    /// A program saját szövege a felület nyelvén (magyar, angol, lengyel, német vagy spanyol).
    pub(crate) fn tr(&self, hu: &str, en: &str, pl: &str, de: &str, es: &str) -> String {
        match self.cfg.language.as_str() {
            "en" => en,
            "pl" => pl,
            "de" => de,
            "es" => es,
            _ => hu,
        }
        .to_string()
    }

    pub(crate) fn check_available(&mut self) {
        self.available = self.cfg.apps.iter().map(sys::app_available).collect();
        // Ami nincs a megadott helyén (pl. egy másik gépről átmásolt config.json), azt
        // megpróbáljuk megtalálni: más mappa, más verzió, Store-app helyett asztali vagy fordítva.
        // A megtalált útvonal bekerül a config.json-ba.
        if self.available.iter().any(|ok| !ok) {
            // A Start menü programjai: a háttérben már felsorolt lista, ha van. Csak induláskor
            // (amikor még nincs kész) soroljuk fel itt, mert ez a fő szálon lassú lehet.
            let enumerated;
            let start: &[apps::StartApp] = if self.start_apps.is_empty() {
                enumerated = apps::enumerate();
                &enumerated
            } else {
                &self.start_apps
            };
            let mut fixed = 0;
            for i in 0..self.cfg.apps.len() {
                if self.available[i] {
                    continue;
                }
                let app = self.cfg.apps[i].clone();
                if let Some(path) = sys::locate_app(&app, start) {
                    let mut moved = app.clone();
                    moved.path = path.clone();
                    if sys::app_available(&moved) {
                        log(format!("program megtalálva máshol: {} ({} → {path})", app.name, app.path));
                        self.cfg.apps[i].path = path;
                        self.available[i] = true;
                        fixed += 1;
                    }
                }
            }
            if fixed > 0 {
                self.matchers = self.cfg.apps.iter().map(Matcher::new).collect();
                self.save_config();
            }
        }
        // Az alap ikonlista Total Commandere nincs ezen a gépen (máshol sem találtuk): helyette a
        // Windows Fájlkezelője kerül a helyére (csak az érintetlen alapbejegyzés, és csak ha a
        // Fájlkezelő még nincs a listában).
        const TC_DEFAULT: &str = r"C:\Program Files\Total Commander\TOTALCMD64.EXE";
        let has_explorer = self.cfg.apps.iter().any(|a| a.path.eq_ignore_ascii_case("explorer.exe"));
        if !has_explorer
            && let Some(i) =
                (0..self.cfg.apps.len()).find(|&i| !self.available[i] && self.cfg.apps[i].path.eq_ignore_ascii_case(TC_DEFAULT))
        {
            let name = self.tr("Fájlkezelő", "File Explorer", "Eksplorator plików", "Explorer", "Explorador de archivos");
            log(format!("a Total Commander nincs telepítve: helyette {name}"));
            self.cfg.apps[i] = config::AppEntry {
                name,
                path: "explorer.exe".into(),
                icon: "folder".into(),
                maximize: Some(false),
                color: "#f2c94c".into(),
                autostart: self.cfg.apps[i].autostart,
                ..Default::default()
            };
            self.available[i] = true;
            self.matchers = self.cfg.apps.iter().map(Matcher::new).collect();
            self.save_config();
        }
        for (a, ok) in self.cfg.apps.iter().zip(&self.available) {
            if !ok {
                log(format!("nincs telepítve, elrejtve: {} ({})", a.name, a.path));
            }
        }
    }

    /// Egy futó program ikonjának címe a felületnek. Store-appnál a csomag ikonja (a Start menü
    /// bejegyzéséből): ezek exe-jében gyakran nincs ikon (pl. WhatsApp), így üres maradna a hely.
    /// Minden más programnál az exe saját ikonja.
    pub(crate) fn program_icon(&self, path: &str) -> String {
        match apps::app_for_path(&self.start_apps, path).filter(|a| a.target.is_empty()) {
            Some(a) => app_icon_url(&a.id),
            None => exe_icon_url(path),
        }
    }

    /// Beépített ikon neve, vagy képfájl esetén a betöltési címe. Az "exe" a program saját ikonja.
    pub(crate) fn icon_for(&self, icon: &str, app_path: &str) -> (String, String) {
        if icon == "exe" {
            return (String::new(), exe_icon_url(app_path));
        }
        let looks_like_file = icon.contains('.') || icon.contains('/') || icon.contains('\\');
        if !looks_like_file {
            return (icon.to_string(), String::new());
        }
        let p = PathBuf::from(config::expand_env(icon));
        if p.is_absolute() {
            (String::new(), file_url(&p.to_string_lossy()))
        } else {
            (String::new(), percent_encode(&icon.replace('\\', "/")))
        }
    }

    /// Az appok a felületnek: sorszám (a config szerint), név, ikon, szín.
    pub(crate) fn app_json(&self, i: usize, a: &config::AppEntry) -> serde_json::Value {
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

    pub(crate) fn send_init(&mut self, idx: usize) {
        let edge = self.bars[idx].edge;
        let part = self.bars[idx].part;
        let c = &self.cfg;
        // Minden program az ikonsávon van (a nem telepítettek kimaradnak).
        let apps: Vec<serde_json::Value> = if part == "left" {
            c.apps
                .iter()
                .enumerate()
                .filter(|(i, _)| self.available.get(*i).copied().unwrap_or(true))
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
        let (ia, ib) = insets_of(c, edge, size);
        let payload = json!({
            "type": "init",
            "part": edge.name(),
            "theme": self.theme_json(),
            "geometry": { "l": size(Edge::Left), "t": size(Edge::Top), "r": size(Edge::Right), "b": size(Edge::Bottom), "ia": ia, "ib": ib },
            "apps": apps,
            "activity": c.activity,
            "claudeUsage": c.claude_usage,
            "sections": c.sections,
            "session": self.session,
            "autostart": autostart::is_enabled(),
            "weather": c.weather,
            "currency": c.currency,
            "note": c.note,
            "iconLabels": c.icon_labels,
            "language": c.language,
        });
        self.eval_bar(idx, &payload);
        if part == "left" {
            self.last_state.clear();
            self.poll();
        }
        if part == "right" {
            self.send_power(true);
            self.send_volume(true);
            self.send_tray(true);
            self.send_recent(true);
            if let Some(m) = &self.last_media {
                self.eval_bar(idx, m);
            }
            if let Some(u) = &self.last_usage {
                self.eval_bar(idx, u);
            }
            if !self.codex_feed.sent.is_empty() {
                self.eval_bar(idx, &self.codex_feed.sent);
            }
            if !self.periph_feed.sent.is_empty() {
                self.eval_bar(idx, &self.periph_feed.sent);
            }
            // az Események modulnak a legutóbb letöltött naptárak (vagy hogy nincs csatolt naptár)
            if let Some((_, data)) = &self.calendar.cache {
                let feeds = serde_json::from_str::<serde_json::Value>(data).unwrap_or_default();
                self.eval_bar(idx, &json!({ "type": "cal-data", "feeds": feeds }));
            } else if self.cfg.calendar.feeds.is_empty() {
                self.eval_bar(idx, &json!({ "type": "cal-data", "feeds": [] }));
            }
            // újratöltés (pl. panelcsere) után is látszódjon, hogy a beállítások nyitva vannak
            if self.settings_visible() && self.settings_bar.is_none_or(|b| b == idx) {
                self.eval_bar(idx, &json!({ "type": "settings-open", "on": true }));
            }
        }
    }

    /// Rövid értesítés a jobb panel alján (a menü fölött).
    pub(crate) fn toast(&self, text: &str) {
        self.eval_info(&json!({ "type": "toast", "text": text }));
        self.settings_toast(text);
    }

    pub(crate) fn set_autostart(&mut self, on: bool) {
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
        self.eval_info(&json!({ "type": "autostart", "on": autostart::is_enabled() }));
        self.send_settings();
    }

    pub(crate) fn save_config(&mut self) -> bool {
        match config::save(&self.cfg_path, &self.cfg) {
            Ok(()) => true,
            Err(e) => {
                log(&e);
                self.toast(&e);
                false
            }
        }
    }

    /// Időzített teendők a fő ciklusban: a felugró ablakok bezárása, ha elvesztik a fókuszt, az
    /// Egészség és az Auto téma, a kapcsolat, a panel élő adatai (Codex, akkumulátor, hangerő)
    /// és az ablakelőnézet.
    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        // A felugró ablakok bezárulnak, ha máshova kattintanak (elvesztik a fókuszt). A figyelés
        // attól kezdve él, hogy az ablak megkapta a fókuszt (a Windows ezt nem azonnal adja át, és
        // addig nem zárhatjuk be); ha valamiért nem kapja meg, 400 ms után akkor is.
        let fg = unsafe { GetForegroundWindow() };
        // a részletes időjárás, a naptár, az események és a Notesz
        if let Some(f) = self.fly.as_mut()
            && f.visible
        {
            if fg == f.hwnd {
                f.focused = true;
            }
            if (f.focused || now.duration_since(f.shown_at) > Duration::from_millis(400)) && fg != f.hwnd {
                self.fly_hide(true);
            }
        }
        // a Start menü és a kereső is bezárul, ha ugyanazon a képernyőn máshova kattintanak
        // (a másik képernyőn tett kattintás vagy ott előjövő program nyitva hagyja)
        if let Some(l) = self.launcher.as_mut()
            && l.visible
        {
            if fg == l.hwnd {
                l.focused = true;
            }
            if (l.focused || now.duration_since(l.shown_at) > Duration::from_millis(400)) && self.launcher_lost_to(fg) {
                self.launcher_hide(true);
            }
        }
        // a saját asztali menü
        self.desk_watch_focus(fg, now);
        // Egészség: lejárt a visszaszámlálás, vagy hosszabb szünet volt (akkor elölről indul)
        if self.health_timer.due.is_some_and(|d| now >= d) && !self.health_visible() {
            self.health_show();
        }
        if now >= self.health_timer.next_idle_check {
            self.health_timer.next_idle_check = now + Duration::from_secs(5);
            if self.health_timer.due.is_some() && !self.health_visible() && sys::idle_time() >= HEALTH_IDLE {
                let left = self.health_timer.due.map(|d| d.saturating_duration_since(now)).unwrap_or_default();
                let full = Duration::from_secs(self.cfg.health.minutes as u64 * 60);
                // csak ha már ment belőle valamennyi (különben minden 5 másodpercben újraindulna)
                if full.saturating_sub(left) > Duration::from_secs(30) {
                    log("egészség: hosszabb szünet volt, a visszaszámlálás elölről indul");
                    self.health_restart();
                }
            }
        }
        // „Auto” megjelenés: napkeltekor világosra, napnyugtakor sötétre vált
        if now >= self.next_scheme_check {
            self.next_scheme_check = now + Duration::from_secs(30);
            if matches!(self.cfg.appearance.as_str(), "auto" | "windows") && self.cfg.resolve_scheme() {
                log(format!("megjelenés ({}): {}", self.cfg.appearance, self.cfg.scheme));
                self.push_theme();
            }
        }
        if now >= self.net_watch.next_check {
            self.net_watch.next_check = now + Duration::from_secs(3);
            self.net_check();
        }
        if self.cfg.sections.shows("codex") && now >= self.codex_feed.next {
            self.codex_send(false);
        }
        // Perifériák: a Bluetooth-eszközök töltöttsége félpercenként (asztali gépen és laptopon is;
        // a modul csak akkor látszik, ha van csatlakoztatott, töltöttséget jelző eszköz)
        if self.cfg.sections.shows("peripherals") && now >= self.periph_feed.next {
            self.periph_feed.next = now + Duration::from_secs(30);
            self.periph_poll();
        }
        // Események: a naptárak tízpercenként (a letöltés a gyorsítótárat is figyeli)
        if self.cfg.sections.shows("events") && now >= self.calendar.next {
            self.calendar.next = now + Duration::from_secs(10 * 60);
            self.calendar_fetch(false);
        }
        if now >= self.power_feed.next {
            self.power_feed.next = now + Duration::from_secs(20);
            self.send_power(false);
        }
        if now >= self.tray_feed.next {
            self.tray_feed.next = now + Duration::from_secs(3);
            self.send_tray(false);
        }
        // a hangerőt a billentyűzet vagy más program is állíthatja: gyakran ránézünk
        if now >= self.volume_feed.next {
            self.volume_feed.next = now + Duration::from_millis(600);
            self.send_volume(false);
        }
        self.peek_fx_tick(now);
        let Some(pk) = self.peek.as_ref() else { return };
        if pk.hide_at.is_some_and(|t| now >= t) {
            // Ha az egér valójában még az előnézet fölött van, nem zárjuk be: a kártya fölötti
            // időzéskor előtérbe hozott ablak egy pillanatra a felugró ablak fölé kerül, amit a
            // WebView elhagyásnak vesz, és álló egérnél nem jön újabb belépés. Kicsit később újra nézzük.
            if self.peek_under_cursor() {
                if let Some(pk) = self.peek.as_mut() {
                    pk.hide_at = Some(now + Duration::from_millis(200));
                }
            } else {
                self.peek_hide();
            }
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
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        let fly = self.fly.as_ref().filter(|f| f.visible).map(|_| Instant::now() + Duration::from_millis(150));
        let launcher = self.launcher.as_ref().filter(|l| l.visible).map(|_| Instant::now() + Duration::from_millis(150));
        let pk = self.peek.as_ref();
        let health = self.health_timer.due.filter(|_| !self.health_visible());
        let fx = pk.and_then(|p| p.fx_restore.iter().map(|(_, at)| *at).min());
        [fly, launcher, health, pk.and_then(|p| p.hide_at), pk.and_then(|p| p.refresh_at), fx].into_iter().flatten().min()
    }

    /// Főkapcsoló: zárolás, alvás, leállítás, újraindítás.
    pub(crate) fn power(&mut self, action: &str) {
        log(format!("főkapcsoló: {action}"));
        let ends_session = matches!(action, "shutdown" | "restart" | "signout");
        if ends_session {
            // a munkamenet véget ér: előbb mindent visszaállítunk
            self.shutdown();
        }
        match sys::power(action) {
            Ok(()) => {
                if ends_session {
                    self.exit_requested = true;
                }
            }
            Err(e) => {
                log(format!("főkapcsoló hiba: {e}"));
                self.toast(&e);
            }
        }
    }

    pub(crate) fn on_ipc(&mut self, idx: usize, raw: &str) {
        // egy közben megszűnt képernyő sávjától jövő üzenet
        if idx >= self.bars.len() {
            return;
        }
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(raw) else {
            return;
        };
        let kind = msg["type"].as_str().unwrap_or("");
        if !matches!(kind, "ready" | "log" | "peek" | "peek-leave" | "quick-pos" | "geo" | "note-set" | "tasks-set") {
            let detail = msg["name"]
                .as_str()
                .map(str::to_string)
                .or_else(|| msg["id"].as_u64().map(|i| i.to_string()))
                .or_else(|| msg["on"].as_bool().map(|b| b.to_string()))
                .unwrap_or_default();
            log(format!("parancs ({}): {kind} {detail}", self.bars[idx].edge.name()));
        }
        // az információs panel moduljainak üzenetei (hub/panel.rs)
        if self.on_panel_ipc(idx, kind, &msg) {
            return;
        }
        match kind {
            "ready" => {
                log(format!("felület betöltve: {}", self.bars[idx].edge.name()));
                self.send_init(idx);
            }
            // az Időjárás helye: ehhez igazodik az Auto téma (napkelte, napnyugta)
            "geo" => {
                if let (Some(lat), Some(lon)) = (msg["lat"].as_f64(), msg["lon"].as_f64())
                    && sys::set_geo(lat, lon)
                    && self.cfg.appearance == "auto"
                    && self.cfg.resolve_scheme()
                {
                    log(format!("megjelenés (auto): {}", self.cfg.scheme));
                    self.push_theme();
                }
            }
            // a beállítások csak akkor záródnak be, ha a program ugyanazon a képernyőn jön elő
            "launch" => {
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
                    if self.settings_visible() {
                        self.settings_eval(&json!({ "type": "hide" }));
                    } else {
                        self.settings_bar = Some(idx);
                        self.settings_show(true);
                    }
                }
                // a saját Start menü és kereső (a panel Indítás és Keresés gombja)
                "start" | "search" => {
                    let mode = if msg["name"] == "start" { "start" } else { "search" };
                    if let Some(a) = msg["along"].as_f64() {
                        self.quick_along.insert((idx, mode), a);
                    }
                    if self.cfg.start_menu.win_key {
                        self.launcher_toggle(mode, Some(idx));
                    } else {
                        // a saját Start menü ki van kapcsolva: a Windows sajátja (Win, illetve Win+S)
                        self.settings_dismiss();
                        if mode == "start" {
                            wintrack::press_keys(&[wintrack::VK_WIN]);
                        } else {
                            wintrack::press_keys(&[wintrack::VK_WIN, b'S']);
                        }
                    }
                }
                _ => {}
            },
            // a Keresés és Indítás gomb helye (a Win billentyűvel nyitott menü is mellé kerüljön)
            "quick-pos" => {
                for mode in ["start", "search"] {
                    if let Some(a) = msg[mode].as_f64() {
                        self.quick_along.insert((idx, mode), a);
                    }
                }
            }
            "app-menu" => self.app_menu(idx, &msg),
            "launch-extra" => {
                self.launch_extra(&msg);
            }
            "peek" => self.peek_open(idx, &msg),
            "peek-leave" => self.peek_leave(true),
            "drag-hover" => self.drag_hover(&msg),
            "drop-open" => self.drop_open(&msg),
            "log" => log(format!("[ui] {}", msg["text"].as_str().unwrap_or(""))),
            _ => {}
        }
    }

    pub(crate) fn poll(&mut self) {
        if self.bars.is_empty() {
            return;
        }
        self.refocus_top();
        let own = self.own_hwnds();
        let fg = unsafe { GetForegroundWindow() };
        if !fg.is_invalid() && !own.contains(&fg) {
            let root = wintrack::root_owner(fg);
            self.last_fg = Some(root);
            if let Some(mon) = self.mon_of_window(root) {
                self.fg_by_mon.insert(mon, root);
            }
        }
        let wins = self.tracker.snapshot(&own);
        self.update_borders(&wins);
        for w in &wins {
            let k = w.hwnd.0 as isize;
            if !self.seen.contains_key(&k) {
                self.seen_seq += 1;
                self.seen.insert(k, self.seen_seq);
            }
        }
        if self.seen.len() > 400 {
            let live = window_ids(&wins);
            self.seen.retain(|k, _| live.contains(k));
        }

        if let Some(watch) = &self.settings_watch {
            if watch.expired() || !self.settings_visible() {
                self.settings_watch = None;
            } else if let Some(w) = watch.first_new(&wins) {
                let h = w.hwnd;
                self.settings_watch = None;
                if self.on_settings_monitor(h) {
                    self.settings_dismiss();
                }
            }
        }

        if let Some(watch) = &self.launch_watch {
            if watch.expired() || self.launcher_visible() {
                self.launch_watch = None;
            } else if let Some(w) = watch.first_new(&wins) {
                let h = w.hwnd;
                self.launch_watch = None;
                if wintrack::root_owner(fg) != h {
                    log(format!("indított program előtérbe: {}", w.exe));
                    wintrack::activate(h);
                }
            }
        }

        if let Some(p) = &self.pending_maximize {
            if p.watch.expired() {
                self.pending_maximize = None;
            } else if let Some(w) = p.watch.first_new_where(&wins, |w| w.maximizable && self.matchers[p.app].matches(w)) {
                wintrack::maximize(w.hwnd);
                self.pending_maximize = None;
            }
        }

        // Egy menetben: kitűzött appokként hány ablak tartozik hozzájuk (egy ablak több apphoz is
        // tartozhat), és a nem kitűzött, de futó programok a megjelenésük sorrendjében. A nem
        // kitűzött programok ablakszámába (`n`) minden azonos kulcsú ablak beleszámít.
        let keys: Vec<Option<String>> = wins.iter().map(extra_key).collect();
        let mut key_counts: HashMap<&str, usize> = HashMap::new();
        for k in keys.iter().flatten() {
            *key_counts.entry(k.as_str()).or_default() += 1;
        }
        let mut counts = vec![0usize; self.matchers.len()];
        let mut present: Vec<(&str, &WinInfo)> = Vec::new();
        for (w, key) in wins.iter().zip(&keys) {
            let mut pinned = false;
            for (c, m) in counts.iter_mut().zip(&self.matchers) {
                if m.matches(w) {
                    *c += 1;
                    pinned = true;
                }
            }
            if !pinned
                && let Some(k) = key
                && !present.iter().any(|(pk, _)| pk == k)
            {
                present.push((k, w));
            }
        }
        let running: Vec<usize> = (0..counts.len()).filter(|&i| counts[i] > 0).collect();
        // több ablakos programok: ezek fölött az ikon előnézetet mutat
        let multi: Vec<usize> = (0..counts.len()).filter(|&i| counts[i] >= 2).collect();

        self.extra_order.retain(|k| present.iter().any(|(pk, _)| pk == k));
        for (k, _) in &present {
            if !self.extra_order.iter().any(|o| o == k) {
                self.extra_order.push(k.to_string());
            }
        }
        // (a sorrendet a ciklus idejére kivesszük: a programnév lekérése módosítja a Hub-ot)
        let order = std::mem::take(&mut self.extra_order);
        let mut extras = Vec::new();
        for k in &order {
            let Some((_, w)) = present.iter().find(|(pk, _)| pk == k) else { continue };
            let (name, icon) = if k.starts_with("hwnd:") {
                (w.title_raw.clone(), String::new())
            } else {
                let name = self.program_name(&w.path);
                (name, self.program_icon(&w.path))
            };
            let n = key_counts.get(k.as_str()).copied().unwrap_or(0);
            extras.push(json!({ "key": k, "name": name, "icon": icon, "n": n }));
        }
        self.extra_order = order;

        // A tálca időnként visszajön (pl. Start menü után) – ilyenkor újra elrejtjük,
        // de nem, amíg a Start menü vagy a keresés van előtérben.
        if self.want_hidden_taskbar && appbar::any_taskbar_visible() {
            let cls = if fg.is_invalid() { String::new() } else { wintrack::class_name(fg) };
            if cls != "Windows.UI.Core.CoreWindow" && cls != "Shell_TrayWnd" && cls != "XamlExplorerHostIslandWindow" {
                appbar::rehide_taskbar();
            }
        }

        // bejelentkezéskor a „géppel induló” programok egyenként, kis szünetekkel indulnak
        if !self.logon_launch.queue.is_empty() && Instant::now() >= self.logon_launch.next {
            if let Some(id) = self.logon_launch.queue.pop_front() {
                let running = counts.get(id).is_some_and(|&c| c > 0);
                if !running {
                    log(format!("automatikus programindítás: {}", self.cfg.apps.get(id).map(|a| a.name.as_str()).unwrap_or("?")));
                    self.launch(id, false);
                }
            }
            self.logon_launch.next = Instant::now() + Duration::from_millis(1500);
        }

        // nyitott beállításoknál másodpercenként processzor- és memóriaterhelés
        if self.settings_visible() && Instant::now() >= self.next_sys {
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

        // A futó programok jelölése minden képernyőn ugyanaz; az aktív program képernyőnként a
        // saját: az ott legutóbb előtérbe került, még ott lévő ablak. Egy képernyőcsoportnál
        // (csak az elsődlegesen vannak sávok) a mindenkori előtérben lévő ablak.
        let groups: Vec<usize> = {
            let mut g: Vec<usize> = self.bars.iter().filter(|b| b.part == "left").map(|b| b.mon).collect();
            g.dedup();
            g
        };
        let single = groups.len() <= 1;
        let states: Vec<(usize, String)> = groups
            .iter()
            .map(|&mon| {
                let fg_h = if single { self.last_fg } else { self.fg_by_mon.get(&mon).copied() };
                let fg_win = fg_h
                    .and_then(|h| wins.iter().find(|w| w.hwnd == h))
                    .filter(|w| !w.iconic && (single || self.mon_of_window(w.hwnd) == Some(mon)));
                let active = fg_win.and_then(|w| self.matchers.iter().position(|m| m.matches(w)));
                let active_extra = fg_win.filter(|_| active.is_none()).and_then(extra_key);
                let state = json!({
                    "type": "state",
                    "running": running,
                    "multi": multi,
                    "active": active,
                    "extras": extras,
                    "activeExtra": active_extra,
                });
                (mon, state.to_string())
            })
            .collect();
        let all = states.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
        if all != self.last_state {
            self.last_state = all;
            for b in self.bars.iter().filter(|b| b.part == "left") {
                if let Some((_, s)) = states.iter().find(|(m, _)| *m == b.mon) {
                    push(&b.webview, s);
                }
            }
        }
    }

    /// Egy saját felugró ablak (beállítások, Start menü, kereső, időjárás) bezárása után az adott
    /// képernyő legfelső programablaka kapja vissza a fókuszt, ahogy a Windows is teszi egy ablak
    /// bezárásakor; így a sávon ismét az ő ikonja aktív. Kicsit később, a következő `poll`-ban
    /// történik, hogy egy közben megnyíló másik felugró ablakot (pl. a Start menüből a
    /// beállításokat) vagy egy közben előtérbe került programot ne zavarjon.
    pub(crate) fn refocus_later(&mut self, popup: HWND) {
        self.refocus = Some((popup, Instant::now() + Duration::from_millis(120)));
    }

    fn refocus_top(&mut self) {
        let Some((popup, due)) = self.refocus else { return };
        if Instant::now() < due {
            return;
        }
        self.refocus = None;
        let popup_open = self.settings_visible() || self.launcher_visible() || self.fly_visible();
        if popup_open {
            return;
        }
        let own = self.own_hwnds();
        let fg = unsafe { GetForegroundWindow() };
        let wins = self.tracker.snapshot(&own);
        if !fg.is_invalid() && wins.iter().any(|w| w.hwnd == wintrack::root_owner(fg)) {
            return;
        }
        let Some(screen) = appbar::monitor_rect_of_window(popup) else { return };
        if let Some(w) = wins.iter().find(|w| !w.iconic && appbar::monitor_rect_of_window(w.hwnd) == Some(screen)) {
            wintrack::activate(w.hwnd);
        }
    }

    /// Az indítás előtt már meglévő ablakok (ezek közül nem az új program ablaka).
    pub(crate) fn known_windows(&mut self) -> HashSet<isize> {
        let own = self.own_hwnds();
        window_ids(&self.tracker.snapshot(&own))
    }

    /// Melyik sávcsoport képernyőjén van az ablak (a sávok helye szerint).
    pub(crate) fn mon_of_window(&self, hwnd: HWND) -> Option<usize> {
        let r = appbar::monitor_rect_of_window(hwnd)?;
        self.bars
            .iter()
            .find(|b| {
                let (cx, cy) = ((b.rect.left + b.rect.right) / 2, (b.rect.top + b.rect.bottom) / 2);
                cx >= r.left && cx < r.right && cy >= r.top && cy < r.bottom
            })
            .map(|b| b.mon)
    }

    pub(crate) fn reload(&mut self) -> bool {
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
    pub(crate) fn apply_config(&mut self, old_wallpaper: Option<String>) {
        self.peek_hide();
        self.fly_hide(false);
        self.matchers = self.cfg.apps.iter().map(Matcher::new).collect();
        self.check_available();
        self.configure_monitor();
        ICONS_EDGE.store(edge_index(self.icons_edge()), Ordering::Relaxed);
        self.layout(true);
        // ha a panelek helye változott, a sávok más felületet töltenek be; a címben a nyelv és
        // az elrendezés is benne van, ezért mindig újratöltjük a címet
        for b in &mut self.bars {
            b.part = part_for(b.edge, &self.cfg);
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
        self.launcher_hide(false);
        if let Some(l) = &self.launcher {
            let _ = l.webview.load_url(&nav_url(&launcher_url(&self.cfg)));
        }
        keyhook::set_enabled(self.cfg.start_menu.win_key);
        clipboard::set_enabled(self.cfg.start_menu.clipboard);
        self.desk_hide(false);
        crate::deskmenu::set_enabled(self.cfg.desktop_menu, self.cfg.desktop_icon_menu);
        if let Some(d) = &self.desk {
            let _ = d.webview.load_url(&nav_url(&desk_url(&self.cfg)));
        }
        if let Some(h) = &self.health {
            let _ = h.webview.load_url(&nav_url(&health_url(&self.cfg)));
        }
        if let Some(n) = &self.net {
            let _ = n.webview.load_url(&nav_url(&net_url(&self.cfg)));
        }
        self.codex_feed.sent.clear();
        if self.cfg.health.enabled != self.health_timer.due.is_some() {
            self.health_restart();
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
        if self.settings_visible() {
            self.settings_show(false);
        }
    }

    /// A config.json megnyitása szerkesztésre: a .json fájlokhoz rendelt programmal, vagy ha
    /// nincs ilyen (különben a Windows csak programválasztót adna), a Jegyzettömbbel.
    pub(crate) fn open_config(&self) {
        let p = self.cfg_path.to_string_lossy().to_string();
        let assoc = wintrack::has_open_handler(".json");
        let ok = assoc && wintrack::shell_open(&p, "", "", false);
        if !ok {
            wintrack::shell_open("notepad.exe", &format!("\"{p}\""), "", false);
        }
        log(format!("konfigurációs fájl megnyitása: {p} ({})", if ok { "a .json programjával" } else { "Jegyzettömbbel" }));
    }

    pub(crate) fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        self.reset_borders();
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
