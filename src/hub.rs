//! A program központi állapota (`Hub`): a sávok, a felugró ablakok és a háttérszálak összefogása.
//! A metódusok témák szerint az alábbi almodulokban vannak; itt az alapok: felületüzenetek,
//! a sávok indulása, időzítés, a sávok parancsai, a futó programok figyelése és az újratöltés.

pub(crate) mod claude;
pub(crate) mod flyout;
pub(crate) mod health;
pub(crate) mod icons;
pub(crate) mod layout;
pub(crate) mod net;
pub(crate) mod panel;
pub(crate) mod preview;
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
use crate::hub::flyout::FlyWin;
use crate::hub::health::HEALTH_IDLE;
use crate::hub::health::HealthWin;
use crate::hub::layout::Bar;
use crate::hub::layout::ICONS_EDGE;
use crate::hub::layout::bar_url;
use crate::hub::layout::edge_index;
use crate::hub::layout::insets_of;
use crate::hub::layout::part_for;
use crate::hub::preview::PeekWin;
use crate::hub::settings::SettingsWin;
use crate::hub::start::LaunchWin;
use crate::log;
use crate::state_path;
use crate::web::fly_url;
use crate::web::health_url;
use crate::web::launcher_url;
use crate::web::nav_url;
use crate::web::net_url;
use crate::web::peek_url;
use crate::web::percent_encode;
use crate::web::push;
use crate::wintrack::Matcher;
use crate::wintrack::Tracker;
use crate::wintrack::WinInfo;
use crate::{appbar, apps, autostart, clipboard, config, keyhook, media, sys, wintrack};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub(crate) struct Pending {
    pub(crate) app: usize,
    pub(crate) known: HashSet<isize>,
    pub(crate) deadline: Instant,
}

pub(crate) struct Hub {
    pub(crate) cfg: Config,
    pub(crate) cfg_path: PathBuf,
    pub(crate) matchers: Vec<Matcher>,
    pub(crate) bars: Vec<Bar>,
    pub(crate) tracker: Tracker,
    pub(crate) last_fg: Option<HWND>,
    pub(crate) last_state: String,
    pub(crate) pending: Option<Pending>,
    /// Nyitott beállítások mellett indított program: az első új ablakáig figyeljük, és csak
    /// akkor zárjuk be a beállításokat, ha az ablak ugyanazon a képernyőn jelenik meg.
    /// (A már ismert ablakok azonosítói és a figyelés határideje.)
    pub(crate) settings_watch: Option<(HashSet<isize>, Instant)>,
    /// Hány képernyőn kellene sávcsoport (ha eltér a meglévőtől, a fő ciklus újraépíti).
    pub(crate) want_groups: usize,
    /// Ha mi rejtettük el a tálcát: az eredeti állapota.
    pub(crate) orig_taskbar: Option<u32>,
    pub(crate) want_hidden_taskbar: bool,
    /// Azok a képernyők, ahol épp teljes képernyős program fut.
    pub(crate) fullscreen: HashSet<usize>,
    pub(crate) scale: f64,
    /// Az elsődleges képernyő munkaterülete a sávok nélkül (a legutóbbi elrendezés szerint).
    pub(crate) work: RECT,
    pub(crate) suppress_until: Instant,
    pub(crate) relayout_pending: bool,
    pub(crate) exit_requested: bool,
    pub(crate) shut_down: bool,
    /// A legutóbbi Claude-használati adat (újratöltéskor azonnal visszaküldjük).
    pub(crate) last_usage: Option<String>,
    pub(crate) usage_wake: Option<Sender<()>>,
    /// Az éppen húzott fájlok (ha egy ikonra ejtik őket, azzal a programmal nyitjuk meg).
    pub(crate) drag_paths: Vec<String>,
    /// Nem kitűzött, de futó programok: megjelenítési sorrend és nevek.
    pub(crate) extra_order: Vec<String>,
    pub(crate) extra_names: HashMap<String, String>,
    pub(crate) claude: Option<ClaudeWeb>,
    /// A háttérszál (állapotsor, közvetlen lekérés) legutóbbi eredménye.
    pub(crate) thread_usage: Option<serde_json::Value>,
    pub(crate) settings: Option<SettingsWin>,
    /// Az appok telepítettsége (a config sorrendjében), indításkor és újratöltéskor ellenőrizve.
    pub(crate) available: Vec<bool>,
    /// A program indulásának ideje: a felület ebből tudja, hogy új munkamenet kezdődött-e.
    pub(crate) session: u64,
    /// Az előző processzoridő-mérés és a következő terhelésküldés ideje (a beállításokhoz).
    pub(crate) cpu_prev: Option<(u64, u64)>,
    pub(crate) next_sys: Instant,
    /// Bejelentkezéskor indítandó programok (sorszám) és a következő indítás ideje.
    pub(crate) launch_queue: std::collections::VecDeque<usize>,
    pub(crate) next_launch: Instant,
    pub(crate) peek: Option<PeekWin>,
    /// Az ablakok első észlelésének sorrendje (az előnézet ebben a sorrendben mutatja őket).
    pub(crate) seen: HashMap<isize, u64>,
    pub(crate) seen_seq: u64,
    pub(crate) fly: Option<FlyWin>,
    /// Az akkumulátor legutóbb elküldött állapota és a következő ellenőrzés ideje.
    pub(crate) last_power: String,
    pub(crate) next_power: Instant,
    /// A hangerő-vezérlő (percenként újranyitjuk, ha közben másik eszköz lett az alapértelmezett),
    /// és a felületnek legutóbb küldött állapot.
    pub(crate) volume: Option<(Instant, sys::Volume)>,
    pub(crate) last_volume: String,
    pub(crate) next_volume: Instant,
    /// A naptárak legutóbbi letöltése (ideje és tartalma), illetve folyamatban van-e.
    pub(crate) cal_cache: Option<(Instant, String)>,
    pub(crate) cal_busy: bool,
    pub(crate) launcher: Option<LaunchWin>,
    /// Sávonként az Indítás és a Keresés gomb közepe a sáv mentén (CSS-pixel).
    pub(crate) quick_along: HashMap<(usize, &'static str), f64>,
    /// Melyik képernyő információs paneljén nyitották meg a beállításokat, illetve a Start
    /// menüt / keresőt: a gomb csak ott világít.
    pub(crate) settings_bar: Option<usize>,
    pub(crate) launcher_bar: Option<usize>,
    /// A telepített programok (a Start menünek), a felsorolás ideje, és hogy folyamatban van-e.
    pub(crate) start_apps: Vec<apps::StartApp>,
    pub(crate) apps_at: Option<Instant>,
    pub(crate) apps_busy: bool,
    pub(crate) health: Option<HealthWin>,
    /// Mikor jár le az Egészség visszaszámlálója (kikapcsolva: nincs), és mikor néztük a tétlenséget.
    pub(crate) health_due: Option<Instant>,
    pub(crate) next_idle_check: Instant,
    /// A megjelenés („auto”) következő ellenőrzése.
    pub(crate) next_scheme_check: Instant,
    /// Parancsok a Spotify-szálnak, és a legutóbbi állapot (újratöltéskor azonnal visszaküldjük).
    pub(crate) media_tx: Option<Sender<media::Cmd>>,
    pub(crate) last_media: Option<String>,
    /// Neonkeret: az ablakok, amelyek keretét átszíneztük (és a beállított szín).
    pub(crate) borders: HashMap<isize, u32>,
    /// A neonkeret-réteg által követett ablak (az utolsó előtérbe került programablak).
    pub(crate) neon_target: Option<isize>,
    /// A gép alkatrészei (a beállítások első megnyitásakor kérdezzük le), és hogy folyamatban van-e.
    pub(crate) hardware: Option<serde_json::Value>,
    pub(crate) hardware_busy: bool,
    /// Internetkapcsolat: a piros sáv ablaka, az egymást követő sikertelen mérések, épp
    /// nincs-e kapcsolat, elrejtették-e a sávot, és a következő mérés ideje.
    pub(crate) net: Option<HealthWin>,
    pub(crate) net_misses: u8,
    pub(crate) net_offline: bool,
    pub(crate) net_dismissed: bool,
    pub(crate) next_net_check: Instant,
    /// A Codex-használat legutóbb elküldött állapota és a következő olvasás ideje.
    pub(crate) last_codex: Option<String>,
    pub(crate) next_codex: Instant,
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

impl Hub {
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

    /// Üzenet egyetlen sáv felületének.
    pub(crate) fn eval_bar(&self, idx: usize, payload: &str) {
        if let Some(b) = self.bars.get(idx) {
            push(&b.webview, payload);
        }
    }

    /// Üzenet minden képernyő adott szélű sávjának.
    pub(crate) fn eval(&self, edge: Edge, payload: &serde_json::Value) {
        for b in self.bars.iter().filter(|b| b.edge == edge) {
            push(&b.webview, payload);
        }
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

    /// A felület nyelvének megfelelő szöveg.
    pub(crate) fn tr(&self, hu: &str, en: &str) -> String {
        if self.cfg.language == "en" { en.to_string() } else { hu.to_string() }
    }

    pub(crate) fn check_available(&mut self) {
        self.available = self.cfg.apps.iter().map(sys::app_available).collect();
        // Ami nincs a megadott helyén (pl. egy másik gépről átmásolt config.json), azt
        // megpróbáljuk megtalálni: más mappa, más verzió, Store-app helyett asztali vagy fordítva.
        // A megtalált útvonal bekerül a config.json-ba.
        if self.available.iter().any(|ok| !ok) {
            let start = crate::apps::enumerate();
            let mut fixed = 0;
            for i in 0..self.cfg.apps.len() {
                if self.available[i] {
                    continue;
                }
                let app = self.cfg.apps[i].clone();
                if let Some(path) = sys::locate_app(&app, &start) {
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
        for (a, ok) in self.cfg.apps.iter().zip(&self.available) {
            if !ok {
                log(format!("nincs telepítve, elrejtve: {} ({})", a.name, a.path));
            }
        }
    }

    /// Beépített ikon neve, vagy képfájl esetén a betöltési címe. Az "exe" a program saját ikonja.
    pub(crate) fn icon_for(&self, icon: &str, app_path: &str) -> (String, String) {
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
            "iconLabels": c.icon_labels,
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
            if let Some(m) = &self.last_media {
                self.eval_bar(idx, m);
            }
            if let Some(u) = &self.last_usage {
                self.eval_bar(idx, u);
            }
            if let Some(c) = &self.last_codex {
                self.eval_bar(idx, c);
            }
            // újratöltés (pl. panelcsere) után is látszódjon, hogy a beállítások nyitva vannak
            if self.settings.as_ref().is_some_and(|s| s.visible) && self.settings_bar.is_none_or(|b| b == idx) {
                self.eval_bar(idx, &json!({ "type": "settings-open", "on": true }).to_string());
            }
        }
    }

    /// Rövid értesítés a jobb panel alján (a menü fölött).
    pub(crate) fn toast(&self, text: &str) {
        self.eval(self.info_edge(), &json!({ "type": "toast", "text": text }));
        self.settings_eval(&json!({ "type": "toast", "text": text }));
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
        self.eval(self.info_edge(), &json!({ "type": "autostart", "on": autostart::is_enabled() }));
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

    /// Időzített teendők a fő ciklusban (az előnézet bezárása, újrarendezése).
    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        // a részletes időjárás bezárul, ha máshova kattintanak (elveszti a fókuszt)
        if let Some(f) = &self.fly
            && f.visible
            && now.duration_since(f.shown_at) > Duration::from_millis(400)
        {
            let fg = unsafe { GetForegroundWindow() };
            if fg != f.hwnd {
                self.fly_hide(true);
            }
        }
        // a Start menü és a kereső is bezárul, ha ugyanazon a képernyőn máshova kattintanak
        // (a másik képernyőn tett kattintás vagy ott előjövő program nyitva hagyja)
        if let Some(l) = &self.launcher
            && l.visible
            && now.duration_since(l.shown_at) > Duration::from_millis(400)
        {
            let fg = unsafe { GetForegroundWindow() };
            if self.launcher_lost_to(fg) {
                self.launcher_hide(true);
            }
        }
        // Egészség: lejárt a visszaszámlálás, vagy hosszabb szünet volt (akkor elölről indul)
        if self.health_due.is_some_and(|d| now >= d) && !self.health.as_ref().is_some_and(|h| h.visible) {
            self.health_show();
        }
        if now >= self.next_idle_check {
            self.next_idle_check = now + Duration::from_secs(5);
            if self.health_due.is_some() && !self.health.as_ref().is_some_and(|h| h.visible) && sys::idle_time() >= HEALTH_IDLE {
                let left = self.health_due.map(|d| d.saturating_duration_since(now)).unwrap_or_default();
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
        if now >= self.next_net_check {
            self.next_net_check = now + Duration::from_secs(3);
            self.net_check();
        }
        if self.cfg.sections.codex && now >= self.next_codex {
            self.codex_send(false);
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
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        let fly = self.fly.as_ref().filter(|f| f.visible).map(|_| Instant::now() + Duration::from_millis(150));
        let launcher = self.launcher.as_ref().filter(|l| l.visible).map(|_| Instant::now() + Duration::from_millis(150));
        let pk = self.peek.as_ref();
        let health = self.health_due.filter(|_| !self.health.as_ref().is_some_and(|h| h.visible));
        [fly, launcher, health, pk.and_then(|p| p.hide_at), pk.and_then(|p| p.refresh_at)].into_iter().flatten().min()
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
        if !matches!(kind, "ready" | "log" | "peek" | "peek-leave" | "quick-pos" | "geo") {
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
                    if self.settings.as_ref().is_some_and(|s| s.visible) {
                        self.settings_eval(&json!({ "type": "hide" }));
                    } else {
                        self.settings_bar = Some(idx);
                        self.settings_show(true);
                    }
                }
                "power" => {
                    let action = msg["action"].as_str().unwrap_or("").to_string();
                    self.power(&action);
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
                // Eszköztár: Számológép, Jegyzettömb, Képmetsző (a Keresés a "search" parancs)
                "tool" => {
                    self.settings_dismiss();
                    let target = match msg["tool"].as_str().unwrap_or("") {
                        "calc" => "calc.exe",
                        "notepad" => "notepad.exe",
                        "snip" => r"shell:AppsFolder\Microsoft.ScreenSketch_8wekyb3d8bbwe!App",
                        _ => "",
                    };
                    if !target.is_empty() && !wintrack::shell_open(target, "", "", false) {
                        log(format!("eszköz nem indult: {target}"));
                    }
                }
                // laptop: a Windows saját Wi-Fi-listája, az akkumulátor, a hálózat és a Bluetooth
                // beállításai
                "wifi" | "battery" | "network" | "bluetooth" => {
                    let uri = match msg["name"].as_str().unwrap_or("") {
                        "wifi" => "ms-availablenetworks:",
                        "battery" => "ms-settings:batterysaver",
                        "network" => "ms-settings:network-status",
                        _ => "ms-settings:bluetooth",
                    };
                    self.settings_dismiss();
                    wintrack::shell_open(uri, "", "", false);
                }
                _ => {}
            },
            // a Keresés és Indítás gomb helye (a Win billentyűvel nyitott menü is mellé kerüljön)
            "quick-pos" => {
                for mode in ["start", "search"] {
                    if let Some(a) = msg[mode].as_f64() {
                        self.quick_along.insert((idx, if mode == "start" { "start" } else { "search" }), a);
                    }
                }
            }
            "weather-open" => self.fly_open(idx, "weather", &msg),
            "calendar-open" => self.fly_open(idx, "calendar", &msg),
            "app-menu" => self.app_menu(idx, &msg),
            "launch-extra" => {
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
            // zene: a Spotify vezérlése
            "media" => self.media_cmd(&msg),
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
            "codex-refresh" => self.codex_send(true),
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

    pub(crate) fn poll(&mut self) {
        if self.bars.is_empty() {
            return;
        }
        let own = self.own_hwnds();
        let fg = unsafe { GetForegroundWindow() };
        if !fg.is_invalid() && !own.contains(&fg) {
            self.last_fg = Some(wintrack::root_owner(fg));
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
            let live: HashSet<isize> = wins.iter().map(|w| w.hwnd.0 as isize).collect();
            self.seen.retain(|k, _| live.contains(k));
        }

        if let Some((known, deadline)) = &self.settings_watch {
            if Instant::now() > *deadline || !self.settings_visible() {
                self.settings_watch = None;
            } else if let Some(w) = wins.iter().find(|w| !known.contains(&(w.hwnd.0 as isize)) && !w.iconic) {
                let h = w.hwnd;
                self.settings_watch = None;
                if self.on_settings_monitor(h) {
                    self.settings_dismiss();
                }
            }
        }

        if let Some(p) = &self.pending {
            if Instant::now() > p.deadline {
                self.pending = None;
            } else if let Some(w) = wins
                .iter()
                .find(|w| !p.known.contains(&(w.hwnd.0 as isize)) && w.maximizable && !w.iconic && self.matchers[p.app].matches(w))
            {
                wintrack::maximize(w.hwnd);
                self.pending = None;
            }
        }

        let running: Vec<usize> =
            self.matchers.iter().enumerate().filter(|(_, m)| wins.iter().any(|w| m.matches(w))).map(|(i, _)| i).collect();
        // több ablakos programok: ezek fölött az ikon előnézetet mutat
        let multi: Vec<usize> =
            self.matchers.iter().enumerate().filter(|(_, m)| wins.iter().filter(|w| m.matches(w)).count() >= 2).map(|(i, _)| i).collect();
        let fg_win = self.last_fg.and_then(|h| wins.iter().find(|w| w.hwnd == h)).filter(|w| !w.iconic);
        let active = fg_win.and_then(|w| self.matchers.iter().position(|m| m.matches(w)));

        // Nem kitűzött, de futó programok, a megjelenésük sorrendjében.
        let mut present: Vec<(String, &WinInfo)> = Vec::new();
        for w in &wins {
            if self.matchers.iter().any(|m| m.matches(w)) {
                continue;
            }
            if let Some(k) = extra_key(w)
                && !present.iter().any(|(pk, _)| *pk == k)
            {
                present.push((k, w));
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
                let name = self.program_name(&w.path);
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
            for b in self.bars.iter().filter(|b| b.part == "left") {
                push(&b.webview, &self.last_state);
            }
        }
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
        if let Some(h) = &self.health {
            let _ = h.webview.load_url(&nav_url(&health_url(&self.cfg)));
        }
        if let Some(n) = &self.net {
            let _ = n.webview.load_url(&nav_url(&net_url(&self.cfg)));
        }
        self.last_codex = None;
        if self.cfg.health.enabled != self.health_due.is_some() {
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
        if self.settings.as_ref().is_some_and(|s| s.visible) {
            self.settings_show(false);
        }
    }

    pub(crate) fn open_config(&self) {
        let p = self.cfg_path.to_string_lossy().to_string();
        if !wintrack::shell_open(&p, "", "", false) {
            wintrack::shell_open("notepad.exe", &format!("\"{p}\""), "", false);
        }
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
