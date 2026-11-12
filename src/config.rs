use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// A felület nyelve: "hu", "en" vagy "auto" (alapérték: a Windows nyelve szerint – magyar
    /// Windowson magyar, minden másnál angol). A beállításokban választott nyelv felülírja.
    pub language: String,
    /// Igaz, ha a fájlban "auto" áll: ilyenkor a `language` a Windows nyelvéből számolt érték,
    /// mentéskor viszont "auto" kerül vissza a fájlba.
    #[serde(skip)]
    pub language_auto: bool,
    /// Induljon-e a CyberShell a Windowsba való bejelentkezéskor. Ha hiányzik a fájlból,
    /// a Windows aktuális beállítását vesszük át.
    pub start_with_windows: Option<bool>,
    /// Elrejtse-e a Windows tálcáját, amíg a CyberShell fut.
    pub hide_taskbar: bool,
    /// A bal oldali ikonsáv szélessége (logikai pixel).
    pub left_width: u32,
    /// A jobb oldali panel szélessége (logikai pixel).
    pub right_width: u32,
    /// Régi (v1.2-es) beállítás: felcserélt panelek. Betöltéskor átírjuk az `iconsEdge` és
    /// `infoEdge` értékre, a fájlba már nem kerül vissza.
    #[serde(skip_serializing)]
    pub swap_sides: bool,
    /// Az ikonsáv helye: "left", "right", "top" vagy "bottom".
    pub icons_edge: String,
    /// Az információs panel helye (nem lehet ugyanott, ahol az ikonsáv).
    pub info_edge: String,
    /// Minden képernyőn megjelenjen-e a hub (különben csak az elsődlegesen).
    pub all_monitors: bool,
    pub frame: Frame,
    pub activity: Activity,
    /// Mutassa-e a Claude 5 órás és heti használatát.
    pub claude_usage: bool,
    pub weather: Weather,
    /// A dátumra kattintva megnyíló naptárban mutatott naptárak (iCal-címek).
    pub calendar: Calendar,
    /// A jobb panel szekcióinak láthatósága.
    pub sections: Sections,
    /// Megjelenés: "dark" (Cyberpunk 2077), "light" (Ghost in the Shell) vagy "auto"
    /// (a Windows világos/sötét módját követi).
    pub appearance: String,
    /// A ténylegesen használt séma ("dark" vagy "light"); "auto" esetén a Windowsból számolva.
    #[serde(skip)]
    pub scheme: String,
    /// A sötét téma színei.
    pub theme: Theme,
    /// A világos téma színei.
    pub theme_light: Theme,
    /// Egészség: szünetre figyelmeztető visszaszámláló.
    pub health: Health,
    /// A saját Start menü és kereső.
    pub start_menu: StartMenu,
    /// A beállításokban választott háttérkép teljes útvonala. Ha a fájlban átírod,
    /// újratöltéskor a CyberShell beállítja a Windows háttérképének.
    pub wallpaper: String,
    pub apps: Vec<AppEntry>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Frame {
    pub top: u32,
    pub bottom: u32,
    /// Oldalsó vékony keret (ha valamelyik panel fent vagy lent van, és oldalt keret marad).
    pub left: u32,
    pub right: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Sections {
    pub claude: bool,
    pub activity: bool,
    pub clock: bool,
    pub date: bool,
    /// Az időjárás szekció (a lekérést a `weather.enabled` kapcsolja ki).
    pub weather: bool,
    /// A zene (Spotify) szekció.
    pub music: bool,
    /// A hangerő szekció.
    pub volume: bool,
    /// A szekciók sorrendje az információs panelen (a Keresés/Indítás és a Főkapcsoló között;
    /// a laptop szekció mindig az időjárás előtt van).
    pub order: Vec<String>,
}

pub const SECTION_KEYS: [&str; 7] = ["claude", "activity", "date", "clock", "weather", "music", "volume"];

impl Default for Sections {
    fn default() -> Self {
        Sections {
            claude: true,
            activity: true,
            clock: true,
            date: true,
            weather: true,
            music: true,
            volume: true,
            // alapból az idő a dátum alatt van, alul az időjárás, a zene és a hangerő
            order: SECTION_KEYS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Sections {
    /// Érvényes sorrend: minden kulcs pontosan egyszer (a hiányzók a végére kerülnek).
    pub fn normalize(&mut self) {
        let mut order: Vec<String> = Vec::new();
        for k in &self.order {
            if SECTION_KEYS.contains(&k.as_str()) && !order.contains(k) {
                order.push(k.clone());
            }
        }
        for k in SECTION_KEYS {
            if !order.iter().any(|o| o == k) {
                order.push(k.to_string());
            }
        }
        self.order = order;
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct StartMenu {
    /// A Win billentyű a CyberShell Start menüjét, a Win+S a keresőjét nyitja meg
    /// (különben a Windows sajátját).
    pub win_key: bool,
    /// A Start menübe kitűzött programok azonosítói (a menü jobb gombos „Kitűzés” parancsa írja).
    pub pinned: Vec<String>,
    /// Webes keresés: a {q} helyére kerül a keresett szöveg.
    pub web_search: String,
    /// Vágólap-előzmények a keresőben (csak a memóriában, legfeljebb 30 szöveg).
    pub clipboard: bool,
}

impl Default for StartMenu {
    fn default() -> Self {
        StartMenu { win_key: true, pinned: Vec::new(), web_search: "https://www.google.com/search?q={q}".into(), clipboard: true }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Weather {
    pub enabled: bool,
    /// A település neve (ebből keresi meg a koordinátákat).
    pub city: String,
    /// Ha megadod, a város helyett ezt használja.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

impl Default for Weather {
    fn default() -> Self {
        Weather { enabled: true, city: "Budapest".into(), latitude: None, longitude: None }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Calendar {
    pub feeds: Vec<CalendarFeed>,
}

/// Egy csatolt naptár, pl. a Google Naptár „Titkos cím iCal formátumban” címe.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarFeed {
    pub name: String,
    pub url: String,
    pub color: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Activity {
    /// A visszaszámláló hossza percben.
    pub minutes: u32,
    /// A program indulásakor magától elkezdjen-e számolni.
    pub auto_start: bool,
    /// Szóljon-e hanggal, amikor lejár.
    pub sound: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    pub main: String,
    pub alt: String,
    pub hot: String,
    pub bg: String,
    /// A szöveg alapszíne.
    pub text: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Health {
    /// Be van-e kapcsolva a szünetre figyelmeztetés.
    pub enabled: bool,
    /// Ennyi perc után jelez (5–480).
    pub minutes: u32,
}

impl Default for Health {
    fn default() -> Self {
        Health { enabled: false, minutes: 90 }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppEntry {
    pub name: String,
    /// Exe elérési útja, mappa vagy URI (pl. "spotify:"). %VÁLTOZÓK% használhatók.
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub args: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    /// Beépített ikon neve (browser, code, folder…), "exe" (a program saját ikonja) vagy képfájl.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Exe-fájlnevek, amelyek ablakait ehhez az ikonhoz soroljuk.
    #[serde(rename = "match", skip_serializing_if = "Vec::is_empty")]
    pub match_exe: Vec<String>,
    /// Ablakcím-részlet, ha az exe alapján nem azonosítható (pl. Store-appok).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub match_title: String,
    /// Az újonnan indított ablakot teljes méretre nyissa-e.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximize: Option<bool>,
    /// Az ikon finom színe (pl. "#ff7139"), ami a programra utal.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Hol jelenjen meg: "left" (alapból, bal sáv) vagy "right" (jobb panel, az Indítás alatt).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub place: String,
    /// A géppel együtt induljon-e (a CyberShell indítja el bejelentkezéskor).
    #[serde(skip_serializing_if = "is_false")]
    pub autostart: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Default for Config {
    fn default() -> Self {
        Config {
            language: "auto".into(),
            language_auto: false,
            start_with_windows: None,
            hide_taskbar: true,
            left_width: 86,
            right_width: 86,
            swap_sides: false,
            icons_edge: "left".into(),
            info_edge: "right".into(),
            all_monitors: true,
            frame: Frame::default(),
            activity: Activity::default(),
            claude_usage: true,
            weather: Weather::default(),
            calendar: Calendar::default(),
            sections: Sections::default(),
            appearance: "dark".into(),
            scheme: "dark".into(),
            theme: Theme::default(),
            theme_light: Theme::light(),
            health: Health::default(),
            start_menu: StartMenu::default(),
            wallpaper: String::new(),
            apps: Vec::new(),
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame { top: 10, bottom: 10, left: 10, right: 10 }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Activity { minutes: 20, auto_start: false, sound: true }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme { main: "#f3e600".into(), alt: "#00e1ff".into(), hot: "#ff2a6d".into(), bg: "#06070b".into(), text: "#c9ccd6".into() }
    }
}

impl Theme {
    /// Világos téma (Ghost in the Shell): hűvös, papírfehér háttér, türkiz és acélkék
    /// vonalak, narancsvörös figyelmeztetés, sötét tintaszínű szöveg.
    pub fn light() -> Self {
        Theme { main: "#00848a".into(), alt: "#2c62a6".into(), hot: "#d6452a".into(), bg: "#e3e9ec".into(), text: "#18262d".into() }
    }
}

impl AppEntry {
    pub fn resolved_path(&self) -> String {
        expand_env(&self.path)
    }

    pub fn maximize(&self) -> bool {
        self.maximize.unwrap_or(true)
    }

    pub fn on_right(&self) -> bool {
        self.place.eq_ignore_ascii_case("right")
    }

    /// Kisbetűs exe-nevek, amelyekhez az ablakokat párosítjuk.
    pub fn exe_names(&self) -> Vec<String> {
        if !self.match_exe.is_empty() {
            return self.match_exe.iter().map(|s| s.to_lowercase()).collect();
        }
        let p = self.resolved_path();
        let lower = p.to_lowercase();
        if lower.ends_with(".exe")
            && let Some(name) = Path::new(&p).file_name()
        {
            return vec![name.to_string_lossy().to_lowercase()];
        }
        Vec::new()
    }
}

/// %VÁLTOZÓ% hivatkozások feloldása környezeti változókból.
pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('%') {
            let name = &after[..end];
            match std::env::var(name) {
                Ok(v) if !name.is_empty() => out.push_str(&v),
                _ => {
                    out.push('%');
                    out.push_str(name);
                    out.push('%');
                }
            }
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

pub fn load(path: &Path) -> Result<Config, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Nem olvasható: {} ({e})", path.display()))?;
    // A BOM-ot (Jegyzettömb menti így) eldobjuk.
    let text = text.trim_start_matches('\u{feff}');
    let mut cfg: Config = serde_json::from_str(text).map_err(|e| format!("Hibás config.json: {e}"))?;
    cfg.sections.normalize();
    cfg.resolve_language();
    cfg.normalize_edges();
    cfg.health.minutes = cfg.health.minutes.clamp(5, 480);
    cfg.resolve_scheme();
    Ok(cfg)
}

pub const EDGE_NAMES: [&str; 4] = ["left", "right", "top", "bottom"];

impl Config {
    /// A panelek helye: érvényes, egymástól eltérő szélek (a régi „felcserélt panelek”
    /// beállítást is átváltja).
    pub fn normalize_edges(&mut self) {
        if self.swap_sides {
            self.swap_sides = false;
            self.icons_edge = "right".into();
            self.info_edge = "left".into();
        }
        if !EDGE_NAMES.contains(&self.icons_edge.as_str()) {
            self.icons_edge = "left".into();
        }
        if !EDGE_NAMES.contains(&self.info_edge.as_str()) || self.info_edge == self.icons_edge {
            self.info_edge = if self.icons_edge == "right" { "left".into() } else { "right".into() };
        }
    }

    /// A megjelenés feloldása: "auto" esetén napszak szerint (napnyugtától napkeltéig sötét).
    /// Igazat ad, ha a séma megváltozott.
    pub fn resolve_scheme(&mut self) -> bool {
        let scheme = match self.appearance.as_str() {
            "light" => "light",
            "auto" => {
                let geo = self.weather.latitude.zip(self.weather.longitude);
                if crate::sys::is_night(geo) { "dark" } else { "light" }
            }
            _ => {
                self.appearance = "dark".into();
                "dark"
            }
        };
        let changed = self.scheme != scheme;
        self.scheme = scheme.to_string();
        changed
    }

    /// A most használt téma színei.
    pub fn active_theme(&self) -> &Theme {
        if self.scheme == "light" { &self.theme_light } else { &self.theme }
    }

    /// A felületek háttérszíne (a WebView a betöltés előtt is ezt mutatja).
    pub fn bg_rgba(&self) -> (u8, u8, u8, u8) {
        let hex = self.active_theme().bg.trim_start_matches('#');
        let c = |i: usize| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        match (c(0), c(2), c(4)) {
            (Some(r), Some(g), Some(b)) if hex.len() >= 6 => (r, g, b, 255),
            _ => {
                if self.scheme == "light" {
                    (227, 233, 236, 255)
                } else {
                    (6, 7, 11, 255)
                }
            }
        }
    }

    /// Az "auto" (vagy ismeretlen) nyelv feloldása a Windows felületének nyelvére.
    pub fn resolve_language(&mut self) {
        match self.language.as_str() {
            "hu" | "en" => self.language_auto = false,
            _ => {
                self.language_auto = true;
                self.language = crate::sys::system_language().to_string();
            }
        }
    }
}

/// A fájlba írt alak: automatikus nyelvnél "auto" kerül bele.
fn to_json(cfg: &Config) -> serde_json::Result<serde_json::Value> {
    let mut v = serde_json::to_value(cfg)?;
    if cfg.language_auto {
        v["language"] = serde_json::Value::from("auto");
    }
    Ok(v)
}

/// Igaz, ha a fájl nem tartalmaz minden opciót (vagy eltér a rendezett formától):
/// ilyenkor érdemes visszaírni, hogy minden beállítás látsszon benne.
pub fn needs_rewrite(path: &Path, cfg: &Config) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else { return false };
    let Ok(on_disk) = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}')) else {
        return false;
    };
    if to_json(cfg).map(|v| v != on_disk).unwrap_or(false) {
        return true;
    }
    // ugyanaz a tartalom, de más sorrendben vagy formában (pl. kézi szerkesztés után)
    let normalize = |t: &str| t.trim_start_matches('\u{feff}').replace("\r\n", "\n").trim().to_string();
    to_json(cfg).and_then(|v| serde_json::to_string_pretty(&v)).map(|pretty| normalize(&pretty) != normalize(&text)).unwrap_or(false)
}

/// Mentés: előbb ideiglenes fájlba, aztán átnevezés, hogy félbeszakadt írás ne rontsa el.
pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    let text = to_json(cfg).and_then(|v| serde_json::to_string_pretty(&v)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Nem menthető: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Nem menthető: {e}"))
}
