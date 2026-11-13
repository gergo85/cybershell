[← Vissza a README-hez](../../README.hu.md) · [English](../en/development.md) · **Magyar**

# Fejlesztés

## Fordítás forrásból

Rust (MSVC) kell hozzá:

```
cargo build --release
```

A kész program a `target\release\cybershell.exe`; másold a projekt mappájába `CyberShell.exe` néven (a `config.json` és az `ui` mappa mellé).

A kód formázása és ellenőrzése (a formázás beállításai a `rustfmt.toml`-ban vannak):

```
cargo fmt
cargo clippy --release
```


Ha a projekt nagyon hosszú útvonalon van, a fordítás a Windows 260 karakteres korlátja miatt elakadhat. Ilyenkor állíts be rövidebb kimeneti mappát a `CARGO_TARGET_DIR` környezeti változóval.

Parancssori kapcsolók: `--quit`, `--restore`, `--keep-taskbar` (nem rejti el a tálcát), `--config <fájl>`, `--exit-after <mp>` (teszteléshez).

## A forráskód felépítése

- `ui\hub.html`: a felület váza; `ui\core.js`: közös segédek; `ui\parts\*.js`: felületenként egy fájl (Ikonok, panel, keretek, beállítások, Start menü, oldalpanel, előnézet, szünet-sáv); `ui\hub.js`: az indító, ami a cím szerint választ közülük. A szövegek az `ui\i18n.js`-ben, a vonalikonok az `ui\icons.js`-ben vannak.
- `src\main.rs`: a belépési pont és az eseményhurok; `src\web.rs`: a felületek kiszolgálása és a felugró ablakok; `src\lifecycle.rs`: indulás, leállás, átállás a régi névről.
- `src\hub.rs` és `src\hub\`: a program központi állapota, témák szerint: elrendezés (`layout`), Ikonok (`icons`), panel (`panel`), Claude (`claude`), beállítások (`settings`), előnézet (`preview`), oldalpanel (`flyout`), Start menü (`start`), Egészség (`health`), téma (`theme`).
- A többi modul a Windows-szal beszélget: sávok (`appbar`), ablakok (`wintrack`), programlista (`apps`), Win billentyű (`keyhook`), vágólap (`clipboard`), Spotify (`media`), rendszer (`sys`), indulás a géppel (`autostart`), beállítások (`config`), Claude-használat (`usage`), ablakképek (`peek`).

A `preview.html` böngészőben megnyitva demóadatokkal mutatja a teljes képernyőt (egy egyszerű helyi webszerveren keresztül, mert a böngészők a `file://` címről nem töltik be a beágyazott részeket). Az elrendezés is kipróbálható: `preview.html?icons=top&info=bottom` (az Ikonok és a panel helye). A Start menü és a kereső külön is megnézhető: `ui/hub.html?part=launcher&mode=start`, illetve `ui/hub.html?part=launcher&mode=search&q=fenykep`.
