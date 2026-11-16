//! Codex-használat: a Codex (CLI, VS Code-bővítmény, alkalmazás) a munkamenet-naplóiba
//! (`%USERPROFILE%\.codex\sessions\ÉÉÉÉ\HH\NN\rollout-*.jsonl`, vagy a `CODEX_HOME` mappába)
//! minden válasz után beírja a használati korlátokat: az 5 órás és a heti keret kihasználtságát
//! százalékban, és hogy mikor nullázódik. A legfrissebb naplók végéről a legutolsó ilyen
//! bejegyzést olvassuk ki.

use serde_json::{Value, json};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn codex_home() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("CODEX_HOME")
        && !h.is_empty()
    {
        return Some(PathBuf::from(h));
    }
    std::env::var("USERPROFILE").ok().map(|p| PathBuf::from(p).join(".codex"))
}

/// Egy mappa almappái név szerint csökkenő sorrendben (a dátumos mappák így a legújabbal kezdődnek).
fn subdirs_desc(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v.reverse();
    v
}

/// A legutóbb módosított néhány naplófájl (a legújabb napok mappáiból).
fn recent_logs(sessions: &Path) -> Vec<(SystemTime, PathBuf)> {
    let mut days = Vec::new();
    'outer: for y in subdirs_desc(sessions) {
        for m in subdirs_desc(&y) {
            for d in subdirs_desc(&m) {
                days.push(d);
                if days.len() >= 3 {
                    break 'outer;
                }
            }
        }
    }
    let mut files: Vec<(SystemTime, PathBuf)> = days
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .filter_map(|p| std::fs::metadata(&p).and_then(|m| m.modified()).ok().map(|t| (t, p)))
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files.truncate(6);
    files
}

/// Egy JSON-értékben (bármilyen mélyen) a `rate_limits` objektum.
fn find_limits(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(o) => {
            if let Some(r) = o.get("rate_limits").filter(|r| r.is_object()) {
                return Some(r);
            }
            o.values().find_map(find_limits)
        }
        Value::Array(a) => a.iter().find_map(find_limits),
        _ => None,
    }
}

/// A fájl végéről (legfeljebb 1 MB) a legutolsó használati bejegyzés.
fn last_limits(path: &Path) -> Option<Value> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = len.saturating_sub(1 << 20);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    text.lines().rev().filter(|l| l.contains("\"rate_limits\"")).find_map(|l| {
        let v: Value = serde_json::from_str(l).ok()?;
        find_limits(&v).cloned()
    })
}

fn secs(t: SystemTime) -> f64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// A panelnek szánt állapot: `status` "ok" (vannak korlátok), "none" (nincs Codex-napló).
pub fn read() -> Value {
    let Some(home) = codex_home() else { return json!({ "type": "codex", "status": "none" }) };
    let now = secs(SystemTime::now());
    for (mtime, path) in recent_logs(&home.join("sessions")) {
        let Some(rl) = last_limits(&path) else { continue };
        let written = secs(mtime);
        let limit = |w: &Value| -> Option<Value> {
            let pct = w["used_percent"].as_f64()?;
            let minutes = w["window_minutes"].as_u64().unwrap_or(0);
            // újabb változat: a nullázás időpontja; régebbi: hány másodperc múlva
            let reset = w["resets_at"].as_f64().or_else(|| w["resets_in_seconds"].as_f64().map(|s| written + s));
            let key = match minutes {
                290..=310 => "five_hour",
                10_000..=10_200 => "seven_day",
                _ => "",
            };
            // ha azóta lejárt az ablak, a kihasználtság ismeretlen (új ablak kezdődött)
            let expired = reset.is_some_and(|r| r <= now);
            Some(json!({
                "key": key,
                "minutes": minutes,
                "pct": if expired { Value::Null } else { json!(pct) },
                "resetsAt": reset.filter(|_| !expired).map(|r| (r * 1000.0) as u64),
            }))
        };
        let limits: Vec<Value> = ["primary", "secondary"].iter().filter_map(|k| limit(&rl[*k])).collect();
        if limits.is_empty() {
            continue;
        }
        return json!({
            "type": "codex",
            "status": "ok",
            "plan": rl["plan_type"].as_str().unwrap_or(""),
            "limits": limits,
            "at": (written * 1000.0) as u64,
        });
    }
    json!({ "type": "codex", "status": "none" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_in_line() {
        let line = r#"{"timestamp":"2026-11-13T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{"primary":{"used_percent":12.5,"window_minutes":300,"resets_in_seconds":3600},"secondary":{"used_percent":40.0,"window_minutes":10080,"resets_at":4102444800}}}}"#;
        let v: Value = serde_json::from_str(line).unwrap();
        let rl = find_limits(&v).unwrap();
        assert_eq!(rl["primary"]["used_percent"], 12.5);
    }
}
