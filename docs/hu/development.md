[← Vissza a README-hez](../../README.hu.md) · [English](../en/development.md) · **Magyar**

# Fejlesztés

## Fordítás forrásból

Rust (MSVC) kell hozzá:

```
cargo build --release
```

A kész program a `target\release\cybershell.exe`; másold a projekt mappájába `CyberShell.exe` néven (az `ui` mappa mellé). A beállításokat a `%LOCALAPPDATA%\CyberShell\config.json`-ból olvassa; a projekt mappájában lévő `config.json`-t csak egyszer, az első induláskor másolja át oda.

A gyári háttérképek a projekt `wallpapers` mappájának képei (jpg, png, bmp): fordításkor mind az exe-be kerül, így cseréjükhöz elég ott kicserélni a fájlokat. Induláskor a program a hiányzókat kicsomagolja a `%LOCALAPPDATA%\CyberShell\wallpapers` mappába, és újraírja azokat, amelyeknek a tartalma eltér az azonos nevű beépített képtől; a korábbi gyári képek ott maradnak, saját képként (az Alapállapot a Lomtárba teszi őket).

A kód formázása és ellenőrzése (a formázás beállításai a `rustfmt.toml`-ban vannak):

```
cargo fmt
cargo clippy --release
```


Ha a projekt nagyon hosszú útvonalon van, a fordítás a Windows 260 karakteres korlátja miatt elakadhat. Ilyenkor állíts be rövidebb kimeneti mappát a `CARGO_TARGET_DIR` környezeti változóval.

Parancssori kapcsolók: `--quit`, `--restore`, `--keep-taskbar` (nem rejti el a tálcát), `--config <fájl>`, `--exit-after <mp>` (teszteléshez).

## MSIX-csomag (Microsoft Store)

A `packaging` mappában van a csomag manifestje (`AppxManifest.xml`) és a csomagot elkészítő szkript. A Windows SDK kell hozzá (`makeappx.exe`):

```
powershell -ExecutionPolicy Bypass -File packaging\build-msix.ps1
```

A szkript lefordítja a programot, az `assets\cybershell.ico`-ból elkészíti az ikonokat, a csomag pedig a `target\msix\CyberShell_<verzió>_x64.msix` fájlba kerül. A verziót a `Cargo.toml`-ból veszi. A csomagba csak az exe kerül, mert a felület és a gyári háttérképek bele vannak építve.

- **A Store-ba** aláírás nélkül kell feltölteni (a Microsoft írja alá). A `-Name`, a `-Publisher` és a `-PublisherDisplayName` értékét a Partner Center adja (Termékazonosító oldal).
- **Helyi kipróbáláshoz** fejlesztői módban aláírás nélkül is telepíthető a kicsomagolt mappa: `Add-AppxPackage -Register target\msix\layout\AppxManifest.xml`. Eltávolítani a `Get-AppxPackage CyberShell | Remove-AppxPackage` paranccsal lehet.

Csomagként telepítve az „Indulás a géppel” a Run kulcs helyett a csomag indítási feladatát használja. A manifestben a feladat `TaskId` értéke egyezzen az `src\autostart.rs` `TASK_ID` értékével.

## A forráskód felépítése

### A program (Rust)

- `src\main.rs`: a belépési pont és az eseményhurok; a háttérszálak `UserEvent` üzenettel jelentkeznek a fő szálon. `src\lifecycle.rs`: indulás és leállás. `src\web.rs`: a felületek kiszolgálása, címe és a felugró ablakok közös váza.
- `src\hub.rs` és `src\hub\`: a program központi állapota (`Hub`). A mezői témák szerint szakaszokba vannak rendezve, a metódusai témánként külön fájlban: elrendezés (`layout`), Ikonok és indítás (`icons`), információs panel (`panel`), Mostanában (`recent`), Claude (`claude`), beállítások (`settings`), ablakelőnézet (`preview`), oldalpanel és naptár (`flyout`), Start menü (`start`), Egészség és a piros figyelmeztető sávok (`health`), asztali menü (`deskmenu`), internetkapcsolat (`net`), téma (`theme`), igazodás a Windowshoz és neonkeret (`sync`).
- A többi modul a Windowszal beszélget: sávok (`appbar`), ablakok (`wintrack`), programlista (`apps`), Win billentyű (`keyhook`), az asztali menü egérfigyelője és a Windows asztali menüjének kiolvasása (`deskmenu`), vágólap (`clipboard`), Spotify (`media`), rendszer (`sys`), rendszerfigyelő (`monitor`), értesítési terület (`tray`), neonkeret-réteg (`neon`), a gép alkatrészei (`hardware`, `wmi`), a Bluetooth-eszközök töltöttsége (`periph`), indulás a géppel (`autostart`), beállítások (`config`), Claude- és Codex-használat (`usage`, `codex`), ablakképek (`peek`), Windows-téma, kiemelőszín, Windows Terminal és ablakkeret (`winsync`).
- Közös segédmodulok: a registry olvasása és írása (`reg`), ikonok PNG-vé alakítása (`bitmap`), COM a háttérszálakon (`com`).

Az információs panel egy új adata a programban: egy `send_…` (vagy háttérszálas lekérdezésnél `…_poll` és `on_…`) függvény a `src\hub\panel.rs`-ben, és egy `Feed` mező a `Hub` panel-adatok szakaszában. A `Feed` a legutóbb küldött üzenetet tartja: változatlan üzenetet nem küldünk újra, a panel újratöltésekor pedig ezt kapja vissza. Lassú lekérdezés (registry, fájlok, eszközkezelő) háttérszálon fusson, mint a Háttérprogramok, a Codex és a Perifériák modulé.

### A felület (HTML, JS, CSS)

- `ui\hub.html`: a felület váza, benne minden szkript és stílus felsorolva. `ui\core.js`: közös segédek (`window.HUB`). `ui\hub.js`: az indító, ami a cím `part` paramétere szerint felépíti a felületet. `ui\parts\*.js`: felületenként egy fájl (Ikonok, panel, keretek, beállítások, Start menü, oldalpanel, előnézet, szünet-sáv, asztali menü).
- A szövegek az `ui\i18n.js`-ben, a vonalikonok az `ui\icons.js`-ben vannak, a közös iCal-értelmező az `ui\ical.js`-ben.
- `ui\modules\`: az információs panel moduljai, modulonként egy `.js` és egy `.css` fájl; új modul írásának leírása: `ui\modules\README.md`.

A nagyobb felületek ugyanarra a mintára épülnek: egy keretre és a hozzá bejegyzett részekre. Egy rész egy objektum: a `html()` adja a jelölését, a `setup(ctx)` bekapcsolja a működését, és horgokat ad vissza (például `message(msg)` a program üzeneteire, `demo()` az előnézethez). A keret csak a közös részt végzi, és a horgokon át szól a részeknek.

| Keret | Bejegyzés | Részek |
|---|---|---|
| `parts\panel.js` | `HUB.defineModule({…})` | `ui\modules\*.js` |
| `parts\flyout.js` | `HUB.flyViews.push({…})` | `parts\fly-weather.js`, `fly-calendar.js`, `fly-note.js` |
| `parts\settings.js` | `HUB.settingsPanes.push({ id, … })` | `parts\settings-programs.js`, `settings-system.js` |

Új részt a `hub.html`-ben is fel kell venni, a keret szkriptje elé. A beállítások Panelek, Megjelenés és Funkciók füle még a `settings.js`-ben van; ha külön fájlba kerül, ugyanezt a mintát kövesse. Egy rész csak a saját elemeihez nyúljon; a keret elemeit (például a fülsávot) a `ctx` függvényein át érje el.

### Előnézet a böngészőben

A `preview.html` böngészőben megnyitva demóadatokkal mutatja a teljes képernyőt (egy egyszerű helyi webszerveren keresztül, mert a böngészők a `file://` címről nem töltik be a beágyazott részeket). Az elrendezés is kipróbálható: `preview.html?icons=top&info=bottom` (az Ikonok és a panel helye). A Start menü és a kereső külön is megnézhető: `ui/hub.html?part=launcher&mode=start`, illetve `ui/hub.html?part=launcher&mode=search&q=fenykep`. Ugyanígy az oldalpanel nézetei (`ui/hub.html?part=fly&view=calendar`, `events`, `note`; alapból az időjárás) és a beállítások fülei (`ui/hub.html?part=settings&tab=programs`, `panels`, `look`, `features`, `system`).
