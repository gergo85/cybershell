# CyberHub

Cyberpunk stílusú felület Windowsra. Bal oldalt ikonsáv, jobb oldalt információs panel, felül és alul vékony neonkeret. A programok középen nyílnak meg, és teljes méretben kitöltik a maradék helyet.

A sávok ugyanúgy foglalják le a képernyő szélét, mint a Windows tálcája, ezért a teljes méretre nyitott ablakok nem lógnak rájuk. Amíg a CyberHub fut, a gyári tálca rejtve marad. Kilépéskor minden visszaáll.

## Indítás és kilépés

- **Indítás:** `CyberHub.exe`. A `config.json` és az `ui` mappa legyen mellette.
- **Kilépés:** jobb alul **// MENÜ** → **Kilépés a CyberHubból**, vagy `Ctrl+Alt+Q`, vagy parancssorból `CyberHub.exe --quit`.
- **Újratöltés:** `Ctrl+Alt+R`, vagy MENÜ → **Újratöltés**. Ilyenkor a program újraolvassa a `config.json`-t és a felület fájljait.
- **Indulás a géppel:** MENÜ → **Indulás a géppel** kapcsoló. Bekapcsolva a CyberHub bejelentkezéskor magától elindul: megvárja, amíg az Intéző elkészül, és csak utána foglalja le a képernyő szélét. Minden indításkor frissíti a bejegyzést, így a mappa áthelyezése után is jó helyről indul.

## Beállítások (MENÜ)

A **// MENÜ** gomb a teljes munkaterületet kitöltő beállítási felületet nyit meg; az oldalsó panelek és a keretek közben is látszanak. Bezárni a **Bezárás** gombbal, az `Esc` billentyűvel vagy a **// MENÜ** gomb újbóli megnyomásával lehet. Amíg nyitva van, a MENÜ gomb sárgán világít.

- **Rendszerterhelés:** a processzor és a memória kihasználtsága, az utolsó egy perc görbéjével. 60% fölött narancs, 85% fölött rózsaszín.

- **Rendszer:** újratöltés, a konfigurációs fájl megnyitása, indulás a géppel, **panelek felcserélése** (az ikonsáv jobbra, az információs panel balra kerül), **nyelv** (magyar vagy angol), kilépés.
- **Látható szekciók:** a jobb panel Claude, Aktivitás, Idő és Dátum része külön-külön ki- és bekapcsolható. A sorrendjük a sor elején lévő jellel húzva állítható.
- **Ikonok:** a bal sáv és a jobb panel programjai. Sorrend a ▲▼ gombokkal, átnevezés a névmezőben, ikon és szín választható, a **Bal/Jobb** kapcsoló dönti el, melyik panelen jelenjen meg, az **Auto** oszlop bekapcsolásával a program bejelentkezéskor magától elindul (ha a CyberHub is a géppel indul), a kuka ikon törli. Új programot a **Program hozzáadása…** gombbal (exe vagy parancsikon), vagy egy futó program nevére kattintva adhatsz hozzá. A változások a **Mentés** gombbal lépnek életbe.
- **Háttérkép:** a `wallpapers` mappa képei kis képként. Kattintásra a CyberHub beállítja a Windows háttérképének (kitöltő elrendezéssel, minden monitoron); az éppen használt kép sárga keretet és „AKTÍV” címkét kap. A **Kép hozzáadása…** gombbal tetszőleges képet (jpg, png, bmp) választhatsz: a program bemásolja a mappába, és rögtön be is állítja. A **Mappa megnyitása** gomb az Intézőben mutatja a mappát; az oda tett képek a következő megnyitáskor megjelennek.

Indításkor a CyberHub ellenőrzi, hogy a programok telepítve vannak-e még. Ami már nincs a gépen, annak az ikonja nem jelenik meg; a beállításokban „NINCS TELEPÍTVE” jelzéssel látszik, onnan törölheted.

A CyberHub mellett egy kis őrfolyamat is fut (a Feladatkezelőben második `CyberHub.exe`-ként látszik). Ha a fő program bármi módon leáll, például a Feladatkezelőből bezárják, az őr visszaadja a tálcát és felszabadítja a képernyő szélét, aztán ő is kilép.

## Bal oldali ikonsáv

| Művelet | Mit csinál |
|---|---|
| Kattintás | Elindítja a programot, vagy előhozza, ha már fut. |
| Kattintás az éppen aktív programra | Kis méretre teszi; ha több ablaka van, a következőre vált. |
| Shift+kattintás vagy középső gomb | Új példányt indít. |

A cián jelzés az ikon mellett azt mutatja, hogy a program fut. Az aktív programnál a jelzés sárga és hosszabb, a csempe kerete kivilágosodik. Ha a programnak több ablaka van nyitva, a jelzés mellett egy második, rövidebb csík is látszik.

**Ablakelőnézet:** ha egy programnak több ablaka van nyitva, és az ikonja fölött egy pillanatig időzöl, az ikon mellett egymás mellett megjelenik az ablakok élő kisképe. Egy kisképre kattintva az az ablak jön elő; a kártya jobb felső sarkában lévő **×** (vagy a középső egérgomb) bezárja az ablakot. A kis méretre tett ablakokról a Windows nem ad képet, ezeknél a program ikonja és a „KIS MÉRETBEN” felirat látszik. Ha az egér elhagyja az ikont és az előnézetet, az magától eltűnik; ha már látszik, egy másik több ablakos programra lépve azonnal átvált.

**Futó programok:** ami fut, de nincs kitűzve, az ikonok alatt jelenik meg egy cián elválasztó után, a program saját ikonjával és nevével (a hosszabb név két sorba törik). Ugyanúgy működik, mint a kitűzött ikonok.

**Fájlhúzás:** ha egy fájlt egy ikon fölé húzol, és ott tartod egy pillanatig, a program ablaka előjön, és beleejtheted a fájlt. Ha magára az ikonra ejted, a program megnyitja a fájlt.

## Jobb oldali panel

A panel fentről lefelé: Keresés és Indítás, Claude, Aktivitás, majd alul az idő és a dátum, az időjárás és a menü.

- **Keresés és Indítás:** a panel tetején. A Keresés a Windows keresőjét nyitja meg (mint a `Win+S`), az Indítás a gyári Start menüt (mint a `Win` billentyű).
- **Idő:** az óra és a perc egy sorban, alattuk a másodpercek 12 szakaszos sávja.
- **Dátum:** külön szekcióban, számokkal: az év, alatta a hónap és a nap, alattuk a hét sorszáma (ISO-hét, pl. „46. HÉT”).
- **Főkapcsoló:** Zárolás, Alvó állapot, Leállítás, Újraindítás. A leállítás és az újraindítás megerősítést kér; előtte a CyberHub mindent visszaállít.
- **Claude:** az 5 órás és a heti használati korlát százalékban, valamint az, hogy mennyi idő múlva áll vissza (↻). 60% fölött cián, 85% fölött rózsaszín a sáv. Ötpercenként frissül, a frissítés idejére kattintva azonnal.
- **Aktivitás:** visszaszámláló, alapból 20 perces. A program indulásakor nem indul magától (a ▶ gombbal indítod); ha mégis így szeretnéd, a `config.json`-ban állítsd az `activity.autoStart` értékét `true`-ra. Ha lejár, a panel és a keretek rózsaszínre váltanak és villognak, a gép pedig hangot ad. **✓:** új kör indul. **+5:** halasztás öt perccel. A három kis gomb: szünet/folytatás, újraindítás és beállítás. A beállításnál tetszőleges hosszt beírhatsz percben (1–240), ezt a program megjegyzi.
- **Időjárás:** a panel alján, az Open-Meteo ingyenes szolgáltatásából, negyedóránként frissül. Rákattintva azonnal frissül. A várost a `config.json` `weather` részében állíthatod be.

### A Claude-használat forrása

Három forrásból dolgozik, ebben a sorrendben:

1. **A claude.ai-fiókod (ajánlott, az asztali Claude-hoz is jó).** A panelen a **Belépés** gombbal megnyílik egy kis ablak, ahol bejelentkezel a claude.ai-ra. Belépés után az ablak magától bezárul, és a CyberHub ötpercenként lekéri ugyanazokat a korlátokat, amelyeket a claude.ai is mutat. A bejelentkezés a CyberHub saját böngészőprofiljában marad (`%LOCALAPPDATA%\CyberHub\WebView2`), az asztali Claude apphoz nem nyúl. A belépést 30 napig megjegyzi, így újraindítás után sem kell újra belépni (hacsak a claude.ai maga ki nem léptet). Ha valaha kijelentkeznél, töröld ezt a mappát, amíg a CyberHub nem fut. A lekérdezés nem hivatalos, egy későbbi claude.ai-változás után előfordulhat, hogy nem működik.

2. **A Claude Code állapotsora.** A Claude Code minden válasz után átadja az állapotsornak az 5 órás és a heti korlátot. Ha a CyberHub az állapotsor, ezeket elmenti, és a panel kiírja. Beállítás a `%USERPROFILE%\.claude\settings.json` fájlban:

   ```json
   "statusLine": { "type": "command", "command": "\"C:\\Users\\<név>\\OneDrive\\Dokumentumok\\GitHub\\cyberhub\\CyberHub.exe\" --statusline" }
   ```

   Ilyenkor a terminálban a Claude Code alján is megjelenik egy rövid sor, például: `Opus · 5h 12% · hét 40%`. Az adat a legutóbbi Claude Code-válasz idejét tükrözi; ha az 5 órás ablak azóta lejárt, a panel „—” jelet mutat.

3. **Közvetlen lekérdezés** a Claude Code bejelentkezésével (`%USERPROFILE%\.claude\.credentials.json`), ha a token olvasható formában van benne. Az újabb Claude Code-változatok ezt már nem így tárolják, ezért ez többnyire nem elérhető.

A második módszer nem hivatalosan dokumentált, ezért egy későbbi változás után előfordulhat, hogy nem működik.

A CyberHub a bejelentkezést csak olvassa, nem frissíti, és a tokent sehova nem írja ki. Ha a panelen az áll, hogy még nincs adat, indíts el egy terminálban egy Claude Code-munkamenetet (`claude`), és küldj egy üzenetet: az első válasz után a panel 30 másodpercen belül frissül. Ha a funkcióra nincs szükséged, a `config.json`-ban állítsd a `claudeUsage` értékét `false`-ra.

## Programok hozzáadása

A `config.json` `apps` listájában minden sor egy ikon:

```json
{ "name": "Lightroom", "path": "C:\\Program Files\\Adobe\\Adobe Lightroom Classic\\Lightroom.exe", "icon": "photo" }
```

| Mező | Jelentés |
|---|---|
| `name` | Az ikon alatti felirat. |
| `path` | Exe, mappa vagy URI. A `%LOCALAPPDATA%`-hoz hasonló változók használhatók. Store-appokhoz: `shell:AppsFolder\<AppID>` (az AppID-t a PowerShell `Get-StartApps` parancsa listázza). |
| `args`, `cwd` | Indítási paraméterek és munkamappa (nem kötelező). |
| `icon` | Beépített ikon: `browser`, `globe`, `code`, `folder`, `panels`, `terminal`, `doc`, `sheet`, `music`, `photo`, `enhance`, `design`, `picker`, `video`, `mail`, `chat`, `game`, `rocket`, `notes`, `settings`, `calendar`, `cloud`, `spark`, `app`. Megadhatsz képfájlt is, például `icons/sajat.png` (az `ui` mappához képest) vagy teljes elérési utat. Ha üresen hagyod, a program neve alapján választ. |
| `match` | Exe-fájlnevek, amelyek ablakait ehhez az ikonhoz kell sorolni. Akkor kell, ha az indító és a futó program neve eltér (pl. `wt.exe` → `windowsterminal.exe`). |
| `matchTitle` | Ablakcím-részlet, ha a program exe alapján nem azonosítható. |
| `maximize` | `false` esetén nem nyitja teljes méretre az új ablakot (alapból `true`). |
| `color` | Az ikon finom színe, ami a programra utal, például `"#ff7139"`. |
| `place` | `"right"` esetén az ikon a jobb panelre kerül, a Keresés és az Indítás gomb fölé (alapból a bal sávon van). |
| `autostart` | `true` esetén a program bejelentkezéskor magától elindul (ha a CyberHub is a géppel indul). |

Mentés után nyomd meg a `Ctrl+Alt+R`-t.

## További beállítások

| Mező | Jelentés |
|---|---|
| `language` | A felület nyelve: `"hu"` vagy `"en"`. A szövegek az `ui\i18n.js` fájlban vannak, új nyelv is felvehető. |
| `startWithWindows` | Induljon-e a CyberHub bejelentkezéskor. A fájl az irányadó: indításkor ehhez igazítja a Windows beállítását. |
| `swapSides` | `true` esetén az ikonsáv jobbra, az információs panel balra kerül. |
| `sections` | A jobb panel szekcióinak láthatósága (`claude`, `activity`, `clock`, `date`) és sorrendje (`order`). |
| `leftWidth`, `rightWidth` | A bal sáv és a jobb panel szélessége pixelben. |
| `frame.top`, `frame.bottom` | A felső és alsó keret vastagsága. A `0` kikapcsolja. |
| `weather.city` | A település neve az időjáráshoz (alapból Budapest). |
| `weather.latitude`, `weather.longitude` | Pontos koordináták; ha megadod, a város helyett ezt használja. |
| `weather.enabled` | Látszódjon-e az időjárás. |
| `activity.minutes` | A visszaszámláló alapértelmezett hossza percben (alapból 20). A panelen beírt érték felülírja. |
| `activity.autoStart` | Induláskor magától elkezdjen-e számolni. |
| `activity.sound` | Szóljon-e hanggal, amikor lejár. |
| `claudeUsage` | Látszódjon-e a Claude-használat. |
| `theme` | A fő (sárga), a kiegészítő (cián), a figyelmeztető (rózsaszín) és a háttérszín. |
| `wallpaper` | A beállításokban legutóbb választott háttérkép teljes útvonala. Ha a fájlban átírod, újratöltéskor a CyberHub beállítja a Windows háttérképének. |
| `hideTaskbar` | `false` esetén a Windows tálcája látható marad, és a keret fölötte helyezkedik el. |

A teljes megjelenés az `ui\hub.css` fájlban van, ezt szabadon átírhatod. Újratöltés után azonnal látszik.

A `preview.html` böngészőben megnyitva demóadatokkal mutatja a teljes képernyőt (egy egyszerű helyi webszerveren keresztül, mert a böngészők a `file://` címről nem töltik be a beágyazott részeket).

## Ha valami félremegy

Ha a program váratlanul leáll, és a tálca rejtve marad vagy a képernyő szélén üres sáv látszik, indítsd el így:

```
CyberHub.exe --restore
```

Ez visszaállítja a tálcát és felszabadítja a lefoglalt helyet. A következő normál indításkor ez magától is megtörténik.

Napló: `%LOCALAPPDATA%\CyberHub\cyberhub.log`, az előző futásé `cyberhub.prev.log`. Minden kattintás és menüparancs bekerül, így látszik, ha valami nem jut el a programig.

## Automatikus indítás

Nyomd meg a `Win+R`-t, írd be: `shell:startup`, és tegyél a megnyíló mappába egy parancsikont a `CyberHub.exe`-hez.

## Fordítás forrásból

Rust (MSVC) kell hozzá:

```
cargo build --release
```

Ha a projekt nagyon hosszú útvonalon van, a fordítás a Windows 260 karakteres korlátja miatt elakadhat. Ilyenkor állíts be rövidebb kimeneti mappát a `CARGO_TARGET_DIR` környezeti változóval.

Parancssori kapcsolók: `--quit`, `--restore`, `--keep-taskbar` (nem rejti el a tálcát), `--config <fájl>`, `--exit-after <mp>` (teszteléshez).
