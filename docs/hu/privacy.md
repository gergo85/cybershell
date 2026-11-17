[← Vissza a README-hez](../../README.hu.md) · [English](../en/privacy.md) · **Magyar**

# A CyberShell adatvédelmi nyilatkozata

Utolsó frissítés: 2026. november 17.

A CyberShell egy ingyenes Windows asztali héj és indító, fejlesztője Szabó Gergő („a fejlesztő”). Ez a nyilatkozat azt írja le, milyen adatokat kezel az alkalmazás. Röviden: **a CyberShellnek nincs saját szervere, nem gyűjt rólad személyes adatot, és nincs benne analitika, telemetria vagy reklám.**

## A gépeden tárolt adatok

Minden, amire az alkalmazásnak szüksége van, helyben tárolódik, és a fejlesztőhöz nem jut el:

- **Beállítások**: a `config.json` és a `wallpapers` mappa, az `%LOCALAPPDATA%\CyberShell` mappában.
- **A claude.ai-bejelentkezés** (nem kötelező): a CyberShell saját böngészőprofiljában, az `%LOCALAPPDATA%\CyberShell\WebView2` mappában. Törölheted a mappát, amíg az alkalmazás nem fut.
- **Vágólap-előzmények** (az utolsó 30 másolt szöveg, nem kötelező): csak a memóriában vannak, kilépéskor elvesznek. A jelszókezelők által rejtettnek jelölt elemeket kihagyja.
- **A legutóbbi fájlok, a telepített programok és az ablakok adatai**: a Windowsból olvassa ki őket a Start menü, a keresés és az ikonsáv megjelenítéséhez. A gépeden kívülre nem kerülnek.
- **A Claude Code és a Codex használati korlátai** (nem kötelező): ezeknek az eszközöknek a helyi fájljaiból olvassa őket (például `%USERPROFILE%\.claude` és `%USERPROFILE%\.codex`). A CyberShell csak olvassa ezeket, a bejelentkezési tokent nem írja át és nem küldi tovább.

## Hálózati kapcsolatok

A CyberShell csak az alábbi funkciókhoz csatlakozik az internetre, közvetlenül a gépedről a megnevezett szolgáltatáshoz. A fejlesztő ezekből semmit nem kap meg.

| Funkció | Szolgáltatás | Elküldött adat |
|---|---|---|
| Időjárás | Open-Meteo (`api.open-meteo.com`, `geocoding-api.open-meteo.com`) | A beállított város neve és koordinátái |
| Árfolyamok | Frankfurter (`api.frankfurter.dev`) | A kiválasztott pénznemek |
| Naptár (nem kötelező) | Google Naptár | A saját magad által megadott privát iCal-cím, az események olvasásához |
| Claude-használat (nem kötelező) | claude.ai és `api.anthropic.com` | A claude.ai-bejelentkezésed, a használati korlátok lekéréséhez |
| Internetkapcsolat-ellenőrzés | Windows hálózati állapot | Az alkalmazásból semmi nem kerül ki |

Ezeknek a szolgáltatásoknak saját adatvédelmi nyilatkozatuk van, és az vonatkozik az általuk kapott adatokra. Ha egy funkciót nem használsz, vagy kikapcsolod a beállításokban, a hozzá tartozó kapcsolat nem jön létre. Ha a Start menüből webes keresést indítasz, az az alapértelmezett böngésződben nyílik meg, és a keresőmotor szabályai érvényesek rá.

## Amit az alkalmazás nem csinál

- Nem gyűjt, nem ad el és nem oszt meg személyes adatot.
- Nem használ analitikát, összeomlásjelentés-feltöltést, reklámot vagy követést.
- A fájljaidat és a vágólapodat csak a fent leírt funkciókhoz olvassa.

## Gyermekek

Az alkalmazás nem gyermekeknek készült, és tudatosan senkitől nem gyűjt adatot.

## Változások

Ha ez a nyilatkozat módosul, az új változat ezen a címen jelenik meg, frissített dátummal.

## Kapcsolat

Szabó Gergő · <https://szabogergo.hu/> · Hibajelentés: <https://github.com/gergo85/cybershell/issues>
