//! A Notesz (jegyzet és teendők) a config.json `note` részében van. A v2.2 egyik köztes
//! buildje külön `notes.json`-ba tette; ha ilyen fájl van, induláskor egyszer visszavesszük a
//! config.json-ba, a fájlt pedig `notes.migrated.json` néven félretesszük.

use crate::config::Note;
use crate::log;

/// Ha van `notes.json`, és a config.json-ban még nincs Notesz, átveszi. Igazat ad, ha a
/// beállítás változott (ilyenkor a config.json-t menteni kell).
pub(crate) fn take_over(note: &mut Note) -> bool {
    let path = crate::data_dir().join("notes.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return false };
    let Ok(mut n) = serde_json::from_str::<Note>(text.trim_start_matches('\u{feff}')) else {
        log("a notes.json hibás, nem vesszük át");
        return false;
    };
    n.normalize();
    let changed = *note == Note::default() && n != Note::default();
    if changed {
        *note = n;
        log(format!("a Notesz visszakerült a config.json-ba ({} teendő)", note.tasks.len()));
    }
    if let Err(e) = std::fs::rename(&path, path.with_file_name("notes.migrated.json")) {
        log(format!("a notes.json nem tehető félre: {e}"));
    }
    changed
}
