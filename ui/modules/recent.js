// Mostanában: ugyanazok a programok, mint a kereső „Gyakran használt” csoportjában, az
// Eszköztárhoz hasonló rácsban (2 oszlop). Normál nézetben legfeljebb 8, kompakt nézetben
// legfeljebb 4 ikon. Kattintásra a program ablaka jön elő (ha nem fut, elindul).
// A listát a Start menü számolja, a program küldi (src/hub/recent.rs: send_recent), csak változáskor.
(() => {
  const { defineModule, $, T, esc } = window.HUB;

  const MAX = 8;
  const MAX_COMPACT = 4;

  // előnézetben: színes betűs ikonok
  const demoIcon = (letter, color) => `data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" rx="3" fill="${color}"/>` +
    `<text x="8" y="12" font-family="Segoe UI,sans-serif" font-size="10" font-weight="700" text-anchor="middle" fill="#0b0e14">${letter}</text></svg>`)}`;
  const DEMO = [
    ['Chrome', '#ffb02e'], ['Visual Studio Code', '#4ea1ff'], ['Fájlkezelő', '#e8b04a'], ['Spotify', '#25d366'],
    ['Photoshop', '#5b8def'], ['Discord', '#5865f2'], ['Steam', '#7aa7c7'], ['Terminal', '#c9ccd6'],
  ].map(([name, color]) => ({ key: name, name, icon: demoIcon(name[0], color) }));

  defineModule({
    id: 'recent',
    title: 'secRecent',
    name: 'sRecent',
    hint: 'recentHint',
    // kompakt nézet: legfeljebb 4 ikon
    compact: 'history',
    className: 'recent',
    defaultOn: false,
    html: () => `
      <div class="rc-grid" id="rcGrid"></div>
      <div class="rc-none" id="rcNone">${T('recentNone')}</div>`,
    setup(ctx) {
      const grid = $('rcGrid');
      let apps = [];
      const render = () => {
        const max = ctx.el.classList.contains('compact') ? MAX_COMPACT : MAX;
        grid.innerHTML = apps.slice(0, max).map((a, i) =>
          `<button class="rc" data-i="${i}" title="${esc(a.name)}"><img src="${esc(a.icon)}" alt="" draggable="false"><b>${esc((a.name || '?').charAt(0))}</b></button>`).join('');
        $('rcNone').hidden = apps.length > 0;
      };
      // ha a kép nem töltődik be, a program nevének kezdőbetűje látszik
      grid.addEventListener('error', (e) => {
        if (e.target.tagName === 'IMG') e.target.closest('.rc').classList.add('noimg');
      }, true);
      const appOf = (e) => {
        const b = e.target.closest('.rc');
        return b && apps[Number(b.dataset.i)];
      };
      grid.addEventListener('click', (e) => {
        const app = appOf(e);
        if (!app) return;
        if (ctx.demo) return ctx.toast(`Demo: ${app.name}`);
        ctx.post({ type: 'cmd', name: 'recent', key: app.key });
      });
      render();
      return {
        message(m) {
          if (m.type !== 'recent') return;
          apps = m.apps || [];
          render();
        },
        // a nézet (normál vagy kompakt) változásakor más a látható ikonok száma
        sections: () => render(),
        demo: () => { apps = DEMO; render(); },
      };
    },
    strings: {
      hu: {
        secRecent: '// MOSTANÁBAN', sRecent: 'Mostanában', recentNone: 'Még nincs gyakran használt program',
        recentHint: 'A gyakran használt programok, mint a keresőben',
      },
      en: {
        secRecent: '// RECENT', sRecent: 'Recent', recentNone: 'No frequently used programs yet',
        recentHint: 'Your frequently used programs, as in search',
      },
      pl: {
        secRecent: '// OSTATNIE', sRecent: 'Ostatnie', recentNone: 'Brak często używanych programów',
        recentHint: 'Często używane programy, jak w wyszukiwarce',
      },
      de: {
        secRecent: '// ZULETZT', sRecent: 'Zuletzt', recentNone: 'Noch keine häufig genutzten Programme',
        recentHint: 'Deine häufig genutzten Programme, wie in der Suche',
      },
      es: {
        secRecent: '// RECIENTES', sRecent: 'Recientes', recentNone: 'Aún no hay programas de uso frecuente',
        recentHint: 'Tus programas de uso frecuente, como en la búsqueda',
      },
    },
  });
})();
