// Codex-használat: az 5 órás és a heti keret (a program a Codex munkamenet-naplóiból olvassa).
// Alapból rejtett. A keretek rajzolója a Claude modulé (modules/claude.js).
(() => {
  const { defineModule, $, T, svg } = window.HUB;

  defineModule({
    id: 'codex',
    title: 'secCodex',
    name: 'sCodex',
    compact: 'code',
    defaultOn: false,
    html: () => `
      <div id="codexUsage"><div class="note">…</div></div>
      <div class="usage-foot">
        <button class="refresh" id="codexRefresh">${svg('reload')}<span id="codexTime">${T('refresh')}</span></button>
        <span class="plan" id="codexPlan"></span>
      </div>`,
    setup(ctx) {
      const { limitsHtml, hhmm, fitLimits } = window.HUB.usageLimits;
      let codex = null;
      const render = () => {
        if (!codex) return;
        $('codexPlan').textContent = (codex.plan || '').toUpperCase();
        if (codex.status !== 'ok') {
          $('codexUsage').innerHTML = `<div class="note">${T('codexNone')}</div>`;
          return;
        }
        $('codexUsage').innerHTML = limitsHtml(codex.limits.map((l) => ({
          ...l, label: l.key ? l.key : T('limitHours', { h: Math.round((l.minutes || 0) / 60) }),
        })));
        fitLimits(ctx, $('codexUsage'));
      };
      window.addEventListener('resize', () => fitLimits(ctx, $('codexUsage')));
      const message = (msg) => {
        if (msg.type !== 'codex') return;
        codex = msg;
        $('codexTime').textContent = msg.at ? hhmm(new Date(msg.at)) : T('refresh');
        render();
        ctx.fit();
      };
      $('codexRefresh').addEventListener('click', () => {
        $('codexTime').textContent = `${T('refresh')}…`;
        if (ctx.demo) {
          return setTimeout(() => message({ type: 'codex', status: 'ok', plan: 'plus', at: Date.now(), limits: [
            { key: 'five_hour', minutes: 300, pct: 23, resetsAt: Date.now() + 2.6 * 3600e3 },
            { key: 'seven_day', minutes: 10080, pct: 41, resetsAt: Date.now() + 3.2 * 86400e3 }] }), 300);
        }
        ctx.post({ type: 'codex-refresh' });
      });
      return {
        message,
        sections: () => requestAnimationFrame(() => fitLimits(ctx, $('codexUsage'))),
        tick: (d) => { if (d.getSeconds() % 30 === 0) render(); },
        demo: () => $('codexRefresh').click(),
      };
    },
    strings: {
      hu: { secCodex: '// CODEX', sCodex: 'Codex', limitHours: '{h} ÓRÁS', codexNone: 'Nincs Codex-adat. A Codex első használata után jelenik meg.' },
      en: { secCodex: '// CODEX', sCodex: 'Codex', limitHours: '{h}-HOUR', codexNone: 'No Codex data yet. It appears after you first use Codex.' },
      pl: { secCodex: '// CODEX', sCodex: 'Codex', limitHours: '{h} GODZ.', codexNone: 'Brak danych Codex. Pojawią się po pierwszym użyciu Codex.' },
      de: { secCodex: '// CODEX', sCodex: 'Codex', limitHours: '{h} STUNDEN', codexNone: 'Noch keine Codex-Daten. Sie erscheinen, nachdem du Codex zum ersten Mal benutzt hast.' },
      es: { secCodex: '// CODEX', sCodex: 'Codex', limitHours: '{h} HORAS', codexNone: 'Aún no hay datos de Codex. Aparecen después de usar Codex por primera vez.' },
    },
  });
})();
