// Claude-használat: az 5 órás és a heti keret kihasználtsága (a program kérdezi le a
// claude.ai-tól vagy a Claude Code állapotsorából). A keretek rajzolóját a Codex is használja.
(() => {
  const { defineModule, $, T, svg, esc, pad } = window.HUB;

  // a visszaállásig hátralévő idő (pl. „2 ó 15 p”)
  const rel = (iso) => {
    if (!iso) return '';
    const ms = new Date(iso).getTime() - Date.now();
    if (!(ms > 0)) return T('soon');
    const m = Math.round(ms / 60000);
    const dd = Math.floor(m / 1440), hh = Math.floor((m % 1440) / 60), mm = m % 60;
    if (dd > 0) return T('relDH', { d: dd, h: hh });
    if (hh > 0) return T('relHM', { h: hh, m: pad(mm) });
    return T('relM', { m: mm });
  };
  // a keretek sávjai: [{ key, label, pct, resetsAt }]
  const limitsHtml = (limits) => limits.map((l) => {
    // ha az ablak azóta újraindult, az érték ismeretlen, amíg újra nem jön adat
    const known = typeof l.pct === 'number';
    const pct = known ? Math.max(0, Math.min(100, Math.round(l.pct))) : 0;
    const lvl = pct >= 85 ? 'hot' : pct >= 60 ? 'warn' : '';
    const label = l.key === 'five_hour' ? T('limit5h') : l.key === 'seven_day' ? T('limitWeek') : esc(l.label.toUpperCase());
    return `
      <div class="lim ${lvl}">
        <div class="lim-head"><span>${label}</span><b>${known ? `${pct}%` : '—'}</b></div>
        <div class="lim-bar"><i style="width:${pct}%"></i></div>
        <div class="lim-reset">${l.resetsAt ? `↻ ${rel(l.resetsAt)}` : '&nbsp;'}</div>
      </div>`;
  }).join('');
  // az adat időbélyege a frissítés gomb mellett (ÓÓ:PP)
  const hhmm = (t) => `${pad(t.getHours())}:${pad(t.getMinutes())}`;
  window.HUB.usageLimits = { limitsHtml, hhmm };

  defineModule({
    id: 'claude',
    title: 'secClaude',
    name: 'sClaude',
    // kompakt nézet: a nullázás ideje és a frissítés sora nélkül
    compact: 'spark',
    defaultOn: false,
    html: () => `
      <div id="usage"><div class="note">…</div></div>
      <div class="usage-foot">
        <button class="refresh" id="usageRefresh">${svg('reload')}<span id="usageTime">${T('refresh')}</span></button>
        <span class="plan" id="plan"></span>
      </div>`,
    setup(ctx) {
      let usage = null;
      const render = () => {
        const box = $('usage');
        if (!usage) return;
        $('plan').textContent = (usage.plan || '').toUpperCase();
        // a claude.ai még nem válaszolt, vagy épp nem érhető el: nem kell újra belépni
        if (usage.status === 'wait' || usage.status === 'offline') {
          box.innerHTML = `<div class="note ${usage.status}">${T(usage.status === 'wait' ? 'usageWait' : 'usageOffline')}</div>`;
          return;
        }
        if (usage.status !== 'ok') {
          const text = usage.status === 'error' ? `${T('usageError')} ${esc(usage.message || '')}` : T('usageLogin');
          box.innerHTML = `<div class="note ${usage.status}">${text}</div>
            <button class="ib login" id="claudeLogin">${T('login')}</button>`;
          $('claudeLogin').addEventListener('click', () => ctx.post({ type: 'cmd', name: 'claude-login' }));
          return;
        }
        box.innerHTML = limitsHtml(usage.limits);
      };
      $('usageRefresh').addEventListener('click', () => {
        $('usageTime').textContent = `${T('refresh')}…`;
        ctx.post({ type: 'usage-refresh' });
      });
      const message = (msg) => {
        // a config.json claudeUsage értéke az egész funkciót kikapcsolja
        if (msg.type === 'init') ctx.setAvailable(msg.claudeUsage !== false);
        if (msg.type === 'usage') {
          usage = msg;
          // az adat saját időbélyege (állapotsorból érkezve ez lehet régebbi is)
          $('usageTime').textContent = hhmm(new Date(msg.status === 'ok' && msg.fetchedAt ? msg.fetchedAt : Date.now()));
          render();
        }
      };
      return {
        message,
        // a hátralévő idők félpercenként frissülnek
        tick: (d) => { if (d.getSeconds() % 30 === 0) render(); },
        demo() {
          const inH = (hrs) => new Date(Date.now() + hrs * 3600000).toISOString();
          message({ type: 'usage', status: 'ok', plan: 'pro', limits: [
            { key: 'five_hour', label: '5 órás', pct: 34, resetsAt: inH(2.3) },
            { key: 'seven_day', label: 'Heti', pct: 71, resetsAt: inH(76) },
          ] });
        },
      };
    },
    strings: {
      hu: {
        secClaude: '// CLAUDE', sClaude: 'Claude', refresh: 'FRISSÍT', limit5h: '5 ÓRÁS', limitWeek: 'HETI',
        soon: 'hamarosan', relDH: '{d} n {h} ó', relHM: '{h} ó {m} p', relM: '{m} p',
        usageError: 'Nem elérhető.', usageWait: 'Kapcsolódás a claude.ai-hoz…', usageOffline: 'A claude.ai most nem érhető el, hamarosan újrapróbálom.',
        usageLogin: 'Lépj be a claude.ai-ra a CyberShellben.', login: 'Belépés',
      },
      en: {
        secClaude: '// CLAUDE', sClaude: 'Claude', refresh: 'REFRESH', limit5h: '5-HOUR', limitWeek: 'WEEKLY',
        soon: 'soon', relDH: '{d}d {h}h', relHM: '{h}h {m}m', relM: '{m}m',
        usageError: 'Not available.', usageWait: 'Connecting to claude.ai…', usageOffline: 'claude.ai is unreachable right now, retrying soon.',
        usageLogin: 'Sign in to claude.ai in CyberShell.', login: 'Sign in',
      },
      pl: {
        secClaude: '// CLAUDE', sClaude: 'Claude', refresh: 'ODŚWIEŻ', limit5h: '5 GODZ.', limitWeek: 'TYDZIEŃ',
        soon: 'wkrótce', relDH: '{d}d {h}h', relHM: '{h}h {m}m', relM: '{m}m',
        usageError: 'Niedostępne.', usageWait: 'Łączenie z claude.ai…', usageOffline: 'claude.ai jest teraz niedostępne, wkrótce spróbuję ponownie.',
        usageLogin: 'Zaloguj się do claude.ai w CyberShell.', login: 'Zaloguj',
      },
    },
  });
})();
