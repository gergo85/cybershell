// Notesz (a config.json-ban és a kódban: note): két rész, Jegyzet (szabad szöveg) és Teendők
// (kipipálható lista). A fülek a kinyíló
// szerkesztőben vannak (parts/flyout.js, "note" nézet); a panel azt mutatja, amelyik fülre ott
// legutóbb kattintottak (config.json: note.tab), és a fejléce is ehhez igazodik.
// A panelen csak az eleje látszik: a jegyzetből legfeljebb 5 sor, a teendőkből 5 (kompakt nézetben
// 3-3; vízszintes panelen annyi, amennyi kifér, és a nézet a szélességet állítja). A teendők
// a panelen egy kattintással kipipálhatók; minden más kattintásra az információs panel mellett
// nagyban kinyílik a szerkesztő ugyanazon a fülön (parts/flyout.js, "note" nézet). Újraindítás
// után is megmarad: a program a config.json-ban tárolja (note), így több
// képernyőn is ugyanaz látszik.
(() => {
  const { defineModule, $, T, esc, store } = window.HUB;

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
    html: () => `
      <div class="nt-text" id="ntText"></div>
      <div class="nt-twrap">
        <div class="nt-tasks" id="ntTasks"></div>
        <div class="nt-prog" id="ntProg" hidden><i><b></b></i><span></span></div>
      </div>`,
    setup(ctx) {
      const box = $('ntText');
      const list = $('ntTasks');
      let note = { text: '', tasks: [], tab: 'note' };

      // a teendőkből annyi sor látszik, amennyi teljesen befér a dobozba (álló panelen a jegyzettel
      // azonos magasságú, a doboz alatti kész / összes sáv egy sort elvesz; vízszintes panelen a
      // doboz a panel magasságát követi); a doboz alatt a kész / összes, és hány nem fért ki
      const ROW = 15;
      const fitTasks = () => {
        const tasks = note.tasks || [];
        const rows = [...list.querySelectorAll('.nt-row')];
        for (const r of rows) r.hidden = false;
        let fit = rows.length;
        if (note.tab === 'tasks' && list.clientHeight) {
          const pad = list.offsetHeight - list.clientHeight + 8;
          fit = Math.max(1, Math.floor((list.offsetHeight - pad + 0.5) / ROW));
          rows.forEach((r, i) => { r.hidden = i >= fit; });
        }
        const prog = $('ntProg');
        prog.hidden = !tasks.length;
        if (!tasks.length) return;
        const done = tasks.filter((t) => t.done).length;
        prog.querySelector('b').style.width = `${(done / tasks.length) * 100}%`;
        prog.querySelector('span').textContent = T('noteProgress', { d: done, n: tasks.length }) + (tasks.length > fit ? ` ${T('noteMore', { n: tasks.length - fit })}` : '');
        prog.classList.toggle('all', done === tasks.length);
      };

      const render = () => {
        const tab = note.tab === 'tasks' ? 'tasks' : 'note';
        ctx.el.dataset.tab = tab;
        const tasks = note.tasks || [];
        // a fejléc: // JEGYZET, illetve // TEENDŐK
        const h = ctx.el.querySelector('h3');
        if (h) h.textContent = T(tab === 'tasks' ? 'secTasks' : 'secNote');
        // jegyzet
        const t = (note.text || '').trim();
        box.classList.toggle('empty', !t);
        box.innerHTML = `<span class="nt-clamp">${esc(t || T('noteHint'))}</span>`;
        // teendők: a felhasználó sorrendjében (ami nem fér ki, azt a fitTasks rejti el)
        list.innerHTML = tasks.length
          ? tasks.map((k, i) =>
            `<div class="nt-row${k.done ? ' done' : ''}" title="${esc(k.text)}"><i class="nt-box" data-i="${i}" title="${T(k.done ? 'noteUndo' : 'noteDone')}"></i><span>${esc(k.text)}</span></div>`).join('')
          : `<div class="nt-none">${T('tasksHint')}</div>`;
        fitTasks();
        fitLines();
        ctx.fit();
      };
      const setNote = (n) => {
        if (!n || typeof n !== 'object') return;
        note = { text: n.text || '', tasks: Array.isArray(n.tasks) ? n.tasks : [], tab: n.tab || 'note' };
        render();
      };
      // vízszintes panelen a jegyzet doboza a panel magasságát követi: annyi sor látszik, amennyi
      // teljesen befér, hogy ne legyen levágott betű; itt a Normál/Kompakt nézet a szélességet állítja
      const fitLines = () => {
        const clamp = box.querySelector('.nt-clamp');
        if (!clamp || !ctx.horiz || !box.clientHeight) return;
        const n = Math.max(1, Math.floor((box.clientHeight - 8 + 0.5) / ROW));
        clamp.style.webkitLineClamp = String(n);
        clamp.style.lineClamp = String(n);
      };
      window.addEventListener('resize', () => { fitLines(); fitTasks(); });

      const saveDemo = () => store.set(KEY, note);
      ctx.el.addEventListener('click', (e) => {
        // jelölőnégyzet: a teendő kipipálása (vagy vissza), a szerkesztő nem nyílik meg
        const cb = e.target.closest('.nt-box');
        if (cb) {
          const i = Number(cb.dataset.i);
          if (ctx.demo) {
            note.tasks[i].done = !note.tasks[i].done;
            saveDemo();
            return render();
          }
          return ctx.post({ type: 'task-toggle', i });
        }
        if (ctx.demo) return ctx.toast('Demo: note');
        ctx.post({ type: 'note-open', ...ctx.rect(ctx.el) });
      });
      render();

      return {
        message(msg) {
          if (msg.type === 'init') setNote(msg.note);
          if (msg.type === 'note') setNote(msg.note);
          // kiemelve, amíg szerkesztésre nyitva van
          if (msg.type === 'fly-state') ctx.el.classList.toggle('open', !!msg.open && msg.kind === 'note');
        },
        // a bekapcsolt szekció itt kap először méretet; a nézet a teendők számát is állítja
        sections: () => render(),
        demo() {
          const saved = store.get(KEY);
          setNote(saved && typeof saved === 'object' ? saved : {
            text: T('noteDemo'), tab: 'tasks',
            tasks: T('tasksDemo').split('|').map((text, i) => ({ text, done: i === 0 })),
          });
        },
      };
    },
    strings: {
      hu: {
        secNote: '// JEGYZET', secTasks: '// TEENDŐK', secNotebook: '// NOTESZ', sNote: 'Notesz', noteHint: 'Kattints ide a jegyzet írásához…', noteOpen: 'Notesz megnyitása',
        noteTabNote: 'Jegyzet', noteTabTasks: 'Teendők', tasksHint: 'Kattints ide teendő felvételéhez…', noteMore: '+{n}', noteProgress: '{d}/{n} KÉSZ',
        noteDone: 'Kész', noteUndo: 'Mégsem kész', taskAdd: 'Új teendő…', taskAddBtn: 'Hozzáadás', taskDel: 'Törlés',
        taskDrag: 'Húzd a sorrendhez', tasksClear: 'Kész teendők törlése', tasksCount: '{d} / {n} kész', tasksEmpty: 'Még nincs teendő. Írd be fent, és nyomj Entert.',
        noteDemo: 'Hétvégén: akksik töltése, kártyák formázása', tasksDemo: 'Akksik töltése|Kártyák formázása|Objektív tisztítása',
        noteSaved: 'Mentve', noteChars: '{n} / 2000 karakter',
        sNoteHint: 'Jegyzet és teendőlista; a kinyíló szerkesztő fülein lehet köztük váltani, a panel a legutóbb választottat mutatja. A panelen az eleje látszik, a teendők egy kattintással kipipálhatók; minden más kattintásra a panel mellett nagyban szerkeszthető. Újraindítás után is megmarad (config.json: note).',
      },
      en: {
        secNote: '// NOTE', secTasks: '// TO-DOS', secNotebook: '// NOTEBOOK', sNote: 'Notebook', noteHint: 'Click here to write a note…', noteOpen: 'Open the notebook',
        noteTabNote: 'Note', noteTabTasks: 'To-dos', tasksHint: 'Click here to add a to-do…', noteMore: '+{n}', noteProgress: '{d}/{n} DONE',
        noteDone: 'Done', noteUndo: 'Not done', taskAdd: 'New to-do…', taskAddBtn: 'Add', taskDel: 'Delete',
        taskDrag: 'Drag to reorder', tasksClear: 'Clear done to-dos', tasksCount: '{d} / {n} done', tasksEmpty: 'No to-dos yet. Type one above and press Enter.',
        noteDemo: 'Weekend: charge batteries, format cards', tasksDemo: 'Charge batteries|Format cards|Clean the lens',
        noteSaved: 'Saved', noteChars: '{n} / 2000 characters',
        sNoteHint: 'A note and a to-do list; switch between them on the tabs of the editor that opens, and the panel shows the last chosen one. The panel shows the beginning; to-dos are ticked off with one click, any other click opens a larger editor next to the panel. Kept after a restart (config.json: note).',
      },
      pl: {
        secNote: '// NOTATKA', secTasks: '// ZADANIA', secNotebook: '// NOTATNIK', sNote: 'Notatnik', noteHint: 'Kliknij, aby napisać notatkę…', noteOpen: 'Otwórz notatnik',
        noteTabNote: 'Notatka', noteTabTasks: 'Zadania', tasksHint: 'Kliknij, aby dodać zadanie…', noteMore: '+{n}', noteProgress: '{d}/{n} GOTOWE',
        noteDone: 'Zrobione', noteUndo: 'Niezrobione', taskAdd: 'Nowe zadanie…', taskAddBtn: 'Dodaj', taskDel: 'Usuń',
        taskDrag: 'Przeciągnij, aby zmienić kolejność', tasksClear: 'Usuń zrobione zadania', tasksCount: '{d} / {n} zrobione', tasksEmpty: 'Brak zadań. Wpisz zadanie powyżej i naciśnij Enter.',
        noteDemo: 'Weekend: naładować baterie, sformatować karty', tasksDemo: 'Naładować baterie|Sformatować karty|Wyczyścić obiektyw',
        noteSaved: 'Zapisano', noteChars: '{n} / 2000 znaków',
        sNoteHint: 'Notatka i lista zadań; przełącza się między nimi na kartach otwieranego edytora, a panel pokazuje ostatnio wybraną. Na panelu widać początek; zadania odhacza się jednym kliknięciem, każde inne kliknięcie otwiera większy edytor obok panelu. Zostaje po ponownym uruchomieniu (config.json: note).',
      },
    },
  });
})();
