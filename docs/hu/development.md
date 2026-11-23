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

- `ui\hub.html`: a felület váza; `ui\core.js`: közös segédek; `ui\parts\*.js`: felületenként egy fájl (Ikonok, panel, keretek, beállítások, Start menü, oldalpanel, előnézet, szünet-sáv); `ui\hub.js`: az indító, ami a cím szerint választ közülük. A szövegek az `ui\i18n.js`-ben, a vonalikonok az `ui\icons.js`-ben vannak.
- `src\main.rs`: a belépési pont és az eseményhurok; `src\web.rs`: a felületek kiszolgálása és a felugró ablakok; `src\lifecycle.rs`: indulás és leállás.
- `ui\ical.js`: a közös iCal-értelmező (a naptár és az Események modul használja).
- `ui\modules\`: az információs panel moduljai, modulonként egy `.js` és egy `.css` fájl; új modul írásának leírása: `ui\modules\README.md`.
- `src\hub.rs` és `src\hub\`: a program központi állapota, témák szerint: elrendezés (`layout`), Ikonok (`icons`), panel (`panel`), Claude (`claude`), beállítások (`settings`), előnézet (`preview`), oldalpanel (`flyout`), Start menü (`start`), Egészség (`health`), téma (`theme`), igazodás a Windowshoz és neonkeret (`sync`).
- A többi modul a Windows-szal beszélget: sávok (`appbar`), ablakok (`wintrack`), programlista (`apps`), Win billentyű (`keyhook`), vágólap (`clipboard`), Spotify (`media`), rendszer (`sys`), a gép alkatrészei (`hardware`, `wmi`), a Bluetooth-eszközök töltöttsége (`periph`), indulás a géppel (`autostart`), beállítások (`config`), Claude-használat (`usage`), ablakképek (`peek`), Windows-téma, kiemelőszín, Windows Terminal és ablakkeret (`winsync`).

A `preview.html` böngészőben megnyitva demóadatokkal mutatja a teljes képernyőt (egy egyszerű helyi webszerveren keresztül, mert a böngészők a `file://` címről nem töltik be a beágyazott részeket). Az elrendezés is kipróbálható: `preview.html?icons=top&info=bottom` (az Ikonok és a panel helye). A Start menü és a kereső külön is megnézhető: `ui/hub.html?part=launcher&mode=start`, illetve `ui/hub.html?part=launcher&mode=search&q=fenykep`.
