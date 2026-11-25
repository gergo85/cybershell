(() => {
  const { $, lang, T, root, demo, post, setTheme, THEMES, svg, esc, pad, store, fetchJson, DATE_FORMATS, fmtDate, dateFormat } = window.HUB;

  // ───────────── beállítások (a munkaterületen, animálva, fülekre bontva) ─────────────
  // Bal oldalt a fülek sávja (Programok, Panelek, Megjelenés, Funkciók, Rendszer), alatta a
  // gép terhelése és a készítő; jobbra az aktív fül kártyái. Minden változás azonnal életbe lép,
  // kivéve az ikonlistát, amit a Mentés gomb ment el. A Programok és a Rendszer fül külön fájlban van
  // (parts/settings-programs.js, parts/settings-system.js).
  function buildSettings() {
    root.className = 'settings';
    const TABS = [
      { id: 'programs', icon: 'app' },
      { id: 'panels', icon: 'panels' },
      { id: 'look', icon: 'design' },
      { id: 'features', icon: 'spark' },
      { id: 'system', icon: 'pc' },
    ];
    const tabKey = (id) => `tab${id[0].toUpperCase()}${id.slice(1)}`;
    const sw = (on) => ({ hu: ['KI', 'BE'], pl: ['WYŁ', 'WŁ'], de: ['AUS', 'EIN'], es: ['NO', 'SÍ'] }[lang] || ['OFF', 'ON'])[on ? 1 : 0];
    // egy be/ki kapcsoló: data-bool = az adat útvonala (és a programnak küldött kulcs)
    const toggle = (path, icon, label, hint, extra = '') =>
      `<button class="sbtn toggle" data-bool="${path}"${hint ? ` title="${esc(hint)}"` : ''}${extra}>${svg(icon)}<span>${label}</span><b class="switch">–</b></button>`;
    // csúszka: data-range = a programnak küldött kulcs
    const slider = (key, label, icon) =>
      `<label class="rng" data-range="${key}">${svg(icon)}<span class="rng-name">${label}</span>` +
      `<input type="range" step="1"><output>–</output></label>`;
    const WEB_SEARCH = {
      google: 'https://www.google.com/search?q={q}',
      bing: 'https://www.bing.com/search?q={q}',
      ddg: 'https://duckduckgo.com/?q={q}',
    };
    const COLOR_KEYS = ['main', 'alt', 'hot', 'bg', 'text'];
    // A külön fájlban lévő fülek (parts/settings-*.js: Programok, Rendszer) a
    // window.HUB.settingsPanes listába jegyzik be magukat: { id, html(h), setup(ctx) → horgok }.
    // A HTML-jük a fülek sorrendjében a helyére kerül, a horgokat a lenti paneHooks gyűjti.
    const PANES = window.HUB.settingsPanes;
    const paneHtml = (id) => {
      const p = PANES.find((x) => x.id === id);
      return p ? p.html({ toggle }) : '';
    };

    root.innerHTML = `
      <div class="set-wrap">
        <header class="set-head">
          <div class="set-title">CYBER<span>SHELL</span><em>${T('settings')}</em></div>
          <div class="toast set-toast" id="toast"></div>
          <button class="set-close" id="setClose">${svg('close')}<span id="setCloseText">${T('close')}</span></button>
        </header>
        <div class="set-body">
          <nav class="set-rail">
            <div class="rail-tabs" id="railTabs" role="tablist" aria-orientation="vertical">
              <i class="rail-glow" id="railGlow"></i>
              ${TABS.map((t, i) => `
                <button class="rail-tab" role="tab" data-tab="${t.id}" id="tab-${t.id}" title="Ctrl+${i + 1}">
                  ${svg(t.icon)}
                  <span class="rt-text"><b>${T(tabKey(t.id))}</b><small>${T(`${tabKey(t.id)}Sub`)}</small></span>
                  <em class="rt-badge" id="badge-${t.id}"></em>
                </button>`).join('')}
            </div>
            <div class="rail-keys">${T('tabKeys')}</div>
            <div class="rail-code" id="railCode" aria-hidden="true"></div>
            <div class="rail-load" id="railLoad" title="${T('systemLoad').replace(/^\/\/\s*/, '')}">
              <div class="rl" id="rlCpu"><span>${T('cpuShort')}</span><i><b></b></i><em>--%</em></div>
              <div class="rl mem" id="rlMem"><span>${T('memShort')}</span><i><b></b></i><em>--%</em></div>
            </div>
            <section class="rail-credit">
              <div class="credit-by"><span>${T('madeBy')}</span><b>${T('author')}</b></div>
              <div class="credit-by credit-ver"><span>${T('version')}</span><b id="appVer">v2.3</b></div>
              <div class="credit-btns">
                <button class="sbtn mini" data-cmd="store" title="${T('storeHint')}">${svg('store')}<span>Microsoft Store</span></button>
                <button class="sbtn mini" data-cmd="github">${svg('github')}<span>GitHub</span></button>
              </div>
            </section>
          </nav>

          <div class="set-stage" id="stage">
            <!-- 01 PROGRAMOK (parts/settings-programs.js) -->
            ${paneHtml('programs')}

            <!-- 02 PANELEK -->
            <section class="set-pane pane-panels" data-pane="panels" role="tabpanel">
              <div class="pane-col">
              <section class="set-card c-layout">
                <h2>${T('layoutCard')}</h2>
                <div class="lay-split">
                  <div class="set-grid">
                    <div class="lang-row pos-row">${svg('panels')}<span>${T('iconsPlace')}</span><div class="seg" id="posIcons"></div></div>
                    <div class="lang-row pos-row">${svg('swap')}<span>${T('infoPlace')}</span><div class="seg" id="posInfo"></div></div>
                    <div class="lang-row" title="${T('iconLabelsHint')}">${svg('text')}<span>${T('iconLabels')}</span>
                      <div class="seg" id="labelSeg"><button data-labels="show">${T('lblShow')}</button><button data-labels="hide">${T('lblHide')}</button></div>
                    </div>
                    ${toggle('allMonitors', 'pc', T('allMonitors'), T('allMonitorsHint'))}
                    ${toggle('hideTaskbar', 'bar', T('hideTaskbar'), T('hideTaskbarHint'))}
                  </div>
                  <div class="lay-pv" id="layPv" aria-hidden="true"></div>
                </div>
              </section>
              <section class="set-card c-sizes">
                <h2>${T('sizesCard')}<button class="sbtn mini card-reset" id="sizesReset" title="${T('sizesResetHint')}">${svg('reload')}<span>${T('wallReset')}</span></button></h2>
                <div class="rng-grid">
                  ${slider('iconsWidth', T('iconsWidth'), 'panels')}
                  ${slider('infoWidth', T('infoWidth'), 'swap')}
                  ${slider('frame.top', T('frameTop'), 'up')}
                  ${slider('frame.bottom', T('frameBottom'), 'down')}
                  ${slider('frame.left', T('frameLeft'), 'left')}
                  ${slider('frame.right', T('frameRight'), 'right')}
                </div>
              </section>
              <section class="set-card c-order">
                <h2>${T('orderCard')}<button class="sbtn mini card-reset" id="orderReset" title="${T('orderResetHint')}">${svg('reload')}<span>${T('wallReset')}</span></button></h2>
                <div class="ord-pair">
                  <div><h3 class="ord-title">${svg('calc')}<span>${T('sTools')}</span></h3><div class="set-list ord-list" id="ordTools"></div></div>
                  <div><h3 class="ord-title">${svg('power')}<span>${T('sPower')}</span></h3><div class="set-list ord-list" id="ordPower"></div></div>
                  <div class="ord-laptop" id="ordLaptopCol"><h3 class="ord-title">${svg('battery')}<span>${T('sLaptop')}</span></h3><div class="set-list ord-list" id="ordLaptop"></div></div>
                </div>
              </section>
              <section class="set-card c-view">
                <h2>${T('viewCard')}</h2>
                <div class="view-grid" id="viewList"></div>
              </section>
              </div>
              <div class="pane-col">
              <section class="set-card c-sections">
                <h2>${T('visibleSections')}</h2>
                <div class="set-list" id="secList"></div>
              </section>
              </div>
            </section>

            <!-- 03 MEGJELENÉS -->
            <section class="set-pane pane-look" data-pane="look" role="tabpanel">
              <section class="set-card c-theme">
                <h2>${T('themeCard')}</h2>
                <div class="lang-row theme-row" title="${T('apHint')}">${svg('moon')}<span>${T('appearance')}</span>
                  <div class="seg" id="themeSeg">
                    <button data-appearance="dark">${T('apDark')}</button><button data-appearance="light">${T('apLight')}</button><button data-appearance="auto" title="${T('apAutoHint')}">${T('apAuto')}</button><button data-appearance="windows" title="${T('apWindowsHint')}">${T('apWindows')}</button>
                  </div>
                </div>
                <div class="pal-pair">
                  ${['dark', 'light'].map((sc) => `
                    <div class="pal" data-scheme="${sc}" id="pal-${sc}">
                      <div class="pal-head"><b>${T(sc === 'dark' ? 'schemeDark' : 'schemeLight')}</b><em class="pal-on">${T('inUse')}</em>
                        <button class="sbtn mini pal-reset" data-reset="${sc}" title="${T('themeResetHint')}">${svg('reload')}<span>${T('themeResetBtn')}</span></button></div>
                      <div class="pal-demo"><i class="pd-bar"></i><span class="pd-t">CYBER<b>SHELL</b></span><span class="pd-l"></span><span class="pd-l short"></span><em class="pd-hot">!</em></div>
                      <div class="pal-sw">
                        ${COLOR_KEYS.map((k) => `<label class="sw" data-color="${k}"><input type="color"><i></i><span>${T(`c_${k}`)}</span><code>#------</code></label>`).join('')}
                      </div>
                    </div>`).join('')}
                </div>
              </section>
              <section class="set-card c-win">
                <h2>${T('winCard')}</h2>
                <div class="set-grid">
                  <div class="lang-row nb-row" title="${T('neonBorderHint')}">${svg('maximize')}<span>${T('neonBorder')}</span>
                    <div class="seg" id="nbSeg"><button data-nb="off">${T('nbOff')}</button><button data-nb="active">${T('nbActive')}</button><button data-nb="all">${T('nbAll')}</button></div>
                  </div>
                  <div class="lang-row acc-row" title="${T('syncAccentHint')}">${svg('picker')}<span>${T('syncAccent')}</span>
                    <div class="seg" id="accSeg"><button data-acc="off">${T('nbOff')}</button><button data-acc="main">${T('accMain')}</button><button data-acc="alt">${T('accAlt')}</button><button data-acc="hot">${T('accHot')}</button></div>
                  </div>
                  ${toggle('windowsSync.darkMode', 'moon', T('syncMode'), T('syncModeHint'))}
                  ${toggle('windowsSync.terminal', 'terminal', T('syncTerminal'), T('syncTerminalHint'))}
                  <button class="sbtn toggle" data-toggle="desktopMenu" title="${T('deskMenuHint')}">${svg('menu')}<span>${T('deskMenu')}</span><b class="switch">–</b></button>
                  <button class="sbtn toggle" data-toggle="desktopIconMenu" title="${T('deskIconMenuHint')}">${svg('file')}<span>${T('deskIconMenu')}</span><b class="switch">–</b></button>
                </div>
              </section>
              <section class="set-card wall">
                <h2>${T('wallpaper')}
                  <span class="wall-tools">
                    <button class="sbtn mini" id="wallAdd">${svg('plus')}<span>${T('addImage')}</span></button>
                    <button class="sbtn mini" id="wallFolder">${svg('folder')}<span>${T('openFolder')}</span></button>
                    <button class="sbtn mini" id="wallReset" title="${T('wallResetHint')}">${svg('reload')}<span>${T('wallReset')}</span></button>
                  </span>
                </h2>
                <div class="wall-list" id="wallList"></div>
              </section>
            </section>

            <!-- 04 FUNKCIÓK -->
            <!-- a kártyák a panel moduljainak sorrendjében (data-mod: a modul azonosítója; renderFeatures) -->
            <section class="set-pane pane-features" data-pane="features" role="tabpanel">
              <div class="feat-cols" id="featCols">
              <section class="set-card wset" id="wsCard" data-mod="weather">
                <h2>${T('wsTitle')}</h2>
                <div class="ws-row ws-grid">
                  <label class="ws-city" title="${T('wsCityHint')}">${svg('place')}<span class="ws-field"><input id="wsCity" type="text" maxlength="80" spellcheck="false" autocomplete="off" placeholder="${T('wsCityPh')}"><button class="ws-go" id="wsGo" title="${T('wsGo')}">${svg('enter')}</button></span></label>
                  <div class="ws-found" id="wsFound"></div>
                  <button class="sbtn toggle ws-wind" id="wsWind" title="${T('wsWindHint')}">${svg('wind')}<span>${T('wsWind')}</span><b class="switch">–</b></button>
                  <div class="sbtn ws-unit" title="${T('wsUnitHint')}">${svg('thermo')}<span>${T('wsUnit')}</span><div class="seg" id="wsUnit"><button data-unit="c">°C</button><button data-unit="f">°F</button></div></div>
                </div>
              </section>
              <section class="set-card c-start" data-mod="start">
                <h2>${T('startCard')}</h2>
                <div class="set-grid">
                  <button class="sbtn toggle" data-toggle="winKey" title="${T('winKeyHint')}">${svg('start')}<span>${T('winKey')}</span><b class="switch">–</b></button>
                  <button class="sbtn toggle" data-toggle="clipboard" title="${T('clipHistHint')}">${svg('clipboard')}<span>${T('clipHist')}</span><b class="switch">–</b></button>
                  <div class="lang-row ws-engine">${svg('search')}<span>${T('webSearch')}</span>
                    <div class="seg" id="engSeg"><button data-eng="google">Google</button><button data-eng="bing">Bing</button><button data-eng="ddg">DuckDuckGo</button><button data-eng="custom">${T('wsCustom')}</button></div>
                  </div>
                  <label class="ws-city url-row" id="engCustom">${svg('globe')}<span class="ws-field"><input id="engUrl" type="text" spellcheck="false" autocomplete="off" placeholder="https://…?q={q}"><button class="ws-go" id="engGo" title="${T('save')}">${svg('enter')}</button></span></label>
                </div>
              </section>
              <section class="set-card c-dt" data-mod="clock date">
                <h2>${T('dtCard')}</h2>
                <div class="set-grid">
                  <div class="lang-row dt-row" title="${T('dtDateHint')}">${svg('calendar')}<span>${T('dtDate')}</span><div class="seg" id="dfSeg"></div></div>
                  <div class="lang-row" title="${T('dtClockHint')}">${svg('clock')}<span>${T('dtClock')}</span>
                    <div class="seg" id="h12Seg"><button data-h12="0">${T('h24')}</button><button data-h12="1">${T('h12')}</button></div>
                  </div>
                </div>
              </section>
              <!-- a panel moduljainak saját kártyái (a modul leírásának settings része) -->
              ${window.HUB.modules.filter((m) => m.settings).map((m) => `
                <section class="set-card mod-card" id="mc-${m.id}" data-mod="${m.id}">
                  <h2>${T(m.settings.title)}</h2>
                  ${m.settings.html()}
                </section>`).join('')}
              <section class="set-card health" id="hlCard" data-mod="health">
                <h2>${T('health')}</h2>
                <div class="hl-row">
                  <button class="sbtn toggle" id="hlToggle">${svg('heart')}<span>${T('hBreak')}</span><b class="switch">–</b></button>
                  <label class="hl-min"><span>${T('hEvery')}</span><input id="hlMin" type="number" min="5" max="480" step="5" inputmode="numeric"><span>${T('hMinutes')}</span></label>
                  <div class="hl-count"><span>${T('hNext')}</span><b id="hlCount">—</b></div>
                  <button class="sbtn mini icon" id="hlReset" title="${T('hRestartHint')}">${svg('reload')}</button>
                </div>
              </section>
              <section class="set-card c-act" data-mod="activity">
                <h2>${T('activityCard')}</h2>
                <div class="set-grid two">
                  ${toggle('activity.autoStart', 'play', T('actAuto'), T('actAutoHint'))}
                  ${toggle('activity.sound', 'volume', T('actSound'), T('actSoundHint'))}
                </div>
              </section>
              <section class="set-card c-cal" data-mod="events">
                <h2>${T('calCard')}</h2>
                <div class="cal-list" id="calList"></div>
                <div class="cal-add">
                  <label class="ws-city">${svg('calendar')}<span class="ws-field"><input id="calUrl" type="text" spellcheck="false" autocomplete="off" placeholder="${T('calAddPh')}"><button class="ws-go" id="calGo" title="${T('calConnect')}">${svg('enter')}</button></span></label>
                  <button class="sbtn mini" id="calHelp" title="${T('calOpenGoogle')}">${svg('globe')}<span>${T('calHelp')}</span></button>
                </div>
              </section>
              </div>
            </section>

            <!-- 05 RENDSZER (parts/settings-system.js) -->
            ${paneHtml('system')}
          </div>
        </div>
      </div>
      `;

    let data = null;      // a programtól kapott legutóbbi állapot
    const sortableLater = []; // a húzható listák (a sortable a lenti definíció után kapcsolódik be)
    // a panel moduljai (modules/*.js): a ki- és bekapcsolható, rendezhető modulok
    const MODULES = window.HUB.modules.filter((m) => m.listed !== false);
    const moduleOf = (id) => MODULES.find((m) => m.id === id);
    // a modulok saját beállításkártyái: a kártya a frissített adatot a render(data) horgában kapja
    const MODULE_CARDS = window.HUB.modules.filter((m) => m.settings).map((m) => {
      const card = $(`mc-${m.id}`);
      const hooks = m.settings.setup ? m.settings.setup({
        el: card, demo, post,
        $: (id) => card.querySelector(`#${id}`),
        // a helyi adat frissítése (a program a mentés után úgyis újraküldi)
        update: (patch) => { if (data) Object.assign(data, patch); },
      }) || {} : {};
      return { m, hooks };
    });
    const get = (path) => path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), data);
    const setLocal = (path, v) => {
      const keys = path.split('.');
      let o = data;
      for (const k of keys.slice(0, -1)) o = o[k] = o[k] || {};
      o[keys[keys.length - 1]] = v;
    };
    const setValue = (key, value) => {
      if (demo) return;
      post({ type: 'set', key, value });
    };

    const toast = (text) => {
      const t = $('toast');
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toast.timer);
      toast.timer = setTimeout(() => t.classList.remove('show'), 3500);
    };

    // ── fülek ──
    const TAB_STORE = 'cybershell.settings.tab';
    let tab = '';
    const placeGlow = () => {
      const b = $(`tab-${tab}`);
      if (!b) return;
      $('railGlow').style.transform = `translateY(${b.offsetTop}px)`;
      $('railGlow').style.height = `${b.offsetHeight}px`;
    };
    const selectTab = (id, opts = {}) => {
      if (!TABS.some((t) => t.id === id)) id = TABS[0].id;
      const changed = id !== tab;
      tab = id;
      store.set(TAB_STORE, id);
      for (const b of root.querySelectorAll('.rail-tab')) {
        const on = b.dataset.tab === id;
        b.classList.toggle('on', on);
        b.setAttribute('aria-selected', on);
        b.tabIndex = on ? 0 : -1;
      }
      for (const p of root.querySelectorAll('.set-pane')) {
        const on = p.dataset.pane === id;
        p.classList.toggle('on', on);
        if (on && changed && !opts.instant) {
          p.classList.remove('enter');
          void p.offsetWidth;
          p.classList.add('enter');
        }
      }
      $('stage').scrollTop = 0;
      placeGlow();
      if (id === 'panels') renderPreview();
      if (opts.focus) $(`tab-${id}`).focus();
    };
    $('railTabs').addEventListener('click', (e) => {
      const b = e.target.closest('.rail-tab');
      if (b) selectTab(b.dataset.tab);
    });
    // nyilakkal a fülek között (ha a fülsáv a fókuszban van)
    $('railTabs').addEventListener('keydown', (e) => {
      const i = TABS.findIndex((t) => t.id === tab);
      const next = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: TABS.length - 1 }[e.key];
      if (next == null) return;
      e.preventDefault();
      selectTab(TABS[(next + TABS.length) % TABS.length].id, { focus: true });
    });
    window.addEventListener('resize', placeGlow);

    // ── díszítés: halványan „futó programkód” a fülek és a terhelésjelző között ──
    // Soronként begépelődik, a régebbi sorok felfelé kifutnak. Csak amíg a beállítások
    // látszanak; csökkentett mozgásnál álló sorok.
    {
      const CODE = [
        'fn poll(&mut self) {', '  let fg = wintrack::foreground();', '  if fg != self.last { self.redraw(); }', '}',
        'for w in tracker.snapshot(&own) {', '  bar.mark(w.hwnd, w.active);', 'appbar::register(edge, rect)',
        'dwm.set_border(hwnd, accent)', 'ipc.recv() -> {"type":"state"}', 'mov rax, qword ptr [rbp-0x18]',
        '0x7ffd3a2c  48 8b 05 f1 2c 00', 'cpu.sample() -> 12.4%', 'mem.commit = 0x3f2a000', 'spawn(guard, pid)',
        'ping 1.1.1.1 · 14 ms', 'hash = fnv1a(path) & 0xffff', "emit('state', running)", 'ex &= !WS_EX_TOOLWINDOW',
        'queue.push(job); ok', 'let tray = tray::list();', 'webview.eval(hub.receive(msg))', 'sync wallpaper · done',
        'if !cfg.weather.enabled { return }', 'await fetch(/__icon/…)', 'sleep(150ms); retry', 'jmp short 0x4012a0',
        'xor eax, eax', 'ret', 'match msg["type"] {', '  "cmd" => self.run(cmd),', '  _ => {}', '}',
        'neon.border(hwnd, 0x2a6dff)', 'peek.thumb(src, rect)', 'heartbeat · 3 s', 'watch(notify_icons)',
      ];
      const box = $('railCode');
      const still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      const MAX_LINES = 26;
      let pick = Math.floor(Math.random() * CODE.length);
      const nextText = () => {
        // általában a következő sor, néha ugrás egy véletlen helyre
        pick = Math.random() < 0.7 ? (pick + 1) % CODE.length : Math.floor(Math.random() * CODE.length);
        return CODE[pick];
      };
      const addLine = (text) => {
        const el = document.createElement('div');
        el.textContent = text;
        box.append(el);
        while (box.childElementCount > MAX_LINES) box.firstElementChild.remove();
        return el;
      };
      if (still) {
        for (let i = 0; i < 22; i++) addLine(nextText());
      } else {
        for (let i = 0; i < 20; i++) addLine(nextText());
        let line = null;
        let text = '';
        let at = 0;
        const step = () => {
          let wait = 34;
          if (root.classList.contains('shown') || demo) {
            if (!line) {
              text = nextText();
              at = 0;
              line = addLine('');
              line.className = 'typing';
            }
            at += 1 + Math.floor(Math.random() * 2);
            line.textContent = text.slice(0, at);
            if (at >= text.length) {
              line.className = '';
              line = null;
              wait = 250 + Math.random() * 900;
            }
          } else {
            wait = 500;
          }
          setTimeout(step, wait);
        };
        setTimeout(step, 600);
      }
    }

    // ── megjelenés és eltűnés animációval ──
    // A beúszó animáció közben a tartalom egy pillanatra lejjebb csúszik: ilyenkor a görgetősáv
    // ne villanjon fel a jobb szélen.
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
      requestAnimationFrame(placeGlow);
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

    const setSwitch = (b, on) => {
      b.classList.toggle('on', on);
      b.querySelector('.switch').textContent = sw(on);
    };

    // ── a panelek elrendezésének élő előnézete (a képernyő kicsiben) ──
    let pvLive = null; // húzás közbeni értékek (még nincs elküldve)
    const renderPreview = () => {
      if (!data) return;
      const lay = data.layout || { icons: 'left', info: 'right' };
      const bars = { ...(data.bars || {}), frame: { ...((data.bars || {}).frame || {}) } };
      if (pvLive) {
        if (pvLive.key === 'iconsWidth') bars.icons = pvLive.v;
        else if (pvLive.key === 'infoWidth') bars.info = pvLive.v;
        else bars.frame[pvLive.key.split('.')[1]] = pvLive.v;
      }
      const scr = data.screen || { w: 1920, h: 1080 };
      const thick = (e) => (e === lay.icons ? bars.icons || 86 : e === lay.info ? bars.info || 86 : bars.frame[e] || 0);
      const el = $('layPv');
      const W = el.clientWidth || 300;
      const k = W / scr.w;
      el.style.height = `${Math.round(scr.h * k)}px`;
      // a két panel teljes hosszban, a keretek közöttük
      const order = [lay.icons, lay.info, ...['left', 'right', 'top', 'bottom'].filter((e) => e !== lay.icons && e !== lay.info)];
      const inset = { left: 0, right: 0, top: 0, bottom: 0 };
      const parts = order.map((e) => {
        const t = thick(e);
        const px = t > 0 ? Math.max(1, Math.round(t * k)) : 0;
        const vert = e === 'left' || e === 'right';
        const st = vert
          ? `${e}:${inset[e]}px;top:${inset.top}px;bottom:${inset.bottom}px;width:${px}px`
          : `${e}:${inset[e]}px;left:${inset.left}px;right:${inset.right}px;height:${px}px`;
        const kind = e === lay.icons ? 'icons' : e === lay.info ? 'info' : 'frame';
        const label = kind === 'frame' ? '' : `<span>${T(kind === 'icons' ? 'pvIcons' : 'pvInfo')}</span>`;
        const html = `<i class="pv-bar ${kind}${vert ? ' v' : ''}" style="${st}">${label}</i>`;
        inset[e] += px;
        return { e, t, html };
      });
      const sum = (a, b) => thick(a) + thick(b);
      el.innerHTML = parts.map((p) => p.html).join('') +
        `<div class="pv-work" style="left:${inset.left}px;right:${inset.right}px;top:${inset.top}px;bottom:${inset.bottom}px">` +
        `<b>${T('pvWork')}</b><span>${Math.max(0, scr.w - sum('left', 'right'))} × ${Math.max(0, scr.h - sum('top', 'bottom'))}</span></div>`;
      // a csúszkák: ahol panel van, ott a keret nem látszik
      for (const r of root.querySelectorAll('[data-range^="frame."]')) {
        const e = r.dataset.range.split('.')[1];
        const taken = e === lay.icons || e === lay.info;
        r.classList.toggle('off', taken);
        r.querySelector('input').disabled = taken;
        r.title = taken ? T('frameTaken') : '';
      }
    };
    const renderRanges = () => {
      const bars = data.bars || {};
      const RANGES = { iconsWidth: [64, 96], infoWidth: [80, 128], 'frame.top': [0, 32], 'frame.bottom': [0, 32], 'frame.left': [0, 40], 'frame.right': [0, 40] };
      const rangeOf = (k) => (bars.ranges || {})[k] || RANGES[k];
      for (const r of root.querySelectorAll('[data-range]')) {
        const key = r.dataset.range;
        const input = r.querySelector('input');
        if (document.activeElement === input && pvLive) continue;
        const isFrame = key.startsWith('frame.');
        [input.min, input.max] = rangeOf(key);
        const v = isFrame ? (bars.frame || {})[key.split('.')[1]] : bars[key === 'iconsWidth' ? 'icons' : 'info'];
        input.value = v ?? 0;
        r.querySelector('output').textContent = `${v ?? 0} px`;
        r.style.setProperty('--p', `${((input.value - input.min) / (input.max - input.min || 1)) * 100}%`);
      }
    };
    for (const r of root.querySelectorAll('[data-range]')) {
      const input = r.querySelector('input');
      input.addEventListener('input', () => {
        pvLive = { key: r.dataset.range, v: Number(input.value) };
        r.querySelector('output').textContent = `${input.value} px`;
        r.style.setProperty('--p', `${((input.value - input.min) / (input.max - input.min || 1)) * 100}%`);
        renderPreview();
      });
      input.addEventListener('change', () => {
        const key = r.dataset.range;
        const v = Number(input.value);
        pvLive = null;
        const bars = data.bars || (data.bars = {});
        if (key === 'iconsWidth') bars.icons = v;
        else if (key === 'infoWidth') bars.info = v;
        else (bars.frame = bars.frame || {})[key.split('.')[1]] = v;
        renderPreview();
        $('sizesReset').disabled = sizesDefault();
        if (demo) return toast(T('posDemo'));
        setValue(key, v);
      });
    }

    // ── téma: a két séma színei ──
    const renderPalettes = () => {
      const th = data.themes || { dark: THEMES.dark, light: THEMES.light };
      for (const sc of ['dark', 'light']) {
        const t = th[sc] || {};
        const pal = $(`pal-${sc}`);
        pal.classList.toggle('active', (data.scheme || 'dark') === sc);
        for (const k of COLOR_KEYS) pal.style.setProperty(`--p-${k}`, t[k]);
        for (const l of pal.querySelectorAll('.sw')) {
          const v = t[l.dataset.color] || '#000000';
          const input = l.querySelector('input');
          if (document.activeElement !== input) input.value = v;
          l.querySelector('i').style.background = v;
          l.querySelector('code').textContent = v.toLowerCase();
        }
        const def = th[sc === 'dark' ? 'defaultDark' : 'defaultLight'];
        const same = def && COLOR_KEYS.every((k) => String(def[k]).toLowerCase() === String(t[k]).toLowerCase());
        pal.querySelector('.pal-reset').disabled = !!same;
      }
    };
    for (const pal of root.querySelectorAll('.pal')) {
      const sc = pal.dataset.scheme;
      pal.addEventListener('input', (e) => {
        const l = e.target.closest('.sw');
        if (!l) return;
        const v = e.target.value;
        l.querySelector('i').style.background = v;
        l.querySelector('code').textContent = v;
        pal.style.setProperty(`--p-${l.dataset.color}`, v);
        // az épp használt séma színe a beállításokban azonnal látszik
        if ((data.scheme || 'dark') === sc) setTheme({ [l.dataset.color]: v });
      });
      pal.addEventListener('change', (e) => {
        const l = e.target.closest('.sw');
        if (!l || !data) return;
        const key = l.dataset.color;
        data.themes = data.themes || {};
        data.themes[sc] = { ...(data.themes[sc] || {}), [key]: e.target.value };
        renderPalettes();
        setValue(`${sc === 'dark' ? 'theme' : 'themeLight'}.${key}`, e.target.value);
      });
      pal.querySelector('.pal-reset').addEventListener('click', () => {
        if (!data) return;
        const def = (data.themes || {})[sc === 'dark' ? 'defaultDark' : 'defaultLight'] || THEMES[sc];
        data.themes[sc] = { ...def };
        renderPalettes();
        if ((data.scheme || 'dark') === sc) setTheme(def);
        setValue('themeReset', sc);
      });
    }

    // ── naptárak (a titkos címet a program nem küldi el, csak a kiszolgáló nevét) ──
    let calArm = -1;
    let calArmTimer = 0;
    const renderCalendars = () => {
      const list = data.calendars || [];
      $('calList').innerHTML = list.length
        ? list.map((c, i) => `
          <div class="cal-row" data-i="${i}">
            <input class="ed-color cal-color" type="color" value="${/^#[0-9a-f]{6}$/i.test(c.color) ? c.color : '#00e1ff'}">
            <input class="ed-name cal-name" value="${esc(c.name)}" maxlength="40" spellcheck="false">
            <span class="cal-host">${esc(c.host || '')}</span>
            <button class="ib danger cal-del${calArm === i ? ' armed' : ''}" title="${T('calRemove')}">${calArm === i ? `<span>${T('calRemoveSure')}</span>` : svg('trash')}</button>
          </div>`).join('')
        : `<div class="ed-none">${T('calNone')}</div>`;
    };
    $('calList').addEventListener('change', (e) => {
      const row = e.target.closest('.cal-row');
      if (!row || demo) return;
      const i = Number(row.dataset.i);
      if (e.target.classList.contains('cal-name')) post({ type: 'cal-edit', i, name: e.target.value });
      if (e.target.classList.contains('cal-color')) post({ type: 'cal-edit', i, color: e.target.value });
    });
    $('calList').addEventListener('keydown', (e) => { if (e.key === 'Enter' && e.target.matches('input')) e.target.blur(); });
    $('calList').addEventListener('click', (e) => {
      const b = e.target.closest('.cal-del');
      if (!b) return;
      const i = Number(b.closest('.cal-row').dataset.i);
      clearTimeout(calArmTimer);
      if (calArm !== i) {
        calArm = i;
        calArmTimer = setTimeout(() => { calArm = -1; renderCalendars(); }, 3500);
        return renderCalendars();
      }
      calArm = -1;
      if (demo) { data.calendars.splice(i, 1); return renderCalendars(); }
      post({ type: 'cal-remove', i });
    });
    const calAdd = () => {
      const url = $('calUrl').value.trim();
      if (!url) return;
      $('calUrl').value = '';
      if (demo) return toast('Demo: calendar');
      post({ type: 'cal-add', url });
    };
    $('calGo').addEventListener('mousedown', (e) => e.preventDefault());
    $('calGo').addEventListener('click', calAdd);
    $('calUrl').addEventListener('keydown', (e) => { if (e.key === 'Enter') calAdd(); });
    $('calHelp').addEventListener('click', () => (demo ? toast('Demo: Google') : post({ type: 'cal-help' })));

    // ── Funkciók fül: a kártyák a panel moduljainak sorrendjében (ahogy a Panelek fülön állnak) ──
    // A Start menü kártyája elöl (a panel tetején a Start gomb), az Egészség a végén; a Dátum és
    // idő ott, ahol az Idő vagy a Dátum modul közül az előbbi.
    const renderFeatures = () => {
      const ids = window.HUB.modules.map((m) => m.id);
      const order = [...((data && data.sections && data.sections.order) || []), ...ids];
      const rank = (card) => {
        const mods = card.dataset.mod.split(' ');
        if (mods[0] === 'start') return -1;
        if (mods[0] === 'health') return 1e6;
        const at = mods.map((m) => order.indexOf(m)).filter((i) => i >= 0);
        return at.length ? Math.min(...at) : 1e5;
      };
      const cards = [...$('featCols').children].map((c, i) => ({ c, r: rank(c), i }));
      cards.sort((a, b) => a.r - b.r || a.i - b.i);
      for (const { c } of cards) $('featCols').appendChild(c);
    };

    // ── Dátum és idő: a dátum formátuma (a mai dátum mintaként) és a 12 vagy 24 órás óra ──
    const renderDateTime = () => {
      const dtc = (data && data.dateTime) || { dateFormat: 'auto', hour12: false };
      const today = new Date();
      $('dfSeg').innerHTML = DATE_FORMATS.map((f) => {
        const label = f === 'auto' ? T('dtAuto') : fmtDate(today, { f });
        const tip = f === 'auto' ? ` title="${esc(T('dtAutoHint', { s: fmtDate(today, { f: dateFormat('auto') }) }))}"` : '';
        return `<button data-df="${f}"${tip} class="${dtc.dateFormat === f ? 'on' : ''}">${esc(label)}</button>`;
      }).join('');
      for (const b of $('h12Seg').children) b.classList.toggle('on', (b.dataset.h12 === '1') === !!dtc.hour12);
    };
    const setDateTime = (patch) => {
      data.dateTime = { dateFormat: 'auto', hour12: false, ...data.dateTime, ...patch };
      renderDateTime();
      // a beállítások saját felülete is azonnal átvált (a többit a program értesíti)
      setTheme({ dateFormat: data.dateTime.dateFormat, hour12: data.dateTime.hour12 });
      const [key, value] = Object.entries(patch)[0];
      setValue(`dateTime.${key}`, value);
    };
    $('dfSeg').addEventListener('click', (e) => {
      const b = e.target.closest('[data-df]');
      if (!b || !data || b.classList.contains('on')) return;
      setDateTime({ dateFormat: b.dataset.df });
    });
    $('h12Seg').addEventListener('click', (e) => {
      const b = e.target.closest('[data-h12]');
      if (!b || !data || b.classList.contains('on')) return;
      setDateTime({ hour12: b.dataset.h12 === '1' });
    });

    // ── webes kereső ──
    const engineOf = (url) => Object.keys(WEB_SEARCH).find((k) => WEB_SEARCH[k] === url) || 'custom';
    let engCustomOpen = false;
    const renderEngine = () => {
      const url = data.webSearch || WEB_SEARCH.google;
      const eng = engCustomOpen ? 'custom' : engineOf(url);
      for (const b of $('engSeg').children) b.classList.toggle('on', b.dataset.eng === eng);
      $('engCustom').classList.toggle('shown', eng === 'custom');
      if (document.activeElement !== $('engUrl')) $('engUrl').value = url;
    };
    $('engSeg').addEventListener('click', (e) => {
      const b = e.target.closest('[data-eng]');
      if (!b || !data) return;
      if (b.dataset.eng === 'custom') {
        engCustomOpen = true;
        renderEngine();
        $('engUrl').focus();
        return;
      }
      engCustomOpen = false;
      data.webSearch = WEB_SEARCH[b.dataset.eng];
      renderEngine();
      setValue('webSearch', data.webSearch);
    });
    const engCommit = () => {
      const v = $('engUrl').value.trim();
      if (!v || v === data.webSearch) return;
      if (!/^https?:\/\//i.test(v) || !v.includes('{q}')) return toast(T('webSearchBad'));
      data.webSearch = v;
      engCustomOpen = engineOf(v) === 'custom';
      renderEngine();
      setValue('webSearch', v);
    };
    $('engGo').addEventListener('mousedown', (e) => e.preventDefault());
    $('engGo').addEventListener('click', engCommit);
    $('engUrl').addEventListener('keydown', (e) => { if (e.key === 'Enter') { engCommit(); $('engUrl').blur(); } });

    // ── az Eszköztár, a Főkapcsoló és a Laptop gombjainak sorrendje (húzással, azonnal életbe lép) ──
    // a gombokat a modul adja (buttons, orderKey): { toolsOrder: { calc: [ikon, név], … }, … }
    const ORDER_ITEMS = Object.fromEntries(MODULES.filter((m) => m.orderKey).map((m) =>
      [m.orderKey, Object.fromEntries(m.buttons.map((b) => [b.key, [b.icon, T(b.name || b.title)]]))]));
    const orderOf = (key) => {
      const all = Object.keys(ORDER_ITEMS[key]);
      const cur = ((data.sections || {})[key] || []).filter((k) => all.includes(k));
      return [...cur, ...all.filter((k) => !cur.includes(k))];
    };
    // ── Megjelenés: kompakt vagy normál nézet a kompakt nézettel rendelkező moduloknál ──
    // csak a bekapcsolt modulok sorai látszanak (a kikapcsolt modul beállítása megmarad, és
    // a bekapcsolásakor rögtön megjelenik a sora)
    const renderView = () => {
      const sec = data.sections || {};
      const compact = sec.compact || [];
      // a sorrend ugyanaz, mint a Modulok listájában (a beállított sorrend, ami hiányzik, a végére)
      const rank = (id) => { const i = (sec.order || []).indexOf(id); return i < 0 ? Infinity : i; };
      const shown = MODULES.filter((m) => m.compact && (!m.laptopOnly || data.laptop) && (sec[m.id] ?? m.defaultOn ?? true) !== false)
        .sort((a, b) => rank(a.id) - rank(b.id));
      $('viewList').innerHTML = shown.length ? shown.map(({ id: k, compact: icon, name }) => `
        <div class="lang-row">${svg(icon)}<span>${T(name)}</span>
          <div class="seg"><button data-view="${k}" data-mode="compact" class="${compact.includes(k) ? 'on' : ''}">${T('viewCompact')}</button><button data-view="${k}" data-mode="normal" class="${compact.includes(k) ? '' : 'on'}">${T('viewNormal')}</button></div>
        </div>`).join('') : `<div class="view-empty">${T('viewEmpty')}</div>`;
    };
    $('viewList').addEventListener('click', (e) => {
      const b = e.target.closest('[data-view]');
      if (!b || !data || b.classList.contains('on')) return;
      const k = b.dataset.view;
      const compact = ((data.sections || {}).compact || []).filter((x) => x !== k);
      if (b.dataset.mode === 'compact') compact.push(k);
      data.sections = { ...data.sections, compact };
      renderView();
      if (!demo) post({ type: 'sections', sections: data.sections });
    });
    // Méretek és Sorrend: vissza az alapállapotba
    const sizesDefault = () => {
      const b = data.bars || {};
      const d = b.defaults || { icons: 86, info: 86, frame: { top: 10, bottom: 10, left: 10, right: 10 } };
      return b.icons === d.icons && b.info === d.info && ['top', 'bottom', 'left', 'right'].every((k) => (b.frame || {})[k] === d.frame[k]);
    };
    $('sizesReset').addEventListener('click', () => {
      if (!data || sizesDefault()) return;
      const d = (data.bars || {}).defaults || { icons: 86, info: 86, frame: { top: 10, bottom: 10, left: 10, right: 10 } };
      data.bars = { ...data.bars, icons: d.icons, info: d.info, frame: { ...d.frame } };
      renderRanges();
      renderPreview();
      $('sizesReset').disabled = true;
      setValue('sizesReset', true);
    });
    // a sorrendezhető gombsorok: [lista, kulcs]; a Laptop csak akkumulátoros gépen
    const ORDERS = [['ordTools', 'toolsOrder'], ['ordPower', 'powerOrder'], ['ordLaptop', 'laptopOrder']];
    const orderKeys = () => ORDERS.map(([, k]) => k).filter((k) => k !== 'laptopOrder' || data.laptop);
    const orderIsDefault = () => orderKeys().every((k) => orderOf(k).join() === Object.keys(ORDER_ITEMS[k]).join());
    $('orderReset').addEventListener('click', () => {
      if (!data) return;
      data.sections = { ...data.sections, ...Object.fromEntries(ORDERS.map(([, k]) => [k, Object.keys(ORDER_ITEMS[k])])) };
      renderOrder();
      if (!demo) post({ type: 'sections', sections: data.sections });
    });

    const renderOrder = () => {
      $('orderReset').disabled = orderIsDefault();
      $('ordLaptopCol').hidden = !data.laptop;
      root.querySelector('.ord-pair').classList.toggle('three', !!data.laptop);
      for (const [id, key] of ORDERS) {
        $(id).innerHTML = orderOf(key).map((k) => {
          const [icon, name] = ORDER_ITEMS[key][k];
          return `<div class="sec-row" data-key="${k}"><span class="grip" title="${T('dragHint')}">${svg('grip')}</span>` +
            `<div class="sbtn ord-item">${svg(icon)}<span>${name}</span></div></div>`;
        }).join('');
      }
    };
    for (const [id, key] of ORDERS) {
      sortableLater.push(() => sortable($(id), '.sec-row', (from, to, rows) => {
        const order = rows.map((r) => r.dataset.key);
        order.splice(to, 0, ...order.splice(from, 1));
        for (const k of order) $(id).appendChild(rows.find((r) => r.dataset.key === k));
        data.sections = { ...data.sections, [key]: order };
        $('orderReset').disabled = orderIsDefault();
        if (!demo) post({ type: 'sections', sections: data.sections });
      }));
    }

    const renderToggles = () => {
      for (const b of root.querySelectorAll('[data-toggle]')) setSwitch(b, !!data[b.dataset.toggle]);
      for (const b of root.querySelectorAll('[data-bool]')) setSwitch(b, !!get(b.dataset.bool));
      for (const b of $('labelSeg').children) b.classList.toggle('on', (b.dataset.labels === 'show') === (data.iconLabels !== false));
      for (const b of $('themeSeg').children) b.classList.toggle('on', b.dataset.appearance === (data.appearance || 'dark'));
      for (const b of $('nbSeg').children) b.classList.toggle('on', b.dataset.nb === (data.neonBorder || 'off'));
      const ws = data.windowsSync || {};
      const acc = ws.accent ? (ws.accentColor || 'hot') : 'off';
      for (const b of $('accSeg').children) b.classList.toggle('on', b.dataset.acc === acc);
      // Windows: ha a téma épp a Windowst követi, a mód visszaírása értelmetlen
      const follow = data.appearance === 'windows';
      const modeBtn = root.querySelector('[data-bool="windowsSync.darkMode"]');
      modeBtn.disabled = follow;
      modeBtn.title = follow ? T('syncModeFollow') : T('syncModeHint');
      const termBtn = root.querySelector('[data-bool="windowsSync.terminal"]');
      termBtn.disabled = data.terminalFound === false && !get('windowsSync.terminal');
      termBtn.title = termBtn.disabled ? T('terminalMissing') : T('syncTerminalHint');
      // a panelek helye: ahol a másik panel áll, az a mező szürke (nem választható)
      const lay = data.layout || { icons: 'left', info: 'right' };
      const posButtons = (which, other) => ['left', 'right', 'top', 'bottom'].map((e) =>
        `<button data-pos="${which}" data-edge="${e}" class="${lay[which] === e ? 'on' : ''}"${lay[other] === e ? ` disabled title="${T(which === 'icons' ? 'posTakenInfo' : 'posTakenIcons')}"` : ''}>${T(`pos_${e}`)}</button>`).join('');
      $('posIcons').innerHTML = posButtons('icons', 'info');
      $('posInfo').innerHTML = posButtons('info', 'icons');
      // a modulok a beállított sorrendben (ami hiányzik belőle, a végére kerül)
      // (a csak laptopon látszó modul asztali gépen nem szerepel)
      // (a listedIf a modul saját feltétele, pl. a Perifériák csak csatlakoztatott eszközzel)
      const listed = MODULES.filter((m) => (!m.laptopOnly || data.laptop) && (!m.listedIf || m.listedIf(data))).map((m) => m.id);
      const order = (data.sections.order || []).filter((k) => listed.includes(k));
      for (const k of listed) if (!order.includes(k)) order.push(k);
      // két oszlopban (kisebb munkaterületen) oszloponként felülről lefelé: ennyi sor kell
      $('secList').style.setProperty('--rows', Math.ceil(order.length / 2));
      $('secList').innerHTML = order.map((k) => {
        const m = moduleOf(k);
        const on = (data.sections[k] ?? m.defaultOn ?? true) !== false;
        return `<div class="sec-row" data-key="${k}">
            <span class="grip" title="${T('dragHint')}">${svg('grip')}</span>
            <button class="sbtn toggle row ${on ? 'on' : ''}" data-section="${k}"${m.hint ? ` title="${T(m.hint)}"` : ''}><span>${T(m.name)}</span><b class="switch">${sw(on)}</b></button>
          </div>`;
      }).join('');
      renderOrder();
      renderView();
      $('sizesReset').disabled = sizesDefault();
      renderRanges();
      renderPreview();
      renderPalettes();
      renderCalendars();
      renderEngine();
      for (const p of paneHooks) if (p.render) p.render(data);
      for (const c of MODULE_CARDS) if (c.hooks.render) c.hooks.render(data);
    };

    // ── sorrend húzással (Látható szekciók és Ikonok): a sor elején lévő jellel ──
    // A húzott sor követi az egeret, a többi sor csúszva adja át a helyét; elengedéskor a sor
    // a végleges helyére úszik, és csak ekkor rendezzük át ténylegesen a listát. Görgethető
    // listánál a lista széléhez érve görget.
    // A lista lehet több oszlopos rács is (oszloponként felülről lefelé töltve): a sorok
    // helyét (slotját) a húzás kezdetén lemérjük, és a húzott sorhoz legközelebbi hely a cél.
    const sortable = (list, rowSel, done) => {
      let drag = null;
      list.addEventListener('pointerdown', (e) => {
        const grip = e.target.closest('.grip');
        if (!grip || drag || e.button !== 0) return;
        const rows = [...list.querySelectorAll(rowSel)];
        const row = grip.closest(rowSel);
        if (!row || rows.length < 2) return;
        e.preventDefault();
        const from = rows.indexOf(row);
        const slots = rows.map((r) => { const b = r.getBoundingClientRect(); return { x: b.left, y: b.top }; });
        // egy oszlopos listában a sor csak függőlegesen mozog
        const column = slots.every((p) => Math.abs(p.x - slots[0].x) < 1);
        drag = { row, rows, from, to: from, startX: e.clientX, startY: e.clientY, x: e.clientX, y: e.clientY, slots, column, scroll0: list.scrollTop };
        row.classList.add('dragging');
        list.classList.add('sorting');
        grip.setPointerCapture(e.pointerId);
      });
      // az i. sor eltolása a j. helyre
      const shiftTo = (i, j) => {
        const a = drag.slots[i], b = drag.slots[j];
        return `translate(${b.x - a.x}px, ${b.y - a.y}px)`;
      };
      const place = () => {
        const { row, rows, from, slots } = drag;
        const n = rows.length;
        const ys = slots.map((p) => p.y), xs = slots.map((p) => p.x);
        const dyRaw = drag.y - drag.startY + (list.scrollTop - drag.scroll0);
        const dy = Math.max(Math.min(...ys) - slots[from].y, Math.min(Math.max(...ys) - slots[from].y, dyRaw));
        const dx = drag.column ? 0 : Math.max(Math.min(...xs) - slots[from].x, Math.min(Math.max(...xs) - slots[from].x, drag.x - drag.startX));
        row.style.transform = `translate(${dx}px, ${dy}px)`;
        // a húzott sor bal felső sarkához legközelebbi hely
        const px = slots[from].x + dx, py = slots[from].y + dy;
        let to = 0, best = Infinity;
        slots.forEach((p, i) => {
          const d = (p.x - px) ** 2 + (p.y - py) ** 2;
          if (d < best) { best = d; to = i; }
        });
        to = Math.max(0, Math.min(n - 1, to));
        if (to === drag.to) return;
        drag.to = to;
        rows.forEach((r, i) => {
          if (r === row) return;
          let j = i;
          if (from < to && i > from && i <= to) j = i - 1;
          if (from > to && i >= to && i < from) j = i + 1;
          r.style.transform = j !== i ? shiftTo(i, j) : '';
        });
      };
      list.addEventListener('pointermove', (e) => {
        if (!drag || drag.done) return;
        drag.x = e.clientX;
        drag.y = e.clientY;
        if (list.scrollHeight > list.clientHeight) {
          const r = list.getBoundingClientRect();
          if (e.clientY < r.top + 32) list.scrollTop -= 14;
          else if (e.clientY > r.bottom - 32) list.scrollTop += 14;
        }
        place();
      });
      const end = () => {
        if (!drag || drag.done) return;
        const d = drag;
        d.done = true;
        d.row.classList.remove('dragging');
        d.row.classList.add('settling');
        d.row.style.transform = shiftTo(d.from, d.to);
        setTimeout(() => {
          list.classList.remove('sorting');
          for (const r of d.rows) { r.style.transform = ''; r.classList.remove('settling'); }
          drag = null;
          if (d.to !== d.from) done(d.from, d.to, d.rows);
        }, 170);
      };
      list.addEventListener('pointerup', end);
      list.addEventListener('pointercancel', end);
    };
    for (const f of sortableLater) f();
    // Látható szekciók: azonnal életbe lép
    sortable($('secList'), '.sec-row', (from, to, rows) => {
      const order = rows.map((r) => r.dataset.key);
      order.splice(to, 0, ...order.splice(from, 1));
      for (const k of order) $('secList').appendChild(rows.find((r) => r.dataset.key === k));
      data.sections = { ...data.sections, order };
      // a Megjelenés kártya és a Funkciók fül is ebben a sorrendben
      renderView();
      renderFeatures();
      if (!demo) post({ type: 'sections', sections: data.sections });
    });

    // a külön fájlban lévő fülek indítása (itt, mert a Programok fül is a sortable-t használja);
    // a horgaik: load() új adatnál, render(data) a kirajzolásnál, message(msg), dirty() és
    // discard() a mentetlen változásokhoz, demo() az előnézethez
    // A fülsáv elemei, amelyeket a fülek frissítenek: a fül jelzése (pl. mentetlen változás) és a
    // terhelés kis kijelzője a fülsáv alján (`which`: 'cpu' vagy 'mem', `level`: '', 'warn', 'hot').
    const setBadge = (tabId, on, title = '') => {
      const b = $(`badge-${tabId}`);
      b.classList.toggle('on', on);
      b.title = title;
    };
    const railLoad = (which, pct, level) => {
      const el = $(which === 'mem' ? 'rlMem' : 'rlCpu');
      el.querySelector('b').style.width = `${pct.toFixed(1)}%`;
      el.querySelector('em').textContent = `${Math.round(pct)}%`;
      el.className = `rl ${which === 'mem' ? 'mem ' : ''}${level}`;
    };
    const paneHooks = PANES.map((p) => p.setup({ root, data: () => data, toast, sortable, setBadge, railLoad }) || {});
    const unsaved = () => paneHooks.some((p) => p.dirty && p.dirty());

    // háttérképek: kattintásra a program beállítja a Windows háttérképének
    const normPath = (p) => String(p || '').replace(/\//g, '\\').toLowerCase();
    const renderWalls = () => {
      const cur = normPath(data.wallpaper);
      const list = data.wallpapers || [];
      $('wallList').innerHTML = list.length
        ? list.map((w, i) => `<button class="wall-tile${normPath(w.path) === cur ? ' on' : ''}" data-wall="${i}">
            <img src="${esc(w.url)}" alt="" draggable="false"><b>${T('wallCurrent')}</b></button>`).join('')
        : `<span class="ed-none">${T('noWallpapers')}</span>`;
      // csak a beépített képek vannak: nincs mit visszaállítani
      if (!resetArm) $('wallReset').disabled = !list.some((w) => w.builtin === false);
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
    // Alapállapot: a később felvett képek a Lomtárba kerülnek (két kattintásra, mint az Összes törlése)
    let resetArm = 0;
    const disarmReset = () => {
      clearTimeout(resetArm);
      resetArm = 0;
      $('wallReset').classList.remove('armed');
      $('wallReset').querySelector('span').textContent = T('wallReset');
    };
    $('wallReset').addEventListener('click', () => {
      if (!resetArm) {
        $('wallReset').classList.add('armed');
        $('wallReset').querySelector('span').textContent = T('wallResetSure');
        resetArm = setTimeout(disarmReset, 3500);
        return;
      }
      disarmReset();
      if (demo) return toast('Demo: reset');
      post({ type: 'wall-reset' });
    });

    // ── Egészség: a visszaszámlálás csak itt, a beállításokban látszik ──
    let health = { enabled: false, minutes: 90, remaining: null, showing: false };
    let hlDue = 0;
    const hms = (sec) => {
      const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), x = sec % 60;
      return h ? `${h}:${pad(m)}:${pad(x)}` : `${pad(m)}:${pad(x)}`;
    };
    const hlTick = () => {
      const el = $('hlCount');
      $('hlCard').classList.toggle('due', health.enabled && health.showing);
      $('hlCard').classList.toggle('off', !health.enabled);
      if (!health.enabled) { el.textContent = '—'; return; }
      if (health.showing) { el.textContent = T('hNow'); return; }
      el.textContent = hms(Math.max(0, Math.round((hlDue - Date.now()) / 1000)));
    };
    const renderHealth = () => {
      setSwitch($('hlToggle'), !!health.enabled);
      if (document.activeElement !== $('hlMin')) $('hlMin').value = health.minutes;
      hlTick();
    };
    setInterval(hlTick, 1000);
    const setHealth = (patch) => {
      health = { ...health, ...patch };
      if (demo) {
        health.showing = false;
        hlDue = Date.now() + health.minutes * 60000;
        return renderHealth();
      }
      post({ type: 'health', enabled: health.enabled, minutes: health.minutes });
    };
    $('hlToggle').addEventListener('click', () => setHealth({ enabled: !health.enabled }));
    $('hlMin').addEventListener('change', () => {
      const v = Math.round(Number($('hlMin').value));
      if (!Number.isFinite(v)) return renderHealth();
      const m = Math.max(5, Math.min(480, v));
      $('hlMin').value = m;
      if (m !== health.minutes) setHealth({ minutes: m });
    });
    $('hlMin').addEventListener('keydown', (e) => { if (e.key === 'Enter') $('hlMin').blur(); });
    $('hlReset').addEventListener('click', () => {
      if (demo) return setHealth({});
      post({ type: 'health-reset' });
    });

    // ── Időjárás: a település (szabad szöveg), a mértékegység (°C/°F) és a Szél kapcsoló ──
    let weather = null;
    const renderWeather = () => {
      for (const b of $('wsUnit').children) b.classList.toggle('on', b.dataset.unit === (weather.unit === 'f' ? 'f' : 'c'));
      if (document.activeElement !== $('wsCity')) $('wsCity').value = weather.city || '';
      setSwitch($('wsWind'), weather.wind !== false);
    };
    $('wsWind').addEventListener('click', () => {
      if (!weather) return;
      weather = { ...weather, wind: weather.wind === false };
      renderWeather();
      if (!demo) post({ type: 'weather', wind: weather.wind });
    });
    // a cím mellett a megtalált hely teljes neve (település, megye, ország)
    const setFound = (text, bad, ok) => {
      const f = $('wsFound');
      f.innerHTML = ok ? `${T('wsSetTo')} <b>${esc(text)}</b>` : esc(text);
      f.title = text;
      f.classList.toggle('bad', !!bad);
    };
    // ugyanazzal a keresővel, amit a panel is használ; a találatot a panel is megkapja (közös tároló)
    const lookup = async (city) => {
      const g = await fetchJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&language=${lang}`);
      const hit = g.results && g.results[0];
      if (!hit) return null;
      const label = [...new Set([hit.name, hit.admin1, hit.country].filter(Boolean))].join(', ');
      store.set(`cybershell.geo.${city.toLowerCase()}`, { lat: hit.latitude, lon: hit.longitude, name: hit.name });
      return label;
    };
    let wsSeq = 0;
    const showPlace = async (city) => {
      const seq = ++wsSeq;
      setFound(T('wsSearching'));
      try {
        const label = await lookup(city);
        if (seq === wsSeq) setFound(label || T('unknownPlace', { p: city }), !label, !!label);
        return label ? 'ok' : 'none';
      } catch (_) {
        if (seq === wsSeq) setFound(T('wsOffline'), true);
        return 'offline';
      }
    };
    const commitCity = async () => {
      const city = $('wsCity').value.trim().replace(/\s+/g, ' ');
      if (!city || !weather) { if (weather) $('wsCity').value = weather.city; return; }
      if (city === weather.city) return;
      const res = await showPlace(city);
      // ismeretlen helyet nem mentünk; kapcsolat nélkül igen (a panel később keresi meg)
      if (res === 'none') return;
      weather = { ...weather, city };
      if (demo) return toast(T('wsSaved', { p: city }));
      post({ type: 'weather', city });
    };
    $('wsCity').addEventListener('change', commitCity);
    // az Enter gomb a mezőben: ugyanaz, mint az Enter billentyű (a fókusz a mezőben marad,
    // a kilépés indítja a keresést)
    $('wsGo').addEventListener('mousedown', (e) => e.preventDefault());
    $('wsGo').addEventListener('click', (e) => {
      e.preventDefault();
      if (document.activeElement === $('wsCity')) $('wsCity').blur();
      else commitCity();
    });
    $('wsCity').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') $('wsCity').blur();
      if (e.key === 'Escape' && weather) { $('wsCity').value = weather.city; $('wsCity').blur(); }
    });
    $('wsUnit').addEventListener('click', (e) => {
      const b = e.target.closest('[data-unit]');
      if (!b || !weather || b.classList.contains('on')) return;
      weather = { ...weather, unit: b.dataset.unit };
      renderWeather();
      if (!demo) post({ type: 'weather', unit: weather.unit });
    });

    let firstLoad = true;
    const load = (msg) => {
      data = msg;
      if (msg.weather) {
        const first = !weather || weather.city !== msg.weather.city;
        weather = { ...msg.weather };
        renderWeather();
        if (first) showPlace(weather.city);
      }
      if (msg.health) {
        health = msg.health;
        hlDue = Date.now() + (msg.health.remaining || 0) * 1000;
        renderHealth();
      }
      if (msg.version) $('appVer').textContent = msg.version;
      if (!data.sections) data.sections = { claude: true, activity: true, clock: true, date: true };
      renderToggles();
      renderFeatures();
      renderDateTime();
      renderWalls();
      for (const p of paneHooks) if (p.load) p.load();
      if (firstLoad) {
        firstLoad = false;
        selectTab(store.get(TAB_STORE) || 'programs', { instant: true });
      }
    };

    // rendszergombok, kapcsolók, szegmensek (minden fülön, kivéve az ikonlistát és a saját
    // kezelővel rendelkező kártyákat)
    root.querySelector('.set-body').addEventListener('click', (e) => {
      const b = e.target.closest('button');
      if (!b || !data || b.disabled) return;
      if (b.dataset.cmd) {
        if (demo) return toast(`Demo: ${b.dataset.cmd}`);
        return post({ type: 'cmd', name: b.dataset.cmd });
      }
      if (b.dataset.toggle === 'autostart') {
        if (demo) { data.autostart = !data.autostart; return renderToggles(); }
        return post({ type: 'cmd', name: 'autostart', on: !data.autostart });
      }
      if (b.dataset.toggle === 'winKey') {
        if (demo) { data.winKey = !data.winKey; return renderToggles(); }
        return post({ type: 'winkey', on: !data.winKey });
      }
      if (b.dataset.toggle === 'clipboard') {
        if (demo) { data.clipboard = !data.clipboard; return renderToggles(); }
        return post({ type: 'clipboard', on: !data.clipboard });
      }
      if (b.dataset.toggle === 'desktopMenu') {
        if (demo) { data.desktopMenu = !data.desktopMenu; return renderToggles(); }
        return post({ type: 'deskmenu', on: !data.desktopMenu });
      }
      if (b.dataset.toggle === 'desktopIconMenu') {
        if (demo) { data.desktopIconMenu = !data.desktopIconMenu; return renderToggles(); }
        return post({ type: 'deskiconmenu', on: !data.desktopIconMenu });
      }
      if (b.dataset.bool) {
        const v = !get(b.dataset.bool);
        setLocal(b.dataset.bool, v);
        renderToggles();
        if (b.dataset.bool === 'windowsSync.accent' && v) toast(T('syncAccentOn'));
        return setValue(b.dataset.bool, v);
      }
      if (b.dataset.acc) {
        const v = b.dataset.acc;
        data.windowsSync = { ...(data.windowsSync || {}), accent: v !== 'off', ...(v !== 'off' ? { accentColor: v } : {}) };
        renderToggles();
        if (v !== 'off') toast(T('syncAccentOn'));
        return setValue('windowsSync.accentColor', v);
      }
      if (b.dataset.nb) {
        if (b.dataset.nb === data.neonBorder) return;
        data.neonBorder = b.dataset.nb;
        renderToggles();
        return setValue('neonBorder', b.dataset.nb);
      }
      if (b.dataset.appearance) {
        const v = b.dataset.appearance;
        if (v === (data.appearance || 'dark')) return;
        data.appearance = v;
        renderToggles();
        if (demo) {
          const light = v === 'light' || ((v === 'auto' || v === 'windows') && matchMedia('(prefers-color-scheme: light)').matches);
          data.scheme = light ? 'light' : 'dark';
          renderPalettes();
          return setTheme(THEMES[light ? 'light' : 'dark']);
        }
        return post({ type: 'appearance', value: v });
      }
      if (b.dataset.pos) {
        const lay = { ...(data.layout || { icons: 'left', info: 'right' }), [b.dataset.pos]: b.dataset.edge };
        if (lay.icons === lay.info || b.classList.contains('on')) return;
        data.layout = lay;
        renderToggles();
        if (demo) return toast(T('posDemo'));
        return post({ type: 'layout', ...lay });
      }
      if (b.dataset.labels) {
        const on = b.dataset.labels === 'show';
        if (on === (data.iconLabels !== false)) return;
        data.iconLabels = on;
        renderToggles();
        if (!demo) post({ type: 'icon-labels', on: data.iconLabels });
        return;
      }
      if (b.dataset.section) {
        const k = b.dataset.section;
        const on = (data.sections[k] ?? moduleOf(k).defaultOn ?? true) !== false;
        data.sections = { ...data.sections, [k]: !on };
        renderToggles();
        if (!demo) post({ type: 'sections', sections: data.sections });
      }
    });

    // bezárás: mentetlen változásnál előbb figyelmeztet (és a Programok fülre vált)
    let closeArmed = false;
    const close = () => {
      if (unsaved() && !closeArmed) {
        closeArmed = true;
        selectTab('programs');
        $('setCloseText').textContent = T('discardQ');
        $('setClose').classList.add('armed');
        toast(T('unsavedWarn'));
        setTimeout(() => { closeArmed = false; $('setCloseText').textContent = T('close'); $('setClose').classList.remove('armed'); }, 4000);
        return;
      }
      closeArmed = false;
      // a második kattintás: a mentetlen változások elvesznek
      for (const p of paneHooks) if (p.dirty && p.dirty() && p.discard) p.discard();
      $('setCloseText').textContent = T('close');
      $('setClose').classList.remove('armed');
      hide();
    };
    $('setClose').addEventListener('click', close);
    document.addEventListener('keydown', (e) => {
      // Ctrl+1–5: fül; Ctrl+Tab / Ctrl+Shift+Tab, Ctrl+PageDown / PageUp: következő / előző fül
      if (e.ctrlKey && !e.altKey) {
        const n = Number(e.key);
        if (n >= 1 && n <= TABS.length) { e.preventDefault(); return selectTab(TABS[n - 1].id); }
        const i = TABS.findIndex((t) => t.id === tab);
        const dir = e.key === 'Tab' ? (e.shiftKey ? -1 : 1) : e.key === 'PageDown' ? 1 : e.key === 'PageUp' ? -1 : 0;
        if (dir) { e.preventDefault(); return selectTab(TABS[(i + dir + TABS.length) % TABS.length].id); }
      }
      if (e.key === 'Escape' && !e.target.closest('input, select')) close();
    });

    const h = (msg) => {
      if (msg.type === 'show') show(msg.instant);
      // a MENÜ gomb újabb megnyomása: ugyanúgy zár, mint a Bezárás gomb
      if (msg.type === 'hide') close();
      if (msg.type === 'settings') { if (msg.theme) setTheme(msg.theme); load(msg); }
      if (msg.type === 'toast') toast(msg.text);
      // a fülek saját üzenetei (terhelés, a programválasztó eredménye)
      for (const p of paneHooks) if (p.message) p.message(msg);
    };
    h.demo = () => {
      const C = ['#ff7139', '#ea4335', '#5b8def', '#d97757', '#22a6f2'];
      const q = new URLSearchParams(location.search);
      if (q.get('tab')) store.set(TAB_STORE, q.get('tab'));
      h({
        type: 'settings', autostart: true, layout: { icons: 'left', info: 'right' }, winKey: true, clipboard: true, desktopMenu: true, desktopIconMenu: true, iconLabels: true, tooltips: true, laptop: true, periph: 3, language: lang, appearance: 'dark',
        scheme: 'dark', monitors: 2, screen: { w: 1920, h: 1080 },
        bars: { icons: 86, info: 86, frame: { top: 10, bottom: 10, left: 10, right: 10 }, range: [56, 160], frameRange: [0, 40] },
        allMonitors: true, hideTaskbar: true, activity: { autoStart: false, sound: true },
        neonBorder: 'active', windowsSync: { darkMode: false, accent: true, accentColor: 'hot', terminal: false }, terminalFound: true,
        webSearch: 'https://www.google.com/search?q={q}',
        calendars: [{ name: 'Google Naptár', color: '#00e1ff', host: 'calendar.google.com' }, { name: 'Ünnepek', color: '#ff2a6d', host: 'calendar.google.com' }],
        themes: { dark: { ...THEMES.dark }, light: { ...THEMES.light }, defaultDark: { ...THEMES.dark }, defaultLight: { ...THEMES.light } },
        health: { enabled: true, minutes: 90, remaining: 3725, showing: false },
        weather: { city: 'Budapest', unit: 'c', wind: true },
        currency: { list: ['EUR', 'USD', 'HUF', 'GBP', 'CHF'] },
        sections: { tools: true, recent: true, claude: true, codex: false, activity: true, clock: true, date: false, weather: true, music: true, volume: true,
          order: ['tools', 'recent', 'claude', 'codex', 'activity', 'clock', 'date', 'weather', 'music', 'volume', 'power'],
          toolsOrder: ['calc', 'notepad', 'search', 'snip'], powerOrder: ['lock', 'sleep', 'shutdown', 'restart'] },
        hardware: { cpu: { name: 'AMD Ryzen 5 3600 6-Core Processor', cores: 6, threads: 12, mhz: 3600 },
          board: { name: 'ASRock B450M Pro4', chipset: 'AMD B450', bios: 'P10.08', biosDate: '2023-11-02', slots: 4, maxMem: '128 GB' },
          mem: { total: '24 GB', type: 'DDR4', speed: 3200, slots: 4, same: false, modules: [
            { size: '8 GB', type: 'DDR4', speed: 3200, maker: 'Kingston', part: 'KF3200C16D4/8GX', slot: 'DIMM 0' },
            { size: '8 GB', type: 'DDR4', speed: 3200, maker: 'Kingston', part: 'KF3200C16D4/8GX', slot: 'DIMM 1' },
            { size: '4 GB', type: 'DDR4', speed: 2666, maker: 'Corsair', part: 'CMK8GX4M2A2666C16', slot: 'DIMM 2' }] },
          gpus: [{ name: 'AMD Radeon RX 6600', vram: '8 GB', memType: 'GDDR6', bus: 128, gbps: 14, boost: 2491 }], os: 'Windows 11 Pro (26300)',
          disks: [{ name: 'Samsung SSD 970 EVO Plus 500GB', media: 'SSD', bus: 'NVMe', size: '500 GB', volumes: [{ letter: 'C:', label: '', free: '17 GB', total: '465 GB', pct: 96 }] },
            { name: 'KINGSTON SA400S37480G', media: 'SSD', bus: 'SATA', size: '480 GB', volumes: [{ letter: 'D:', label: 'Fotók', free: '8,4 GB', total: '447 GB', pct: 98 }] },
            { name: 'Hitachi HUA723020ALA641', media: 'HDD', bus: 'SATA', size: '2 TB', volumes: [{ letter: 'E:', label: 'Torrent', free: '65 GB', total: '1863 GB', pct: 96 }] },
            { name: 'TOSHIBA HDWD120', media: 'HDD', bus: 'SATA', size: '2 TB', volumes: [{ letter: 'F:', label: 'Dolgaim', free: '120 GB', total: '1863 GB', pct: 94 }] }] },
        apps: [['Firefox', 'globe'], ['Chrome', 'browser'], ['Total Cmd', 'panels'], ['Claude', 'spark'], ['VS Code', 'code']]
          .map(([name, icon], id) => ({ id, name, rawIcon: icon, icon, color: C[id], path: `C:\\Program Files\\${name}\\${name}.exe`,
            available: id !== 2, canExeIcon: true, autostart: id === 3 }))
          .concat([{ id: 5, name: 'Cyberpunk', rawIcon: 'game', icon: 'game', color: '#ff3b5c', path: 'steam://rungameid/1091500', available: true }]),
        running: ['Jegyzettömb', 'Számológép', 'Windows Intéző', 'Paint', 'Microsoft Edge', 'Discord', 'Steam', 'OBS Studio', 'GIMP', 'Thunderbird', 'Telegram', 'Blender', 'Audacity', 'VLC']
          .map((name) => ({ name, path: `C:\\Apps\\${name}.exe`, img: '' })),
        wallpapers: [...Array(8)].map((_, i) => `CyberShell_background_${pad(i + 1)}`)
          .map((n) => ({ path: `C:\\CyberShell\\wallpapers\\${n}.jpg`, url: `../wallpapers/${n}.jpg`, builtin: true })),
        wallpaper: 'C:\\CyberShell\\wallpapers\\CyberShell_background_03.jpg',
      });
      show();
      for (const p of paneHooks) if (p.demo) p.demo();
    };
    return h;
  }

  window.HUB.parts.settings = buildSettings;
})();
