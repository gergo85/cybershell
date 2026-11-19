use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// A felület nyelve: "hu", "en", "pl" vagy "auto" (alapérték: a Windows nyelve szerint –
    /// magyar Windowson magyar, lengyelen lengyel, minden másnál angol). A beállításokban
    /// választott nyelv felülírja.
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
    /// Az információs panel helye (nem lehet ugyanott, ahol az ikonsáv). "auto" (alapérték):
    /// laptopon (akkumulátoros gépen) lent, mert oldalt a kisebb képernyőn nem fér ki minden,
    /// asztali gépen jobbra.
    pub info_edge: String,
    /// Igaz, ha a fájlban "auto" áll: a mentéskor "auto" kerül vissza.
    #[serde(skip)]
    pub info_edge_auto: bool,
    /// Minden képernyőn megjelenjen-e a hub (különben csak az elsődlegesen).
    pub all_monitors: bool,
    /// Látszódjon-e a programok neve az ikonjuk alatt.
    pub icon_labels: bool,
    /// Súgó szövegek: az elemek fölött időzve megjelenő magyarázatok (a HTML title-ök) mindenhol.
    pub tooltips: bool,
    pub frame: Frame,
    pub activity: Activity,
    /// Mutassa-e a Claude 5 órás és heti használatát.
    pub claude_usage: bool,
    pub weather: Weather,
    /// A panel Hálózat moduljának beállítása (a ping célja).
    pub network: Network,
    /// A panel Valuta moduljának beállítása (a választható pénznemek).
    pub currency: Currency,
    /// A dátumra kattintva megnyíló naptárban mutatott naptárak (iCal-címek).
    pub calendar: Calendar,
    /// A panel Jegyzet moduljának szövege.
    pub note: Note,
    /// Az információs panel moduljai: láthatóság, sorrend, nézet.
    pub sections: Sections,
    /// Megjelenés: "dark" (Cyberpunk 2077), "light" (Ghost in the Shell), "auto" (napkeltétől
    /// napnyugtáig világos, éjjel sötét) vagy "windows" (a Windows világos/sötét módját követi).
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
    /// Neonkeret más programok ablakain (Windows 11): "off", "active" (csak az aktív ablak
    /// kerete kapja a téma fő színét) vagy "all" (a többi ablaké halványabban).
    pub neon_border: String,
    /// A Windows és a Windows Terminal igazodása a CyberShell témájához.
    pub windows_sync: WindowsSync,
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

/// Az információs panel egy modulja (a felülete: ui/modules/<id>.js és .css). Új modulnál ide
/// is fel kell venni, hogy a beállításokban ki- és bekapcsolható, rendezhető legyen.
pub struct PanelModule {
    /// A modul azonosítója: a config.json `sections` kulcsa és a modul `id`-je a felületen.
    pub id: &'static str,
    /// Látszik-e alapból.
    pub default: Shown,
    /// Van-e kompakt nézete (a beállítások Megjelenés kártyáján).
    pub compact: bool,
}

/// Egy modul alapértelmezett láthatósága.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shown {
    Always,
    Never,
    /// Csak asztali gépen: laptopon (kisebb képernyő) alapból rejtett.
    DesktopOnly,
}

/// A panel modulai az alapsorrendben. A Laptop csak akkumulátoros gépen látszik (és csak ott
/// szerepel a beállítások listájában), a beállítástól függetlenül.
pub const PANEL_MODULES: &[PanelModule] = &[
    PanelModule { id: "tools", default: Shown::Always, compact: false },
    PanelModule { id: "claude", default: Shown::Never, compact: true },
    PanelModule { id: "codex", default: Shown::Never, compact: true },
    PanelModule { id: "activity", default: Shown::Never, compact: false },
    PanelModule { id: "note", default: Shown::Never, compact: true },
    PanelModule { id: "load", default: Shown::Never, compact: false },
    PanelModule { id: "network", default: Shown::Always, compact: true },
    PanelModule { id: "clock", default: Shown::Always, compact: true },
    PanelModule { id: "date", default: Shown::Never, compact: true },
    PanelModule { id: "events", default: Shown::Never, compact: true },
    PanelModule { id: "laptop", default: Shown::Always, compact: false },
    PanelModule { id: "peripherals", default: Shown::Always, compact: false },
    PanelModule { id: "weather", default: Shown::Always, compact: true },
    PanelModule { id: "currency", default: Shown::Never, compact: true },
    PanelModule { id: "music", default: Shown::DesktopOnly, compact: false },
    PanelModule { id: "volume", default: Shown::Always, compact: false },
    PanelModule { id: "tray", default: Shown::Always, compact: false },
    PanelModule { id: "power", default: Shown::Always, compact: false },
];

/// Az információs panel moduljainak láthatósága, sorrendje és nézete.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Sections {
    /// A modulok láthatósága azonosító szerint (a config.json-ban a `sections` közvetlen kulcsai,
    /// pl. "clock": true). A `normalize` után minden modul szerepel, a `PANEL_MODULES` sorrendjében.
    #[serde(flatten)]
    pub visible: serde_json::Map<String, serde_json::Value>,
    /// A modulok sorrendje a panelen (a Start és a MENÜ között).
    pub order: Vec<String>,
    /// Az Eszköztár gombjainak sorrendje: "calc", "notepad", "search", "snip".
    pub tools_order: Vec<String>,
    /// A Főkapcsoló gombjainak sorrendje: "lock", "sleep", "shutdown", "restart".
    pub power_order: Vec<String>,
    /// A Laptop gombjainak sorrendje: "wifi", "battery", "network", "bluetooth".
    pub laptop_order: Vec<String>,
    /// Kompakt nézetben lévő modulok (csak a kompakt nézettel rendelkezők): a Claude és a Codex
    /// a nullázásig hátralévő idő és a frissítés sora nélkül, az Idő a másodperc-sáv, a Dátum a
    /// hét nélkül.
    pub compact: Vec<String>,
}

pub const TOOL_KEYS: [&str; 4] = ["calc", "notepad", "search", "snip"];
pub const POWER_KEYS: [&str; 4] = ["lock", "sleep", "shutdown", "restart"];
pub const LAPTOP_KEYS: [&str; 4] = ["wifi", "battery", "network", "bluetooth"];

/// Érvényes lista: csak ismert kulcsok, mindegyik legfeljebb egyszer. `complete` esetén a
/// hiányzók a végére kerülnek.
fn normalize_keys(list: &mut Vec<String>, keys: &[&str], complete: bool) {
    let mut out: Vec<String> = Vec::new();
    for k in list.iter() {
        if keys.contains(&k.as_str()) && !out.contains(k) {
            out.push(k.clone());
        }
    }
    if complete {
        for k in keys {
            if !out.iter().any(|o| o == k) {
                out.push(k.to_string());
            }
        }
    }
    *list = out;
}

impl Default for Sections {
    fn default() -> Self {
        let mut s = Sections {
            visible: serde_json::Map::new(),
            order: Vec::new(),
            tools_order: Vec::new(),
            power_order: Vec::new(),
            laptop_order: Vec::new(),
            compact: Vec::new(),
        };
        s.normalize();
        s
    }
}

impl Sections {
    /// Látszik-e a modul (ismeretlen azonosítónál nem).
    pub fn shows(&self, id: &str) -> bool {
        self.visible.get(id).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    /// Érvényes állapot: minden modul láthatósága megvan (ami hiányzik, az alapérték: laptopon
    /// az asztali gépre szánt modulok rejtve), a sorrendek teljesek, ismeretlen kulcs nincs.
    pub fn normalize(&mut self) {
        // akkumulátoros gép-e (csak akkor kérdezzük, ha kell)
        let mut laptop: Option<bool> = None;
        let visible = PANEL_MODULES
            .iter()
            .map(|m| {
                let on = self.visible.get(m.id).and_then(|v| v.as_bool()).unwrap_or_else(|| match m.default {
                    Shown::Always => true,
                    Shown::Never => false,
                    Shown::DesktopOnly => !*laptop.get_or_insert_with(|| crate::sys::battery().is_some()),
                });
                (m.id.to_string(), serde_json::Value::Bool(on))
            })
            .collect();
        self.visible = visible;
        let ids: Vec<&str> = PANEL_MODULES.iter().map(|m| m.id).collect();
        let first_time = self.order.is_empty();
        normalize_keys(&mut self.order, &ids, false);
        // az újabb modulok az alapsorrend szerinti szomszédjuk után kerülnek (pl. a Codex a
        // Claude alá), nem a lista végére
        for (i, id) in ids.iter().enumerate() {
            if self.order.iter().any(|o| o == id) {
                continue;
            }
            let at = if first_time {
                self.order.len()
            } else {
                ids[..i].iter().rev().find_map(|prev| self.order.iter().position(|o| o == prev)).map_or(0, |p| p + 1)
            };
            self.order.insert(at, id.to_string());
        }
        normalize_keys(&mut self.tools_order, &TOOL_KEYS, true);
        normalize_keys(&mut self.power_order, &POWER_KEYS, true);
        normalize_keys(&mut self.laptop_order, &LAPTOP_KEYS, true);
        let compact: Vec<&str> = PANEL_MODULES.iter().filter(|m| m.compact).map(|m| m.id).collect();
        normalize_keys(&mut self.compact, &compact, false);
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

/// Szinkron a Windows-zal: bekapcsolva a CyberShell témaváltása a Windowsra is átvezetődik.
/// Kikapcsoláskor az eredeti Windows-beállítás visszaáll.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowsSync {
    /// A Windows sötét/világos módja (alkalmazások és rendszer) követi a CyberShell sémáját.
    pub dark_mode: bool,
    /// A Windows kiemelőszíne a téma egyik színe (`accent_color`).
    pub accent: bool,
    /// Melyik színe: "main", "alt" vagy "hot" (alapból "hot": a sárga és a cián a Windows
    /// világos felületein nehezen olvasható).
    pub accent_color: String,
    /// A Windows Terminal színsémája a CyberShell témája („CyberShell Dark” / „CyberShell Light”).
    pub terminal: bool,
}

impl Default for WindowsSync {
    fn default() -> Self {
        WindowsSync { dark_mode: false, accent: false, accent_color: "hot".into(), terminal: false }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Network {
    /// A ping célja: IP-cím vagy név (pl. "1.1.1.1", "google.com").
    pub ping_host: String,
}

impl Default for Network {
    fn default() -> Self {
        Network { ping_host: "1.1.1.1".into() }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Currency {
    /// A panel listáiban választható pénznemek ISO-kóddal (pl. "EUR"), ebben a sorrendben.
    pub list: Vec<String>,
}

impl Default for Currency {
    fn default() -> Self {
        Currency { list: ["EUR", "USD", "HUF", "GBP", "CHF"].iter().map(|s| s.to_string()).collect() }
    }
}

impl Currency {
    /// Érvényes lista: háromjegyű nagybetűs kódok, mindegyik egyszer; legalább kettő kell
    /// (különben az alapértelmezett lista).
    pub fn normalize(&mut self) {
        let mut out: Vec<String> = Vec::new();
        for c in &self.list {
            let c = c.trim().to_uppercase();
            if c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase()) && !out.contains(&c) {
                out.push(c);
            }
        }
        self.list = if out.len() >= 2 { out } else { Currency::default().list };
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
    /// Régi (v2.0) kapcsoló: a szél kijelzése. Helyette a modul kompakt nézete (szél nélkül)
    /// való; betöltéskor a `false` érték a kompakt nézetbe költözik, mentéskor már nem íródik ki.
    #[serde(skip_serializing)]
    pub wind: Option<bool>,
    /// A hőmérséklet mértékegysége: "c" (Celsius) vagy "f" (Fahrenheit).
    pub unit: String,
}

impl Default for Weather {
    fn default() -> Self {
        Weather { enabled: true, city: "Budapest".into(), latitude: None, longitude: None, wind: None, unit: "c".into() }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Calendar {
    pub feeds: Vec<CalendarFeed>,
}

/// A panel Jegyzet modulja: egy mindig látható, szerkeszthető szöveg.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Note {
    pub text: String,
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
    /// Régi (v1.3-ig) beállítás: "right" esetén az információs panelen jelent meg. Mostantól
    /// minden program az Ikonok között van; betöltéskor elfogadjuk, a fájlba már nem kerül vissza.
    #[serde(skip_serializing)]
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
            info_edge: "auto".into(),
            info_edge_auto: false,
            all_monitors: true,
            icon_labels: true,
            tooltips: true,
            frame: Frame::default(),
            activity: Activity::default(),
            claude_usage: true,
            weather: Weather::default(),
            network: Network::default(),
            currency: Currency::default(),
            calendar: Calendar::default(),
            note: Note::default(),
            sections: Sections::default(),
            appearance: "dark".into(),
            scheme: "dark".into(),
            theme: Theme::default(),
            theme_light: Theme::light(),
            health: Health::default(),
            start_menu: StartMenu::default(),
            neon_border: "off".into(),
            windows_sync: WindowsSync::default(),
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
    cfg.finish();
    Ok(cfg)
}

/// A frame és a sávok vastagságának megengedett tartománya (logikai pixel).
/// A sávok és a keretek megengedett vastagsága (logikai pixel): az ikonsáv, a panelsáv,
/// a fenti és lenti, illetve az oldalsó keret.
pub const ICONS_RANGE: (u32, u32) = (64, 96);
pub const INFO_RANGE: (u32, u32) = (80, 128);
/// A panelsáv vastagsága, ha fent vagy lent áll (vízszintesen ennél vastagabb nem kell).
pub const INFO_RANGE_HORIZ: (u32, u32) = (80, 100);
pub const FRAME_TB_RANGE: (u32, u32) = (0, 32);
pub const FRAME_SIDE_RANGE: (u32, u32) = (0, 40);

/// Érvényes #rrggbb szín-e.
pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub const EDGE_NAMES: [&str; 4] = ["left", "right", "top", "bottom"];

impl Config {
    /// A panelsáv megengedett vastagsága: fent vagy lent legfeljebb 100, oldalt 128 pont.
    pub fn info_range(&self) -> (u32, u32) {
        if matches!(self.info_edge.as_str(), "top" | "bottom") { INFO_RANGE_HORIZ } else { INFO_RANGE }
    }

    /// Betöltés utáni rendbetétel: sorrend, nyelv, panelhelyek, tartományok, séma.
    pub fn finish(&mut self) {
        // a régi Szél kapcsoló kikapcsolt állása: az Időjárás kompakt nézete (szél nélkül)
        if self.weather.wind.take() == Some(false) && !self.sections.compact.iter().any(|k| k == "weather") {
            self.sections.compact.push("weather".into());
        }
        self.sections.normalize();
        self.currency.normalize();
        if self.network.ping_host.trim().is_empty() {
            self.network.ping_host = Network::default().ping_host;
        }
        self.resolve_language();
        self.normalize_edges();
        self.health.minutes = self.health.minutes.clamp(5, 480);
        self.left_width = self.left_width.clamp(ICONS_RANGE.0, ICONS_RANGE.1);
        let r = self.info_range();
        self.right_width = self.right_width.clamp(r.0, r.1);
        self.frame.top = self.frame.top.clamp(FRAME_TB_RANGE.0, FRAME_TB_RANGE.1);
        self.frame.bottom = self.frame.bottom.clamp(FRAME_TB_RANGE.0, FRAME_TB_RANGE.1);
        self.frame.left = self.frame.left.clamp(FRAME_SIDE_RANGE.0, FRAME_SIDE_RANGE.1);
        self.frame.right = self.frame.right.clamp(FRAME_SIDE_RANGE.0, FRAME_SIDE_RANGE.1);
        if !["main", "alt", "hot"].contains(&self.windows_sync.accent_color.as_str()) {
            self.windows_sync.accent_color = "hot".into();
        }
        if !["off", "active", "all"].contains(&self.neon_border.as_str()) {
            self.neon_border = "off".into();
        }
        if !["c", "f"].contains(&self.weather.unit.as_str()) {
            self.weather.unit = "c".into();
        }
        self.resolve_scheme();
    }

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
        if self.info_edge == "auto" || self.info_edge.is_empty() {
            self.info_edge_auto = true;
            let laptop = crate::sys::battery().is_some();
            self.info_edge = if laptop && self.icons_edge != "bottom" { "bottom".into() } else { "right".into() };
        }
        if !EDGE_NAMES.contains(&self.info_edge.as_str()) || self.info_edge == self.icons_edge {
            self.info_edge = if self.icons_edge == "right" { "left".into() } else { "right".into() };
        }
    }

    /// A megjelenés feloldása: "auto" esetén napszak szerint (napnyugtától napkeltéig sötét),
    /// "windows" esetén a Windows alkalmazás-módja szerint.
    /// Igazat ad, ha a séma megváltozott.
    pub fn resolve_scheme(&mut self) -> bool {
        let scheme = match self.appearance.as_str() {
            "light" => "light",
            "windows" => {
                if crate::winsync::windows_light().unwrap_or(false) {
                    "light"
                } else {
                    "dark"
                }
            }
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
            "hu" | "en" | "pl" => self.language_auto = false,
            _ => {
                self.language_auto = true;
                self.language = crate::sys::system_language().to_string();
            }
        }
    }
}

/// A fájlba írt alak: automatikus nyelvnél "auto" kerül bele.
pub fn to_json(cfg: &Config) -> serde_json::Result<serde_json::Value> {
    let mut v = serde_json::to_value(cfg)?;
    if cfg.language_auto {
        v["language"] = serde_json::Value::from("auto");
    }
    if cfg.info_edge_auto {
        v["infoEdge"] = serde_json::Value::from("auto");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sections(json: &str) -> Sections {
        let mut s: Sections = serde_json::from_str(json).unwrap();
        s.normalize();
        s
    }

    #[test]
    fn modules_complete() {
        // a megadott érték megmarad, a hiányzó az alapérték, az ismeretlen kulcs eltűnik
        let s = sections(r#"{ "clock": false, "codex": true, "nincsilyen": true, "order": ["clock", "nincsilyen", "clock"] }"#);
        assert!(!s.shows("clock") && s.shows("codex"));
        assert!(s.shows("tools") && !s.shows("activity"));
        assert!(!s.visible.contains_key("nincsilyen") && !s.shows("nincsilyen"));
        assert_eq!(s.visible.len(), PANEL_MODULES.len());
        // a sorrend teljes, minden modul egyszer; a hiányzók az alapsorrend szerinti szomszédjuk
        // mellé kerülnek (az Eszköztárnak nincs előzője, így az elejére)
        assert_eq!(s.order.len(), PANEL_MODULES.len());
        assert_eq!(s.order.iter().filter(|o| *o == "clock").count(), 1);
        assert_eq!(s.order[0], "tools");
    }

    #[test]
    fn new_module_after_its_neighbour() {
        // régi (Codex előtti) sorrend: a Codex a Claude alá kerül, nem a végére
        let s = sections(r#"{ "order": ["power", "claude", "tools", "activity", "date", "clock", "weather", "music", "volume"] }"#);
        assert_eq!(&s.order[..4], ["power", "claude", "codex", "tools"]);
        // üres sorrend: az alapsorrend
        let d = sections("{}");
        let ids: Vec<&str> = PANEL_MODULES.iter().map(|m| m.id).collect();
        assert_eq!(d.order, ids);
    }

    #[test]
    fn compact_and_buttons() {
        let s = sections(r#"{ "compact": ["clock", "tools", "clock"], "toolsOrder": ["snip", "x"] }"#);
        // kompakt nézete csak a kompakt nézettel rendelkező modulnak lehet
        assert_eq!(s.compact, ["clock"]);
        assert_eq!(s.tools_order, ["snip", "calc", "notepad", "search"]);
        assert_eq!(s.power_order, POWER_KEYS);
        assert_eq!(s.laptop_order, LAPTOP_KEYS);
    }

    #[test]
    fn old_wind_switch_becomes_compact() {
        // a v2.0 kikapcsolt Szél kapcsolója: az Időjárás kompakt nézete; mentéskor már nem íródik ki
        let mut c: Config = serde_json::from_str(r#"{ "weather": { "wind": false } }"#).unwrap();
        c.finish();
        assert_eq!(c.sections.compact, ["weather"]);
        assert!(serde_json::to_value(&c).unwrap()["weather"].get("wind").is_none());
        let mut on: Config = serde_json::from_str(r#"{ "weather": { "wind": true } }"#).unwrap();
        on.finish();
        assert!(on.sections.compact.is_empty());
    }

    #[test]
    fn json_layout() {
        // a config.json-ban a modulok kulcsai a sections elején vannak, utánuk a sorrendek
        let v = serde_json::to_value(sections(r#"{ "music": false }"#)).unwrap();
        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "tools");
        assert_eq!(keys[PANEL_MODULES.len()], "order");
        assert_eq!(v["music"], false);
    }
}
