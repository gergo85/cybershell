//! A Claude 5 órás és heti használati korlátjának lekérése.
//!
//! A Claude Code bejelentkezését használja (%USERPROFILE%\.claude\.credentials.json).
//! A tokent csak olvassuk, nem frissítjük, így nem zavarjuk össze a Claude Code
//! bejelentkezését. Ha lejárt, a Claude Code következő indítása frissíti.
//! A tokent sehova nem írjuk ki és nem naplózzuk.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const URL: &str = "https://api.anthropic.com/api/oauth/usage";

fn credentials_path() -> PathBuf {
    if let Ok(d) = std::env::var("CLAUDE_CONFIG_DIR")
        && !d.trim().is_empty()
    {
        return PathBuf::from(d).join(".credentials.json");
    }
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    PathBuf::from(home).join(".claude").join(".credentials.json")
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn status(status: &str, plan: &str, message: &str) -> Value {
    json!({ "type": "usage", "status": status, "plan": plan, "message": message })
}

/// A fájl szövege. UTF-8 (BOM-mal vagy anélkül) és UTF-16 kódolást is elfogad.
fn read_text(path: &std::path::Path) -> Result<String, std::io::Error> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    Ok(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string())
}

/// Megkeresi az OAuth-adatokat tartalmazó objektumot: elsősorban a
/// „claudeAiOauth” kulcs alatt, de ha a formátum változik, a fában is keres.
fn find_oauth(v: &Value, depth: u32) -> Option<&Value> {
    if let Some(o) = v.get("claudeAiOauth").filter(|o| o.get("accessToken").is_some()) {
        return Some(o);
    }
    if v.get("accessToken").and_then(Value::as_str).is_some() {
        return Some(v);
    }
    if depth == 0 {
        return None;
    }
    v.as_object()?.values().find_map(|x| find_oauth(x, depth - 1))
}

/// Ide menti a Claude Code állapotsor-parancsa (CyberShell.exe --statusline) a korlátokat.
pub fn limits_file() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join("CyberShell");
    let _ = std::fs::create_dir_all(&d);
    d.join("claude-limits.json")
}

/// A Claude Code állapotsorától kapott legutóbbi adatok, ha vannak.
fn from_statusline(plan: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(limits_file()).ok()?).ok()?;
    let written = v["writtenAt"].as_i64().unwrap_or(0);
    let mut limits = Vec::new();
    for (key, label) in [("five_hour", "5 órás"), ("seven_day", "Heti")] {
        let l = &v[key];
        let Some(pct) = l["used_percentage"].as_f64() else { continue };
        // a visszaállás ideje másodpercben érkezik; ha szövegként, a felület értelmezi
        let resets_ms = l["resets_at"].as_f64().map(|s| if s < 100_000_000_000.0 { s * 1000.0 } else { s } as i64);
        let resets_iso = l["resets_at"].as_str().map(str::to_string);
        // ha az ablak azóta lejárt, a régi érték már nem érvényes
        let expired = resets_ms.map(|r| r <= now_ms()).unwrap_or(false);
        limits.push(json!({
            "key": key,
            "label": label,
            "pct": if expired { Value::Null } else { json!(pct) },
            "resetsAt": if expired { Value::Null } else if let Some(r) = resets_ms { json!(r) } else { json!(resets_iso) },
        }));
    }
    if limits.is_empty() {
        return None;
    }
    Some(json!({ "type": "usage", "status": "ok", "plan": plan, "limits": limits, "fetchedAt": written, "source": "statusline" }))
}

/// Egy lekérés: előbb közvetlenül a szerverről, ha az nem megy, a Claude Code
/// állapotsorától kapott adatokból. Az eredmény mindig a felületnek szánt üzenet.
pub fn fetch() -> Value {
    let direct = fetch_direct();
    if direct["status"] == "ok" {
        return direct;
    }
    let plan = direct["plan"].as_str().unwrap_or("").to_string();
    from_statusline(&plan).unwrap_or(direct)
}

fn fetch_direct() -> Value {
    let path = credentials_path();
    if !path.exists() {
        return status("missing", "", "Nincs bejelentkezési fájl.");
    }
    let text = match read_text(&path) {
        Ok(t) => t,
        Err(e) => return status("error", "", &format!("A bejelentkezési fájl nem olvasható ({:?}).", e.kind())),
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return status("error", "", "A bejelentkezési fájl formátuma ismeretlen.");
    };
    let Some(o) = find_oauth(&v, 3) else {
        return status("missing", "", "A fájlban nincs Claude-bejelentkezés.");
    };
    let plan = o["subscriptionType"].as_str().unwrap_or("").to_string();
    let token = o["accessToken"].as_str().unwrap_or("");
    if token.is_empty() {
        return status("missing", &plan, "A bejelentkezésből hiányzik a token.");
    }
    // ezredmásodperc vagy másodperc is lehet
    let mut exp = o["expiresAt"].as_i64().or_else(|| o["expiresAt"].as_f64().map(|f| f as i64)).unwrap_or(0);
    if exp > 0 && exp < 100_000_000_000 {
        exp *= 1000;
    }
    if exp > 0 && exp < now_ms() + 30_000 {
        return status("expired", &plan, "");
    }

    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().new_agent();
    let res = agent
        .get(URL)
        .header("Authorization", &format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .header("User-Agent", "CyberShell/0.2")
        .call();

    let body: Value = match res {
        Ok(mut r) => match r.body_mut().read_json::<Value>() {
            Ok(b) => b,
            Err(_) => return status("error", &plan, "Érthetetlen válasz érkezett."),
        },
        Err(ureq::Error::StatusCode(401)) => return status("expired", &plan, "A szerver elutasította (HTTP 401)."),
        Err(ureq::Error::StatusCode(code)) => return status("error", &plan, &format!("HTTP {code}")),
        Err(_) => return status("error", &plan, "Nincs kapcsolat."),
    };

    // A két fő korlát mindig látszik, a modellenkéntiek csak akkor, ha használtad őket.
    let keys = [
        ("five_hour", "5 órás", true),
        ("seven_day", "Heti", true),
        ("seven_day_opus", "Heti · Opus", false),
        ("seven_day_sonnet", "Heti · Sonnet", false),
    ];
    let mut limits = Vec::new();
    for (key, label, always) in keys {
        let l = &body[key];
        if let Some(pct) = l["utilization"].as_f64()
            && (always || pct > 0.0)
        {
            limits.push(json!({
                "key": key,
                "label": label,
                "pct": pct,
                "resetsAt": l["resets_at"].as_str(),
            }));
        }
    }
    if limits.is_empty() {
        return status("error", &plan, "A válaszban nincs használati adat.");
    }
    json!({ "type": "usage", "status": "ok", "plan": plan, "limits": limits, "fetchedAt": now_ms() })
}

/// Háttérszál: rendszeresen lekérdez, és az eredményt átadja. A `wake` csatornán
/// azonnali frissítés kérhető.
pub fn spawn(wake: Receiver<()>, deliver: impl Fn(Value) + Send + 'static) {
    std::thread::spawn(move || {
        loop {
            let result = fetch();
            // Sikeres lekérés után 5 percet várunk; lejárt bejelentkezésnél csak a
            // helyi fájlt figyeljük, azt percenként újraolvassuk.
            let wait = match result["status"].as_str() {
                // az állapotsor fájlját gyakrabban újraolvassuk, az olcsó
                Some("ok") if result["source"] == "statusline" => 30,
                Some("ok") => 300,
                Some("error") => 120,
                _ => 60,
            };
            deliver(result);
            match wake.recv_timeout(Duration::from_secs(wait)) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => while wake.try_recv().is_ok() {},
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}
