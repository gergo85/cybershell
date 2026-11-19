// Jegyzet: kitűzött szöveg a panelen. A panelen csak az eleje látszik (legfeljebb 5, kompakt nézetben 3 sor; vízszintes
// panelen 3–4, amennyi kifér, és a nézet a szélességet állítja), hogy ne
// foglaljon sok helyet; kattintásra az információs panel mellett kinyílik nagyban, és ott szerkeszthető
// (parts/flyout.js, "note" nézet). Újraindítás után is megmarad: a program a config.json-ban
// tárolja (note.text), így több képernyőn is ugyanaz látszik.
(() => {
  const { defineModule, T, esc, store } = window.HUB;

  const KEY = 'cybershell.note';   // csak az előnézetben (program nélkül) használt tároló

  defineModule({
    id: 'note',
    title: 'secNote',
    name: 'sNote',
    hint: 'sNoteHint',
    tip: 'noteOpen',
    compact: 'notes',
    className: 'note-mod',
    defaultOn: false,
    html: () => '<div class="nt-text" id="ntText"></div>',
    setup(ctx) {
      const box = ctx.el.querySelector('#ntText');

      const setText = (text) => {
        if (typeof text !== 'string') return;
        const t = text.trim();
        box.classList.toggle('empty', !t);
        box.innerHTML = `<span class="nt-clamp">${esc(t || T('noteHint'))}</span>`;
        fitLines();
        ctx.fit();
      };
      // vízszintes panelen annyi sor (legfeljebb 4), amennyi teljesen kifér a szekció aljáig,
      // hogy ne legyen levágott betű; itt a Normál/Kompakt nézet a szélességet állítja
      const fitLines = () => {
        const clamp = box.querySelector('.nt-clamp');
        if (!clamp || !ctx.horiz || !box.offsetHeight) return;
        const lh = parseFloat(getComputedStyle(clamp).lineHeight) || 15;
        const avail = ctx.el.getBoundingClientRect().bottom - 4 - box.getBoundingClientRect().top - (box.offsetHeight - box.clientHeight) - 8;
        const n = Math.max(1, Math.min(4, Math.floor((avail + 0.5) / lh)));
        clamp.style.webkitLineClamp = String(n);
        clamp.style.lineClamp = String(n);
      };
      window.addEventListener('resize', fitLines);

      ctx.el.addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: note');
        ctx.post({ type: 'note-open', ...ctx.rect(ctx.el) });
      });
      setText('');

      return {
        message(msg) {
          if (msg.type === 'init') setText((msg.note && msg.note.text) || '');
          if (msg.type === 'note') setText(msg.text);
          // kiemelve, amíg szerkesztésre nyitva van
          if (msg.type === 'fly-state') ctx.el.classList.toggle('open', !!msg.open && msg.kind === 'note');
        },
        // a bekapcsolt szekció itt kap először méretet
        sections: () => fitLines(),
        demo() { setText(store.get(KEY) || T('noteDemo')); },
      };
    },
    strings: {
      hu: {
        secNote: '// JEGYZET', sNote: 'Jegyzet', noteHint: 'Kattints ide a jegyzet írásához…', noteOpen: 'Jegyzet szerkesztése',
        noteDemo: 'Hétvégén: akksik töltése, kártyák formázása', noteSaved: 'Mentve', noteChars: '{n} / 2000 karakter',
        sNoteHint: 'Kitűzött jegyzet a panelen: az első 5 sora látszik (kompakt nézetben 3), kattintásra a panel mellett nagyban szerkeszthető, és újraindítás után is megmarad (config.json: note.text).',
      },
      en: {
        secNote: '// NOTE', sNote: 'Note', noteHint: 'Click here to write a note…', noteOpen: 'Edit the note',
        noteDemo: 'Weekend: charge batteries, format cards', noteSaved: 'Saved', noteChars: '{n} / 2000 characters',
        sNoteHint: 'A pinned note on the panel: its first 5 lines show (3 in compact view), click to edit it in a larger view next to the panel; it is kept after a restart (config.json: note.text).',
      },
      pl: {
        secNote: '// NOTATKA', sNote: 'Notatka', noteHint: 'Kliknij, aby napisać notatkę…', noteOpen: 'Edytuj notatkę',
        noteDemo: 'Weekend: naładować baterie, sformatować karty', noteSaved: 'Zapisano', noteChars: '{n} / 2000 znaków',
        sNoteHint: 'Przypięta notatka na panelu: widać jej pierwsze 5 wierszy (3 w widoku kompaktowym), po kliknięciu można ją edytować w większym widoku obok panelu; zostaje po ponownym uruchomieniu (config.json: note.text).',
      },
    },
  });
})();
