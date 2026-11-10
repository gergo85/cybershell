use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Elrejtse-e a Windows tálcáját, amíg a HUB fut.
    pub hide_taskbar: bool,
    /// A bal oldali ikonsáv szélessége (logikai pixel).
    pub left_width: u32,
    /// A jobb oldali panel szélessége (logikai pixel).
    pub right_width: u32,
    pub frame: Frame,
    pub activity: Activity,
    /// Mutassa-e a Claude 5 órás és heti használatát.
    pub claude_usage: bool,
    pub weather: Weather,
    pub theme: Theme,
    pub apps: Vec<AppEntry>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Frame {
    pub top: u32,
    pub bottom: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Weather {
    pub enabled: bool,
    /// A település neve (ebből keresi meg a koordinátákat).
    pub city: String,
    /// Ha megadod, a város helyett ezt használja.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

impl Default for Weather {
    fn default() -> Self {
        Weather { enabled: true, city: "Budapest".into(), latitude: None, longitude: None }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Activity {
    /// A visszaszámláló hossza percben.
    pub minutes: u32,
    /// Induláskor magától elkezdjen-e számolni.
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
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppEntry {
    pub name: String,
    /// Exe elérési útja, mappa vagy URI (pl. "spotify:"). %VÁLTOZÓK% használhatók.
    pub path: String,
    pub args: String,
    pub cwd: String,
    /// Beépített ikon neve (browser, code, folder…) vagy képfájl útvonala.
    pub icon: String,
    /// Exe-fájlnevek, amelyek ablakait ehhez az ikonhoz soroljuk.
    #[serde(rename = "match")]
    pub match_exe: Vec<String>,
    /// Ablakcím-részlet, ha az exe alapján nem azonosítható (pl. Store-appok).
    pub match_title: String,
    /// Az újonnan indított ablakot teljes méretre nyissa-e.
    pub maximize: Option<bool>,
    /// Az ikon finom színe (pl. "#ff7139"), ami a programra utal.
    pub color: String,
    /// Hol jelenjen meg: "left" (alapból, bal sáv) vagy "right" (jobb panel, az Indítás alatt).
    pub place: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hide_taskbar: true,
            left_width: 86,
            right_width: 86,
            frame: Frame::default(),
            activity: Activity::default(),
            claude_usage: true,
            weather: Weather::default(),
            theme: Theme::default(),
            apps: Vec::new(),
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame { top: 10, bottom: 10 }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Activity { minutes: 20, auto_start: true, sound: true }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            main: "#f3e600".into(),
            alt: "#00e1ff".into(),
            hot: "#ff2a6d".into(),
            bg: "#06070b".into(),
        }
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
        if lower.ends_with(".exe") {
            if let Some(name) = Path::new(&p).file_name() {
                return vec![name.to_string_lossy().to_lowercase()];
            }
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
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Nem olvasható: {} ({e})", path.display()))?;
    // A BOM-ot (Jegyzettömb menti így) eldobjuk.
    let text = text.trim_start_matches('\u{feff}');
    serde_json::from_str(text).map_err(|e| format!("Hibás config.json: {e}"))
}
