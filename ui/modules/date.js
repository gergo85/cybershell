// Dátum: év, hónap és nap, alatta a hét sorszáma (ISO 8601). Kattintásra a panel mellett
// kinyílik a naptár (parts/flyout.js).
(() => {
  const { defineModule, $, T, pad } = window.HUB;

  // a hét sorszáma (ISO 8601: a hét hétfőn kezdődik, az év első hete a januári csütörtököt tartalmazza)
  const isoWeek = (d) => {
    const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
    const day = t.getUTCDay() || 7;
    t.setUTCDate(t.getUTCDate() + 4 - day);
    const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
    return Math.ceil(((t - yearStart) / 86400000 + 1) / 7);
  };

  defineModule({
    id: 'date',
    title: 'secDate',
    name: 'sDate',
    defaultOn: false,
    // kompakt nézet: a hét nélkül
    compact: 'calendar',
    className: 'date',
    tip: 'calOpen',
    html: () => `
      <div class="date-stack"><span id="dy">----</span><span id="dmd">--</span></div>
      <div class="week" id="week">&nbsp;</div>`,
    setup(ctx) {
      ctx.el.addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: calendar');
        ctx.post({ type: 'calendar-open', ...ctx.rect(ctx.el) });
      });
      return {
        tick(d) {
          $('dy').textContent = T('dateYear', { y: d.getFullYear() });
          $('dmd').textContent = T('dateMonthDay', { m: pad(d.getMonth() + 1), d: pad(d.getDate()) });
          $('week').textContent = T('week', { n: isoWeek(d) });
        },
        // a dátum kiemelve, amíg a naptár nyitva van
        message(msg) {
          if (msg.type === 'fly-state') ctx.el.classList.toggle('open', !!msg.open && msg.kind === 'calendar');
        },
      };
    },
    strings: {
      hu: { secDate: '// DÁTUM', sDate: 'Dátum', week: '{n}. HÉT', dateYear: '{y}.', dateMonthDay: '{m}. {d}.', calOpen: 'Naptár' },
      en: { secDate: '// DATE', sDate: 'Date', week: 'WEEK {n}', dateYear: '{y}', dateMonthDay: '{m}-{d}', calOpen: 'Calendar' },
      pl: { secDate: '// DATA', sDate: 'Data', week: 'TYDZ. {n}', dateYear: '{y}', dateMonthDay: '{d}.{m}', calOpen: 'Kalendarz' },
    },
  });
})();
