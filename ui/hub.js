(() => {
  const q = new URLSearchParams(location.search);
  const part = q.get('part') || 'left';
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

  const parts = { left: buildLeft, right: buildRight, top: buildStrip, bottom: buildStrip };
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
    root.className = 'left';
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

    const applyState = (running, active) => {
      const run = new Set(running || []);
      for (const el of appsEl.querySelectorAll('.app.pinned')) {
        const id = Number(el.dataset.id);
        el.classList.toggle('running', run.has(id));
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
      for (const el of $('extras').querySelectorAll('.app')) el.classList.toggle('active', el.dataset.key === activeKey);
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
    appsEl.addEventListener('click', (e) => {
      const el = e.target.closest('.app');
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
      if (msg.type === 'state') { applyState(msg.running, msg.active); renderExtras(msg.extras, msg.activeExtra); }
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
      applyState(demoState.running, demoState.active);
      renderExtras([
        { key: 'demo:notepad', name: 'Jegyzettömb', icon: '' },
        { key: 'demo:calc', name: 'Számológép', icon: '' },
      ], null);
    };
    return h;
  }

  // ───────────── jobb oldali panel ─────────────
  function buildRight() {
    root.className = 'right';
    root.innerHTML = `
      <div class="edge"></div>
      <div class="quick">
        <button class="qbtn" data-cmd="search" style="--app:#4fd1c5"><span class="qtile">${svg('search')}</span><span class="qlabel">Keresés</span></button>
        <button class="qbtn" data-cmd="start" style="--app:#3b9eff"><span class="qtile">${svg('start')}</span><span class="qlabel">Indítás</span></button>
        <div class="quick-apps" id="quickApps"></div>
      </div>
      <section class="sec" id="claudeSec">
        <h3>// CLAUDE</h3>
        <div id="usage"><div class="note">Betöltés…</div></div>
        <div class="usage-foot">
          <button class="refresh" id="usageRefresh">${svg('reload')}<span id="usageTime">FRISSÍT</span></button>
          <span class="plan" id="plan"></span>
        </div>
      </section>
      <section class="sec act" id="act">
        <h3>// AKTIVITÁS</h3>
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
          <label class="act-edit-label" for="actMin">HOSSZ (PERC)</label>
          <input id="actMin" type="number" min="1" max="240" step="1" inputmode="numeric">
          <div class="act-err" id="actErr"></div>
          <div class="act-btns">
            <button class="ib" id="actSave">${svg('check')}</button>
            <button class="ib" id="actCancel">${svg('close')}</button>
          </div>
        </div>
        <div class="act-alert">
          <div class="act-alert-title">MOZDULJ MEG!</div>
          <div class="act-alert-text">Állj fel, nyújtózz, nézz messzire.</div>
          <div class="act-btns">
            <button class="ib" id="actDone">${svg('check')}</button>
            <button class="ib" id="actSnooze">+5</button>
          </div>
        </div>
      </section>
      <section class="sec clock" id="clockSec">
        <h3>// IDŐ</h3>
        <div class="clock-stack"><span id="hh">--</span><span id="mm">--</span></div>
        <div class="sec-line"><div class="segs" id="segs">${'<i></i>'.repeat(12)}</div><span class="ss" id="ss">--</span></div>
        <div class="date-stack"><span id="dy">----.</span><span id="dmd">--. --.</span></div>
      </section>
      <section class="sec weather" id="weatherSec">
        <h3>// IDŐJÁRÁS</h3>
        <div class="w-main"><span class="w-icon" id="wIcon"></span><span class="w-temp" id="wTemp">--°</span></div>
        <div class="w-desc" id="wDesc">&nbsp;</div>
        <div class="w-minmax" id="wMinMax">&nbsp;</div>
        <div class="w-wind" id="wWind">&nbsp;</div>
        <div class="w-city" id="wCity">&nbsp;</div>
      </section>
      <div class="sys">
        <button class="sys-btn" id="sysBtn">// MENÜ</button>
      </div>
      <div class="menu" id="menu">
        <button data-cmd="reload">${svg('reload')}Újratölt</button>
        <button data-cmd="config">${svg('settings')}Beállítás</button>
        <button data-cmd="autostart" class="toggle" id="autoBtn">${svg('power')}<span>Indulás<br>a géppel</span><b class="switch" id="autoState">KI</b></button>
        <button data-cmd="exit" class="danger">${svg('close')}Kilépés</button>
      </div>
      <div class="toast" id="toast"></div>`;

    // Keresés és Indítás: a Windows saját keresője és Start menüje; alattuk a
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

    // ── menü (a panel alján) ──
    const menu = $('menu');
    const sysBtn = $('sysBtn');
    let autostart = false;
    const showAutostart = (on) => {
      autostart = !!on;
      $('autoState').textContent = autostart ? 'BE' : 'KI';
      $('autoBtn').classList.toggle('on', autostart);
    };
    const anchorAbove = (el) => { el.style.bottom = `${root.clientHeight - sysBtn.offsetTop + 8}px`; };
    const toggleMenu = (open = !menu.classList.contains('open')) => {
      if (open) anchorAbove(menu);
      menu.classList.toggle('open', open);
      sysBtn.classList.toggle('open', open);
    };
    sysBtn.addEventListener('click', () => toggleMenu());
    root.addEventListener('contextmenu', () => toggleMenu());
    menu.addEventListener('click', (e) => {
      const b = e.target.closest('button');
      if (!b) return;
      const cmd = b.dataset.cmd;
      if (cmd === 'autostart') {
        // a kapcsoló nyitva hagyja a menüt, hogy látszódjon az új állapot
        if (demo) return showAutostart(!autostart);
        return post({ type: 'cmd', name: 'autostart', on: !autostart });
      }
      toggleMenu(false);
      if (demo) return toast(`Demó mód: „${cmd}” parancs`);
      post({ type: 'cmd', name: cmd });
    });
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') toggleMenu(false); });
    document.addEventListener('mousedown', (e) => {
      if (e.button === 0 && !e.target.closest('#menu, #sysBtn')) toggleMenu(false);
    });

    let toastTimer = 0;
    const toast = (text) => {
      const t = $('toast');
      anchorAbove(t);
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => t.classList.remove('show'), 4500);
    };

    // ── óra és dátum (csak számok) ──
    const segs = [...$('segs').children];
    const tick = () => {
      const d = new Date();
      const s = d.getSeconds();
      $('hh').textContent = pad(d.getHours());
      $('mm').textContent = pad(d.getMinutes());
      $('ss').textContent = pad(s);
      const cur = Math.floor(s / 5);
      segs.forEach((el, i) => { el.classList.toggle('on', i < cur); el.classList.toggle('now', i === cur); });
      $('dy').textContent = `${d.getFullYear()}.`;
      $('dmd').textContent = `${pad(d.getMonth() + 1)}. ${pad(d.getDate())}.`;
      actTick();
      if (s % 30 === 0) renderUsage();
      setTimeout(tick, 1000 - new Date().getMilliseconds() + 5);
    };

    // ── Claude-használat ──
    let usage = null;
    const rel = (iso) => {
      if (!iso) return '';
      const ms = new Date(iso).getTime() - Date.now();
      if (!(ms > 0)) return 'hamarosan';
      const m = Math.round(ms / 60000);
      const dd = Math.floor(m / 1440), hh = Math.floor((m % 1440) / 60), mm = m % 60;
      if (dd > 0) return `${dd} n ${hh} ó`;
      if (hh > 0) return `${hh} ó ${pad(mm)} p`;
      return `${mm} p`;
    };
    const renderUsage = () => {
      const box = $('usage');
      if (!usage) return;
      $('plan').textContent = (usage.plan || '').toUpperCase();
      if (usage.status !== 'ok') {
        const text = usage.status === 'error'
          ? `Nem elérhető. ${esc(usage.message || '')}`
          : 'Lépj be a claude.ai-ra a CyberHubban.';
        box.innerHTML = `<div class="note ${usage.status}">${text}</div>
          <button class="ib login" id="claudeLogin">Belépés</button>`;
        $('claudeLogin').addEventListener('click', () => post({ type: 'cmd', name: 'claude-login' }));
        return;
      }
      box.innerHTML = usage.limits.map((l) => {
        // ha az ablak azóta újraindult, az érték ismeretlen, amíg a Claude Code újra nem jelez
        const known = typeof l.pct === 'number';
        const pct = known ? Math.max(0, Math.min(100, Math.round(l.pct))) : 0;
        const lvl = pct >= 85 ? 'hot' : pct >= 60 ? 'warn' : '';
        const label = l.key === 'five_hour' ? '5 ÓRA' : l.key === 'seven_day' ? 'HÉT' : esc(l.label.toUpperCase());
        return `
          <div class="lim ${lvl}">
            <div class="lim-head"><span>${label}</span><b>${known ? `${pct}%` : '—'}</b></div>
            <div class="lim-bar"><i style="width:${pct}%"></i></div>
            <div class="lim-reset">${l.resetsAt ? `↻ ${rel(l.resetsAt)}` : '&nbsp;'}</div>
          </div>`;
      }).join('');
    };
    $('usageRefresh').addEventListener('click', () => {
      $('usageTime').textContent = 'FRISSÍT…';
      post({ type: 'usage-refresh' });
    });

    // ── aktivitás-visszaszámláló ──
    const KEY = 'cyberhub.activity';
    const KEY_MIN = 'cyberhub.activity.minutes';
    const store = {
      get(k) { try { return JSON.parse(localStorage.getItem(k) || 'null'); } catch (_) { return null; } },
      set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (_) {} },
    };
    let cfgAct = { minutes: 20, autoStart: true };
    let act = null; // { running, endAt, remaining, alert }
    const minutes = () => store.get(KEY_MIN) || cfgAct.minutes;
    const total = () => minutes() * 60000;
    const save = () => store.set(KEY, act);
    const remaining = () => (act.running ? act.endAt - Date.now() : act.remaining);
    // az alert állapotot a setActAlert kapcsolja, itt csak megőrizzük
    const startCycle = (ms = total()) => {
      act = { running: true, endAt: Date.now() + ms, remaining: ms, alert: act ? act.alert : false };
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
        $('actInfo').textContent = `MOZGÁS ${pad(end.getHours())}:${pad(end.getMinutes())}`;
      } else if (!act.alert) {
        $('actInfo').textContent = 'MEGÁLLÍTVA';
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
        $('actErr').textContent = '1 és 240 közötti egész szám kell.';
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

    const initActivity = () => {
      const saved = store.get(KEY);
      if (saved && typeof saved.running === 'boolean') {
        act = saved;
        const wasAlert = act.alert;
        act.alert = false;
        if (wasAlert) setActAlert(true);
      } else if (cfgAct.autoStart) {
        startCycle();
      } else {
        act = { running: false, endAt: 0, remaining: total(), alert: false };
        save();
      }
      actTick();
    };

    // ── időjárás (Open-Meteo, kulcs nélkül) ──
    const WMO = {
      0: ['Derült', 'sun'], 1: ['Többnyire derült', 'sun'], 2: ['Részben felhős', 'partly'], 3: ['Borult', 'overcast'],
      45: ['Köd', 'fog'], 48: ['Zúzmarás köd', 'fog'],
      51: ['Gyenge szitálás', 'rain'], 53: ['Szitálás', 'rain'], 55: ['Erős szitálás', 'rain'],
      56: ['Ónos szitálás', 'rain'], 57: ['Ónos szitálás', 'rain'],
      61: ['Gyenge eső', 'rain'], 63: ['Eső', 'rain'], 65: ['Heves eső', 'rain'], 66: ['Ónos eső', 'rain'], 67: ['Ónos eső', 'rain'],
      71: ['Gyenge havazás', 'snow'], 73: ['Havazás', 'snow'], 75: ['Erős havazás', 'snow'], 77: ['Hószemcse', 'snow'],
      80: ['Zápor', 'rain'], 81: ['Zápor', 'rain'], 82: ['Heves zápor', 'rain'], 85: ['Hózápor', 'snow'], 86: ['Hózápor', 'snow'],
      95: ['Zivatar', 'storm'], 96: ['Zivatar, jég', 'storm'], 99: ['Zivatar, jég', 'storm'],
    };
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
            const g = await fetchJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(place)}&count=1&language=hu`);
            const hit = g.results && g.results[0];
            if (!hit) throw new Error(`Nincs ilyen hely: ${place}`);
            geo = { lat: hit.latitude, lon: hit.longitude, name: hit.name };
            store.set(key, geo);
          }
          lat = geo.lat; lon = geo.lon; place = geo.name;
        }
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}` +
          '&current=temperature_2m,weather_code,is_day,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1');
        const c = w.current;
        let [desc, icon] = WMO[c.weather_code] || ['Ismeretlen', 'overcast'];
        if (!c.is_day && icon === 'sun') icon = 'moon';
        if (!c.is_day && icon === 'partly') icon = 'overcast';
        $('wIcon').innerHTML = svg(icon);
        $('wTemp').textContent = `${Math.round(c.temperature_2m)}°`;
        $('wDesc').textContent = desc.toUpperCase();
        const max = Math.round(w.daily.temperature_2m_max[0]), min = Math.round(w.daily.temperature_2m_min[0]);
        $('wMinMax').textContent = `↑${max}° ↓${min}°`;
        $('wWind').textContent = `${Math.round(c.wind_speed_10m)} km/h`;
        $('wCity').textContent = place.toUpperCase();
        $('weatherSec').classList.remove('err');
      } catch (e) {
        $('wDesc').textContent = 'NINCS ADAT';
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
        if (msg.activity) cfgAct = { minutes: Math.max(1, msg.activity.minutes || 20), autoStart: msg.activity.autoStart !== false };
        $('claudeSec').style.display = msg.claudeUsage === false ? 'none' : '';
        if (!act) initActivity();
        showAutostart(msg.autostart);
        wcfg = msg.weather || { enabled: true, city: 'Budapest' };
        $('weatherSec').style.display = wcfg.enabled === false ? 'none' : '';
        if (wcfg.enabled !== false) fetchWeather();
      }
      if (msg.type === 'init') renderQuickApps(msg.apps);
      if (msg.type === 'state') applyQuickState(msg.running, msg.active);
      if (msg.type === 'autostart') showAutostart(msg.on);
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
      h({ type: 'init', activity: { minutes: 20, autoStart: true }, claudeUsage: true, autostart: false, weather: { enabled: true, city: 'Budapest' },
        apps: [{ id: 13, name: 'Cyberpunk', icon: 'game', color: '#f3e600' }] });
      const inH = (hrs) => new Date(Date.now() + hrs * 3600000).toISOString();
      h({ type: 'usage', status: 'ok', plan: 'pro', limits: [
        { key: 'five_hour', label: '5 órás', pct: 34, resetsAt: inH(2.3) },
        { key: 'seven_day', label: 'Heti', pct: 71, resetsAt: inH(76) },
      ] });
    };
    return h;
  }
})();
