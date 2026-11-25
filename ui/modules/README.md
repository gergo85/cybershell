# Az információs panel moduljai

Az információs panel (alapból jobb oldalt) minden szekciója egy modul: egy `.js` és egy
`.css` fájl ebben a mappában, azonos névvel (pl. `clock.js`, `clock.css`). A panel kerete
(`parts/panel.js`) csak a közös részt adja: Indítás és MENÜ gomb, értesítés, sorrend és
láthatóság, helykihasználás, másodpercenkénti óraütés, üzenetek szétosztása.

## Új modul, lépésenként

1. **`ui/modules/<id>.js`**: a modul leírása és működése (lásd lent a sablont).
2. **`ui/modules/<id>.css`**: a modul stílusa (nem kötelező). A vízszintes panel és a világos téma
   szabályai is ide kerülnek, a fájl végére (`.right.horiz …`, `html[data-scheme="light"] …`).
3. **`ui/hub.html`**: egy `<script src="modules/<id>.js">` és egy `<link … href="modules/<id>.css">`
   sor a többi modul közé. A szkriptek sorrendje az alapsorrend a panelen.
4. **`src/config.rs`, `PANEL_MODULES`**: egy sor a modul azonosítójával, alapértelmezett
   láthatóságával és azzal, hogy van-e kompakt nézete. Így a beállítások Panelek fülén ki- és
   bekapcsolható, rendezhető, és a config.json `sections` részébe is bekerül.
5. Ha a modulnak a programtól kell adat (pl. rendszerinformáció) vagy a programnak kell valamit
   tennie (pl. megnyitni egy Windows-beállítást): **`src/hub/panel.rs`**, `on_panel_ipc` (a
   felület üzenetei) és egy `send_…` függvény, amit a `Hub::tick` hív (az adat a panelnek). A
   legutóbb küldött üzenetet egy `Feed` mező tartja a `Hub`-ban (lásd ott a panel-adatok
   szakaszát): változatlan adatot nem küldünk újra, a panel újratöltésekor pedig ezt kapja vissza.
   Lassú lekérdezés háttérszálon fusson, mint a `send_tray` vagy a `codex_send`. Folyamatos méréshez (mint a Terhelés és a Hálózat) a `src/monitor.rs` háttérszála a minta: csak
   akkor mér, ha a modul látszik, és az eredményt üzenetként küldi a panelnek.

A beállítások felülete a modulok listáját, nevét, súgóját, kompakt nézetét és gombsorrendjét
magától veszi át, ott nincs teendő.

## Közös segédek

`window.HUB` adja a felület közös segédeit (core.js): `T` (szöveg), `svg` (ikon), `esc`, `pad`,
`store` (helyi tároló), `fetchJson`, `sparkPath(értékek, szélesség, magasság, min, max)` (kis grafikon
vonala), `lang`, `demo`.

## Sablon

```js
// Jegyzet: egy rövid szöveg a panelen (példa).
(() => {
  const { defineModule, $, T, svg, store } = window.HUB;

  defineModule({
    id: 'note',                 // a config.json kulcsa (sections.note, sections.order)
    title: 'secNote',           // a szekció fejléce (szövegkulcs)
    name: 'sNote',              // a neve a beállításokban
    className: 'note-mod',      // a <section> további osztályai (nem kötelező)
    html: () => `<div class="nt-text" id="ntText"></div>`,
    setup(ctx) {
      const render = () => { $('ntText').textContent = store.get('cybershell.note') || '—'; };
      render();
      ctx.el.addEventListener('click', () => ctx.toast(T('noteHint')));
      return {
        message(msg) { if (msg.type === 'note') render(); },  // a program üzenetei
        tick(now) {},                                         // másodpercenként
        demo() {},                                            // előnézetben (preview.html)
      };
    },
    strings: {
      hu: { secNote: '// JEGYZET', sNote: 'Jegyzet', noteHint: 'Kattints a szerkesztéshez' },
      en: { secNote: '// NOTE', sNote: 'Note', noteHint: 'Click to edit' },
    },
  });
})();
```

## A leírás mezői

| Mező | Kötelező | Jelentés |
| --- | --- | --- |
| `id` | igen | Azonosító: a config.json `sections` kulcsa, és a szekció elemének azonosítója is ebből lesz (`<id>Sec`). |
| `title` | igen | A szekció fejlécének szövegkulcsa (`// …`). |
| `name` | igen* | A modul neve a beállításokban (*a nem kapcsolható modulnál nem kell). |
| `html()` | igen | A szekció tartalma a fejléc alatt (HTML-szöveg). |
| `setup(ctx)` | nem | Az eseménykezelők bekötése; a visszaadott objektum a modul horgai (lásd lent). |
| `strings` | nem | A modul saját szövegei nyelvenként (`hu`, `en`, `pl`). Betöltéskor a közös szótárba kerülnek. |
| `hint` | nem | Súgó szövegkulcsa a beállításokban. |
| `tip` | nem | A szekció súgója a panelen (szövegkulcs). |
| `className` | nem | A `<section class="sec …">` további osztályai. |
| `sectionId` | nem | A szekció elemének azonosítója, ha nem `<id>Sec`. |
| `defaultOn` | nem | `false`: alapból rejtett (mint a Codex). A program a config.json-ból mindig küld értéket, ez csak az előnézetben számít. |
| `compact` | nem | Van kompakt nézete; az érték az ikon neve a beállításokban (`icons.js`). Kompakt nézetben a szekció `compact` osztályt kap. |
| `ownsWheel` | nem | A modul maga használja a görgőt (vízszintes panelen ilyenkor nem görget a panel). |
| `listed` | nem | `false`: a beállításokban nem szerepel, nem kapcsolható és nem rendezhető (a panelen a végére, a MENÜ elé kerül). |
| `laptopOnly` | nem | Csak akkumulátoros gépen szerepel a beállítások listájában (mint a Laptop); a panelen a modul maga rejti el magát (`ctx.setAvailable`). |
| `listedIf` | nem | `(data) => bool`: a modul saját feltétele, hogy szerepeljen-e a beállítások Modulok listájában (a beállítások adataiból, pl. a Perifériák: `data.periph > 0`). |
| `buttons`, `orderKey` | nem | Rendezhető gombok (Eszköztár, Főkapcsoló): `[{ key, icon, title }]` és a config.json kulcsa. |
| `settings` | nem | Saját kártya a beállítások Funkciók fülén: `{ title, html(), setup(sx) }`, a `setup` egy `{ render(data) }` objektumot ad vissza (lásd a Valuta modult). Az `sx`: `el`, `$(id)` (a kártyán belül), `post`, `demo`, `update(patch)` (a helyi adat frissítése). |

## `ctx`: amit a keret ad a modulnak

| | |
| --- | --- |
| `ctx.el` | A szekció eleme. |
| `ctx.post(msg)` | Üzenet a programnak (`{ type: '…', … }`); a `src/hub/panel.rs` `on_panel_ipc` fogadja. |
| `ctx.toast(text)` | Rövid értesítés a MENÜ fölött. |
| `ctx.demo` | Igaz az előnézetben (program nélkül): ilyenkor ne küldjön üzenetet, mutasson mintaadatot. |
| `ctx.horiz` | Vízszintes (fent vagy lent álló) panel. |
| `ctx.setAvailable(on)` | A modul maga rejti el magát, a beállítástól függetlenül (pl. nincs akkumulátor). |
| `ctx.fit()` | Azonnali újramérés, ha a szekció mérete ugrásszerűen változott. Kisebb változásnál ez magától megtörténik. |
| `ctx.rect(el)` | Az elem helye a képernyőn (a mellé nyíló ablakoknak). |
| `ctx.along(el)` | Az elem közepe a panel mentén. |
| `ctx.reorder(grid, keys)` | Egy rács gombjainak sorrendje `data-k` szerint. |
| `ctx.sections()` | A panel beállításai (láthatóság, sorrend, kompakt nézet). |
| `ctx.call(id, name, …)` | Egy másik modul `api` függvénye (pl. a zene görgője: `ctx.call('volume', 'nudge', 2)`). |

## Horgok (a `setup` visszatérési értéke)

| | |
| --- | --- |
| `message(msg)` | A program minden üzenete (`init`, `sections`, a modulok saját üzenetei…); a modul a `msg.type` szerint válogat. |
| `tick(now)` | Másodpercenként, a többi modullal együtt. |
| `sections(s)` | A beállítások változásakor (pl. a gombok sorrendje). |
| `demo()` | Az előnézetben: mintaadat megjelenítése. |
| `api` | Más modulok által hívható függvények. |

## Kipróbálás

A `preview.html` böngészőben megnyitva (helyi webszerverrel) a teljes képernyőt mutatja
mintaadatokkal, a program nélkül; a modul `demo()` horga itt fut. Fent vagy lent álló
panellel: `preview.html?icons=top&info=bottom`, világos témával: `?scheme=light`.
