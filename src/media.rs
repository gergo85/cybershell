//! Zene: a Spotify asztali alkalmazás lejátszása a Windows médiavezérlőjén keresztül
//! (ugyanaz, amit a hangerő-felugró ablak és a zárolási képernyő is mutat).
//!
//! Nem kell hozzá Spotify-fiók összekötése vagy fejlesztői kulcs: a Spotify alkalmazás maga
//! jelenti a Windowsnak, mit játszik (cím, előadó, album, borító, pozíció), és fogadja a
//! lejátszás/szünet, előző/következő, keverés és tekerés parancsokat. Csak a Spotify
//! munkamenetét figyeljük, más lejátszókét (böngésző, filmlejátszó) nem.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde_json::json;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

/// A felülettől érkező parancsok.
pub enum Cmd {
    Toggle,
    Next,
    Prev,
    /// Tekerés: a szám elejétől mért másodperc.
    Seek(f64),
    /// Véletlenszerű sorrend be/ki.
    Shuffle,
}

/// A legutóbbi albumborító: sorszám, típus, tartalom (a felület `/__art/<sorszám>` címen kéri).
static ART: Mutex<Option<(u64, String, Vec<u8>)>> = Mutex::new(None);

pub fn art() -> Option<(String, Vec<u8>)> {
    ART.lock().ok().and_then(|g| g.as_ref().map(|(_, m, d)| (m.clone(), d.clone())))
}

/// 100 ns-os egységek 1601 óta (Windows-idő) → most.
fn now_ticks() -> i64 {
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64 / 100).unwrap_or(0);
    unix + 116_444_736_000_000_000
}

fn find_spotify(mgr: &Manager) -> Option<Session> {
    let list = mgr.GetSessions().ok()?;
    let n = list.Size().ok()?;
    (0..n)
        .filter_map(|i| list.GetAt(i).ok())
        .find(|s| s.SourceAppUserModelId().map(|id| id.to_string().to_lowercase().contains("spotify")).unwrap_or(false))
}

fn read_thumb(s: &Session) -> Option<(String, Vec<u8>)> {
    let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?.min(4 * 1024 * 1024) as u32;
    if size == 0 {
        return None;
    }
    let mime = stream.ContentType().map(|c| c.to_string()).unwrap_or_default();
    let buf = Buffer::Create(size).ok()?;
    let filled = stream.ReadAsync(&buf, size, InputStreamOptions::None).ok()?.join().ok()?;
    let reader = DataReader::FromBuffer(&filled).ok()?;
    let len = reader.UnconsumedBufferLength().ok()? as usize;
    let mut data = vec![0u8; len];
    reader.ReadBytes(&mut data).ok()?;
    let mime = if mime.starts_with("image/") { mime } else { "image/jpeg".to_string() };
    Some((mime, data))
}

/// A lejátszás állapota. A pozíciót a legutóbbi frissítés óta eltelt idővel kiegészítjük
/// (a Spotify csak változáskor jelenti).
fn snapshot(s: &Session) -> serde_json::Value {
    let props = s.TryGetMediaPropertiesAsync().ok().and_then(|op| op.join().ok());
    let text = |f: &dyn Fn() -> windows::core::Result<windows::core::HSTRING>| f().map(|h| h.to_string()).unwrap_or_default();
    let (title, artist, album) = match &props {
        Some(p) => (text(&|| p.Title()), text(&|| p.Artist()), text(&|| p.AlbumTitle())),
        None => Default::default(),
    };
    let info = s.GetPlaybackInfo().ok();
    let status = info.as_ref().and_then(|i| i.PlaybackStatus().ok()).unwrap_or(Status::Closed);
    let playing = status == Status::Playing;
    let shuffle = info.as_ref().and_then(|i| i.IsShuffleActive().ok()).and_then(|r| r.Value().ok());
    let controls = info.as_ref().and_then(|i| i.Controls().ok());
    let can_seek = controls.as_ref().and_then(|c| c.IsPlaybackPositionEnabled().ok()).unwrap_or(false);
    let (mut pos, mut dur) = (0.0, 0.0);
    if let Ok(t) = s.GetTimelineProperties() {
        let end = t.EndTime().map(|x| x.Duration).unwrap_or(0) - t.StartTime().map(|x| x.Duration).unwrap_or(0);
        let mut p = t.Position().map(|x| x.Duration).unwrap_or(0);
        if playing && let Ok(upd) = t.LastUpdatedTime() {
            let gap = now_ticks() - upd.UniversalTime;
            if gap > 0 && gap < 6 * 3600 * 10_000_000 {
                p += gap;
            }
        }
        if end > 0 {
            dur = end as f64 / 1e7;
            pos = (p.clamp(0, end)) as f64 / 1e7;
        }
    }
    json!({
        "type": "media", "running": true, "title": title, "artist": artist, "album": album,
        "playing": playing, "shuffle": shuffle, "canSeek": can_seek && dur > 0.0,
        "pos": (pos * 10.0).round() / 10.0, "dur": (dur * 10.0).round() / 10.0, "at": crate::unix_ms(),
    })
}

fn run_cmd(s: &Session, cmd: Cmd) {
    let r = match cmd {
        Cmd::Toggle => s.TryTogglePlayPauseAsync().map(|op| op.join().is_ok()),
        Cmd::Next => s.TrySkipNextAsync().map(|op| op.join().is_ok()),
        Cmd::Prev => s.TrySkipPreviousAsync().map(|op| op.join().is_ok()),
        Cmd::Seek(sec) => {
            let start = s.GetTimelineProperties().and_then(|t| t.StartTime()).map(|x| x.Duration).unwrap_or(0);
            s.TryChangePlaybackPositionAsync(start + (sec.max(0.0) * 1e7) as i64).map(|op| op.join().is_ok())
        }
        Cmd::Shuffle => {
            let on = s.GetPlaybackInfo().and_then(|i| i.IsShuffleActive()).and_then(|r| r.Value()).unwrap_or(false);
            s.TryChangeShuffleActiveAsync(!on).map(|op| op.join().is_ok())
        }
    };
    if r.is_err() {
        crate::log("zene: a parancs nem ment át a Spotifynak");
    }
}

/// Háttérszál: másodpercenként ránéz a Spotifyra, és változáskor (vagy ha a pozíció
/// elcsúszott a becsülthöz képest) üzenetet küld. A parancsokat azonnal végrehajtja.
pub fn spawn(cmds: Receiver<Cmd>, deliver: impl Fn(serde_json::Value) + Send + 'static) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mgr = match Manager::RequestAsync().and_then(|op| op.join()) {
            Ok(m) => m,
            Err(e) => {
                crate::log(format!("zene: a Windows médiavezérlője nem érhető el ({e})"));
                deliver(json!({ "type": "media", "running": false, "unavailable": true }));
                return;
            }
        };
        let mut last_key = String::new();
        let mut last_sent: Option<(f64, u64, bool)> = None; // pozíció, idő, szól-e
        let mut art_key = String::new();
        let mut art_seq = 0u64;
        let mut session: Option<Session> = None;
        let mut next_lookup = Instant::now();
        let mut idle_sent = false;
        loop {
            // a munkamenetet néhány másodpercenként újra megkeressük (a Spotify újraindulhat)
            if session.is_none() || Instant::now() >= next_lookup {
                session = find_spotify(&mgr);
                next_lookup = Instant::now() + Duration::from_secs(if session.is_some() { 5 } else { 2 });
            }
            match &session {
                None => {
                    if !idle_sent {
                        idle_sent = true;
                        last_key.clear();
                        last_sent = None;
                        deliver(json!({ "type": "media", "running": false }));
                    }
                }
                Some(s) => {
                    idle_sent = false;
                    let mut v = snapshot(s);
                    let key = format!(
                        "{}|{}|{}|{}|{}|{}|{}",
                        v["title"], v["artist"], v["album"], v["playing"], v["shuffle"], v["dur"], v["canSeek"]
                    );
                    // új szám: az albumborító is frissül
                    let a_key = format!("{}|{}|{}", v["title"], v["artist"], v["album"]);
                    if a_key != art_key {
                        art_key = a_key;
                        art_seq += 1;
                        let thumb = read_thumb(s);
                        if let Ok(mut g) = ART.lock() {
                            *g = thumb.map(|(m, d)| (art_seq, m, d));
                        }
                    }
                    let has_art = ART.lock().ok().is_some_and(|g| g.is_some());
                    v["art"] = if has_art { json!(format!("/__art/{art_seq}")) } else { json!("") };
                    let pos = v["pos"].as_f64().unwrap_or(0.0);
                    let playing = v["playing"].as_bool().unwrap_or(false);
                    let drift = match last_sent {
                        Some((p, at, was_playing)) => {
                            let expected = if was_playing { p + (crate::unix_ms().saturating_sub(at)) as f64 / 1000.0 } else { p };
                            (expected - pos).abs() > 1.5
                        }
                        None => true,
                    };
                    if key != last_key || drift {
                        last_key = key;
                        last_sent = Some((pos, crate::unix_ms(), playing));
                        deliver(v);
                    }
                }
            }
            match cmds.recv_timeout(Duration::from_millis(if session.is_some() { 1000 } else { 2000 })) {
                Ok(cmd) => {
                    if session.is_none() {
                        session = find_spotify(&mgr);
                    }
                    if let Some(s) = &session {
                        run_cmd(s, cmd);
                        // a Spotify kis késéssel frissíti az állapotát
                        std::thread::sleep(Duration::from_millis(180));
                        last_key.clear();
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}
