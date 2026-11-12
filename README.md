# CyberShell

> A v1.3-tól CyberShell (korábban CyberHub). Az első indításkor a program szabályosan leállítja a még futó CyberHubot, átveszi az adatmappáját (a claude.ai-belépéssel együtt) és az „Indulás a géppel” beállítását.

Cyberpunk stílusú felület Windowsra. Bal oldalt az Ikonok, jobb oldalt az információs panel, felül és alul vékony neonkeret. A programok középen nyílnak meg, és teljes méretben kitöltik a maradék helyet.

Több képernyőnél mindegyiken megjelenik a teljes felület (Ikonok, információs panel, keretek); képernyő csatlakoztatásakor vagy leválasztásakor magától igazodik. A sávok ugyanúgy foglalják le a képernyő szélét, mint a Windows tálcája, ezért a teljes méretre nyitott ablakok nem lógnak rájuk. Amíg a CyberShell fut, a gyári tálca rejtve marad. Kilépéskor minden visszaáll.

## Indítás és kilépés

- **Indítás:** `CyberShell.exe`. A `config.json` és az `ui` mappa legyen mellette.
- **Kilépés:** jobb alul **// MENÜ** → **Kilépés a CyberShellből**, vagy `Ctrl+Alt+Q`, vagy parancssorból `CyberShell.exe --quit`.
- **Újratöltés:** `Ctrl+Alt+R`, vagy MENÜ → **Újratöltés**. Ilyenkor a program újraolvassa a `config.json`-t és a felület fájljait.
- **Indulás a géppel:** MENÜ → **Indulás a géppel** kapcsoló. Bekapcsolva a CyberShell bejelentkezéskor magától elindul: megvárja, amíg az Intéző elkészül, és csak utána foglalja le a képernyő szélét. Minden indításkor frissíti a bejegyzést, így a mappa áthelyezése után is jó helyről indul. A Windows *Indítási alkalmazások* listájában (Feladatkezelő, Beállítások) is engedélyezi – az újabb Windows 11-ek enélkül nem indítják el. Ha ott letiltod, a CyberShell is kikapcsoltnak veszi.

## Beállítások (MENÜ)

A **// MENÜ** gomb a teljes munkaterületet kitöltő beállítási felületet nyit meg; az oldalsó panelek és a keretek közben is látszanak. Bezárni a **Bezárás** gombbal, az `Esc` billentyűvel vagy a **// MENÜ** gomb újbóli megnyomásával lehet. Amíg nyitva van, a MENÜ gomb sárgán világít.

- **Rendszerterhelés:** a processzor és a memória kihasználtsága, az utolsó egy perc görbéjével. 60% fölött narancs, 85% fölött rózsaszín.

- **Készítő:** a verziószám mellett a **GitHub** ikon a projekt oldalát (github.com/gergo85/cybershell), a **Weboldal** gomb a készítő oldalát nyitja meg.
- **Rendszer:** újratöltés (a `config.json` újraolvasása kézi szerkesztés után, és az összes felület újratöltése – ugyanaz, mint a Ctrl+Alt+R), a konfigurációs fájl megnyitása, indulás a géppel, a Win billentyű, kilépés. Ha nyitott beállítások mellett egy programikonra, a naptárra vagy az időjárásra kattintasz, a beállítások bezárulnak.
- **Látható szekciók:** az információs panel Claude, Aktivitás, Dátum, Idő, Időjárás, Zene és Hangerő része külön-külön ki- és bekapcsolható, és mindegyik sorrendje a sor elején lévő jellel húzva állítható. Alapból a Dátum van felül, alatta az Idő, alul az Időjárás, a Zene és a Hangerő. A Keresés és az Indítás mindig legelöl, a Főkapcsoló mindig a végén van; a Laptop szekció mindig közvetlenül az Időjárás előtt. A Rendszer kártya e kártya alatt van.
- **Saját Start menü:** bekapcsolva (alapból) a `Win` billentyű és az információs panel **Indítás** gombja a CyberShell Start menüjét, a `Win+S` és a **Keresés** gomb a keresőjét nyitja meg. Kikapcsolva a billentyűk és a gombok is a Windows saját Start menüjét és keresőjét nyitják. A többi Win-kombináció (`Win+E`, `Win+L`, `Win+Shift+S`…) változatlanul működik. Rendszergazdaként futó ablak előtt a Windows nem engedi elfogni a billentyűt, ott a gyári Start menü nyílik meg.
- **Megjelenés:** a **Téma** (Sötét, Világos vagy Auto), a **Nyelv** (magyar vagy angol; első induláskor a Windows nyelvéhez igazodik), valamint az Ikonok és a panel helye. A **Sötét** téma a megszokott, Cyberpunk 2077 hangulatú neonsárga–cián–rózsaszín. A **Világos** a Ghost in the Shell látványvilágát idézi: hűvös, papírfehér felületek finom műszaki rácshálóval, türkiz és acélkék vonalak, narancsvörös jelzések, sötét tintaszínű szöveg, a neonfény helyett éles vonalak. Más a formavilága is: a levágott sarkok helyett lekerekített, vékony vonalas ikonkeretek és gombok, a képernyő keretén a veszélycsíkok és a trapéz helyett pontsorok, kis gyűrűk és vonalkázott „adatkapszula”. Az **Auto** napszak szerint vált: napkeltétől napnyugtáig világos, éjszaka sötét. A napkeltét és a napnyugtát az Időjárás szekció helyére számolja (amíg az nem ismert, 7 és 19 óra között világos). A téma azonnal, újratöltés nélkül vált minden felületen.
- **Egészség:** szünetre figyelmeztet, ha sokáig dolgozol egyfolytában. A **Szünet emlékeztető** kapcsolóval kapcsolható be (alapból ki), az **Időtartam** percben állítható (5–480, alapból 90). Ha letelik, a munkaterület tetején, középen egy piros sáv jelenik meg, hogy ideje szünetet tartani; a **Bezárás** gombjával eltűnik, és a visszaszámlálás elölről indul. A hátralévő idő csak itt, a beállításokban látszik (a ↻ gomb újraindítja). Ha legalább 5 percig nem nyúlsz a géphez (pl. zárolt gép, ebédszünet), az is szünetnek számít: a visszaszámlálás magától elölről indul.
- **A panelek helye:** az **Ikonok helye** és a **Panel helye** sorban külön-külön választható, hogy az Ikonok és az információs panel a képernyő melyik szélén legyen: **Bal**, **Jobb**, **Fent** vagy **Lent**. A kettő nem lehet ugyanott: amelyik szélen a másik panel áll, az a mező szürke, nem választható. A megmaradt két szélen vékony keret fut. Fent és lent a panelek tartalma sorba rendeződik (az információs panel szekciói egymás mellé kerülnek), és a felugró ablakok (naptár, időjárás, ablakelőnézet, Start menü) is a panel alá, illetve fölé nyílnak. Ha két szomszédos szélt választasz (pl. bal és fent), a sarkot az Ikonok kapják.
- **Ikonok:** az Ikonok és az információs panel programjai. Sorrend a ▲▼ gombokkal, átnevezés a névmezőben, ikon és szín választható, a **Sáv/Panel** kapcsoló dönti el, melyiken jelenjen meg, az **Auto** oszlop bekapcsolásával a program bejelentkezéskor magától elindul (ha a CyberShell is a géppel indul), a kuka ikon törli. Új programot a **Program hozzáadása…** gombbal (exe vagy parancsikon), vagy egy futó program nevére kattintva adhatsz hozzá. A változások a **Mentés** gombbal lépnek életbe.
- **Háttérkép:** a `wallpapers` mappa képei kis képként. Kattintásra a CyberShell beállítja a Windows háttérképének (kitöltő elrendezéssel, minden monitoron); az éppen használt kép sárga keretet és „AKTÍV” címkét kap. A **Kép hozzáadása…** gombbal tetszőleges képet (jpg, png, bmp) választhatsz: a program bemásolja a mappába, és rögtön be is állítja. A **Mappa megnyitása** gomb az Intézőben mutatja a mappát; az oda tett képek a következő megnyitáskor megjelennek.

Indításkor a CyberShell ellenőrzi, hogy a programok telepítve vannak-e még. Ami már nincs a gépen, annak az ikonja nem jelenik meg; a beállításokban „NINCS TELEPÍTVE” jelzéssel látszik, onnan törölheted.

A CyberShell mellett egy kis őrfolyamat is fut (a Feladatkezelőben második `CyberShell.exe`-ként látszik). Ha a fő program bármi módon leáll, például a Feladatkezelőből bezárják, az őr visszaadja a tálcát és felszabadítja a képernyő szélét, aztán ő is kilép.

## Ikonok (alapból bal oldalt)

| Művelet | Mit csinál |
|---|---|
| Kattintás | Elindítja a programot, vagy előhozza, ha már fut. |
| Kattintás az éppen aktív programra | Kis méretre teszi; ha több ablaka van, a következőre vált. |
| Shift+kattintás vagy középső gomb | Új példányt indít. |

A cián jelzés az ikon mellett azt mutatja, hogy a program fut. Az aktív programnál a jelzés sárga és hosszabb, a csempe kerete kivilágosodik. Ha a programnak több ablaka van nyitva, a jelzés mellett egy második, rövidebb csík is látszik.

**Ablakelőnézet:** ha egy programnak több ablaka van nyitva, és az ikonja fölött egy pillanatig időzöl, az ikon mellett egymás mellett megjelenik az ablakok élő kisképe. Egy kisképre kattintva az az ablak jön elő; a kártya jobb felső sarkában lévő **×** (vagy a középső egérgomb) bezárja az ablakot. A kis méretre tett ablakokról a Windows nem ad képet, ezeknél a program ikonja és a „KIS MÉRETBEN” felirat látszik. Ha az egér elhagyja az ikont és az előnézetet, az magától eltűnik; ha már látszik, egy másik több ablakos programra lépve azonnal átvált.

**Futó programok:** ami fut, de nincs kitűzve, az ikonok alatt jelenik meg egy cián elválasztó után, a program saját ikonjával és nevével (a hosszabb név két sorba törik). Ugyanúgy működik, mint a kitűzött ikonok.

**Fájlhúzás:** ha egy fájlt egy ikon fölé húzol, és ott tartod egy pillanatig, a program ablaka előjön, és beleejtheted a fájlt. Ha magára az ikonra ejted, a program megnyitja a fájlt.

## Információs panel (alapból jobb oldalt)

A panel fentről lefelé: Keresés és Indítás, Claude, Aktivitás, majd alul a dátum és az idő, laptopon a Laptop szekció, az időjárás, a zene, a hangerő, a főkapcsoló és a menü. Ha nem fér ki minden (kisebb képernyő, laptop), a panel magától összébb húzza a szekciók közti réseket.

- **Keresés és Indítás:** a panel tetején. Mindkettő a CyberShell saját, a felület stílusához igazodó ablakát nyitja meg (lásd lent: *Start menü és kereső*). Nyitva a gomb sárgán világít; újabb kattintásra, `Esc`-re vagy máshova kattintva bezárul.
- **Zene:** a Spotify asztali alkalmazás lejátszása, a Hangerő fölött. A borító a szekció háttere; alatta a szám címe (a hosszú cím lassan körbefut), az előadó, a haladásjelző (kattintással tekerhető, fölötte időzve az idő is látszik), valamint négy gomb két sorban: **véletlenszerű sorrend** (bekapcsolva sárga) és **lejátszás/szünet**, alattuk **előző** és **következő szám**. A borítóra vagy a címre kattintva előjön a Spotify. Ha a Spotify nem fut, egy **SPOTIFY** gomb indítja el. Spotify-fiók összekötése nem kell: a Spotify alkalmazás maga jelenti a Windows médiavezérlőjének, mit játszik (ugyanezt mutatja a Windows hangerő-felugró ablaka is), a CyberShell ezt olvassa és ezen át vezérel. Más lejátszókat (böngésző, filmlejátszó) nem figyel. A szekció fölött görgetve a hangerő változik.
- **Idő:** az óra és a perc egy sorban, alattuk a másodpercek 12 szakaszos sávja.
- **Dátum:** külön szekcióban, számokkal: az év, alatta a hónap és a nap, alattuk a hét sorszáma (ISO-hét, pl. „46. HÉT”). Rákattintva a panel mellett, a szekció magasságában középre igazítva kinyílik a **naptár**: havi nézet a hetek sorszámával, a napokon eseményenként egy színes pötty, alatta a kiválasztott nap eseményei. A hónapok a nyilakkal vagy görgetéssel lapozhatók, a **MA** gomb visszaugrik a mai napra.
- **Google Naptár:** a naptár alján csatolható. A Google Naptár beállításaiban válaszd ki a naptárad, a *Naptár integrálása* résznél másold ki a **Titkos cím iCal formátumban** címet, és illeszd be. Több naptár is csatolható (+ gomb), mindegyik saját színt kap; eltávolítani a név melletti ×-re kétszer kattintva lehet. A CyberShell tízpercenként, illetve a frissítés gombra tölti le őket; az ismétlődő események is látszanak. A **névnapok** (évente ismétlődő, egész napos bejegyzés, aminek a címe csak keresztnév, pl. „Rózsa” vagy „Konrád, Zelmira”) nem kapnak pöttyöt, a nap listájában külön sorban látszanak. A címet a `config.json` `calendar` része tárolja – titkos, ne oszd meg. A naptárat csak olvassa, módosítani nem tud.
- **Hangerő:** a Windows alapértelmezett hangeszközének hangereje. A szekció fölött görgetve 2%-onként állítható, a csúszka kattintható és húzható, a hangszóró ikon némít. A százalékra kattintva megnyílik a Windows hangerőkeverője. Ha máshol (pl. a billentyűzettel) állítod, a panel is követi.
- **Főkapcsoló:** Zárolás, Alvó állapot, Leállítás, Újraindítás. A leállítás és az újraindítás megerősítést kér; előtte a CyberShell mindent visszaállít.
- **Claude:** az 5 órás és a heti használati korlát százalékban, valamint az, hogy mennyi idő múlva áll vissza (↻). 60% fölött cián, 85% fölött rózsaszín a sáv. Ötpercenként frissül, a frissítés idejére kattintva azonnal.
- **Aktivitás:** visszaszámláló, alapból 20 perces. A program indulásakor nem indul magától (a ▶ gombbal indítod); ha mégis így szeretnéd, a `config.json`-ban állítsd az `activity.autoStart` értékét `true`-ra. Ha lejár, a panel és a keretek rózsaszínre váltanak és villognak, a gép pedig hangot ad. **✓:** új kör indul. **+5:** halasztás öt perccel. A három kis gomb: szünet/folytatás, újraindítás és beállítás. A beállításnál tetszőleges hosszt beírhatsz percben (1–240), ezt a program megjegyzi.
- **Laptop:** csak akkumulátoros gépen jelenik meg, az időjárás fölött. A **Wi-Fi** gomb a Windows saját hálózatlistáját nyitja meg, az akkumulátor gomb (töltöttség, töltés jelzése) a Windows Energiagazdálkodás és akkumulátor beállításait. 20% alatt rózsaszínre vált.
- **Időjárás:** a panel alján, az Open-Meteo ingyenes szolgáltatásából, negyedóránként frissül. Az ikon, a hőmérséklet, a leírás és a szél (iránnyal) látszik. Rákattintva a panel mellett, a szekció magasságában középre igazítva kinyílik a részletes előrejelzés: a következő 24 óra háromóránként (hőmérséklet-görbe, csapadék esélye), hőérzet, páratartalom, szél és széllökés, a mai csapadék, UV-index, napkelte és napnyugta, valamint a következő három nap. Bezárul, ha máshova kattintasz, az `Esc`-re vagy újabb kattintásra. A várost a `config.json` `weather` részében állíthatod be.

### Start menü és kereső

Két elrendezés, közös keresővel. Ötleteket a nyílt forráskódú Windows-indítókból vettünk (PowerToys Run, Flow Launcher, Open-Shell, Seelen UI): ékezet nélküli és betűszavas keresés, számológép, parancsfuttatás, rendszergazdaként indítás billentyűvel, a gyakran használt programok előresorolása.

- **Start menü** (`Win`, vagy az Indítás gomb): az információs panel mellett nyílik. Bal oldalt a **kitűzött** programok csempéi (húzással rendezhetők), a **gyakran használt** programok és a **legutóbbi fájlok**; jobb oldalt **minden alkalmazás** betűrendben, a szélén betűugróval. Ugyanaz a lista, amit a Windows Start menüje „Minden alkalmazás” néven mutat, a Store-alkalmazásokkal együtt; a súgók, eltávolítók és weboldal-hivatkozások kimaradnak. Alul a teljes neved (ahogy a Windows bejelentkező képernyője mutatja), mellette két ikon: a Windows **fiókbeállításai** és a **bejelentkezési lehetőségek**. Mellettük gyorsgombok (Dokumentumok, Letöltések, Képek, Ez a gép, Windows beállítások) és a főkapcsoló: zárolás, kijelentkezés, alvás, újraindítás, leállítás (a kijelentkezés, az újraindítás és a leállítás második kattintásra indul).
- **Kereső** (`Win+S`, vagy a Keresés gomb): egyetlen keresősáv a képernyő közepén, alján a billentyűparancsokkal. Üresen a gyakran használt programokat (két oszlopban), a vágólapot és a legutóbbi fájlokat mutatja. A vágólap és a fájlok tételei mellett kuka ikon van, a csoportcímben **Összes törlése** (második kattintásra törli). A legutóbbi fájloknál csak a listáról kerülnek le (a Windows listájából is), maguk a fájlok megmaradnak.
- **Gépelés:** bármelyik elrendezésben azonnal keres. A találatok csoportjai: programok, rendszer (zárolás, alvás, újraindítás, leállítás, CyberShell beállítások, lomtár), Windows beállítások és rendszereszközök (pl. „wifi”, „háttér”, „eszközkezelő”), legutóbbi fájlok, vágólap, végül keresés a weben. A legjobban illeszkedő csoport kerül előre. Ékezet nélkül is talál („fenykep” → Fényképek), szókezdetre, betűszóra („vsc” → Visual Studio Code) és elgépelésre is.
- **Számológép:** egy kifejezés (pl. `12*(3+4)/2`, `2^10`, `15%*80`, `sqrt(2)`) eredménye azonnal látszik; `Enter` a vágólapra másolja.
- **Parancs:** `>` után úgy fut, mint a Futtatás ablakban (pl. `> cmd`, `> notepad jegyzet.txt`). Mappaútvonal (`C:\…`, `%APPDATA%`) és webcím (`github.com`) közvetlenül is megnyitható.
- **Vágólap-előzmények:** az utolsó 30 másolt szöveg. Üres keresőben a legutóbbi öt látszik, gépeléskor a szövegükben is keres, `v:` után pedig csak ezeket mutatja (pl. `v: számla`). `Enter` beilleszti abba az ablakba, ahonnan a menüt megnyitottad, `Ctrl+Enter` csak a vágólapra teszi; jobb gombbal egy elem vagy az egész lista törölhető. Csak a memóriában tárolódik (bezáráskor elvész), és amit a jelszókezelők rejtettnek jelölnek, az nem kerül bele. A beállításokban (Rendszer) kikapcsolható.
- **Billentyűk:** `↑` `↓` választás, `Tab` a következő csoport, `Enter` megnyitás, `Ctrl+Shift+Enter` futtatás rendszergazdaként, `Ctrl+Enter` a fájl helye, `Esc` törli a keresést, másodszorra bezár.
- **Jobb gomb** egy programon: kitűzés a Start menübe (vagy levétel), **hozzáadás az Ikonokhoz** (rögtön ikon lesz belőle), futtatás rendszergazdaként, a fájl helyének megnyitása.
- A gyakran indított programok a keresésben is előrébb kerülnek. A webes keresés címe a `config.json` `startMenu.webSearch` értékével állítható.

### A Claude-használat forrása

Három forrásból dolgozik, ebben a sorrendben:

1. **A claude.ai-fiókod (ajánlott, az asztali Claude-hoz is jó).** A panelen a **Belépés** gombbal megnyílik egy kis ablak, ahol bejelentkezel a claude.ai-ra. Belépés után az ablak magától bezárul, és a CyberShell ötpercenként lekéri ugyanazokat a korlátokat, amelyeket a claude.ai is mutat. A bejelentkezés a CyberShell saját böngészőprofiljában marad (`%LOCALAPPDATA%\CyberShell\WebView2`), az asztali Claude apphoz nem nyúl. A belépést 30 napig megjegyzi, így újraindítás után sem kell újra belépni (hacsak a claude.ai maga ki nem léptet). Ha induláskor a claude.ai még nem érhető el (pl. nincs még hálózat), a panel „Kapcsolódás…” állapotot mutat, és fél percen belül újrapróbálja – belépést csak akkor kér, ha a claude.ai valóban kijelentkezett állapotot jelez. Ha valaha kijelentkeznél, töröld ezt a mappát, amíg a CyberShell nem fut. A lekérdezés nem hivatalos, egy későbbi claude.ai-változás után előfordulhat, hogy nem működik.

2. **A Claude Code állapotsora.** A Claude Code minden válasz után átadja az állapotsornak az 5 órás és a heti korlátot. Ha a CyberShell az állapotsor, ezeket elmenti, és a panel kiírja. Beállítás a `%USERPROFILE%\.claude\settings.json` fájlban:

   ```json
   "statusLine": { "type": "command", "command": "\"C:\\Users\\<név>\\OneDrive\\Dokumentumok\\GitHub\\cybershell\\CyberShell.exe\" --statusline" }
   ```

   Ilyenkor a terminálban a Claude Code alján is megjelenik egy rövid sor, például: `Opus · 5h 12% · hét 40%`. Az adat a legutóbbi Claude Code-válasz idejét tükrözi; ha az 5 órás ablak azóta lejárt, a panel „—” jelet mutat.

3. **Közvetlen lekérdezés** a Claude Code bejelentkezésével (`%USERPROFILE%\.claude\.credentials.json`), ha a token olvasható formában van benne. Az újabb Claude Code-változatok ezt már nem így tárolják, ezért ez többnyire nem elérhető.

A második módszer nem hivatalosan dokumentált, ezért egy későbbi változás után előfordulhat, hogy nem működik.

A CyberShell a bejelentkezést csak olvassa, nem frissíti, és a tokent sehova nem írja ki. Ha a panelen az áll, hogy még nincs adat, indíts el egy terminálban egy Claude Code-munkamenetet (`claude`), és küldj egy üzenetet: az első válasz után a panel 30 másodpercen belül frissül. Ha a funkcióra nincs szükséged, a `config.json`-ban állítsd a `claudeUsage` értékét `false`-ra.

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
| `place` | `"right"` esetén az ikon az információs panelre kerül, a Keresés és az Indítás gomb mellé (alapból az Ikonok között van). |
| `autostart` | `true` esetén a program bejelentkezéskor magától elindul (ha a CyberShell is a géppel indul). |

Mentés után nyomd meg a `Ctrl+Alt+R`-t.

## További beállítások

| Mező | Jelentés |
|---|---|
| `language` | A felület nyelve: `"auto"` (alapérték: magyar Windowson magyar, minden másnál angol), `"hu"` vagy `"en"`. A beállításokban választott nyelv felülírja az automatikusat. A szövegek az `ui\i18n.js` fájlban vannak, új nyelv is felvehető. |
| `startWithWindows` | Induljon-e a CyberShell bejelentkezéskor. A fájl az irányadó: indításkor ehhez igazítja a Windows beállítását. |
| `iconsEdge`, `infoEdge` | Az Ikonok és az információs panel helye: `"left"`, `"right"`, `"top"` vagy `"bottom"` (alapból `left` és `right`). Nem lehetnek ugyanott. A régi `swapSides: true` beállítást a program magától `right`/`left`-re váltja. |
| `allMonitors` | `false` esetén a CyberShell csak az elsődleges képernyőn jelenik meg (alapból mindegyiken). |
| `calendar.feeds` | A csatolt naptárak (név, iCal-cím, szín). A naptárban is felvehetők és törölhetők. |
| `sections` | Az információs panel szekcióinak láthatósága (`claude`, `activity`, `date`, `clock`, `weather`, `music`, `volume`) és sorrendje (`order`). Ami a sorrendből hiányzik, a végére kerül. |
| `startMenu.winKey` | `true` (alapból): a `Win` billentyű a CyberShell Start menüjét, a `Win+S` a keresőjét nyitja meg. `false` esetén a Windows sajátja jön. |
| `startMenu.pinned` | A Start menübe kitűzött programok azonosítói (a menüben jobb gombbal kitűzve kerülnek ide). |
| `startMenu.webSearch` | A webes keresés címe, a `{q}` helyére kerül a keresett szöveg (alapból Google). |
| `startMenu.clipboard` | `true` (alapból): vágólap-előzmények a keresőben. `false` esetén a program nem figyeli a vágólapot. |
| `leftWidth`, `rightWidth` | Az Ikonok és az információs panel vastagsága pixelben (oldalt a szélessége, fent vagy lent a magassága). |
| `frame.top`, `frame.bottom`, `frame.left`, `frame.right` | A vékony keretek vastagsága az adott szélen (ahol épp nincs panel). A `0` kikapcsolja. |
| `weather.city` | A település neve az időjáráshoz (alapból Budapest). |
| `weather.latitude`, `weather.longitude` | Pontos koordináták; ha megadod, a város helyett ezt használja. |
| `weather.enabled` | Látszódjon-e az időjárás. |
| `activity.minutes` | A visszaszámláló alapértelmezett hossza percben (alapból 20). A panelen beírt érték felülírja. |
| `activity.autoStart` | Induláskor magától elkezdjen-e számolni. |
| `activity.sound` | Szóljon-e hanggal, amikor lejár. |
| `claudeUsage` | Látszódjon-e a Claude-használat. |
| `appearance` | A téma: `"dark"` (Cyberpunk 2077), `"light"` (Ghost in the Shell) vagy `"auto"` (nappal világos, éjszaka sötét, az Időjárás helyének napkeltéje és napnyugtája szerint). |
| `theme` | A sötét téma színei: fő (sárga), kiegészítő (cián), figyelmeztető (rózsaszín), háttér és szöveg (`text`). |
| `themeLight` | A világos téma színei, ugyanezekkel a kulcsokkal (türkiz, acélkék, narancsvörös, papírfehér háttér, sötét szöveg). |
| `health.enabled`, `health.minutes` | Az Egészség szünet-emlékeztető be- és kikapcsolása, illetve hossza percben (5–480, alapból 90). |
| `wallpaper` | A beállításokban legutóbb választott háttérkép teljes útvonala. Ha a fájlban átírod, újratöltéskor a CyberShell beállítja a Windows háttérképének. |
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

### A forráskód felépítése

- `ui\hub.html`: a felület váza; `ui\core.js`: közös segédek; `ui\parts\*.js`: felületenként egy fájl (Ikonok, panel, keretek, beállítások, Start menü, oldalpanel, előnézet, szünet-sáv); `ui\hub.js`: az indító, ami a cím szerint választ közülük. A szövegek az `ui\i18n.js`-ben, a vonalikonok az `ui\icons.js`-ben vannak.
- `src\main.rs`: a belépési pont és az eseményhurok; `src\web.rs`: a felületek kiszolgálása és a felugró ablakok; `src\lifecycle.rs`: indulás, leállás, átállás a régi névről.
- `src\hub.rs` és `src\hub\`: a program központi állapota, témák szerint: elrendezés (`layout`), Ikonok (`icons`), panel (`panel`), Claude (`claude`), beállítások (`settings`), előnézet (`preview`), oldalpanel (`flyout`), Start menü (`start`), Egészség (`health`), téma (`theme`).
- A többi modul a Windows-szal beszélget: sávok (`appbar`), ablakok (`wintrack`), programlista (`apps`), Win billentyű (`keyhook`), vágólap (`clipboard`), Spotify (`media`), rendszer (`sys`), indulás a géppel (`autostart`), beállítások (`config`), Claude-használat (`usage`), ablakképek (`peek`).

A `preview.html` böngészőben megnyitva demóadatokkal mutatja a teljes képernyőt (egy egyszerű helyi webszerveren keresztül, mert a böngészők a `file://` címről nem töltik be a beágyazott részeket). Az elrendezés is kipróbálható: `preview.html?icons=top&info=bottom` (az Ikonok és a panel helye). A Start menü és a kereső külön is megnézhető: `ui/hub.html?part=launcher&mode=start`, illetve `ui/hub.html?part=launcher&mode=search&q=fenykep`.

## Ha valami félremegy

Ha a program váratlanul leáll, és a tálca rejtve marad vagy a képernyő szélén üres sáv látszik, indítsd el így:

```
CyberShell.exe --restore
```

Ez visszaállítja a tálcát és felszabadítja a lefoglalt helyet. A következő normál indításkor ez magától is megtörténik.

Napló: `%LOCALAPPDATA%\CyberShell\cybershell.log`, az előző futásé `cybershell.prev.log`. Minden kattintás és menüparancs bekerül, így látszik, ha valami nem jut el a programig.

## Automatikus indítás

Nyomd meg a `Win+R`-t, írd be: `shell:startup`, és tegyél a megnyíló mappába egy parancsikont a `CyberShell.exe`-hez.

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
