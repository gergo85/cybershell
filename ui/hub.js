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

  // helyi tároló (a felületek közösen használják, pl. a település koordinátáihoz)
  const store = {
    get(k) { try { return JSON.parse(localStorage.getItem(k) || 'null'); } catch (_) { return null; } },
    set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (_) {} },
  };

  // ── időjárás: közös segédek (Open-Meteo, kulcs nélkül) ──
  const fetchJson = async (url) => {
    const r = await fetch(url, { cache: 'no-store' });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return r.json();
  };
  // a település koordinátái (a keresés eredményét megjegyezzük)
  const resolvePlace = async (wcfg) => {
    let place = wcfg.city || '';
    if (wcfg.latitude != null && wcfg.longitude != null) return { lat: wcfg.latitude, lon: wcfg.longitude, name: place };
    const key = `cyberhub.geo.${place.toLowerCase()}`;
    let geo = store.get(key);
    if (!geo) {
      const g = await fetchJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(place)}&count=1&language=${lang}`);
      const hit = g.results && g.results[0];
      if (!hit) throw new Error(T('unknownPlace', { p: place }));
      geo = { lat: hit.latitude, lon: hit.longitude, name: hit.name };
      store.set(key, geo);
    }
    return geo;
  };
  const wmo = (code) => window.HUB_WMO[code] || ['Ismeretlen', 'Unknown', 'overcast'];
  const wmoText = (code) => wmo(code)[lang === 'en' ? 1 : 0];
  const wmoIcon = (code, isDay) => {
    const icon = wmo(code)[2];
    if (!isDay && icon === 'sun') return 'moon';
    if (!isDay && icon === 'partly') return 'overcast';
    return icon;
  };

  const parts = { left: buildLeft, right: buildRight, settings: buildSettings, peek: buildPeek, fly: buildFly, weather: buildFly, top: buildStrip, bottom: buildStrip };
  const handler = (parts[part] || buildStrip)();

  window.hub = {
    receive(msg) {
      // ha a beállított nyelv eltér attól, amivel az oldal betöltődött, átváltunk
      if (msg.language && msg.language !== lang && window.HUB_I18N[msg.language]) {
        const u = new URL(location.href);
        u.searchParams.set('lang', msg.language);
        post({ type: 'log', text: `nyelvváltás: ${lang} → ${msg.language}` });
        location.replace(u.toString());
        return;
      }
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
        <div class="act-panes">
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
      <section class="sec date" id="dateSec" title="${T('calOpen')}">
        <h3>${T('secDate')}</h3>
        <div class="date-stack"><span id="dy">----</span><span id="dmd">--</span></div>
        <div class="week" id="week">&nbsp;</div>
      </section>
      <section class="sec laptop" id="laptopSec" style="display:none">
        <h3>${T('secLaptop')}</h3>
        <div class="lap-grid">
          <button class="lap" id="lapWifi" data-cmd="wifi" style="--app:#4fd1c5">${svg('wifi')}<span>${T('wifi')}</span></button>
          <button class="lap" id="lapBat" data-cmd="battery" style="--app:#7ee787"><span class="bat" id="batIcon"></span><span id="batPct">--%</span></button>
        </div>
      </section>
      <section class="sec weather" id="weatherSec" title="${T('wDetails')}">
        <h3>${T('secWeather')}</h3>
        <div class="w-main"><span class="w-icon" id="wIcon"></span><span class="w-temp" id="wTemp">--°</span></div>
        <div class="w-desc" id="wDesc">&nbsp;</div>
        <div class="w-wind" id="wWind">&nbsp;</div>
        <div class="w-city" id="wCity">&nbsp;</div>
      </section>
      <section class="sec vol" id="volSec" title="${T('volHint')}">
        <h3>${T('secVolume')}</h3>
        <div class="vol-row">
          <button class="vol-mute" id="volMute">${svg('volume')}</button>
          <button class="vol-pct" id="volPct">--</button>
        </div>
        <div class="vol-bar" id="volBar"><span><i id="volFill"></i></span></div>
      </section>
      <section class="sec power" id="powerSec">
        <h3>${T('secPower')}</h3>
        <div class="pw-panes">
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
    let sections = { claude: true, activity: true, clock: true, date: true, order: ['claude', 'activity', 'date', 'clock'] };
    const SECTION_EL = { claude: 'claudeSec', activity: 'act', clock: 'clockSec', date: 'dateSec' };
    const applySections = () => {
      $('claudeSec').style.display = claudeEnabled && sections.claude !== false ? '' : 'none';
      $('act').style.display = sections.activity !== false ? '' : 'none';
      $('clockSec').style.display = sections.clock !== false ? '' : 'none';
      $('dateSec').style.display = sections.date !== false ? '' : 'none';
      $('volSec').style.display = sections.volume !== false ? '' : 'none';
      // a négy szekció a beállított sorrendben, a laptop és az időjárás előtt
      const anchor = $('laptopSec');
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
      // a claude.ai még nem válaszolt, vagy épp nem érhető el: nem kell újra belépni
      if (usage.status === 'wait' || usage.status === 'offline') {
        box.innerHTML = `<div class="note ${usage.status}">${T(usage.status === 'wait' ? 'usageWait' : 'usageOffline')}</div>`;
        return;
      }
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

    // Több képernyőnél minden információs panel ugyanazt a visszaszámlálót mutatja:
    // ha egy másik panelen változott, átvesszük.
    window.addEventListener('storage', (e) => {
      if (e.key !== KEY || !e.newValue) return;
      try {
        const v = JSON.parse(e.newValue);
        if (!v || typeof v.running !== 'boolean') return;
        const wasAlert = act && act.alert;
        act = v;
        if (!!v.alert !== !!wasAlert) {
          $('act').classList.toggle('alerting', !!v.alert);
          setAlert(!!v.alert);
        }
        actTick();
      } catch (_) {}
    });
    window.addEventListener('storage', (e) => { if (e.key === KEY_MIN) actTick(); });

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
    let wcfg = null;
    let wTimer = 0;
    const fetchWeather = async () => {
      clearTimeout(wTimer);
      wTimer = setTimeout(fetchWeather, 15 * 60000);
      try {
        const geo = await resolvePlace(wcfg);
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${geo.lat}&longitude=${geo.lon}` +
          '&current=temperature_2m,weather_code,is_day,wind_speed_10m,wind_direction_10m&timezone=auto&forecast_days=1');
        const c = w.current;
        $('wIcon').innerHTML = svg(wmoIcon(c.weather_code, c.is_day));
        $('wTemp').textContent = `${Math.round(c.temperature_2m)}°`;
        $('wDesc').textContent = wmoText(c.weather_code).toUpperCase();
        // a nyíl arra mutat, amerre a szél fúj (a jelentett irány az, ahonnan fúj)
        $('wWind').innerHTML = `<i class="w-arrow" style="transform:rotate(${(c.wind_direction_10m + 180) % 360}deg)">↑</i>` +
          `<b>${Math.round(c.wind_speed_10m)}</b><span>km/h</span>`;
        $('wCity').textContent = (geo.name || '').toUpperCase();
        $('weatherSec').classList.remove('err');
      } catch (e) {
        $('wDesc').textContent = T('noData');
        $('wWind').textContent = String(e.message || e).slice(0, 30);
        $('weatherSec').classList.add('err');
        clearTimeout(wTimer);
        wTimer = setTimeout(fetchWeather, 2 * 60000);
      }
    };
    // kattintásra a panel mellett kinyílik a részletes mai előrejelzés
    $('weatherSec').addEventListener('click', () => {
      if (!wcfg) return;
      const r = $('weatherSec').getBoundingClientRect();
      if (demo) return toast('Demo: weather details');
      post({ type: 'weather-open', top: Math.round(r.top), bottom: Math.round(r.bottom) });
    });

    // ── hangerő: a csúszka húzható és kattintható, a szekció fölött görgetni is lehet ──
    let vol = { ok: false, pct: 0, muted: false };
    let volDrag = false, volHold = 0, volSent = 0;
    const renderVol = () => {
      const sec = $('volSec');
      sec.classList.toggle('off', !vol.ok);
      sec.classList.toggle('muted', vol.ok && vol.muted);
      // a % jel kisebb, háromjegyű értéknél (100) a szám is: így a panel szélén sem lóg ki
      $('volPct').innerHTML = vol.ok ? `${vol.pct}<small>%</small>` : '—';
      $('volPct').classList.toggle('full', vol.ok && vol.pct >= 100);
      $('volFill').style.width = `${vol.ok ? vol.pct : 0}%`;
      $('volMute').innerHTML = svg(vol.ok && (vol.muted || vol.pct === 0) ? 'mute' : 'volume');
      $('volMute').title = T(vol.muted ? 'volUnmute' : 'volMuteT');
    };
    const setVol = (pct) => {
      pct = Math.max(0, Math.min(100, Math.round(pct)));
      if (!vol.ok) return;
      vol.pct = pct; vol.muted = false;
      volHold = Date.now() + 800; // a közben érkező régebbi állapot ne ugrassza vissza
      renderVol();
      if (demo) return;
      // húzás közben nem küldünk minden egyes mozdulatot
      const now = Date.now();
      if (now - volSent > 40 || !volDrag) { volSent = now; post({ type: 'volume-set', pct }); }
    };
    const volFromX = (x) => {
      const r = $('volBar').getBoundingClientRect();
      return ((x - r.left) / r.width) * 100;
    };
    $('volBar').addEventListener('pointerdown', (e) => {
      if (!vol.ok) return;
      volDrag = true;
      $('volBar').setPointerCapture(e.pointerId);
      setVol(volFromX(e.clientX));
    });
    $('volBar').addEventListener('pointermove', (e) => { if (volDrag) setVol(volFromX(e.clientX)); });
    const volEnd = (e) => {
      if (!volDrag) return;
      volDrag = false;
      setVol(volFromX(e.clientX));
      if (!demo) post({ type: 'volume-set', pct: vol.pct });
    };
    $('volBar').addEventListener('pointerup', volEnd);
    $('volBar').addEventListener('pointercancel', volEnd);
    $('volSec').addEventListener('wheel', (e) => {
      e.preventDefault();
      setVol(vol.pct + (e.deltaY < 0 ? 2 : -2));
    }, { passive: false });
    $('volMute').addEventListener('click', () => {
      if (!vol.ok) return;
      vol.muted = !vol.muted;
      volHold = Date.now() + 800;
      renderVol();
      if (!demo) post({ type: 'volume-mute', on: vol.muted });
    });
    $('volPct').addEventListener('click', () => {
      if (demo) return toast('Demo: volume mixer');
      post({ type: 'volume-open' });
    });
    if (demo) { vol = { ok: true, pct: 42, muted: false }; }
    renderVol();

    // a dátumra kattintva a naptár nyílik ki a panel mellett
    $('dateSec').addEventListener('click', () => {
      const r = $('dateSec').getBoundingClientRect();
      if (demo) return toast('Demo: calendar');
      post({ type: 'calendar-open', top: Math.round(r.top), bottom: Math.round(r.bottom) });
    });

    // ── laptop: Wi-Fi és akkumulátor (a Windows saját felületét nyitják meg) ──
    root.querySelector('.lap-grid').addEventListener('click', (e) => {
      const b = e.target.closest('.lap');
      if (!b) return;
      if (demo) return toast(`Demo: ${b.dataset.cmd}`);
      post({ type: 'cmd', name: b.dataset.cmd });
    });
    const batSvg = (pct, charging) => {
      const w = Math.max(0, Math.min(100, pct == null ? 0 : pct)) / 100 * 13;
      return `<svg viewBox="0 0 24 24"><path d="M3 8h16v8H3z"/><path d="M21 10.5v3" stroke-width="2"/>` +
        `<rect class="bat-fill" x="5" y="10" width="${w.toFixed(1)}" height="4"/>` +
        (charging ? '<path class="bat-bolt" d="M12.5 6.5l-3 5.5h4l-3 5.5"/>' : '') + '</svg>';
    };
    const showPower = (m) => {
      $('laptopSec').style.display = m.laptop ? '' : 'none';
      if (!m.laptop) return;
      const pct = typeof m.pct === 'number' ? m.pct : null;
      $('batIcon').innerHTML = batSvg(pct, m.charging);
      $('batPct').textContent = pct == null ? '--%' : `${pct}%`;
      const lap = $('lapBat');
      lap.classList.toggle('charging', !!m.charging);
      lap.classList.toggle('low', pct != null && pct <= 20 && !m.charging);
    };
    const showOnline = () => $('lapWifi').classList.toggle('offline', !navigator.onLine);
    window.addEventListener('online', showOnline);
    window.addEventListener('offline', showOnline);
    showOnline();

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
      if (msg.type === 'power') showPower(msg);
      if (msg.type === 'volume') {
        if (volDrag || Date.now() < volHold) return;
        vol = { ok: !!msg.ok, pct: msg.pct | 0, muted: !!msg.muted };
        renderVol();
      }
      if (msg.type === 'fly-state') {
        $('weatherSec').classList.toggle('open', !!msg.open && msg.kind === 'weather');
        $('dateSec').classList.toggle('open', !!msg.open && msg.kind === 'calendar');
      }
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
      h({ type: 'power', laptop: true, pct: 76, charging: true });
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

  // ───────────── oldalpanel: részletes időjárás és naptár (az információs panel mellett) ─────────────
  function buildFly() {
    root.className = 'wfly';
    root.innerHTML = `
      <div class="fview" id="vWeather">
        <header class="wf-head">
          <span class="wf-title">${T('secWeather')}<b id="wfCity"></b></span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="wf-body" id="wfBody"><div class="wf-note">${T('wLoading')}</div></div>
        <footer class="wf-foot" id="wfFoot">&nbsp;</footer>
      </div>
      <div class="fview" id="vCal">
        <header class="wf-head">
          <span class="wf-title">${T('secCalendar')}<b id="calTitle"></b></span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="cal-nav">
          <button class="cal-arrow" id="calPrev">${svg('left')}</button>
          <span class="cal-month" id="calMonth"></span>
          <button class="cal-today" id="calToday">${T('calToday')}</button>
          <button class="cal-arrow" id="calNext">${svg('right')}</button>
        </div>
        <div class="cal-grid" id="calGrid"></div>
        <div class="cal-day" id="calDay"></div>
        <footer class="cal-foot" id="calFoot"></footer>
      </div>`;
    const close = () => post({ type: 'close' });
    root.addEventListener('click', (e) => { if (e.target.closest('[data-close]')) close(); });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !e.target.closest('input')) close();
    });
    const showView = (id) => {
      for (const v of root.querySelectorAll('.fview')) v.classList.toggle('on', v.id === id);
      // minden megnyitáskor beúszik
      root.classList.remove('in');
      void root.offsetWidth;
      root.classList.add('in');
    };
    const locale = lang === 'en' ? 'en-GB' : 'hu-HU';
    const hm = (iso) => (iso || '').slice(11, 16);

    // ════════════ időjárás ════════════
    let wcfg = null;
    let last = null;   // { at, geo, w }
    let busy = false;
    const dec = (v) => (lang === 'en' ? v : v.replace('.', ','));
    const uvText = (u) => (u < 3 ? T('wUvLow') : u < 6 ? T('wUvMid') : u < 8 ? T('wUvHigh') : T('wUvVeryHigh'));
    const dayName = (iso) => new Intl.DateTimeFormat(locale, { weekday: 'short' })
      .format(new Date(`${iso}T12:00:00`)).replace('.', '').toUpperCase();

    const renderWeather = () => {
      const { geo, w, at } = last;
      const c = w.current, d = w.daily, hr = w.hourly;
      $('wfCity').textContent = (geo.name || '').toUpperCase();
      // a következő 24 óra háromóránként, a mostani órától
      const nowHour = c.time.slice(0, 13);
      let i0 = hr.time.findIndex((t) => t.slice(0, 13) === nowHour);
      if (i0 < 0) i0 = 0;
      const hours = [];
      for (let i = i0; i < hr.time.length && hours.length < 8; i += 3) hours.push(i);
      const temps = hours.map((i) => hr.temperature_2m[i]);
      const tMin = Math.min(...temps), tMax = Math.max(...temps);
      const yOf = (t) => (tMax === tMin ? 50 : 80 - ((t - tMin) / (tMax - tMin)) * 60);
      const xOf = (k) => (k + 0.5) * (100 / hours.length);
      const line = temps.map((t, k) => `${k ? 'L' : 'M'}${xOf(k).toFixed(2)},${yOf(t).toFixed(2)}`).join(' ');
      const prob = (i) => hr.precipitation_probability[i] ?? 0;
      const uv = d.uv_index_max[0];
      const tiles = [
        ['thermo', T('wFeels'), `${Math.round(c.apparent_temperature)}°`, ''],
        ['drop', T('wHumidity'), `${Math.round(c.relative_humidity_2m)}%`, ''],
        ['wind', T('wWindGust'), `${Math.round(c.wind_speed_10m)}/${Math.round(c.wind_gusts_10m)}`, 'km/h'],
        ['rain', T('wPrecip'), dec((d.precipitation_sum[0] || 0).toFixed(1)), `mm · ${d.precipitation_probability_max[0] ?? 0}%`],
        ['uv', T('wUv'), `${Math.round(uv ?? 0)}`, uv == null ? '' : uvText(uv)],
        ['sunrise', T('wSun'), hm(d.sunrise[0]), hm(d.sunset[0])],
      ];
      const days = [1, 2, 3].filter((k) => d.time[k]);
      const dMin = Math.min(...days.map((k) => d.temperature_2m_min[k]));
      const dMax = Math.max(...days.map((k) => d.temperature_2m_max[k]));
      const pct = (t) => ((t - dMin) / Math.max(1, dMax - dMin)) * 100;
      $('wfBody').innerHTML = `
        <section class="wf-hero">
          <span class="wf-icon">${svg(wmoIcon(c.weather_code, c.is_day))}</span>
          <div class="wf-now">
            <b>${Math.round(c.temperature_2m)}°</b>
            <span>${esc(wmoText(c.weather_code).toUpperCase())}</span>
            <em>${T('wMinMax', { min: Math.round(d.temperature_2m_min[0]), max: Math.round(d.temperature_2m_max[0]) })}</em>
          </div>
        </section>
        <section class="wf-sec">
          <h4>${T('wHourly')}</h4>
          <div class="wf-hours" style="--n:${hours.length}">
            <div class="wf-plot">
              <svg class="wf-curve" viewBox="0 0 100 100" preserveAspectRatio="none"><path d="${line}"/></svg>
              ${hours.map((i, k) => `<b class="wf-hv" style="left:${xOf(k).toFixed(2)}%;top:${yOf(temps[k]).toFixed(1)}%">${Math.round(temps[k])}°</b>`).join('')}
            </div>
            ${hours.map((i, k) => `
              <div class="wf-h${k === 0 ? ' now' : ''}">
                <span class="wf-hi">${svg(wmoIcon(hr.weather_code[i], hr.is_day[i]))}</span>
                <span class="wf-hp${prob(i) >= 50 ? ' wet' : ''}"><i style="height:${prob(i)}%"></i></span>
                <span class="wf-hpv">${prob(i)}%</span>
                <span class="wf-ht">${k === 0 ? T('wNow') : hm(hr.time[i])}</span>
              </div>`).join('')}
          </div>
        </section>
        <section class="wf-tiles">
          ${tiles.map(([ic, label, val, sub]) => `
            <div class="wf-tile">
              <span class="wf-tl">${svg(ic)}<span>${label}</span></span>
              <b>${val}${sub ? `<small>${esc(sub)}</small>` : ''}</b>
            </div>`).join('')}
        </section>
        <section class="wf-sec">
          <h4>${T('wDays')}</h4>
          <div class="wf-days">
            ${days.map((k) => `
              <div class="wf-day">
                <span class="wf-dn">${dayName(d.time[k])}</span>
                <span class="wf-di">${svg(wmoIcon(d.weather_code[k], 1))}</span>
                <span class="wf-dmin">${Math.round(d.temperature_2m_min[k])}°</span>
                <span class="wf-dbar"><i style="left:${pct(d.temperature_2m_min[k]).toFixed(1)}%;right:${(100 - pct(d.temperature_2m_max[k])).toFixed(1)}%"></i></span>
                <span class="wf-dmax">${Math.round(d.temperature_2m_max[k])}°</span>
              </div>`).join('')}
          </div>
        </section>`;
      const t = new Date(at);
      $('wfFoot').textContent = `${T('wUpdated', { t: `${pad(t.getHours())}:${pad(t.getMinutes())}` })} · Open-Meteo`;
    };

    const loadWeather = async (force) => {
      if (!wcfg || busy) return;
      if (last && !force && Date.now() - last.at < 10 * 60000) return renderWeather();
      busy = true;
      if (!last) $('wfBody').innerHTML = `<div class="wf-note">${T('wLoading')}</div>`;
      try {
        const geo = await resolvePlace(wcfg);
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${geo.lat}&longitude=${geo.lon}` +
          '&current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,is_day,wind_speed_10m,wind_gusts_10m' +
          '&hourly=temperature_2m,weather_code,precipitation_probability,is_day' +
          '&daily=weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset,uv_index_max,precipitation_sum,precipitation_probability_max' +
          '&timezone=auto&forecast_days=4');
        last = { at: Date.now(), geo, w };
        renderWeather();
      } catch (e) {
        post({ type: 'log', text: String(e.message || e) });
        if (!last) $('wfBody').innerHTML = `<div class="wf-note err">${T('noData')}<br>${esc(String(e.message || e).slice(0, 60))}</div>`;
      } finally {
        busy = false;
      }
    };

    // ════════════ naptár ════════════
    // Az eseményeket a program tölti le (iCal), itt csak értelmezzük és kirajzoljuk.
    const LOCAL_TZ = Intl.DateTimeFormat().resolvedOptions().timeZone;
    let feeds = [];          // csatolt naptárak: { name, color }
    let calEvents = [];      // értelmezett események
    let calStatus = [];      // letöltési eredmények: { name, color, ok, error }
    let calAt = 0;
    let calLoading = false;
    let addOpen = false;
    let view = new Date();   // a mutatott hónap
    let sel = new Date();    // a kiválasztott nap
    view = new Date(view.getFullYear(), view.getMonth(), 1);
    sel = new Date(sel.getFullYear(), sel.getMonth(), sel.getDate());

    const sameDay = (a, b) => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
    const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
    const isoWeek = (d) => {
      const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
      const day = t.getUTCDay() || 7;
      t.setUTCDate(t.getUTCDate() + 4 - day);
      return Math.ceil(((t - new Date(Date.UTC(t.getUTCFullYear(), 0, 1))) / 86400000 + 1) / 7);
    };

    // ── iCal értelmezése ──
    const unescapeIcs = (v) => v.replace(/\\n/gi, '\n').replace(/\\([,;\\])/g, '$1');
    const splitProp = (line) => {
      // az első idézőjelen kívüli kettőspont választja el a nevet és a paramétereket az értéktől
      let q = false;
      for (let i = 0; i < line.length; i++) {
        const ch = line[i];
        if (ch === '"') q = !q;
        else if (ch === ':' && !q) return [line.slice(0, i), line.slice(i + 1)];
      }
      return null;
    };
    const fmtCache = {};
    const tzOffset = (tz, t) => {
      const f = fmtCache[tz] || (fmtCache[tz] = new Intl.DateTimeFormat('en-US', {
        timeZone: tz, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
      }));
      const p = Object.fromEntries(f.formatToParts(new Date(t)).map((x) => [x.type, x.value]));
      return Date.UTC(+p.year, +p.month - 1, +p.day, +p.hour % 24, +p.minute, +p.second) - t;
    };
    const zoned = (y, mo, d, h, mi, s, tz) => {
      try {
        const guess = Date.UTC(y, mo, d, h, mi, s);
        const off = tzOffset(tz, guess);
        let t = guess - off;
        const off2 = tzOffset(tz, t);
        if (off2 !== off) t = guess - off2;
        return new Date(t);
      } catch (_) {
        // ismeretlen időzóna-név (pl. Windows-név): helyi időnek vesszük
        return new Date(y, mo, d, h, mi, s);
      }
    };
    const parseDt = (o) => {
      if (!o) return null;
      const v = o.v.trim();
      if (o.p.VALUE === 'DATE' || /^\d{8}$/.test(v)) {
        return { t: new Date(+v.slice(0, 4), +v.slice(4, 6) - 1, +v.slice(6, 8)), allDay: true };
      }
      const m = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})(Z?)$/.exec(v);
      if (!m) return null;
      const [y, mo, d, h, mi, s] = m.slice(1, 7).map(Number);
      if (m[7]) return { t: new Date(Date.UTC(y, mo - 1, d, h, mi, s)), allDay: false };
      const tz = o.p.TZID;
      if (!tz || tz === LOCAL_TZ) return { t: new Date(y, mo - 1, d, h, mi, s), allDay: false };
      return { t: zoned(y, mo - 1, d, h, mi, s, tz), allDay: false };
    };
    const parseDuration = (v) => {
      const m = /^([+-])?P(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$/.exec(v || '');
      if (!m) return 0;
      const ms = ((+m[2] || 0) * 7 * 86400 + (+m[3] || 0) * 86400 + (+m[4] || 0) * 3600 + (+m[5] || 0) * 60 + (+m[6] || 0)) * 1000;
      return m[1] === '-' ? -ms : ms;
    };
    const parseIcs = (text, feed) => {
      const lines = text.replace(/\r?\n[ \t]/g, '').split(/\r?\n/);
      const raw = [];
      let cur = null;
      let depth = 0; // a VEVENT-en belüli blokkok (pl. VALARM) kihagyása
      for (const line of lines) {
        if (line === 'BEGIN:VEVENT') { cur = { EXDATE: [] }; depth = 0; continue; }
        if (line === 'END:VEVENT') { if (cur) raw.push(cur); cur = null; continue; }
        if (!cur) continue;
        if (line.startsWith('BEGIN:')) { depth++; continue; }
        if (line.startsWith('END:')) { depth--; continue; }
        if (depth > 0) continue;
        const kv = splitProp(line);
        if (!kv) continue;
        const [head, val] = kv;
        const [name, ...params] = head.split(';');
        const p = {};
        for (const x of params) {
          const i = x.indexOf('=');
          if (i > 0) p[x.slice(0, i).toUpperCase()] = x.slice(i + 1).replace(/^"|"$/g, '');
        }
        const N = name.toUpperCase();
        if (N === 'EXDATE') for (const v of val.split(',')) cur.EXDATE.push({ v, p });
        else cur[N] = { v: val, p };
      }
      const out = [];
      // Névnap: évente ismétlődő, egész napos bejegyzés, aminek a címe csak keresztnév vagy
      // keresztnevek felsorolása („Rózsa”, „Konrád, Zelmira”), vagy a címében benne van, hogy névnap.
      // Ezek nem kapnak pöttyöt a naptárban, és a nap listájában külön sorban látszanak.
      const NAMES_RE = /^\p{Lu}\p{Ll}+(?:\s*,\s*\p{Lu}\p{Ll}+)*$/u;
      const isNameDay = (summary, allDay, rrule) =>
        /n[ée]vnap|name ?day/i.test(summary) || (allDay && !!rrule && rrule.FREQ === 'YEARLY' && NAMES_RE.test(summary.trim()));
      for (const r of raw) {
        if (r.STATUS && r.STATUS.v.toUpperCase() === 'CANCELLED') continue;
        const s = parseDt(r.DTSTART);
        if (!s) continue;
        const e = parseDt(r.DTEND);
        let dur = e ? e.t - s.t : r.DURATION ? parseDuration(r.DURATION.v) : s.allDay ? 86400000 : 0;
        if (dur < 0) dur = 0;
        let rrule = null;
        if (r.RRULE) rrule = Object.fromEntries(r.RRULE.v.split(';').map((x) => x.split('=')).map(([k, v]) => [k.toUpperCase(), v]));
        const summary = r.SUMMARY ? unescapeIcs(r.SUMMARY.v) : '—';
        out.push({
          uid: r.UID ? r.UID.v : '',
          nameday: isNameDay(summary, s.allDay, rrule),
          rid: r['RECURRENCE-ID'] ? parseDt(r['RECURRENCE-ID']) : null,
          summary,
          location: r.LOCATION ? unescapeIcs(r.LOCATION.v) : '',
          start: s.t, allDay: s.allDay, dur, rrule,
          exdates: r.EXDATE.map(parseDt).filter(Boolean).map((x) => +x.t),
          feed,
        });
      }
      // az ismétlődő esemény egyedileg módosított alkalmai: a sorozatból ezeket kihagyjuk
      const overridden = {};
      for (const ev of out) if (ev.rid) (overridden[ev.uid] || (overridden[ev.uid] = new Set())).add(+ev.rid.t);
      for (const ev of out) if (!ev.rid && ev.rrule) ev.overridden = overridden[ev.uid] || new Set();
      const nameUids = new Set(out.filter((ev) => ev.nameday && !ev.rid).map((ev) => ev.uid));
      for (const ev of out) if (ev.rid && nameUids.has(ev.uid)) ev.nameday = true;
      return out;
    };

    // ── ismétlődés kibontása egy időszakra (DAILY, WEEKLY, MONTHLY, YEARLY; INTERVAL, COUNT, UNTIL, BYDAY, BYMONTHDAY) ──
    const WD = { SU: 0, MO: 1, TU: 2, WE: 3, TH: 4, FR: 5, SA: 6 };
    const expand = (ev, from, to) => {
      const out = [];
      const push = (t) => {
        if (ev.exdates.includes(+t) || (ev.overridden && ev.overridden.has(+t))) return;
        const end = new Date(+t + ev.dur);
        if ((end > from || +end === +t && t >= from) && t < to) out.push({ ev, s: t, e: end });
      };
      if (!ev.rrule) { push(ev.start); return out; }
      const r = ev.rrule;
      const start = ev.start;
      const interval = Math.max(1, +(r.INTERVAL || 1));
      const count = r.COUNT ? +r.COUNT : Infinity;
      const until = r.UNTIL ? (parseDt({ v: r.UNTIL, p: {} }) || {}).t : null;
      const hh = start.getHours(), mm = start.getMinutes(), ss = start.getSeconds();
      const at = (y, m, d) => new Date(y, m, d, hh, mm, ss);
      const days = r.BYDAY ? r.BYDAY.split(',') : null;
      const nth = (y, m, spec) => {
        const mm2 = /^([+-]?\d+)?(SU|MO|TU|WE|TH|FR|SA)$/.exec(spec);
        if (!mm2) return [];
        const wd = WD[mm2[2]];
        const all = [];
        const first = new Date(y, m, 1);
        for (let d = new Date(first); d.getMonth() === first.getMonth(); d.setDate(d.getDate() + 1)) {
          if (d.getDay() === wd) all.push(at(d.getFullYear(), d.getMonth(), d.getDate()));
        }
        if (!mm2[1]) return all;
        const n = +mm2[1];
        const hit = n > 0 ? all[n - 1] : all[all.length + n];
        return hit ? [hit] : [];
      };
      // hosszú, darabszám nélküli sorozatoknál a mutatott időszak elé ugrunk
      let i0 = 0;
      const periodDays = { DAILY: 1, WEEKLY: 7 }[r.FREQ];
      if (periodDays && count === Infinity && from > start) {
        i0 = Math.max(0, Math.floor((from - start) / (periodDays * interval * 86400000)) - 1);
      }
      let n = 0;
      for (let i = i0; i < i0 + 3000 && n < count; i++) {
        let cands = [];
        if (r.FREQ === 'DAILY') {
          cands = [at(start.getFullYear(), start.getMonth(), start.getDate() + i * interval)];
        } else if (r.FREQ === 'WEEKLY') {
          const base = at(start.getFullYear(), start.getMonth(), start.getDate() + i * 7 * interval);
          if (!days) cands = [base];
          else {
            const mon = addDays(base, -((base.getDay() + 6) % 7));
            cands = days.map((d) => WD[d.slice(-2)]).filter((w) => w != null)
              .map((w) => at(mon.getFullYear(), mon.getMonth(), mon.getDate() + ((w + 6) % 7)))
              .sort((a, b) => a - b);
          }
        } else if (r.FREQ === 'MONTHLY') {
          const first = new Date(start.getFullYear(), start.getMonth() + i * interval, 1);
          const y = first.getFullYear(), m = first.getMonth();
          if (days) cands = days.flatMap((d) => nth(y, m, d));
          else if (r.BYMONTHDAY) {
            cands = r.BYMONTHDAY.split(',').map(Number)
              .map((d) => (d > 0 ? at(y, m, d) : at(y, m + 1, d + 1)))
              .filter((t) => t.getMonth() === m);
          } else {
            const t = at(y, m, start.getDate());
            if (t.getMonth() === m) cands = [t];
          }
          cands.sort((a, b) => a - b);
        } else if (r.FREQ === 'YEARLY') {
          const t = at(start.getFullYear() + i * interval, start.getMonth(), start.getDate());
          if (t.getMonth() === start.getMonth()) cands = [t];
        } else {
          push(start);
          break;
        }
        let stop = false;
        for (const t of cands) {
          if (t < start) continue;
          if ((until && t > until) || n >= count || t >= to) { stop = true; break; }
          n++;
          push(t);
        }
        if (stop) break;
      }
      return out;
    };

    // ── kirajzolás ──
    const gridStart = () => addDays(view, -((view.getDay() + 6) % 7));
    const occurrences = (from, to) => calEvents.flatMap((ev) => expand(ev, from, to));
    const dayOcc = (occ, d) => {
      const a = d, b = addDays(d, 1);
      return occ.filter((o) => o.s < b && (o.e > a || (+o.e === +o.s && o.s >= a)))
        .sort((x, y) => (y.ev.allDay - x.ev.allDay) || (x.s - y.s));
    };
    const timeOf = (d) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

    const renderCal = () => {
      const g0 = gridStart();
      const g1 = addDays(g0, 42);
      const occ = occurrences(g0, g1);
      const today = new Date();
      $('calMonth').textContent = new Intl.DateTimeFormat(locale, { year: 'numeric', month: 'long' }).format(view).toUpperCase();
      const wdNames = [...Array(7)].map((_, k) => new Intl.DateTimeFormat(locale, { weekday: 'narrow' })
        .format(new Date(2024, 0, 1 + k)).toUpperCase());
      let html = `<span class="cg-wk cg-h">${T('calWeekShort')}</span>` + wdNames.map((n, k) => `<span class="cg-h${k > 4 ? ' we' : ''}">${n}</span>`).join('');
      for (let w = 0; w < 6; w++) {
        const monday = addDays(g0, w * 7);
        html += `<span class="cg-wk">${isoWeek(monday)}</span>`;
        for (let k = 0; k < 7; k++) {
          const d = addDays(monday, k);
          const list = dayOcc(occ, d);
          const real = list.filter((o) => !o.ev.nameday);
          const colors = real.slice(0, 6).map((o) => o.ev.feed.color);
          const cls = ['cg-d'];
          if (d.getMonth() !== view.getMonth()) cls.push('out');
          if (k > 4) cls.push('we');
          if (sameDay(d, today)) cls.push('today');
          if (sameDay(d, sel)) cls.push('sel');
          const tip = real.length ? ` title="${real.length} ${T('calEventsN')}"` : '';
          html += `<button class="${cls.join(' ')}" data-day="${+d}"${tip}><span>${d.getDate()}</span>` +
            `<i class="cg-dots">${colors.map((c) => `<b style="--c:${esc(c)}"></b>`).join('')}</i></button>`;
        }
      }
      $('calGrid').innerHTML = html;

      // a kiválasztott nap eseményei
      const head = new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric', weekday: 'long' }).format(sel).toUpperCase();
      const dayAll = dayOcc(occurrences(sel, addDays(sel, 1)), sel);
      const names = [...new Set(dayAll.filter((o) => o.ev.nameday).map((o) => o.ev.summary.replace(/\s*n[ée]vnap(ja)?\s*/ig, ' ').trim()))];
      const list = dayAll.filter((o) => !o.ev.nameday);
      const nameHtml = names.length ? `<div class="cal-names">${svg('user')}<span>${T('calNameDay')}</span><b>${esc(names.join(', '))}</b></div>` : '';
      let body;
      if (!feeds.length) {
        body = connectHtml();
      } else if (calLoading && !calAt) {
        body = `<div class="wf-note">${T('wLoading')}</div>`;
      } else if (!list.length) {
        body = `${nameHtml}<div class="cal-empty">${T('calNoEvents')}</div>`;
      } else {
        body = `${nameHtml}<div class="cal-events">${list.map((o) => {
          const ev = o.ev;
          const when = ev.allDay ? T('calAllDay') : `${timeOf(o.s)}${ev.dur ? `<em>${timeOf(o.e)}</em>` : ''}`;
          const now = new Date();
          const live = !ev.allDay && o.s <= now && o.e > now;
          return `<div class="cal-ev${live ? ' live' : ''}" style="--c:${esc(ev.feed.color)}">
              <span class="cal-when">${when}</span>
              <span class="cal-what"><b>${esc(ev.summary)}</b>${ev.location ? `<small>${esc(ev.location)}</small>` : ''}</span>
            </div>`;
        }).join('')}</div>`;
      }
      $('calDay').innerHTML = `<h4 class="cal-dh">// ${esc(head)}</h4>${body}${feeds.length && addOpen ? connectHtml() : ''}`;
      renderFoot();
    };

    const connectHtml = () => `
      <div class="cal-connect">
        <div class="cal-c-title">${svg('calendar')}<b>${T(feeds.length ? 'calAddTitle' : 'calConnectTitle')}</b></div>
        <ol class="cal-steps">${T('calSteps')}</ol>
        <div class="cal-form">
          <input id="calUrl" type="text" spellcheck="false" placeholder="https://calendar.google.com/calendar/ical/…/basic.ics">
          <button class="cal-btn" id="calAdd">${T('calConnect')}</button>
        </div>
        <div class="cal-err" id="calErr"></div>
        <button class="cal-link" id="calHelp">${svg('globe')}<span>${T('calOpenGoogle')}</span></button>
      </div>`;

    const renderFoot = () => {
      if (!feeds.length) { $('calFoot').innerHTML = `<span class="cal-src">${T('calNoFeeds')}</span>`; return; }
      const chips = feeds.map((f, i) => {
        const st = calStatus.find((x) => x.name === f.name);
        const bad = st && !st.ok;
        return `<span class="cal-chip${bad ? ' bad' : ''}" style="--c:${esc(f.color)}" title="${bad ? esc(T('calFeedError', { e: st.error })) : ''}">` +
          `<i></i>${esc(f.name)}<button data-remove="${i}" title="${T('calRemove')}">${svg('close')}</button></span>`;
      }).join('');
      const t = calAt ? new Date(calAt) : null;
      $('calFoot').innerHTML = `<div class="cal-chips">${chips}<button class="cal-plus${addOpen ? ' on' : ''}" id="calPlus" title="${T('calAddTitle')}">${svg('plus')}</button></div>
        <button class="cal-refresh${calLoading ? ' busy' : ''}" id="calRefresh">${svg('reload')}<span>${t ? `${pad(t.getHours())}:${pad(t.getMinutes())}` : '--:--'}</span></button>`;
    };

    // kattintások a naptárnézetben
    $('vCal').addEventListener('click', (e) => {
      const day = e.target.closest('[data-day]');
      if (day) {
        sel = new Date(Number(day.dataset.day));
        if (sel.getMonth() !== view.getMonth()) view = new Date(sel.getFullYear(), sel.getMonth(), 1);
        return renderCal();
      }
      const rm = e.target.closest('[data-remove]');
      if (rm) {
        // második kattintásra törli (véletlen törlés ellen)
        if (rm.classList.contains('armed')) return post({ type: 'cal-remove', i: Number(rm.dataset.remove) });
        rm.classList.add('armed');
        setTimeout(() => rm.isConnected && rm.classList.remove('armed'), 2500);
        return;
      }
      const id = (e.target.closest('button') || {}).id;
      if (id === 'calPrev') { view = new Date(view.getFullYear(), view.getMonth() - 1, 1); renderCal(); }
      if (id === 'calNext') { view = new Date(view.getFullYear(), view.getMonth() + 1, 1); renderCal(); }
      if (id === 'calToday') {
        const t = new Date();
        sel = new Date(t.getFullYear(), t.getMonth(), t.getDate());
        view = new Date(t.getFullYear(), t.getMonth(), 1);
        renderCal();
      }
      if (id === 'calRefresh') { calLoading = true; renderFoot(); post({ type: 'cal-fetch', force: true }); }
      if (id === 'calPlus') { addOpen = !addOpen; renderCal(); if (addOpen) $('calUrl').focus(); }
      if (id === 'calHelp') post({ type: 'cal-help' });
      if (id === 'calAdd') submitUrl();
    });
    $('vCal').addEventListener('keydown', (e) => { if (e.key === 'Enter' && e.target.id === 'calUrl') submitUrl(); });
    // görgővel lapozás a hónapok között
    $('calGrid').addEventListener('wheel', (e) => {
      e.preventDefault();
      view = new Date(view.getFullYear(), view.getMonth() + (e.deltaY > 0 ? 1 : -1), 1);
      renderCal();
    }, { passive: false });
    const submitUrl = () => {
      const v = ($('calUrl').value || '').trim();
      if (!/^(https|webcal):\/\//i.test(v)) { $('calErr').textContent = T('calErrUrl'); return; }
      $('calErr').textContent = '';
      calLoading = true;
      if (demo) return toastDemo();
      post({ type: 'cal-add', url: v });
    };
    const toastDemo = () => { $('calErr').textContent = 'Demo'; };

    const onCalData = (list) => {
      calLoading = false;
      calAt = Date.now();
      calStatus = (list || []).map(({ name, color, ok, error }) => ({ name, color, ok, error }));
      calEvents = [];
      for (const f of list || []) {
        if (!f.ok) continue;
        try { calEvents.push(...parseIcs(f.ics, { name: f.name, color: f.color })); } catch (e) { post({ type: 'log', text: `iCal: ${e.message}` }); }
      }
      if (calStatus.some((x) => x.ok)) addOpen = false;
      renderCal();
    };

    const h = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      if (msg.type === 'weather-open') {
        const next = msg.weather || { city: 'Budapest' };
        if (wcfg && JSON.stringify(wcfg) !== JSON.stringify(next)) last = null;
        wcfg = next;
        showView('vWeather');
        loadWeather(false);
      }
      if (msg.type === 'calendar-open') {
        feeds = msg.feeds || [];
        if (!feeds.length) { calEvents = []; calStatus = []; calAt = 0; }
        const t = new Date();
        sel = new Date(t.getFullYear(), t.getMonth(), t.getDate());
        view = new Date(t.getFullYear(), t.getMonth(), 1);
        addOpen = false;
        showView('vCal');
        renderCal();
        if (feeds.length) { calLoading = true; renderFoot(); post({ type: 'cal-fetch', force: false }); }
      }
      if (msg.type === 'cal-feeds') {
        feeds = msg.feeds || [];
        if (!feeds.length) { calEvents = []; calStatus = []; calAt = 0; }
        renderCal();
      }
      if (msg.type === 'cal-data') onCalData(msg.feeds);
      if (msg.type === 'cal-error') {
        calLoading = false;
        const el = $('calErr');
        if (el) el.textContent = T(msg.error === 'dup' ? 'calErrDup' : 'calErrUrl');
        renderFoot();
      }
    };
    h.demo = () => {
      const part = new URLSearchParams(location.search).get('view');
      if (part !== 'calendar') return h({ type: 'weather-open', weather: { city: 'Budapest' } });
      h({ type: 'calendar-open', feeds: [{ name: 'Google Naptár', color: '#00e1ff' }, { name: 'Munka', color: '#f3e600' }] });
      const d = new Date();
      const ymd = (x) => `${x.getFullYear()}${pad(x.getMonth() + 1)}${pad(x.getDate())}`;
      const ics1 = ['BEGIN:VCALENDAR', 'BEGIN:VEVENT', 'UID:a', `DTSTART:${ymd(d)}T090000`, `DTEND:${ymd(d)}T093000`, 'SUMMARY:Napi egyeztetés',
        'RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:b', `DTSTART;VALUE=DATE:${ymd(d)}`, `DTEND;VALUE=DATE:${ymd(addDays(d, 1))}`, 'SUMMARY:Szülinap – Anna', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:n', `DTSTART;VALUE=DATE:${ymd(d)}`, `DTEND;VALUE=DATE:${ymd(addDays(d, 1))}`, 'RRULE:FREQ=YEARLY', 'SUMMARY:Jenő\, Ágota', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:c', `DTSTART:${ymd(addDays(d, 2))}T140000`, `DTEND:${ymd(addDays(d, 2))}T153000`, 'SUMMARY:Fotózás', 'LOCATION:Budapest\\, Margitsziget', 'END:VEVENT',
        'END:VCALENDAR'].join('\r\n');
      const ics2 = ['BEGIN:VCALENDAR', 'BEGIN:VEVENT', 'UID:d', `DTSTART:${ymd(d)}T130000`, `DTEND:${ymd(d)}T140000`, 'SUMMARY:Ügyféltalálkozó', 'LOCATION:Iroda', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:e', `DTSTART:${ymd(addDays(d, 5))}T100000`, `DTEND:${ymd(addDays(d, 5))}T110000`, 'SUMMARY:Havi riport', 'RRULE:FREQ=MONTHLY', 'END:VEVENT',
        'END:VCALENDAR'].join('\r\n');
      setTimeout(() => onCalData([
        { name: 'Google Naptár', color: '#00e1ff', ok: true, ics: ics1 },
        { name: 'Munka', color: '#f3e600', ok: true, ics: ics2 },
      ]), 300);
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
          <div class="toast set-toast" id="toast"></div>
          <button class="set-close" id="setClose">${svg('close')}<span id="setCloseText">${T('close')}</span></button>
        </header>
        <div class="set-body">
          <aside class="set-side">
            <section class="set-card">
              <h2>${T('system')}</h2>
              <div class="set-grid">
                <button class="sbtn" data-cmd="reload" title="${T('reloadHint')}">${svg('reload')}<span>${T('reload')}</span></button>
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
              <div class="set-list sec-fixed" id="secFixed"></div>
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
              <div class="credit-by credit-ver"><span>${T('version')}</span><b id="appVer">v1.2</b></div>
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
                <div class="ed-chips" id="edRunning" title="${T('runningTitle').replace(/^\/\/\s*/, '')}"></div>
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
      `;

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
    // A beúszó animáció közben a tartalom egy pillanatra lejjebb csúszik: ilyenkor a görgetősáv
    // ne villanjon fel a jobb szélen (sárga függőleges csík volt).
    let animTimer = 0;
    const noScrollFor = (ms) => {
      clearTimeout(animTimer);
      root.classList.add('anim');
      animTimer = setTimeout(() => root.classList.remove('anim'), ms);
    };
    const show = (instant) => {
      root.classList.remove('leaving', 'shown', 'instant');
      void root.offsetWidth;
      if (!instant) noScrollFor(650);
      root.classList.add('shown');
      if (instant) root.classList.add('instant');
    };
    let closing = false;
    const hide = () => {
      if (closing) return;
      closing = true;
      noScrollFor(400);
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
      // a hangerő helye rögzített (a főkapcsoló fölött), csak ki-be kapcsolható
      const von = data.sections.volume !== false;
      $('secFixed').innerHTML = `<div class="sec-row fixed">
          <span class="grip" title="${T('sFixedHint')}">${svg('volume')}</span>
          <button class="sbtn toggle row ${von ? 'on' : ''}" data-section="volume" title="${T('sFixedHint')}"><span>${T('sVolume')}</span><b class="switch">${lang === 'hu' ? (von ? 'BE' : 'KI') : (von ? 'ON' : 'OFF')}</b></button>
        </div>`;
    };

    // ── szekciók sorrendje: a bal oldali jellel húzható (egér/pointer alapon) ──
    // A húzott sor követi az egeret, a többi sor csúszva adja át a helyét; elengedéskor a sor
    // a végleges helyére úszik, és csak ekkor rendezzük át ténylegesen a listát.
    let drag = null;
    $('secList').addEventListener('pointerdown', (e) => {
      const grip = e.target.closest('.grip');
      if (!grip || drag) return;
      e.preventDefault();
      const list = $('secList');
      const rows = [...list.querySelectorAll('.sec-row')];
      const row = grip.closest('.sec-row');
      const from = rows.indexOf(row);
      const r0 = rows[0].getBoundingClientRect();
      const step = rows.length > 1 ? rows[1].getBoundingClientRect().top - r0.top : r0.height;
      drag = { row, rows, from, to: from, startY: e.clientY, step };
      row.classList.add('dragging');
      list.classList.add('sorting');
      grip.setPointerCapture(e.pointerId);
    });
    $('secList').addEventListener('pointermove', (e) => {
      if (!drag || drag.done) return;
      const { row, rows, from, step } = drag;
      const n = rows.length;
      const dy = Math.max(-from * step, Math.min((n - 1 - from) * step, e.clientY - drag.startY));
      row.style.transform = `translateY(${dy}px)`;
      const to = Math.max(0, Math.min(n - 1, Math.round(from + dy / step)));
      if (to === drag.to) return;
      drag.to = to;
      rows.forEach((r, i) => {
        if (r === row) return;
        let shift = 0;
        if (from < to && i > from && i <= to) shift = -step;
        if (from > to && i >= to && i < from) shift = step;
        r.style.transform = shift ? `translateY(${shift}px)` : '';
      });
    });
    const endDrag = () => {
      if (!drag || drag.done) return;
      const d = drag;
      d.done = true;
      const list = $('secList');
      // a végleges helyére úszik
      d.row.classList.remove('dragging');
      d.row.classList.add('settling');
      d.row.style.transform = `translateY(${(d.to - d.from) * d.step}px)`;
      setTimeout(() => {
        list.classList.remove('sorting');
        for (const r of d.rows) { r.style.transform = ''; r.classList.remove('settling'); }
        const order = d.rows.map((r) => r.dataset.key);
        order.splice(d.to, 0, ...order.splice(d.from, 1));
        for (const k of order) list.appendChild(d.rows.find((r) => r.dataset.key === k));
        drag = null;
        if (JSON.stringify(order) !== JSON.stringify(data.sections.order)) {
          data.sections = { ...data.sections, order };
          if (!demo) post({ type: 'sections', sections: data.sections });
        }
      }, 170);
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
      if (msg.version) $('appVer').textContent = msg.version;
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
