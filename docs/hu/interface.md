[← Vissza a README-hez](../../README.hu.md) · [English](../en/interface.md) · **Magyar**

# Felület: az Ikonok sáv és az információs panel

Cyberpunk stílusú felület Windowsra. Bal oldalt az Ikonok, jobb oldalt az információs panel, felül és alul vékony neonkeret. A programok középen nyílnak meg, és teljes méretben kitöltik a maradék helyet.

Több képernyőnél mindegyiken megjelenik a teljes felület (Ikonok, információs panel, keretek); képernyő csatlakoztatásakor vagy leválasztásakor magától igazodik. A sávok ugyanúgy foglalják le a képernyő szélét, mint a Windows tálcája, ezért a teljes méretre nyitott ablakok nem lógnak rájuk. Amíg a CyberShell fut, a gyári tálca rejtve marad. Kilépéskor minden visszaáll.

## Ikonok (alapból bal oldalt)

| Művelet | Mit csinál |
|---|---|
| Kattintás | Elindítja a programot, vagy előhozza, ha már fut. |
| Kattintás az éppen aktív programra | Kis méretre teszi; ha több ablaka van, a következőre vált. |
| Shift+kattintás vagy középső gomb | Új példányt indít. |
| Jobb kattintás | Ha a programnak pontosan egy ablaka van nyitva, kis menü, mint a Windows tálcáján: **Teljes képernyőre** (ha az ablak még nincs úgy) és **Ablak bezárása**. |

A cián jelzés az ikon mellett azt mutatja, hogy a program fut. Az aktív programnál a jelzés sárga és hosszabb, a csempe kerete kivilágosodik. Ha a programnak több ablaka van nyitva, a jelzés mellett egy második, rövidebb csík is látszik.

**Ablakelőnézet:** ha egy programnak több ablaka van nyitva, és az ikonja fölött egy pillanatig időzöl, az ikon mellett egymás mellett megjelenik az ablakok élő kisképe. Egy kisképre kattintva az az ablak jön elő; a kártya jobb felső sarkában lévő **×** (vagy a középső egérgomb) bezárja az ablakot. A kis méretre tett ablakokról a Windows nem ad képet, ezeknél a program ikonja és a „KIS MÉRETBEN” felirat látszik. Ha az egér elhagyja az ikont és az előnézetet, az magától eltűnik; ha már látszik, egy másik több ablakos programra lépve azonnal átvált.

**Futó programok:** ami fut, de nincs kitűzve, az ikonok alatt jelenik meg egy cián elválasztó után, a program saját ikonjával és nevével (a hosszabb név két sorba törik). Ugyanúgy működik, mint a kitűzött ikonok.

**Fájlhúzás:** ha egy fájlt egy ikon fölé húzol, és ott tartod egy pillanatig, a program ablaka előjön, és beleejtheted a fájlt. Ha magára az ikonra ejted, a program megnyitja a fájlt.

## Információs panel (alapból jobb oldalt)

A panel fentről lefelé: Start, Eszköztár, Claude, Aktivitás, majd alul a dátum és az idő, laptopon a Laptop szekció, az időjárás, a zene, a hangerő, a főkapcsoló és a menü. Ha nem fér ki minden (kisebb képernyő, laptop), a panel magától összébb húzza a szekciók közti réseket; ha így sem fér ki, a panel görgethető lesz (egérgörgővel), a Start és a MENÜ gomb pedig mindig a helyén marad.

- **Start:** a panel tetején, ugyanolyan gomb, mint alul a MENÜ. A CyberShell saját Start menüjét nyitja meg (lásd: [Start menü és kereső](start-menu.md)). Nyitva a gomb sárgán világít; újabb kattintásra, `Esc`-re vagy máshova kattintva bezárul.
- **Eszköztár:** négy ikon két sorban: **Számológép**, **Jegyzettömb**, **Keresés** (a CyberShell keresője) és **Képmetsző** (a Windows Képmetszője). Az ikon fölött időzve a súgó mondja meg, melyik mi.
- **Zene:** a Spotify asztali alkalmazás lejátszása, a Hangerő fölött. A borító a szekció háttere; alatta a szám címe (a hosszú cím lassan körbefut), az előadó, a haladásjelző (kattintással tekerhető, fölötte időzve az idő is látszik), valamint négy gomb két sorban: **véletlenszerű sorrend** (bekapcsolva sárga) és **lejátszás/szünet**, alattuk **előző** és **következő szám**. A borítóra vagy a címre kattintva előjön a Spotify. Ha a Spotify nem fut, egy **SPOTIFY** gomb indítja el. Spotify-fiók összekötése nem kell: a Spotify alkalmazás maga jelenti a Windows médiavezérlőjének, mit játszik (ugyanezt mutatja a Windows hangerő-felugró ablaka is), a CyberShell ezt olvassa és ezen át vezérel. Más lejátszókat (böngésző, filmlejátszó) nem figyel. A szekció fölött görgetve a hangerő változik.
- **Idő:** az óra és a perc egy sorban, alattuk a másodpercek 12 szakaszos sávja.
- **Dátum:** külön szekcióban, számokkal: az év, alatta a hónap és a nap, alattuk a hét sorszáma (ISO-hét, pl. „46. HÉT”). Rákattintva a panel mellett, a szekció magasságában középre igazítva kinyílik a **naptár**: havi nézet a hetek sorszámával, a napokon eseményenként egy színes pötty, alatta a kiválasztott nap eseményei. A hónapok a nyilakkal vagy görgetéssel lapozhatók, a **MA** gomb visszaugrik a mai napra.
- **Google Naptár:** a naptár alján csatolható. A Google Naptár beállításaiban válaszd ki a naptárad, a *Naptár integrálása* résznél másold ki a **Titkos cím iCal formátumban** címet, és illeszd be. Több naptár is csatolható (+ gomb), mindegyik saját színt kap; eltávolítani a név melletti ×-re kétszer kattintva lehet. A CyberShell tízpercenként, illetve a frissítés gombra tölti le őket; az ismétlődő események is látszanak. A **névnapok** (évente ismétlődő, egész napos bejegyzés, aminek a címe csak keresztnév, pl. „Rózsa” vagy „Konrád, Zelmira”) nem kapnak pöttyöt, a nap listájában külön sorban látszanak. A címet a `config.json` `calendar` része tárolja – titkos, ne oszd meg. A naptárat csak olvassa, módosítani nem tud.
- **Hangerő:** a Windows alapértelmezett hangeszközének hangereje. A szekció fölött görgetve 2%-onként állítható, a csúszka kattintható és húzható, a hangszóró ikon némít. A százalékra kattintva megnyílik a Windows hangerőkeverője. Ha máshol (pl. a billentyűzettel) állítod, a panel is követi.
- **Főkapcsoló:** Zárolás, Alvó állapot, Leállítás, Újraindítás. A leállítás és az újraindítás megerősítést kér; előtte a CyberShell mindent visszaállít.
- **Claude:** az 5 órás és a heti használati korlát százalékban, valamint az, hogy mennyi idő múlva áll vissza (↻). 60% fölött cián, 85% fölött rózsaszín a sáv. Ötpercenként frissül, a frissítés idejére kattintva azonnal.
- **Aktivitás:** visszaszámláló, alapból 20 perces. A program indulásakor nem indul magától (a ▶ gombbal indítod); ha mégis így szeretnéd, a `config.json`-ban állítsd az `activity.autoStart` értékét `true`-ra. Ha lejár, a panel és a keretek rózsaszínre váltanak és villognak, a gép pedig hangot ad. **✓:** új kör indul. **+5:** halasztás öt perccel. A három kis gomb: szünet/folytatás, újraindítás és beállítás. A beállításnál tetszőleges hosszt beírhatsz percben (1–240), ezt a program megjegyzi.
- **Laptop:** csak akkumulátoros gépen jelenik meg, az időjárás fölött. A **Wi-Fi** gomb a Windows saját hálózatlistáját nyitja meg, az akkumulátor gomb (töltöttség, töltés jelzése) a Windows Energiagazdálkodás és akkumulátor beállításait. 20% alatt rózsaszínre vált.
- **Időjárás:** a panel alján, az Open-Meteo ingyenes szolgáltatásából, negyedóránként frissül. Az ikon, a hőmérséklet, a leírás és a szél (iránnyal; a beállításokban kikapcsolható) látszik. Rákattintva a panel mellett, a szekció magasságában középre igazítva kinyílik a részletes előrejelzés: a következő 24 óra háromóránként (hőmérséklet-görbe, csapadék esélye), hőérzet, páratartalom, szél és széllökés, a mai csapadék, UV-index, napkelte és napnyugta, valamint a következő három nap. Bezárul, ha máshova kattintasz, az `Esc`-re vagy újabb kattintásra. A települést a beállítások Időjárás kártyáján állíthatod be.
