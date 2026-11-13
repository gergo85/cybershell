[← Vissza a README-hez](../../README.hu.md) · [English](../en/getting-started.md) · **Magyar**

# Első lépések

## Indítás és kilépés

- **Indítás:** `CyberShell.exe`. Az exe magában is működik: a felület és a gyári háttérképek bele vannak építve. Első indításkor létrehozza maga mellett a `config.json`-t és a `wallpapers` mappát. Ha mellette van `ui` mappa, a beépített felület helyett azt használja (így a megjelenés szerkeszthető).
- **Kilépés:** jobb alul **// MENÜ** → **Kilépés a CyberShellből**, vagy `Ctrl+Alt+Q`, vagy parancssorból `CyberShell.exe --quit`.
- **Újratöltés:** `Ctrl+Alt+R`, vagy MENÜ → **Újratöltés**. Ilyenkor a program újraolvassa a `config.json`-t és a felület fájljait.
- **Indulás a géppel:** MENÜ → **Indulás a géppel** kapcsoló. Bekapcsolva a CyberShell bejelentkezéskor magától elindul: megvárja, amíg az Intéző elkészül, és csak utána foglalja le a képernyő szélét. Minden indításkor frissíti a bejegyzést, így a mappa áthelyezése után is jó helyről indul. A Windows *Indítási alkalmazások* listájában (Feladatkezelő, Beállítások) is engedélyezi – az újabb Windows 11-ek enélkül nem indítják el. Ha ott letiltod, a CyberShell is kikapcsoltnak veszi.

## Automatikus indítás

Nyomd meg a `Win+R`-t, írd be: `shell:startup`, és tegyél a megnyíló mappába egy parancsikont a `CyberShell.exe`-hez.

## Ha valami félremegy

Ha a program váratlanul leáll, és a tálca rejtve marad vagy a képernyő szélén üres sáv látszik, indítsd el így:

```
CyberShell.exe --restore
```

Ez visszaállítja a tálcát és felszabadítja a lefoglalt helyet. A következő normál indításkor ez magától is megtörténik.

Napló: `%LOCALAPPDATA%\CyberShell\cybershell.log`, az előző futásé `cybershell.prev.log`. Minden kattintás és menüparancs bekerül, így látszik, ha valami nem jut el a programig.
