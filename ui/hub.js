(() => {
  const q = new URLSearchParams(location.search);
  const part = q.get('part') || 'left';
  // melyik képernyőszélen van a panel (felcserélt paneleknél az ikonsáv is lehet jobbra)
  const side = q.get('side') || (part === 'right' ? 'right' : 'left');
  // nyelv: a szövegek az i18n.js-ben vannak
  const lang = (window.HUB_I18N || {})[q.get('lang')] ? q.get('lang') : 'hu';
  document.documentElement.lang = lang;
  const T = (key, vars) => {
    let s = (window.HUB_I18N[lang] || {})[key] ?? window.HUB_I18N.hu[key] ?? key;
    for (const [k, v] of Object.entries(vars || {})) s = s.replaceAll(`{${k}}`, v);
    return s;
  };
  const root = document.getElementById('root');
  const css = document.documentElement.style;

  // A gazdaprogram üzenetcsatornája (wry → window.ipc, tartalékként a WebView2 saját API-ja).
  const send = window.ipc && window.ipc.postMessage
    ? (s) => window.ipc.postMessage(s)
    : window.chrome && window.chrome.webview
      ? (s) => window.chrome.webview.postMessage(s)
      : null;
  const demo = !send;
  const post = (msg) => { if (send) send(JSON.stringify(msg)); };

  window.addEventListener('error', (e) => post({ type: 'log', text: `${e.message} @${e.lineno}` }));
  document.addEventListener('contextmenu', (e) => e.preventDefault());

  const setGeometry = (g) => {
    for (const k of ['l', 't', 'r', 'b']) if (g && g[k] != null) css.setProperty(`--${k}`, g[k]);
  };
  setGeometry({ l: q.get('l'), t: q.get('t'), r: q.get('r'), b: q.get('b') });

  const setTheme = (t) => {
    if (!t) return;
    for (const k of ['main', 'alt', 'hot', 'bg']) if (t[k]) css.setProperty(`--theme-${k}`, t[k]);
  };
  const setAlert = (on) => document.documentElement.classList.toggle('alert', !!on);

  const svg = (name) =>
    `<svg viewBox="0 0 24 24">${(window.HUB_ICONS || {})[name] || window.HUB_ICONS.app}</svg>`;
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  const pad = (n) => String(n).padStart(2, '0');
  const $ = (id) => document.getElementById(id);
  // az ikon finom színe (config "color"), csak érvényes hex értéket fogadunk el
  const appColor = (a) => (/^#[0-9a-f]{3,8}$/i.test(a.color || '') ? ` style="--app:${a.color}"` : '');

  const parts = { left: buildLeft, right: buildRight, settings: buildSettings, peek: buildPeek, top: buildStrip, bottom: buildStrip };
  const handler = (parts[part] || buildStrip)();

  window.hub = {
    receive(msg) {
      if (msg.type === 'init') { setTheme(msg.theme); setGeometry(msg.geometry); }
      if (msg.type === 'alert') setAlert(msg.on);
      if (handler) handler(msg);
    },
  };
  if (!demo) post({ type: 'ready' });
  else if (handler && handler.demo) handler.demo();

  // ───────────── vékony keretek (felső, alsó) ─────────────
  function buildStrip() {
    root.className = `strip ${part}`;
    const deco = {
      top: ['ticks', 'line', 'sweep', 'notch', 'chip c1', 'chip c2', 'chip c3'],
      bottom: ['ticks', 'line', 'sweep', 'notch', 'hazard-s', 'chip c1', 'chip c3'],
    }[part] || [];
    root.innerHTML = deco.map((c) => `<div class="${c}"></div>`).join('');
    return null;
  }

  // ───────────── bal oldali ikonsáv ─────────────
  function buildLeft() {
    root.className = `left side-${side}`;
    root.innerHTML = `
      <div class="edge"></div>
      <header class="brand"><div class="brand-name">CYBER<span>HUB</span></div></header>
      <div class="hazard"></div>
      <nav class="apps" id="apps"></nav>`;

    const appsEl = $('apps');
    let apps = [];

    const iconHtml = (app) => {
      if (app.img) return `<img src="${esc(app.img)}" alt="">`;
      const icon = app.icon || '';
      if (icon && window.HUB_ICONS[icon]) return svg(icon);
      const hay = `${app.name} ${app.hint || ''}`.toLowerCase();
      const hit = (window.HUB_GUESS || []).find(([re]) => re.test(hay));
      return svg(hit ? hit[1] : 'app');
    };

    const renderApps = () => {
      appsEl.innerHTML = apps.map((a) => `
        <button class="app pinned" data-id="${a.id}"${appColor(a)}>
          <span class="tile"><span class="tile-in">${iconHtml(a)}</span></span>
          <span class="label">${esc(a.name)}</span>
        </button>`).join('') + '<div class="extras" id="extras"></div>';
      extrasSig = '';
    };

    const applyState = (running, active, multi) => {
      const run = new Set(running || []);
      const many = new Set(multi || []);
      for (const el of appsEl.querySelectorAll('.app.pinned')) {
        const id = Number(el.dataset.id);
        el.classList.toggle('running', run.has(id));
        el.classList.toggle('multi', many.has(id));
        el.classList.toggle('active', id === active);
        if (run.has(id)) el.classList.remove('launching');
      }
    };

    // Nem kitűzött, de futó programok az ikonok alatt.
    let extrasSig = '';
    const renderExtras = (extras, activeKey) => {
      extras = extras || [];
      const sig = extras.map((x) => `${x.key}|${x.name}`).join('\n');
      if (sig !== extrasSig) {
        extrasSig = sig;
        $('extras').innerHTML = extras.length ? `<div class="extras-sep"></div>` + extras.map((x) => `
          <button class="app extra running" data-key="${esc(x.key)}">
            <span class="tile"><span class="tile-in">${x.icon ? `<img src="${esc(x.icon)}" alt="">` : svg('app')}</span></span>
            <span class="label">${esc(x.name)}</span>
          </button>`).join('') : '';
      }
      const counts = Object.fromEntries(extras.map((x) => [x.key, x.n || 1]));
      for (const el of $('extras').querySelectorAll('.app')) {
        el.classList.toggle('active', el.dataset.key === activeKey);
        el.classList.toggle('multi', (counts[el.dataset.key] || 0) >= 2);
      }
    };
    const target = (el) => (el.dataset.key ? { key: el.dataset.key } : { id: Number(el.dataset.id) });

    // Fájlhúzás: az ikon fölött időzve előjön a program, ráejtve azzal nyílik meg.
    let dragEl = null;
    let dragTimer = 0;
    const appAt = (x, y) => {
      const r = window.devicePixelRatio || 1;
      const el = document.elementFromPoint(x / r, y / r);
      return el && el.closest('.app');
    };
    const setDragEl = (el) => {
      if (el === dragEl) return;
      if (dragEl) dragEl.classList.remove('drop');
      clearTimeout(dragTimer);
      dragEl = el;
      if (el) {
        el.classList.add('drop');
        dragTimer = setTimeout(() => post({ type: 'drag-hover', ...target(el) }), 450);
      }
    };
    const onDrag = (msg) => {
      if (msg.phase === 'over') setDragEl(appAt(msg.x, msg.y));
      else if (msg.phase === 'drop') {
        const el = appAt(msg.x, msg.y);
        setDragEl(null);
        if (el) post({ type: 'drop-open', ...target(el) });
      } else setDragEl(null);
    };

    let demoState = { running: [0, 1, 3], active: 1 };
    const DEMO_COLORS = ['#ff7139', '#ea4335', '#5b8def', '#d97757', '#22a6f2', '#2b7cd3', '#21a366',
      '#6fc3ff', '#2f80ff', '#8b6cff', '#ff4fa3', '#b26bff', '#1ed760'];
    const launch = (el, newInstance) => {
      if (el.dataset.key) {
        if (!demo) post({ type: 'launch-extra', key: el.dataset.key, newInstance });
        return;
      }
      const id = Number(el.dataset.id);
      el.classList.remove('launching');
      void el.offsetWidth;
      if (!el.classList.contains('running') || newInstance) el.classList.add('launching');
      if (!demo) return post({ type: 'launch', id, newInstance });
      const r = new Set(demoState.running);
      if (demoState.active === id) demoState.active = null;
      else { r.add(id); demoState.active = id; }
      demoState.running = [...r];
      applyState(demoState.running, demoState.active);
    };
    // Ablakelőnézet: több ablakos program ikonja fölött időzve a program egymás
    // mellett mutatja az ablakok kisképét. Ha már látszik, a váltás azonnali.
    let hoverEl = null;
    let peekTimer = 0;
    let peekSent = false;
    let peekOpen = false;
    const peekLeave = () => {
      clearTimeout(peekTimer);
      if (peekSent) { peekSent = false; post({ type: 'peek-leave' }); }
    };
    const peekHover = (el) => {
      if (el === hoverEl) return;
      peekLeave();
      hoverEl = el;
      if (!el || !el.classList.contains('multi') || demo) return;
      peekTimer = setTimeout(() => {
        if (hoverEl !== el || !el.isConnected) return;
        const r = el.querySelector('.tile').getBoundingClientRect();
        peekSent = true;
        post({ type: 'peek', ...target(el), y: Math.round(r.top + r.height / 2) });
      }, peekOpen ? 80 : 450);
    };
    appsEl.addEventListener('mouseover', (e) => peekHover(e.target.closest('.app')));
    appsEl.addEventListener('mouseleave', () => peekHover(null));

    appsEl.addEventListener('click', (e) => {
      const el = e.target.closest('.app');
      clearTimeout(peekTimer);
      peekSent = false;
      if (el) launch(el, e.shiftKey);
    });
    appsEl.addEventListener('auxclick', (e) => {
      const el = e.target.closest('.app');
      if (el && e.button === 1) launch(el, true);
    });
    appsEl.addEventListener('animationend', (e) => {
      const el = e.target.closest('.app');
      if (el && e.animationName === 'pulse') el.classList.remove('launching');
    });

    const h = (msg) => {
      if (msg.type === 'init') { apps = msg.apps || []; renderApps(); }
      if (msg.type === 'state') { applyState(msg.running, msg.active, msg.multi); renderExtras(msg.extras, msg.activeExtra); }
      if (msg.type === 'peek-state') peekOpen = !!msg.open;
      if (msg.type === 'drag') onDrag(msg);
    };
    h.demo = () => {
      h({
        type: 'init',
        apps: [
          ['Firefox', 'globe'], ['Chrome', 'browser'], ['Total Cmd', 'panels'], ['Claude', 'spark'],
          ['VS Code', 'code'], ['Word', 'doc'], ['Excel', 'sheet'], ['Lightroom', 'photo'],
          ['Photoshop', 'design'], ['Topaz AI', 'enhance'], ['ColorCop', 'picker'], ['ArtCraft', 'rocket'],
          ['Spotify', 'music'],
        ].map(([name, icon], id) => ({ id, name, icon, color: DEMO_COLORS[id] })),
      });
      applyState(demoState.running, demoState.active, [1]);
      renderExtras([
        { key: 'demo:notepad', name: 'Jegyzettömb', icon: '', n: 2 },
        { key: 'demo:calc', name: 'Számológép', icon: '' },
      ], null);
    };
    return h;
  }

  // ───────────── információs panel (alapból jobbra) ─────────────
  function buildRight() {
    root.className = `right side-${side}`;
    root.innerHTML = `
      <div class="edge"></div>
      <div class="quick">
        <div class="quick-apps" id="quickApps"></div>
        <button class="qbtn" data-cmd="search" style="--app:#4fd1c5"><span class="qtile">${svg('search')}</span><span class="qlabel">${T('search')}</span></button>
        <button class="qbtn" data-cmd="start" style="--app:#3b9eff"><span class="qtile">${svg('start')}</span><span class="qlabel">${T('start')}</span></button>
      </div>
      <section class="sec" id="claudeSec">
        <h3>${T('secClaude')}</h3>
        <div id="usage"><div class="note">…</div></div>
        <div class="usage-foot">
          <button class="refresh" id="usageRefresh">${svg('reload')}<span id="usageTime">${T('refresh')}</span></button>
          <span class="plan" id="plan"></span>
        </div>
      </section>
      <section class="sec act" id="act">
        <h3>${T('secActivity')}</h3>
        <div class="act-normal">
          <div class="act-time" id="actTime">--:--</div>
          <div class="act-segs" id="actSegs">${'<i></i>'.repeat(10)}</div>
          <div class="act-info" id="actInfo">&nbsp;</div>
          <div class="act-btns">
            <button class="ib" id="actToggle"></button>
            <button class="ib" id="actReset">${svg('reload')}</button>
            <button class="ib" id="actSet">${svg('settings')}</button>
          </div>
        </div>
        <div class="act-edit">
          <label class="act-edit-label" for="actMin">${T('actLength')}</label>
          <input id="actMin" type="number" min="1" max="240" step="1" inputmode="numeric">
          <div class="act-err" id="actErr"></div>
          <div class="act-btns">
            <button class="ib" id="actSave">${svg('check')}</button>
            <button class="ib" id="actCancel">${svg('close')}</button>
          </div>
        </div>
        <div class="act-alert">
          <div class="act-alert-title">${T('actAlertTitle')}</div>
          <div class="act-alert-text">${T('actAlertText')}</div>
          <div class="act-btns">
            <button class="ib" id="actDone">${svg('check')}</button>
            <button class="ib" id="actSnooze">+5</button>
          </div>
        </div>
      </section>
      <section class="sec clock" id="clockSec">
        <h3>${T('secClock')}</h3>
        <div class="clock-row"><span id="hh">--</span><span class="colon">:</span><span id="mm">--</span></div>
        <div class="sec-line"><div class="segs" id="segs">${'<i></i>'.repeat(12)}</div><span class="ss" id="ss">--</span></div>
      </section>
      <section class="sec date" id="dateSec">
        <h3>${T('secDate')}</h3>
        <div class="date-stack"><span id="dy">----</span><span id="dmd">--</span></div>
        <div class="week" id="week">&nbsp;</div>
      </section>
      <section class="sec weather" id="weatherSec">
        <h3>${T('secWeather')}</h3>
        <div class="w-main"><span class="w-icon" id="wIcon"></span><span class="w-temp" id="wTemp">--°</span></div>
        <div class="w-desc" id="wDesc">&nbsp;</div>
        <div class="w-minmax" id="wMinMax">&nbsp;</div>
        <div class="w-wind" id="wWind">&nbsp;</div>
        <div class="w-city" id="wCity">&nbsp;</div>
      </section>
      <section class="sec power" id="powerSec">
        <h3>${T('secPower')}</h3>
        <div class="pw-grid">
          <button class="pw" data-act="lock" style="--app:#4fd1c5">${svg('lock')}<span>${T('pwLock')}</span></button>
          <button class="pw" data-act="sleep" style="--app:#8b6cff">${svg('moon')}<span>${T('pwSleep')}</span></button>
          <button class="pw" data-act="shutdown" style="--app:#ff2a6d">${svg('power')}<span>${T('pwOff')}</span></button>
          <button class="pw" data-act="restart" style="--app:#ffb02e">${svg('reload')}<span>${T('pwRestart')}</span></button>
        </div>
        <div class="pw-confirm" id="pwConfirm">
          <div class="pw-q" id="pwQ"></div>
          <div class="act-btns">
            <button class="ib danger" id="pwYes">${svg('check')}</button>
            <button class="ib" id="pwNo">${svg('close')}</button>
          </div>
        </div>
      </section>
      <div class="sys">
        <button class="sys-btn" id="sysBtn">${T('menu')}</button>
      </div>
      <div class="toast" id="toast"></div>`;

    // Keresés és Indítás: a Windows saját keresője és Start menüje; fölöttük a
    // jobb oldalra tett programok (config "place": "right")
    root.querySelector('.quick').addEventListener('click', (e) => {
      const b = e.target.closest('.qbtn');
      if (!b) return;
      if (b.dataset.id != null) post({ type: 'launch', id: Number(b.dataset.id), newInstance: e.shiftKey });
      else post({ type: 'cmd', name: b.dataset.cmd });
    });
    const renderQuickApps = (apps) => {
      $('quickApps').innerHTML = (apps || []).map((a) => {
        const icon = a.img ? `<img src="${esc(a.img)}" alt="">` : svg(window.HUB_ICONS[a.icon] ? a.icon : 'app');
        return `<button class="qbtn qapp" data-id="${a.id}"${appColor(a)}><span class="qtile">${icon}</span><span class="qlabel">${esc(a.name)}</span></button>`;
      }).join('');
    };
    const applyQuickState = (running, active) => {
      const run = new Set(running || []);
      for (const el of root.querySelectorAll('.qapp')) {
        const id = Number(el.dataset.id);
        el.classList.toggle('running', run.has(id));
        el.classList.toggle('active', id === active);
      }
    };

    // MENÜ: a beállításokat nyitja meg
    const sysBtn = $('sysBtn');
    sysBtn.addEventListener('click', () => {
      if (demo) return toast('Demo: settings');
      post({ type: 'cmd', name: 'settings' });
    });

    let toastTimer = 0;
    const toast = (text) => {
      const t = $('toast');
      t.style.bottom = `${root.clientHeight - sysBtn.offsetTop + 8}px`;
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => t.classList.remove('show'), 4500);
    };

    // ── főkapcsoló: a leállítás és az újraindítás megerősítést kér ──
    const POWER_Q = { shutdown: T('pwConfirmOff'), restart: T('pwConfirmRestart') };
    let pendingPower = null;
    let pwTimer = 0;
    const closeConfirm = () => {
      pendingPower = null;
      clearTimeout(pwTimer);
      $('powerSec').classList.remove('confirming');
    };
    root.querySelector('.pw-grid').addEventListener('click', (e) => {
      const b = e.target.closest('.pw');
      if (!b) return;
      const act = b.dataset.act;
      if (POWER_Q[act]) {
        pendingPower = act;
        $('pwQ').textContent = POWER_Q[act];
        $('powerSec').classList.add('confirming');
        clearTimeout(pwTimer);
        pwTimer = setTimeout(closeConfirm, 8000);
        return;
      }
      if (demo) return toast(`Demo: ${act}`);
      post({ type: 'cmd', name: 'power', action: act });
    });
    $('pwYes').addEventListener('click', () => {
      const act = pendingPower;
      closeConfirm();
      if (!act) return;
      if (demo) return toast(`Demo: ${act}`);
      post({ type: 'cmd', name: 'power', action: act });
    });
    $('pwNo').addEventListener('click', closeConfirm);

    // ── szekciók láthatósága és sorrendje (a beállításokból) ──
    let claudeEnabled = true;
    let sections = { claude: true, activity: true, clock: true, date: true, order: ['claude', 'activity', 'clock', 'date'] };
    const SECTION_EL = { claude: 'claudeSec', activity: 'act', clock: 'clockSec', date: 'dateSec' };
    const applySections = () => {
      $('claudeSec').style.display = claudeEnabled && sections.claude !== false ? '' : 'none';
      $('act').style.display = sections.activity !== false ? '' : 'none';
      $('clockSec').style.display = sections.clock !== false ? '' : 'none';
      $('dateSec').style.display = sections.date !== false ? '' : 'none';
      // a négy szekció a beállított sorrendben, az időjárás előtt
      const anchor = $('weatherSec');
      for (const k of sections.order || []) {
        const el = $(SECTION_EL[k]);
        if (el) root.insertBefore(el, anchor);
      }
    };

    // ── óra, dátum és a hét sorszáma (ISO 8601: a hét hétfőn kezdődik) ──
    const isoWeek = (d) => {
      const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
      const day = t.getUTCDay() || 7;
      t.setUTCDate(t.getUTCDate() + 4 - day);
      const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
      return Math.ceil(((t - yearStart) / 86400000 + 1) / 7);
    };
    const segs = [...$('segs').children];
    const tick = () => {
      const d = new Date();
      const s = d.getSeconds();
      $('hh').textContent = pad(d.getHours());
      $('mm').textContent = pad(d.getMinutes());
      $('ss').textContent = pad(s);
      const cur = Math.floor(s / 5);
      segs.forEach((el, i) => { el.classList.toggle('on', i < cur); el.classList.toggle('now', i === cur); });
      $('dy').textContent = T('dateYear', { y: d.getFullYear() });
      $('dmd').textContent = T('dateMonthDay', { m: pad(d.getMonth() + 1), d: pad(d.getDate()) });
      $('week').textContent = T('week', { n: isoWeek(d) });
      actTick();
      if (s % 30 === 0) renderUsage();
      setTimeout(tick, 1000 - new Date().getMilliseconds() + 5);
    };

    // ── Claude-használat ──
    let usage = null;
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
    const renderUsage = () => {
      const box = $('usage');
      if (!usage) return;
      $('plan').textContent = (usage.plan || '').toUpperCase();
      if (usage.status !== 'ok') {
        const text = usage.status === 'error' ? `${T('usageError')} ${esc(usage.message || '')}` : T('usageLogin');
        box.innerHTML = `<div class="note ${usage.status}">${text}</div>
          <button class="ib login" id="claudeLogin">${T('login')}</button>`;
        $('claudeLogin').addEventListener('click', () => post({ type: 'cmd', name: 'claude-login' }));
        return;
      }
      box.innerHTML = usage.limits.map((l) => {
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
    };
    $('usageRefresh').addEventListener('click', () => {
      $('usageTime').textContent = `${T('refresh')}…`;
      post({ type: 'usage-refresh' });
    });

    // ── aktivitás-visszaszámláló ──
    const KEY = 'cyberhub.activity';
    const KEY_MIN = 'cyberhub.activity.minutes';
    const store = {
      get(k) { try { return JSON.parse(localStorage.getItem(k) || 'null'); } catch (_) { return null; } },
      set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (_) {} },
    };
    let cfgAct = { minutes: 20, autoStart: false };
    let session = 0;
    let act = null; // { running, endAt, remaining, alert, session }
    const minutes = () => store.get(KEY_MIN) || cfgAct.minutes;
    const total = () => minutes() * 60000;
    const save = () => store.set(KEY, act);
    const remaining = () => (act.running ? act.endAt - Date.now() : act.remaining);
    // az alert állapotot a setActAlert kapcsolja, itt csak megőrizzük
    const startCycle = (ms = total()) => {
      act = { running: true, endAt: Date.now() + ms, remaining: ms, alert: act ? act.alert : false, session };
      save();
    };
    const setActAlert = (on) => {
      if (act.alert === on) return;
      act.alert = on;
      save();
      $('act').classList.toggle('alerting', on);
      setAlert(on);
      post({ type: 'alert', on });
    };
    const actSegs = [...$('actSegs').children];
    function actTick() {
      if (!act) return;
      let left = remaining();
      if (act.running && left <= 0) {
        act.running = false;
        act.remaining = 0;
        left = 0;
        setActAlert(true);
      }
      const s = Math.ceil(Math.max(0, left) / 1000);
      $('actTime').textContent = `${pad(Math.floor(s / 60))}:${pad(s % 60)}`;
      const n = Math.ceil((Math.max(0, left) / total()) * actSegs.length);
      actSegs.forEach((el, i) => el.classList.toggle('on', i < n));
      $('act').classList.toggle('paused', !act.running && !act.alert);
      if (act.running) {
        const end = new Date(act.endAt);
        $('actInfo').textContent = T('actMoveAt', { t: `${pad(end.getHours())}:${pad(end.getMinutes())}` });
      } else if (!act.alert) {
        $('actInfo').textContent = T('actPaused');
      }
      $('actToggle').innerHTML = act.running ? svg('pause') : svg('play');
    }
    $('actToggle').addEventListener('click', () => {
      if (act.running) { act.remaining = Math.max(0, act.endAt - Date.now()); act.running = false; }
      else { act.endAt = Date.now() + (act.remaining > 0 ? act.remaining : total()); act.running = true; }
      save(); actTick();
    });
    $('actReset').addEventListener('click', () => { startCycle(); actTick(); });
    $('actDone').addEventListener('click', () => { startCycle(); setActAlert(false); actTick(); });
    $('actSnooze').addEventListener('click', () => { startCycle(5 * 60000); setActAlert(false); actTick(); });

    // egyedi hossz beállítása
    const actEl = $('act');
    const input = $('actMin');
    const openEdit = () => {
      input.value = minutes();
      $('actErr').textContent = '';
      actEl.classList.add('editing');
      input.focus();
      input.select();
    };
    const closeEdit = () => actEl.classList.remove('editing');
    const saveEdit = () => {
      const v = Number(input.value);
      if (!Number.isInteger(v) || v < 1 || v > 240) {
        $('actErr').textContent = T('actError');
        input.focus();
        return;
      }
      store.set(KEY_MIN, v);
      closeEdit();
      startCycle();
      actTick();
    };
    $('actSet').addEventListener('click', openEdit);
    $('actSave').addEventListener('click', saveEdit);
    $('actCancel').addEventListener('click', closeEdit);
    input.addEventListener('input', () => { $('actErr').textContent = ''; });
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') saveEdit();
      if (e.key === 'Escape') closeEdit();
    });

    // A program új indulásakor (új munkamenet) a visszaszámláló nem indul magától,
    // csak ha a beállítás ezt kéri. A felület újratöltésekor viszont folytatódik.
    const initActivity = () => {
      const saved = store.get(KEY);
      if (saved && typeof saved.running === 'boolean' && saved.session === session) {
        act = saved;
        const wasAlert = act.alert;
        act.alert = false;
        if (wasAlert) setActAlert(true);
      } else if (cfgAct.autoStart) {
        act = null;
        startCycle();
      } else {
        act = { running: false, endAt: 0, remaining: total(), alert: false, session };
        save();
      }
      actTick();
    };

    // ── időjárás (Open-Meteo, kulcs nélkül) ──
    const li = lang === 'en' ? 1 : 0;
    let wcfg = null;
    let wTimer = 0;
    const fetchJson = async (url) => {
      const r = await fetch(url, { cache: 'no-store' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return r.json();
    };
    const fetchWeather = async () => {
      clearTimeout(wTimer);
      wTimer = setTimeout(fetchWeather, 15 * 60000);
      try {
        let lat = wcfg.latitude, lon = wcfg.longitude, place = wcfg.city || '';
        if (lat == null || lon == null) {
          const key = `cyberhub.geo.${place.toLowerCase()}`;
          let geo = store.get(key);
          if (!geo) {
            const g = await fetchJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(place)}&count=1&language=${lang}`);
            const hit = g.results && g.results[0];
            if (!hit) throw new Error(T('unknownPlace', { p: place }));
            geo = { lat: hit.latitude, lon: hit.longitude, name: hit.name };
            store.set(key, geo);
          }
          lat = geo.lat; lon = geo.lon; place = geo.name;
        }
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}` +
          '&current=temperature_2m,weather_code,is_day,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1');
        const c = w.current;
        const row = window.HUB_WMO[c.weather_code] || ['Ismeretlen', 'Unknown', 'overcast'];
        let icon = row[2];
        if (!c.is_day && icon === 'sun') icon = 'moon';
        if (!c.is_day && icon === 'partly') icon = 'overcast';
        $('wIcon').innerHTML = svg(icon);
        $('wTemp').textContent = `${Math.round(c.temperature_2m)}°`;
        $('wDesc').textContent = row[li].toUpperCase();
        const max = Math.round(w.daily.temperature_2m_max[0]), min = Math.round(w.daily.temperature_2m_min[0]);
        $('wMinMax').textContent = `↑${max}° ↓${min}°`;
        $('wWind').textContent = `${Math.round(c.wind_speed_10m)} km/h`;
        $('wCity').textContent = place.toUpperCase();
        $('weatherSec').classList.remove('err');
      } catch (e) {
        $('wDesc').textContent = T('noData');
        $('wMinMax').textContent = String(e.message || e).slice(0, 30);
        $('wWind').innerHTML = '&nbsp;';
        $('weatherSec').classList.add('err');
        clearTimeout(wTimer);
        wTimer = setTimeout(fetchWeather, 2 * 60000);
      }
    };
    $('weatherSec').addEventListener('click', () => { if (wcfg) fetchWeather(); });

    tick();

    const h = (msg) => {
      if (msg.type === 'init') {
        if (msg.activity) cfgAct = { minutes: Math.max(1, msg.activity.minutes || 20), autoStart: msg.activity.autoStart === true };
        if (msg.session) session = msg.session;
        claudeEnabled = msg.claudeUsage !== false;
        if (msg.sections) sections = msg.sections;
        applySections();
        if (!act) initActivity();
        renderQuickApps(msg.apps);
        wcfg = msg.weather || { enabled: true, city: 'Budapest' };
        $('weatherSec').style.display = wcfg.enabled === false ? 'none' : '';
        if (wcfg.enabled !== false) fetchWeather();
      }
      if (msg.type === 'sections') { sections = msg.sections || sections; applySections(); }
      if (msg.type === 'settings-open') sysBtn.classList.toggle('open', !!msg.on);
      if (msg.type === 'state') applyQuickState(msg.running, msg.active);
      if (msg.type === 'toast') toast(msg.text);
      if (msg.type === 'usage') {
        usage = msg;
        // az adat saját időbélyege (állapotsorból érkezve ez lehet régebbi is)
        const t = new Date(msg.status === 'ok' && msg.fetchedAt ? msg.fetchedAt : Date.now());
        $('usageTime').textContent = `${pad(t.getHours())}:${pad(t.getMinutes())}`;
        renderUsage();
      }
    };
    h.demo = () => {
      h({ type: 'init', activity: { minutes: 20, autoStart: false }, session: 1, claudeUsage: true, weather: { enabled: true, city: 'Budapest' },
        apps: [{ id: 13, name: 'Cyberpunk', icon: 'game', color: '#ff3b5c' }] });
      const inH = (hrs) => new Date(Date.now() + hrs * 3600000).toISOString();
      h({ type: 'usage', status: 'ok', plan: 'pro', limits: [
        { key: 'five_hour', label: '5 órás', pct: 34, resetsAt: inH(2.3) },
        { key: 'seven_day', label: 'Heti', pct: 71, resetsAt: inH(76) },
      ] });
    };
    return h;
  }

  // ───────────── ablakelőnézet (több ablakos programok kisképei) ─────────────
  // A kisképeket a Windows rajzolja az ablakra, pontosan a .pk-frame helyére;
  // a felület csak a keretet, a címeket és a gombokat adja.
  function buildPeek() {
    root.className = 'peek';
    root.innerHTML = `<div class="pk-head" id="pkHead"></div><div class="pk-items" id="pkItems"></div>`;
    const html = document.documentElement;
    html.addEventListener('mouseenter', () => post({ type: 'peek-enter' }));
    html.addEventListener('mouseleave', () => post({ type: 'peek-leave' }));
    const box = (r) => `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;

    const render = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      $('pkHead').innerHTML = `<span class="pk-name">// ${esc(msg.name || '')}</span><span class="pk-count">${T('peekWindows', { n: msg.count })}</span>`;
      $('pkItems').innerHTML = (msg.items || []).map((it) => {
        const c = it.card;
        const rel = (r) => ({ x: r.x - c.x, y: r.y - c.y, w: r.w, h: r.h });
        const icon = it.icon ? `<img src="${esc(it.icon)}" alt="">` : svg('app');
        const inner = it.thumb
          ? `<i class="pk-frame" style="${box(rel(it.thumb))}"></i>`
          : `<div class="pk-min">${icon}<span>${T('peekMin')}</span></div>`;
        return `<div class="pk-card${it.active ? ' active' : ''}${it.iconic ? ' iconic' : ''}" data-hwnd="${it.hwnd}" style="${box(c)}">
            <div class="pk-title">${icon}<span>${esc(it.title)}</span><button class="pk-x" data-close>${svg('close')}</button></div>
            <div class="pk-box" style="${box(rel(it.box))}">${inner}</div>
          </div>`;
      }).join('');
      // kirajzolva: a program ekkor jegyzi be a kisképeket és jeleníti meg az ablakot
      post({ type: 'peek-ready', seq: msg.seq });
    };

    const closeCard = (card) => {
      card.classList.add('closing');
      post({ type: 'peek-close', hwnd: Number(card.dataset.hwnd) });
    };
    $('pkItems').addEventListener('click', (e) => {
      const card = e.target.closest('.pk-card');
      if (!card || card.classList.contains('closing')) return;
      if (e.target.closest('[data-close]')) return closeCard(card);
      post({ type: 'peek-activate', hwnd: Number(card.dataset.hwnd) });
    });
    // középső gomb: bezárás, mint a tálcán
    $('pkItems').addEventListener('auxclick', (e) => {
      const card = e.target.closest('.pk-card');
      if (card && e.button === 1 && !card.classList.contains('closing')) closeCard(card);
    });

    const h = (msg) => {
      if (msg.type === 'peek') render(msg);
      if (msg.type === 'peek-clear') { $('pkHead').innerHTML = ''; $('pkItems').innerHTML = ''; }
    };
    h.demo = () => {
      const card = (i) => ({ x: 14 + i * 248, y: 40, w: 236, h: 170 });
      const titles = ['Új lap – Mozilla Firefox', 'GitHub · cyberhub – Mozilla Firefox', 'Letöltések – Mozilla Firefox'];
      render({
        seq: 1, name: 'Firefox', count: 3,
        items: titles.map((title, i) => {
          const c = card(i);
          const b = { x: c.x + 6, y: c.y + 30, w: 224, h: 134 };
          return { hwnd: i + 1, title, icon: '', active: i === 1, iconic: i === 2, card: c, box: b,
            thumb: i === 2 ? null : { x: b.x, y: b.y + 4, w: 224, h: 126 } };
        }),
      });
    };
    return h;
  }

  // ───────────── beállítások (a munkaterületen, animálva) ─────────────
  function buildSettings() {
    root.className = 'settings';
    const ICON_CHOICES = ['browser', 'globe', 'code', 'folder', 'panels', 'terminal', 'doc', 'sheet', 'notes', 'music',
      'photo', 'enhance', 'design', 'picker', 'video', 'mail', 'chat', 'game', 'rocket', 'spark', 'calendar', 'cloud', 'settings', 'app'];
    root.innerHTML = `
      <div class="set-wrap">
        <header class="set-head">
          <div class="set-title">CYBER<span>HUB</span><em>${T('settings')}</em></div>
          <button class="set-close" id="setClose">${svg('close')}<span id="setCloseText">${T('close')}</span></button>
        </header>
        <div class="set-body">
          <aside class="set-side">
            <section class="set-card">
              <h2>${T('system')}</h2>
              <div class="set-grid">
                <button class="sbtn" data-cmd="reload">${svg('reload')}<span>${T('reload')}</span></button>
                <button class="sbtn" data-cmd="config">${svg('notes')}<span>${T('configFile')}</span></button>
                <button class="sbtn toggle" data-toggle="autostart">${svg('power')}<span>${T('startWithWindows')}</span><b class="switch">–</b></button>
                <button class="sbtn toggle" data-toggle="swap">${svg('swap')}<span>${T('swapPanels')}</span><b class="switch">–</b></button>
                <div class="lang-row">${svg('globe')}<span>${T('language')}</span>
                  <div class="seg" id="langSeg"><button data-lang="hu">HU</button><button data-lang="en">EN</button></div>
                </div>
                <button class="sbtn danger wide" data-cmd="exit">${svg('close')}<span>${T('exit')}</span></button>
              </div>
            </section>
            <section class="set-card">
              <h2>${T('visibleSections')}</h2>
              <div class="set-list" id="secList"></div>
            </section>
            <section class="set-card load">
              <h2>${T('systemLoad')}</h2>
              <div class="meter" id="cpuMeter">
                <div class="meter-head"><span>${T('cpu')}</span><b id="cpuVal">--<small>%</small></b></div>
                <svg class="spark" viewBox="0 0 120 40" preserveAspectRatio="none"><path class="spark-area" id="cpuArea"/><path class="spark-line" id="cpuLine"/></svg>
                <div class="meter-sub"><span id="cpuCores">&nbsp;</span><span id="cpuAvg">&nbsp;</span></div>
              </div>
              <div class="meter mem" id="memMeter">
                <div class="meter-head"><span>${T('memory')}</span><b id="memVal">--<small>%</small></b></div>
                <div class="mem-bar"><i id="memBar"></i></div>
                <svg class="spark" viewBox="0 0 120 40" preserveAspectRatio="none"><path class="spark-area" id="memArea"/><path class="spark-line" id="memLine"/></svg>
                <div class="meter-sub"><span id="memSub">&nbsp;</span><span id="memFree">&nbsp;</span></div>
              </div>
            </section>
            <section class="set-card credit">
              <div class="credit-by"><span>${T('madeBy')}</span><b>Szabó Gergő</b></div>
              <button class="sbtn mini" data-cmd="website">${svg('globe')}<span>${T('website')}</span></button>
            </section>
          </aside>
          <div class="set-col">
          <main class="set-card set-main">
            <h2>${T('icons')}</h2>
            <div class="ed-head"><span></span><span></span><span>${T('colName')}</span><span>${T('colProgram')}</span><span>${T('colIcon')}</span><span>${T('colColor')}</span><span>${T('colPlace')}</span><span>${T('colAuto')}</span><span></span></div>
            <div class="ed-list" id="edList"></div>
            <div class="ed-add">
              <button class="sbtn" id="edBrowse">${svg('plus')}<span>${T('addProgram')}</span></button>
              <div class="ed-running">
                <div class="ed-running-title">${T('runningTitle')}</div>
                <div class="ed-chips" id="edRunning"></div>
              </div>
            </div>
            <footer class="ed-foot">
              <span class="ed-dirty" id="edDirty"></span>
              <button class="sbtn" id="edCancel">${svg('close')}<span>${T('discard')}</span></button>
              <button class="sbtn primary" id="edSave">${svg('check')}<span>${T('save')}</span></button>
            </footer>
          </main>
          <section class="set-card wall">
            <h2>${T('wallpaper')}
              <span class="wall-tools">
                <button class="sbtn mini" id="wallAdd">${svg('plus')}<span>${T('addImage')}</span></button>
                <button class="sbtn mini" id="wallFolder">${svg('folder')}<span>${T('openFolder')}</span></button>
              </span>
            </h2>
            <div class="wall-list" id="wallList"></div>
          </section>
          </div>
        </div>
      </div>
      <div class="toast set-toast" id="toast"></div>`;

    let data = null;      // a programtól kapott legutóbbi állapot
    let draft = [];       // a szerkesztett ikonlista
    let original = '[]';
    const SECTION_NAMES = { claude: T('sClaude'), activity: T('sActivity'), clock: T('sClock'), date: T('sDate') };

    const toast = (text) => {
      const t = $('toast');
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toast.timer);
      toast.timer = setTimeout(() => t.classList.remove('show'), 3500);
    };

    // ── megjelenés és eltűnés animációval ──
    const show = (instant) => {
      root.classList.remove('leaving', 'shown', 'instant');
      void root.offsetWidth;
      root.classList.add('shown');
      if (instant) root.classList.add('instant');
    };
    let closing = false;
    const hide = () => {
      if (closing) return;
      closing = true;
      root.classList.remove('shown', 'instant');
      root.classList.add('leaving');
      setTimeout(() => {
        closing = false;
        if (!demo) post({ type: 'close' });
        else show();
      }, 230);
    };

    const editable = (a) => ({
      id: a.id, name: a.name, path: a.path, args: a.args || '', icon: a.rawIcon != null ? a.rawIcon : (a.icon || ''),
      color: a.color || '', place: a.place || 'left', img: a.img || '', canExeIcon: !!a.canExeIcon,
      available: a.available !== false, hint: a.hint || '', autostart: !!a.autostart,
    });
    const strip = (a) => ({ id: a.id, name: a.name, path: a.path, args: a.args, icon: a.icon, color: a.color, place: a.place, autostart: a.autostart });
    const dirty = () => JSON.stringify(draft.map(strip)) !== original;

    const iconPreview = (a) => {
      if (a.icon === 'exe' && a.img) return `<img src="${esc(a.img)}" alt="">`;
      if (a.icon && window.HUB_ICONS[a.icon]) return svg(a.icon);
      const hay = `${a.name} ${a.path}`.toLowerCase();
      const hit = (window.HUB_GUESS || []).find(([re]) => re.test(hay));
      return svg(hit ? hit[1] : 'app');
    };

    const renderToggles = () => {
      for (const b of root.querySelectorAll('[data-toggle]')) {
        const on = !!data[b.dataset.toggle];
        b.classList.toggle('on', on);
        b.querySelector('.switch').textContent = on ? 'ON' : 'OFF';
        if (lang === 'hu') b.querySelector('.switch').textContent = on ? 'BE' : 'KI';
      }
      for (const b of $('langSeg').children) b.classList.toggle('on', b.dataset.lang === (data.language || lang));
      const order = (data.sections.order && data.sections.order.length) ? data.sections.order : Object.keys(SECTION_NAMES);
      $('secList').innerHTML = order.filter((k) => SECTION_NAMES[k]).map((k) => {
        const on = data.sections[k] !== false;
        const sw = lang === 'hu' ? (on ? 'BE' : 'KI') : (on ? 'ON' : 'OFF');
        return `<div class="sec-row" data-key="${k}">
            <span class="grip">${svg('grip')}</span>
            <button class="sbtn toggle row ${on ? 'on' : ''}" data-section="${k}"><span>${SECTION_NAMES[k]}</span><b class="switch">${sw}</b></button>
          </div>`;
      }).join('');
    };

    // ── szekciók sorrendje: a bal oldali jellel húzható (egér/pointer alapon) ──
    let dragRow = null;
    $('secList').addEventListener('pointerdown', (e) => {
      const grip = e.target.closest('.grip');
      if (!grip) return;
      e.preventDefault();
      dragRow = grip.closest('.sec-row');
      dragRow.classList.add('dragging');
      grip.setPointerCapture(e.pointerId);
    });
    $('secList').addEventListener('pointermove', (e) => {
      if (!dragRow) return;
      const list = $('secList');
      const others = [...list.querySelectorAll('.sec-row')].filter((r) => r !== dragRow);
      const before = others.find((r) => {
        const rc = r.getBoundingClientRect();
        return e.clientY < rc.top + rc.height / 2;
      });
      if (before) { if (dragRow.nextElementSibling !== before) list.insertBefore(dragRow, before); }
      else if (list.lastElementChild !== dragRow) list.appendChild(dragRow);
    });
    const endDrag = () => {
      if (!dragRow) return;
      dragRow.classList.remove('dragging');
      dragRow = null;
      const order = [...$('secList').querySelectorAll('.sec-row')].map((r) => r.dataset.key);
      if (JSON.stringify(order) !== JSON.stringify(data.sections.order)) {
        data.sections = { ...data.sections, order };
        if (!demo) post({ type: 'sections', sections: data.sections });
      }
    };
    $('secList').addEventListener('pointerup', endDrag);
    $('secList').addEventListener('pointercancel', endDrag);

    const renderRunning = () => {
      const have = new Set(draft.map((a) => a.path.toLowerCase()));
      const list = (data.running || []).filter((r) => !have.has(r.path.toLowerCase()));
      $('edRunning').innerHTML = list.length
        ? list.map((r, i) => `<button class="run-chip" data-run="${i}"><img src="${esc(r.img)}" alt=""><span>${esc(r.name)}</span>${svg('plus')}</button>`).join('')
        : `<span class="ed-none">${T('noRunning')}</span>`;
      $('edRunning').dataset.list = JSON.stringify(list);
    };

    const renderList = () => {
      $('edList').innerHTML = draft.map((a, i) => {
        const opts = [
          a.canExeIcon ? `<option value="exe"${a.icon === 'exe' ? ' selected' : ''}>${T('programIcon')}</option>` : '',
          `<option value=""${!a.icon ? ' selected' : ''}>${T('automatic')}</option>`,
          ...ICON_CHOICES.map((n) => `<option value="${n}"${a.icon === n ? ' selected' : ''}>${n}</option>`),
        ].join('');
        const color = /^#[0-9a-f]{6}$/i.test(a.color) ? a.color : '#8a8f9c';
        return `
          <div class="ed-row${a.available ? '' : ' missing'}" data-i="${i}">
            <div class="ed-move">
              <button class="ib" data-act="up"${i === 0 ? ' disabled' : ''}>${svg('up')}</button>
              <button class="ib" data-act="down"${i === draft.length - 1 ? ' disabled' : ''}>${svg('down')}</button>
            </div>
            <span class="ed-icon"${a.color ? ` style="--app:${esc(a.color)}"` : ''}>${iconPreview(a)}</span>
            <input class="ed-name" data-field="name" value="${esc(a.name)}" maxlength="24" spellcheck="false">
            <div class="ed-path">${esc(a.path)}${a.available ? '' : `<b>${T('notInstalled')}</b>`}</div>
            <select class="ed-sel" data-field="icon">${opts}</select>
            <input class="ed-color" type="color" data-field="color" value="${color}">
            <div class="seg">
              <button data-place="left" class="${a.place !== 'right' ? 'on' : ''}">${T('left')}</button>
              <button data-place="right" class="${a.place === 'right' ? 'on' : ''}">${T('right')}</button>
            </div>
            <button class="ib auto${a.autostart ? ' on' : ''}" data-act="auto">${svg('power')}</button>
            <button class="ib danger" data-act="del">${svg('trash')}</button>
          </div>`;
      }).join('') || `<div class="ed-none">${T('noIcons')}</div>`;
      renderRunning();
      updateDirty();
    };

    const updateDirty = () => {
      const d = dirty();
      $('edDirty').textContent = d ? T('unsaved') : '';
      $('edSave').disabled = !d;
      $('edCancel').disabled = !d;
    };

    // háttérképek: kattintásra a program beállítja a Windows háttérképének
    const normPath = (p) => String(p || '').replace(/\//g, '\\').toLowerCase();
    const renderWalls = () => {
      const cur = normPath(data.wallpaper);
      const list = data.wallpapers || [];
      $('wallList').innerHTML = list.length
        ? list.map((w, i) => `<button class="wall-tile${normPath(w.path) === cur ? ' on' : ''}" data-wall="${i}">
            <img src="${esc(w.url)}" alt="" draggable="false"><b>${T('wallCurrent')}</b></button>`).join('')
        : `<span class="ed-none">${T('noWallpapers')}</span>`;
    };
    $('wallList').addEventListener('click', (e) => {
      const b = e.target.closest('[data-wall]');
      if (!b || !data) return;
      const w = (data.wallpapers || [])[Number(b.dataset.wall)];
      if (!w) return;
      for (const t of $('wallList').querySelectorAll('.wall-tile')) t.classList.toggle('on', t === b);
      data.wallpaper = w.path;
      if (demo) return toast(T('wallDemo'));
      post({ type: 'wall-set', path: w.path });
    });
    $('wallAdd').addEventListener('click', () => (demo ? toast('Demo: add image') : post({ type: 'wall-add' })));
    $('wallFolder').addEventListener('click', () => (demo ? toast('Demo: folder') : post({ type: 'wall-folder' })));

    const load = (msg) => {
      data = msg;
      if (!data.sections) data.sections = { claude: true, activity: true, clock: true, date: true };
      renderToggles();
      renderWalls();
      // ha épp szerkesztesz, a listádat nem írjuk felül
      if (!dirty() || !draft.length) {
        draft = (msg.apps || []).map(editable);
        original = JSON.stringify(draft.map(strip));
        renderList();
      } else {
        renderRunning();
      }
    };

    // lista szerkesztése
    $('edList').addEventListener('click', (e) => {
      const row = e.target.closest('.ed-row');
      if (!row) return;
      const i = Number(row.dataset.i);
      const b = e.target.closest('button');
      if (!b) return;
      if (b.dataset.place) { draft[i].place = b.dataset.place; return renderList(); }
      const act = b.dataset.act;
      if (act === 'up' && i > 0) [draft[i - 1], draft[i]] = [draft[i], draft[i - 1]];
      if (act === 'down' && i < draft.length - 1) [draft[i + 1], draft[i]] = [draft[i], draft[i + 1]];
      if (act === 'del') draft.splice(i, 1);
      if (act === 'auto') {
        const a = draft[i];
        a.autostart = !a.autostart;
        toast(a.autostart && !data.autostart ? T('autoHint') : T(a.autostart ? 'autoOn' : 'autoOff', { n: a.name }));
      }
      renderList();
    });
    $('edList').addEventListener('input', (e) => {
      const row = e.target.closest('.ed-row');
      if (!row || !e.target.dataset.field) return;
      const a = draft[Number(row.dataset.i)];
      a[e.target.dataset.field] = e.target.value;
      if (e.target.dataset.field !== 'name') {
        const ic = row.querySelector('.ed-icon');
        ic.innerHTML = iconPreview(a);
        if (a.color) ic.style.setProperty('--app', a.color);
      }
      updateDirty();
    });
    // a nevet ne lehessen üresre törölni
    $('edList').addEventListener('change', (e) => {
      if (e.target.dataset.field === 'name' && !e.target.value.trim()) {
        const a = draft[Number(e.target.closest('.ed-row').dataset.i)];
        a.name = T('untitled');
        e.target.value = a.name;
        toast(T('emptyName'));
        updateDirty();
      }
    });

    const addApp = (app) => {
      if (draft.some((a) => a.path.toLowerCase() === app.path.toLowerCase())) return toast(T('alreadyListed'));
      draft.push({
        id: null, name: app.name || 'Program', path: app.path, args: app.args || '', icon: app.icon || '',
        color: '', place: 'left', img: app.img || '', canExeIcon: !!app.canExeIcon, available: true, hint: '', autostart: false,
      });
      renderList();
      const rows = $('edList').querySelectorAll('.ed-row');
      if (rows.length) rows[rows.length - 1].scrollIntoView({ block: 'nearest' });
    };
    $('edRunning').addEventListener('click', (e) => {
      const b = e.target.closest('[data-run]');
      if (!b) return;
      const list = JSON.parse($('edRunning').dataset.list || '[]');
      const r = list[Number(b.dataset.run)];
      if (r) addApp({ name: r.name, path: r.path, icon: 'exe', img: r.img, canExeIcon: true });
    });
    $('edBrowse').addEventListener('click', () => {
      if (demo) return addApp({ name: 'Notepad', path: 'C:\\Windows\\notepad.exe', icon: '', canExeIcon: true });
      post({ type: 'browse' });
    });
    $('edSave').addEventListener('click', () => {
      if (draft.find((a) => !a.name.trim())) return toast(T('needName'));
      if (!demo) post({ type: 'save-apps', apps: draft.map(strip) });
      original = JSON.stringify(draft.map(strip));
      updateDirty();
    });
    $('edCancel').addEventListener('click', () => {
      draft = (data.apps || []).map(editable);
      original = JSON.stringify(draft.map(strip));
      renderList();
    });

    // rendszergombok, kapcsolók és nyelv
    root.querySelector('.set-side').addEventListener('click', (e) => {
      const b = e.target.closest('button');
      if (!b || !data) return;
      if (b.dataset.cmd) {
        if (demo) return toast(`Demo: ${b.dataset.cmd}`);
        return post({ type: 'cmd', name: b.dataset.cmd });
      }
      if (b.dataset.lang) {
        if (b.dataset.lang === (data.language || lang)) return;
        if (demo) { location.search = `?part=settings&lang=${b.dataset.lang}`; return; }
        return post({ type: 'language', lang: b.dataset.lang });
      }
      if (b.dataset.toggle === 'autostart') {
        if (demo) { data.autostart = !data.autostart; return renderToggles(); }
        return post({ type: 'cmd', name: 'autostart', on: !data.autostart });
      }
      if (b.dataset.toggle === 'swap') {
        if (demo) { data.swap = !data.swap; return renderToggles(); }
        return post({ type: 'swap', on: !data.swap });
      }
      if (b.dataset.section) {
        const k = b.dataset.section;
        data.sections = { ...data.sections, [k]: data.sections[k] === false };
        renderToggles();
        if (!demo) post({ type: 'sections', sections: data.sections });
      }
    });

    // bezárás: mentetlen változásnál előbb figyelmeztet
    let closeArmed = false;
    const close = () => {
      if (dirty() && !closeArmed) {
        closeArmed = true;
        $('setCloseText').textContent = T('discardQ');
        $('setClose').classList.add('armed');
        toast(T('unsavedWarn'));
        setTimeout(() => { closeArmed = false; $('setCloseText').textContent = T('close'); $('setClose').classList.remove('armed'); }, 4000);
        return;
      }
      closeArmed = false;
      if (dirty()) {
        draft = (data.apps || []).map(editable);
        original = JSON.stringify(draft.map(strip));
        renderList();
      }
      $('setCloseText').textContent = T('close');
      $('setClose').classList.remove('armed');
      hide();
    };
    $('setClose').addEventListener('click', close);
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !e.target.closest('input, select')) close();
    });

    // ── rendszerterhelés: az utolsó 60 másodperc görbéje ──
    const HIST = 60;
    const cpuHist = [];
    const memHist = [];
    const gb = (b) => (b / 1073741824).toFixed(1).replace('.', T('gbSep'));
    const sparkPaths = (hist) => {
      if (!hist.length) return ['', ''];
      const step = 120 / (HIST - 1);
      const x0 = 120 - (hist.length - 1) * step;
      const pts = hist.map((v, i) => `${(x0 + i * step).toFixed(2)},${(39 - (v / 100) * 37).toFixed(2)}`);
      const line = `M${pts.join(' L')}`;
      return [line, `${line} L120,40 L${x0.toFixed(2)},40 Z`];
    };
    const level = (v) => (v >= 85 ? 'hot' : v >= 60 ? 'warn' : '');
    const showSys = (s) => {
      if (typeof s.cpu === 'number') {
        cpuHist.push(s.cpu);
        if (cpuHist.length > HIST) cpuHist.shift();
        const [line, area] = sparkPaths(cpuHist);
        $('cpuLine').setAttribute('d', line);
        $('cpuArea').setAttribute('d', area);
        $('cpuVal').innerHTML = `${Math.round(s.cpu)}<small>%</small>`;
        $('cpuMeter').className = `meter ${level(s.cpu)}`;
        const avg = cpuHist.reduce((a, b) => a + b, 0) / cpuHist.length;
        $('cpuAvg').textContent = T('avg1m', { v: Math.round(avg) });
      }
      if (s.cores) $('cpuCores').textContent = T('threads', { n: s.cores });
      if (s.memTotal) {
        const pct = (s.memUsed / s.memTotal) * 100;
        memHist.push(pct);
        if (memHist.length > HIST) memHist.shift();
        const [line, area] = sparkPaths(memHist);
        $('memLine').setAttribute('d', line);
        $('memArea').setAttribute('d', area);
        $('memVal').innerHTML = `${Math.round(pct)}<small>%</small>`;
        $('memBar').style.width = `${pct.toFixed(1)}%`;
        $('memMeter').className = `meter mem ${level(pct)}`;
        $('memSub').textContent = `${gb(s.memUsed)} / ${gb(s.memTotal)} GB`;
        $('memFree').textContent = T('free', { v: gb(s.memTotal - s.memUsed) });
      }
    };

    const h = (msg) => {
      if (msg.type === 'sys') showSys(msg);
      if (msg.type === 'show') show(msg.instant);
      // a MENÜ gomb újabb megnyomása: ugyanúgy zár, mint a Bezárás gomb
      if (msg.type === 'hide') close();
      if (msg.type === 'settings') { if (msg.theme) setTheme(msg.theme); load(msg); }
      if (msg.type === 'picked' && msg.app) addApp(msg.app);
      if (msg.type === 'toast') toast(msg.text);
    };
    h.demo = () => {
      const C = ['#ff7139', '#ea4335', '#5b8def', '#d97757', '#22a6f2'];
      h({
        type: 'settings', autostart: true, swap: false, language: lang,
        sections: { claude: true, activity: true, clock: true, date: false, order: ['claude', 'activity', 'clock', 'date'] },
        apps: [['Firefox', 'globe'], ['Chrome', 'browser'], ['Total Cmd', 'panels'], ['Claude', 'spark'], ['VS Code', 'code']]
          .map(([name, icon], id) => ({ id, name, rawIcon: icon, icon, color: C[id], path: `C:\\Program Files\\${name}\\${name}.exe`,
            place: 'left', available: id !== 2, canExeIcon: true, autostart: id === 3 }))
          .concat([{ id: 5, name: 'Cyberpunk', rawIcon: 'game', icon: 'game', color: '#ff3b5c', path: 'steam://rungameid/1091500', place: 'right', available: true }]),
        running: [{ name: 'Notepad', path: 'C:\\Windows\\notepad.exe', img: '' }],
        wallpapers: ['01-dust-bridge', '02-tech-plates', '03-ink-alley', '04-neon-street', '05-hooded-runner', '06-red-moon']
          .map((n) => ({ path: `C:\\CyberHub\\wallpapers\\${n}.jpg`, url: `../wallpapers/${n}.jpg` })),
        wallpaper: 'C:\\CyberHub\\wallpapers\\04-neon-street.jpg',
      });
      show();
      // demó terhelés: véletlen bolyongás
      let c = 22, m = 58;
      const tot = 32 * 1073741824;
      const step = () => {
        c = Math.max(3, Math.min(97, c + (Math.random() - 0.48) * 14));
        m = Math.max(40, Math.min(80, m + (Math.random() - 0.5) * 2));
        showSys({ cpu: c, memUsed: (m / 100) * tot, memTotal: tot, cores: 16 });
      };
      for (let i = 0; i < HIST; i++) step();
      setInterval(step, 1000);
    };
    return h;
  }
})();
