(() => {
  const { $, part, side, lang, T, root, demo, post, setTheme, THEMES, svg, esc, pad, store, fetchJson } = window.HUB;

  // ───────────── beállítások (a munkaterületen, animálva) ─────────────
  function buildSettings() {
    root.className = 'settings';
    const ICON_CHOICES = ['browser', 'globe', 'code', 'folder', 'panels', 'terminal', 'doc', 'sheet', 'notes', 'music',
      'photo', 'enhance', 'design', 'picker', 'video', 'mail', 'chat', 'game', 'rocket', 'spark', 'calendar', 'cloud', 'settings', 'app'];
    root.innerHTML = `
      <div class="set-wrap">
        <header class="set-head">
          <div class="set-title">CYBER<span>SHELL</span><em>${T('settings')}</em></div>
          <div class="toast set-toast" id="toast"></div>
          <button class="set-close" id="setClose">${svg('close')}<span id="setCloseText">${T('close')}</span></button>
        </header>
        <div class="set-body">
          <aside class="set-side">
            <section class="set-card">
              <h2>${T('visibleSections')}</h2>
              <div class="set-list" id="secList"></div>
            </section>
            <section class="set-card">
              <h2>${T('system')}</h2>
              <div class="set-grid">
                <button class="sbtn toggle" data-toggle="autostart">${svg('power')}<span>${T('startWithWindows')}</span><b class="switch">–</b></button>
                <button class="sbtn toggle wide" data-toggle="winKey" title="${T('winKeyHint')}">${svg('start')}<span>${T('winKey')}</span><b class="switch">–</b></button>
                <button class="sbtn" data-cmd="config">${svg('notes')}<span>${T('configFile')}</span></button>
                <button class="sbtn" data-cmd="reload" title="${T('reloadHint')}">${svg('reload')}<span>${T('reload')}</span></button>
                <button class="sbtn danger wide" data-cmd="exit">${svg('close')}<span>${T('exit')}</span></button>
              </div>
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
              <div class="credit-by credit-ver"><span>${T('version')}</span><b id="appVer">v1.4</b></div>
              <button class="sbtn mini" data-cmd="github">${svg('github')}<span>GitHub</span></button>
              <button class="sbtn mini icon" data-cmd="website" title="${T('website')}">${svg('globe')}</button>
            </section>
          </aside>
          <div class="set-col">
          <main class="set-card set-main">
            <h2>${T('icons')}</h2>
            <div class="ed-head"><span></span><span></span><span>${T('colName')}</span><span>${T('colProgram')}</span><span>${T('colIcon')}</span><span>${T('colColor')}</span><span>${T('colAuto')}</span><span></span></div>
            <div class="ed-list" id="edList"></div>
            <!-- egy tördelődő sor: hozzáadás, futó programok, a végén a mentés (a sor jobb szélén) -->
            <div class="ed-add">
              <button class="sbtn" id="edBrowse">${svg('plus')}<span>${T('addProgram')}</span></button>
              <div class="ed-chips" id="edRunning"></div>
              <footer class="ed-foot">
                <span class="ed-dirty" id="edDirty"></span>
                <button class="sbtn" id="edCancel">${svg('close')}<span>${T('discard')}</span></button>
                <button class="sbtn primary" id="edSave">${svg('check')}<span>${T('save')}</span></button>
              </footer>
            </div>
          </main>
          <div class="set-pair">
          <section class="set-card set-look">
            <h2>${T('look')}</h2>
            <div class="set-grid">
                <div class="lang-row theme-row" title="${T('apHint')}">${svg('moon')}<span>${T('appearance')}</span>
                  <div class="seg" id="themeSeg"><button data-appearance="dark">${T('apDark')}</button><button data-appearance="light">${T('apLight')}</button><button data-appearance="auto" title="${T('apAutoHint')}">${T('apAuto')}</button></div>
                </div>
                <div class="lang-row">${svg('globe')}<span>${T('language')}</span>
                  <div class="seg" id="langSeg"><button data-lang="hu">HU</button><button data-lang="en">EN</button></div>
                </div>
                <div class="lang-row pos-row">${svg('panels')}<span>${T('iconsPlace')}</span><div class="seg" id="posIcons"></div></div>
                <div class="lang-row pos-row">${svg('swap')}<span>${T('infoPlace')}</span><div class="seg" id="posInfo"></div></div>
                <div class="lang-row" title="${T('iconLabelsHint')}">${svg('text')}<span>${T('iconLabels')}</span>
                  <div class="seg" id="labelSeg"><button data-labels="show">${T('lblShow')}</button><button data-labels="hide">${T('lblHide')}</button></div>
                </div>
            </div>
          </section>
          <div class="set-stack">
          <section class="set-card health" id="hlCard">
            <h2>${T('health')}<small title="${T('hHint')}">${T('hHint')}</small></h2>
            <div class="hl-row">
              <button class="sbtn toggle" id="hlToggle">${svg('heart')}<span>${T('hBreak')}</span><b class="switch">–</b></button>
              <label class="hl-min"><span>${T('hEvery')}</span><input id="hlMin" type="number" min="5" max="480" step="5" inputmode="numeric"><span>${T('hMinutes')}</span></label>
              <div class="hl-count"><span>${T('hNext')}</span><b id="hlCount">—</b></div>
              <button class="sbtn mini icon" id="hlReset" title="${T('hRestartHint')}">${svg('reload')}</button>
            </div>
          </section>
          <section class="set-card wset" id="wsCard">
            <h2>${T('wsTitle')}<small id="wsFound" title=""></small></h2>
            <div class="ws-row">
              <label class="ws-city" title="${T('wsCityHint')}">${svg('place')}<span class="ws-field"><input id="wsCity" type="text" maxlength="80" spellcheck="false" autocomplete="off" placeholder="${T('wsCityPh')}"><button class="ws-go" id="wsGo" title="${T('wsGo')}">${svg('enter')}</button></span></label>
              <button class="sbtn toggle" id="wsWind" title="${T('wsWindHint')}">${svg('wind')}<span>${T('wsWind')}</span><b class="switch">–</b></button>
            </div>
          </section>
          </div>
          </div>
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
          </div>
        </div>
      </div>
      `;

    let data = null;      // a programtól kapott legutóbbi állapot
    let draft = [];       // a szerkesztett ikonlista
    let original = '[]';
    const SECTION_NAMES = { tools: T('sTools'), power: T('sPower'), claude: T('sClaude'), activity: T('sActivity'), date: T('sDate'), clock: T('sClock'),
      weather: T('sWeather'), music: T('sMusic'), volume: T('sVolume') };
    const SECTION_HINT = { music: T('sMusicHint'), weather: T('sWeatherHint') };

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
      color: a.color || '', img: a.img || '', canExeIcon: !!a.canExeIcon,
      available: a.available !== false, hint: a.hint || '', autostart: !!a.autostart,
    });
    const strip = (a) => ({ id: a.id, name: a.name, path: a.path, args: a.args, icon: a.icon, color: a.color, autostart: a.autostart });
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
      for (const b of $('labelSeg').children) b.classList.toggle('on', (b.dataset.labels === 'show') === (data.iconLabels !== false));
      for (const b of $('themeSeg').children) b.classList.toggle('on', b.dataset.appearance === (data.appearance || 'dark'));
      // a panelek helye: ahol a másik panel áll, az a mező szürke (nem választható)
      const lay = data.layout || { icons: 'left', info: 'right' };
      const posButtons = (which, other) => ['left', 'right', 'top', 'bottom'].map((e) =>
        `<button data-pos="${which}" data-edge="${e}" class="${lay[which] === e ? 'on' : ''}"${lay[other] === e ? ` disabled title="${T(which === 'icons' ? 'posTakenInfo' : 'posTakenIcons')}"` : ''}>${T(`pos_${e}`)}</button>`).join('');
      $('posIcons').innerHTML = posButtons('icons', 'info');
      $('posInfo').innerHTML = posButtons('info', 'icons');
      const order = [...((data.sections.order && data.sections.order.length) ? data.sections.order : [])];
      if (!order.includes('tools')) order.splice(Math.max(0, order.indexOf('claude')), 0, 'tools');
      for (const k of Object.keys(SECTION_NAMES)) if (!order.includes(k)) order.push(k);
      $('secList').innerHTML = order.filter((k) => SECTION_NAMES[k]).map((k) => {
        const on = data.sections[k] !== false;
        const sw = lang === 'hu' ? (on ? 'BE' : 'KI') : (on ? 'ON' : 'OFF');
        return `<div class="sec-row" data-key="${k}">
            <span class="grip" title="${T('dragHint')}">${svg('grip')}</span>
            <button class="sbtn toggle row ${on ? 'on' : ''}" data-section="${k}"${SECTION_HINT[k] ? ` title="${SECTION_HINT[k]}"` : ''}><span>${SECTION_NAMES[k]}</span><b class="switch">${sw}</b></button>
          </div>`;
      }).join('');
    };

    // ── sorrend húzással (Látható szekciók és Ikonok): a sor elején lévő jellel ──
    // A húzott sor követi az egeret, a többi sor csúszva adja át a helyét; elengedéskor a sor
    // a végleges helyére úszik, és csak ekkor rendezzük át ténylegesen a listát. Görgethető
    // listánál a lista széléhez érve görget.
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
        const step = rows[1].getBoundingClientRect().top - rows[0].getBoundingClientRect().top;
        drag = { row, rows, from, to: from, startY: e.clientY, y: e.clientY, step, scroll0: list.scrollTop };
        row.classList.add('dragging');
        list.classList.add('sorting');
        grip.setPointerCapture(e.pointerId);
      });
      const place = () => {
        const { row, rows, from, step } = drag;
        const n = rows.length;
        const raw = drag.y - drag.startY + (list.scrollTop - drag.scroll0);
        const dy = Math.max(-from * step, Math.min((n - 1 - from) * step, raw));
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
      };
      list.addEventListener('pointermove', (e) => {
        if (!drag || drag.done) return;
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
        d.row.style.transform = `translateY(${(d.to - d.from) * d.step}px)`;
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
    // Látható szekciók: azonnal életbe lép
    sortable($('secList'), '.sec-row', (from, to, rows) => {
      const order = rows.map((r) => r.dataset.key);
      order.splice(to, 0, ...order.splice(from, 1));
      for (const k of order) $('secList').appendChild(rows.find((r) => r.dataset.key === k));
      data.sections = { ...data.sections, order };
      if (!demo) post({ type: 'sections', sections: data.sections });
    });

    const renderRunning = () => {
      const have = new Set(draft.map((a) => a.path.toLowerCase()));
      const list = (data.running || []).filter((r) => !have.has(r.path.toLowerCase()));
      $('edRunning').innerHTML = list.length
        ? list.map((r, i) => `<button class="run-chip" data-run="${i}" title="${esc(T('runningTitle').replace(/^\/\/\s*/, ''))}"><img src="${esc(r.img)}" alt=""><span>${esc(r.name)}</span>${svg('plus')}</button>`).join('')
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
          <div class="ed-item" data-i="${i}">
          <span class="grip" title="${T('dragHint')}">${svg('grip')}</span>
          <div class="ed-row${a.available ? '' : ' missing'}" data-i="${i}">
            <span class="ed-icon"${a.color ? ` style="--app:${esc(a.color)}"` : ''}>${iconPreview(a)}</span>
            <input class="ed-name" data-field="name" value="${esc(a.name)}" maxlength="24" spellcheck="false">
            <div class="ed-path">${esc(a.path)}${a.available ? '' : `<b>${T('notInstalled')}</b>`}</div>
            <select class="ed-sel" data-field="icon">${opts}</select>
            <input class="ed-color" type="color" data-field="color" value="${color}">
            <button class="ib auto${a.autostart ? ' on' : ''}" data-act="auto">${svg('power')}</button>
            <button class="ib danger" data-act="del">${svg('trash')}</button>
          </div>
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
      const on = !!health.enabled;
      $('hlToggle').classList.toggle('on', on);
      $('hlToggle').querySelector('.switch').textContent = lang === 'hu' ? (on ? 'BE' : 'KI') : (on ? 'ON' : 'OFF');
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

    // ── Időjárás: a település (szabad szöveg) és a szél kijelzése a panelen ──
    let weather = null;
    const renderWeather = () => {
      const on = weather.wind !== false;
      $('wsWind').classList.toggle('on', on);
      $('wsWind').querySelector('.switch').textContent = lang === 'hu' ? (on ? 'BE' : 'KI') : (on ? 'ON' : 'OFF');
      if (document.activeElement !== $('wsCity')) $('wsCity').value = weather.city || '';
    };
    // a cím mellett a megtalált hely teljes neve (település, megye, ország)
    const setFound = (text, bad) => {
      const f = $('wsFound');
      f.textContent = text;
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
        if (seq === wsSeq) setFound(label || T('unknownPlace', { p: city }), !label);
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
    $('wsWind').addEventListener('click', () => {
      if (!weather) return;
      weather = { ...weather, wind: weather.wind === false };
      renderWeather();
      if (!demo) post({ type: 'weather', wind: weather.wind });
    });

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
      const act = b.dataset.act;
      if (act === 'del') draft.splice(i, 1);
      if (act === 'auto') {
        const a = draft[i];
        a.autostart = !a.autostart;
        toast(a.autostart && !data.autostart ? T('autoHint') : T(a.autostart ? 'autoOn' : 'autoOff', { n: a.name }));
      }
      renderList();
    });
    // Ikonok: a sorrend a vázlatban változik, a Mentés gombbal lép életbe
    sortable($('edList'), '.ed-item', (from, to) => {
      draft.splice(to, 0, ...draft.splice(from, 1));
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
        color: '', img: app.img || '', canExeIcon: !!app.canExeIcon, available: true, hint: '', autostart: false,
      });
      renderList();
      const rows = $('edList').querySelectorAll('.ed-item');
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
    // a bal oszlop és a Megjelenés kártya gombjai
    for (const host of root.querySelectorAll('.set-side, .set-look')) host.addEventListener('click', (e) => {
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
      if (b.dataset.appearance) {
        const v = b.dataset.appearance;
        if (v === (data.appearance || 'dark')) return;
        data.appearance = v;
        renderToggles();
        if (demo) {
          const light = v === 'light' || (v === 'auto' && matchMedia('(prefers-color-scheme: light)').matches);
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
      if (b.dataset.toggle === 'winKey') {
        if (demo) { data.winKey = !data.winKey; return renderToggles(); }
        return post({ type: 'winkey', on: !data.winKey });
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
        type: 'settings', autostart: true, layout: { icons: 'left', info: 'right' }, winKey: true, iconLabels: true, language: lang, appearance: 'dark',
        health: { enabled: true, minutes: 90, remaining: 3725, showing: false },
        weather: { city: 'Budapest', wind: true },
        sections: { tools: true, claude: true, activity: true, clock: true, date: false, weather: true, music: true, volume: true,
          order: ['tools', 'claude', 'activity', 'clock', 'date', 'weather', 'music', 'volume', 'power'] },
        apps: [['Firefox', 'globe'], ['Chrome', 'browser'], ['Total Cmd', 'panels'], ['Claude', 'spark'], ['VS Code', 'code']]
          .map(([name, icon], id) => ({ id, name, rawIcon: icon, icon, color: C[id], path: `C:\\Program Files\\${name}\\${name}.exe`,
            available: id !== 2, canExeIcon: true, autostart: id === 3 }))
          .concat([{ id: 5, name: 'Cyberpunk', rawIcon: 'game', icon: 'game', color: '#ff3b5c', path: 'steam://rungameid/1091500', available: true }]),
        running: [{ name: 'Notepad', path: 'C:\\Windows\\notepad.exe', img: '' }],
        wallpapers: ['blue-hexagons', 'circuit-lines', 'dust-bridge', 'hud-interface', 'motherboard', 'tech-plates']
          .map((n) => ({ path: `C:\\CyberShell\\wallpapers\\${n}.jpg`, url: `../wallpapers/${n}.jpg` })),
        wallpaper: 'C:\\CyberShell\\wallpapers\\dust-bridge.jpg',
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

  window.HUB.parts.settings = buildSettings;
})();
