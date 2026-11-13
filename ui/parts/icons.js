(() => {
  const { $, side, horiz, root, demo, post, svg, esc, appColor } = window.HUB;

  // ───────────── bal oldali ikonsáv ─────────────
  function buildLeft() {
    root.className = `left side-${side}${horiz ? ' horiz' : ''}`;
    root.innerHTML = `
      <div class="edge"></div>
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
        post({ type: 'peek', ...target(el), y: Math.round(r.top + r.height / 2), x: Math.round(r.left + r.width / 2) });
      }, peekOpen ? 80 : 450);
    };
    appsEl.addEventListener('mouseover', (e) => peekHover(e.target.closest('.app')));
    // vízszintes sávon a görgő oldalra görget
    appsEl.addEventListener('wheel', (e) => {
      if (!horiz || !e.deltaY) return;
      e.preventDefault();
      appsEl.scrollLeft += e.deltaY;
    }, { passive: false });
    appsEl.addEventListener('mouseleave', () => peekHover(null));

    appsEl.addEventListener('click', (e) => {
      const el = e.target.closest('.app');
      clearTimeout(peekTimer);
      peekSent = false;
      if (el) launch(el, e.shiftKey);
    });
    // jobb gomb: kis menü a program ablakához (Teljes képernyőre, Ablak bezárása), ha
    // pontosan egy ablaka van nyitva
    appsEl.addEventListener('contextmenu', (e) => {
      const el = e.target.closest('.app');
      if (!el || demo) return;
      clearTimeout(peekTimer);
      // a menü az ikon mellett nyílik; ha az egér elhagyja az ikont és a menüt, bezárul
      const r = el.querySelector('.tile').getBoundingClientRect();
      peekSent = true;
      post({ type: 'app-menu', ...target(el), y: Math.round(r.top + r.height / 2), x: Math.round(r.left + r.width / 2) });
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
      if (msg.type === 'init') { apps = msg.apps || []; renderApps(); root.classList.toggle('no-labels', msg.iconLabels === false); }
      // Ikonnév mutatása (a beállításokból)
      if (msg.type === 'icon-labels') root.classList.toggle('no-labels', msg.on === false);
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

  window.HUB.parts.left = buildLeft;
})();
