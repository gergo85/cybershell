// Háttérprogramok: az értesítési terület (a Windows tálcáján a dátum melletti ikonsor) most
// futó programjai. Álló panelen 3 oszlopban, legfeljebb 9 ikon, a többi a „Továbbiak” alatt;
// fekvő panelen 3 sorban, mind. Kattintásra a program ablaka jön elő (ha nincs, elindul).
// A listát a program küldi (src/tray.rs, src/hub/panel.rs: send_tray), csak változáskor.
(() => {
  const { defineModule, $, T, esc } = window.HUB;

  // álló panelen ennyi ikon látszik a „Továbbiak” nélkül (3 sor)
  const MAX = 9;

  // előnézetben: színes betűs ikonok
  const demoIcon = (letter, color) => `data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" rx="3" fill="${color}"/>` +
    `<text x="8" y="12" font-family="Segoe UI,sans-serif" font-size="10" font-weight="700" text-anchor="middle" fill="#0b0e14">${letter}</text></svg>`)}`;
  const DEMO = [
    ['OneDrive', '#4ea1ff'], ['Steam', '#7aa7c7'], ['Viber', '#8b6cff'], ['Discord', '#5865f2'], ['Radeon', '#ff2a6d'],
    ['WhatsApp', '#25d366'], ['Teams', '#6264a7'], ['Chrome', '#ffb02e'], ['Claude', '#d97757'], ['PenTablet', '#4fd1c5'],
    ['CCleaner', '#7ee787'], ['IDrive', '#e8b04a'],
  ].map(([name, color]) => ({ name, path: name, icon: demoIcon(name[0], color) }));

  defineModule({
    id: 'tray',
    title: 'secTray',
    name: 'sTray',
    hint: 'trayHint',
    className: 'traymod',
    defaultOn: false,
    html: () => `
      <div class="tr-grid" id="trGrid"></div>
      <div class="tr-none" id="trNone">${T('trayNone')}</div>
      <button class="tr-more" id="trMore" hidden></button>`,
    setup(ctx) {
      const grid = $('trGrid');
      const more = $('trMore');
      let apps = [];
      let expanded = false;
      const render = () => {
        const cut = !ctx.horiz && !expanded && apps.length > MAX;
        grid.innerHTML = (cut ? apps.slice(0, MAX) : apps).map((a, i) =>
          `<button class="tr" data-i="${i}" title="${esc(a.name)}"><img src="${esc(a.icon)}" alt="" draggable="false"><b>${esc((a.name || '?').charAt(0))}</b></button>`).join('');
        $('trNone').hidden = apps.length > 0;
        more.hidden = ctx.horiz || apps.length <= MAX;
        more.textContent = T(expanded ? 'trayLess' : 'trayMore');
      };
      // ha a kép nem töltődik be, a program nevének kezdőbetűje látszik
      grid.addEventListener('error', (e) => {
        if (e.target.tagName === 'IMG') e.target.closest('.tr').classList.add('noimg');
      }, true);
      grid.addEventListener('click', (e) => {
        const b = e.target.closest('.tr');
        const app = b && apps[Number(b.dataset.i)];
        if (!app) return;
        if (ctx.demo) return ctx.toast(`Demo: ${app.name}`);
        ctx.post({ type: 'cmd', name: 'tray', path: app.path });
      });
      more.addEventListener('click', () => {
        expanded = !expanded;
        render();
        ctx.fit();
      });
      render();
      return {
        message(m) {
          if (m.type !== 'tray') return;
          apps = m.apps || [];
          if (apps.length <= MAX) expanded = false;
          render();
        },
        demo: () => { apps = DEMO; render(); },
      };
    },
    strings: {
      hu: {
        secTray: '// HÁTTÉRBEN', sTray: 'Háttérprogramok', trayNone: 'Nincs háttérprogram',
        trayMore: 'Továbbiak', trayLess: 'Bezárás',
        trayHint: 'A háttérben futó programok, mint a Windows tálcáján a dátum melletti ikonok',
      },
      en: {
        secTray: '// BACKGROUND', sTray: 'Background apps', trayNone: 'No background apps',
        trayMore: 'More', trayLess: 'Close',
        trayHint: 'Programs running in the background, like the icons next to the clock on the Windows taskbar',
      },
      pl: {
        secTray: '// W TLE', sTray: 'Programy w tle', trayNone: 'Brak programów w tle',
        trayMore: 'Więcej', trayLess: 'Zwiń',
        trayHint: 'Programy działające w tle, jak ikony obok zegara na pasku zadań Windows',
      },
    },
  });
})();
