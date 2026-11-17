[← Vissza a README-hez](../../README.hu.md) · [English](../en/configuration.md) · **Magyar**

# Testreszabás

## Programok hozzáadása

A `config.json` `apps` listájában minden sor egy ikon:

```json
{ "name": "Lightroom", "path": "C:\\Program Files\\Adobe\\Adobe Lightroom Classic\\Lightroom.exe", "icon": "photo" }
```

| Mező | Jelentés |
|---|---|
| `name` | Az ikon alatti felirat (a beállításokban elrejthető). |
| `path` | Exe, mappa vagy URI. A `%LOCALAPPDATA%`-hoz hasonló változók használhatók. Store-appokhoz: `shell:AppsFolder\<AppID>` (az AppID-t a PowerShell `Get-StartApps` parancsa listázza). |
| `args`, `cwd` | Indítási paraméterek és munkamappa (nem kötelező). |
| `icon` | Beépített ikon: `browser`, `globe`, `code`, `folder`, `panels`, `terminal`, `doc`, `sheet`, `music`, `photo`, `enhance`, `design`, `picker`, `video`, `mail`, `chat`, `game`, `rocket`, `notes`, `settings`, `calendar`, `cloud`, `spark`, `app`. Megadhatsz képfájlt is, például `icons/sajat.png` (az `ui` mappához képest) vagy teljes elérési utat. Ha üresen hagyod, a program neve alapján választ. |
| `match` | Exe-fájlnevek, amelyek ablakait ehhez az ikonhoz kell sorolni. Akkor kell, ha az indító és a futó program neve eltér (pl. `wt.exe` → `windowsterminal.exe`). |
| `matchTitle` | Ablakcím-részlet, ha a program exe alapján nem azonosítható. |
| `maximize` | `false` esetén nem nyitja teljes méretre az új ablakot (alapból `true`). |
| `color` | Az ikon finom színe, ami a programra utal, például `"#ff7139"`. |
| `autostart` | `true` esetén a program bejelentkezéskor magától elindul (ha a CyberShell is a géppel indul). |

Mentés után nyomd meg a `Ctrl+Alt+R`-t.

## További beállítások

| Mező | Jelentés |
|---|---|
| `language` | A felület nyelve: `"auto"` (alapérték: magyar Windowson magyar, lengyelen lengyel, minden másnál angol), `"en"`, `"hu"` vagy `"pl"`. A beállításokban választott nyelv felülírja az automatikusat. A szövegek az `ui\i18n.js` fájlban vannak, új nyelv is felvehető. |
| `startWithWindows` | Induljon-e a CyberShell bejelentkezéskor. A fájl az irányadó: indításkor ehhez igazítja a Windows beállítását. |
| `iconsEdge`, `infoEdge` | Az Ikonok és az információs panel helye: `"left"`, `"right"`, `"top"` vagy `"bottom"` (az Ikonok alapból `left`). Az `infoEdge` lehet `"auto"` is (ez az alapérték): laptopon (akkumulátoros gépen) `bottom`, mert oldalt a kisebb képernyőn nem fér ki minden, asztali gépen `right`. Nem lehetnek ugyanott. A régi `swapSides: true` beállítást a program magától `right`/`left`-re váltja. |
| `iconLabels` | `false` esetén az Ikonok sávján nem látszik a programok neve (alapból `true`). |
| `tooltips` | Súgó szövegek: `false` esetén sehol nem jelennek meg az elemek fölött időzve felbukkanó magyarázatok (alapból `true`). |
| `allMonitors` | `false` esetén a CyberShell csak az elsődleges képernyőn jelenik meg (alapból mindegyiken). |
| `calendar.feeds` | A csatolt naptárak (név, iCal-cím, szín). A naptárban is felvehetők és törölhetők. |
| `sections` | Az információs panel moduljainak láthatósága (`tools`, `claude`, `codex`, `activity`, `load`, `network`, `date`, `clock`, `laptop`, `weather`, `currency`, `music`, `volume`, `tray`, `power`; alapból rejtett a `claude`, a `codex`, az `activity`, a `load`, a `date` és a `currency`; a `laptop` csak akkumulátoros gépen látszik) és sorrendje (`order`). Ami a sorrendből hiányzik, az alapsorrend szerinti szomszédja mellé kerül. |
| `startMenu.winKey` | `true` (alapból): a `Win` billentyű a CyberShell Start menüjét, a `Win+S` a keresőjét nyitja meg. `false` esetén a Windows sajátja jön. |
| `startMenu.pinned` | A Start menübe kitűzött programok azonosítói (a menüben jobb gombbal kitűzve kerülnek ide). |
| `startMenu.webSearch` | A webes keresés címe, a `{q}` helyére kerül a keresett szöveg (alapból Google). |
| `startMenu.clipboard` | `true` (alapból): vágólap-előzmények a keresőben. `false` esetén a program nem figyeli a vágólapot. |
| `leftWidth`, `rightWidth` | Az Ikonok és az információs panel vastagsága pixelben (oldalt a szélessége, fent vagy lent a magassága). |
| `frame.top`, `frame.bottom`, `frame.left`, `frame.right` | A vékony keretek vastagsága az adott szélen (ahol épp nincs panel). A `0` kikapcsolja. |
| `weather.city` | A település neve az időjáráshoz (alapból Budapest; a beállításokban is megadható). |
| `weather.latitude`, `weather.longitude` | Pontos koordináták; ha megadod, a város helyett ezt használja. |
| `weather.wind` | Látszódjon-e a szél az Időjárás szekcióban (alapból `true`). |
| `weather.enabled` | Látszódjon-e az időjárás. |
| `activity.minutes` | A visszaszámláló alapértelmezett hossza percben (alapból 20). A panelen beírt érték felülírja. |
| `activity.autoStart` | Induláskor magától elkezdjen-e számolni. |
| `activity.sound` | Szóljon-e hanggal, amikor lejár. |
| `claudeUsage` | Látszódjon-e a Claude-használat. |
| `appearance` | A téma: `"dark"` (Cyberpunk 2077), `"light"` (Ghost in the Shell) `"auto"` (nappal világos, éjszaka sötét, az Időjárás helyének napkeltéje és napnyugtája szerint) vagy `"windows"` (a Windows világos/sötét módját követi, váltáskor azonnal). |
| `theme` | A sötét téma színei: fő (sárga), kiegészítő (cián), figyelmeztető (rózsaszín), háttér és szöveg (`text`). |
| `themeLight` | A világos téma színei, ugyanezekkel a kulcsokkal (türkiz, acélkék, narancsvörös, papírfehér háttér, sötét szöveg). |
| `health.enabled`, `health.minutes` | Az Egészség szünet-emlékeztető be- és kikapcsolása, illetve hossza percben (5–480, alapból 90). |
| `wallpaper` | A beállításokban legutóbb választott háttérkép teljes útvonala. Ha a fájlban átírod, újratöltéskor a CyberShell beállítja a Windows háttérképének. |
| `neonBorder` | Neonkeret: `"off"` (alapból), `"active"` (az előtérben lévő program körül a téma fő színével fénylő keret) vagy `"all"` (emellett a többi ablak kerete is a téma színét kapja, halványabban; Windows 11). |
| `windowsSync.darkMode` | `true` esetén a Windows sötét/világos módja (alkalmazások és rendszer) a CyberShell témájával együtt vált. |
| `windowsSync.accent` | `true` esetén a Windows kiemelőszíne a téma fő színe lesz. |
| `windowsSync.terminal` | `true` esetén a Windows Terminal kap egy `CyberShell Dark` és `CyberShell Light` színsémát, és az alapprofil a témával együtt vált köztük. |
| `hideTaskbar` | `false` esetén a Windows tálcája látható marad, és a keret fölötte helyezkedik el. |

A megjelenés az `ui\css` mappában van, komponensenként egy fájlban (ezeket szabadon átírhatod; újratöltés után azonnal látszik):

| Fájl | Mit tartalmaz |
|---|---|
| `base.css` | Színek és betűk (változók), közös csempe, a panelek neonvonala, közös animációk. |
| `strip.css` | A vékony keretek a képernyő szélén. |
| `icons.css` | Az Ikonok sávja. |
| `panel.css` | Az információs panel szekciói. |
| `peek.css`, `flyout.css`, `launcher.css`, `health.css` | Az ablakelőnézet, az oldalpanel (időjárás, naptár), a Start menü és kereső, a szünet-sáv. |
| `settings.css` | A beállítások oldal. |
| `horizontal.css` | A fent vagy lent álló panelek vízszintes elrendezése. |
| `light.css` | A világos téma (Ghost in the Shell). |

A fájlok sorrendje számít (a `ui\hub.html` ebben a sorrendben tölti be őket): a vízszintes elrendezés és a világos téma a végén van, mert ezek írják felül az alapot.
