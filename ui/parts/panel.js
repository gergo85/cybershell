// Az információs panel (alapból jobbra) kerete. A szekciókat a modulok adják (modules/*.js,
// leírás: modules/README.md); itt csak a közös rész van: az Indítás és a MENÜ gomb, az
// értesítés, a modulok sorrendje és láthatósága, a helykihasználás (szűkítés, görgetés), a
// másodpercenkénti óraütés és a program üzeneteinek szétosztása.
(() => {
  const { $, side, horiz, T, root, demo, post, svg, esc, pad, modules } = window.HUB;

  function buildRight() {
    root.className = `right side-${side}${horiz ? ' horiz' : ''}`;
    const sectionHtml = (def) => {
      const id = def.sectionId || `${def.id}Sec`;
      const tip = def.tip ? ` title="${T(def.tip)}"` : '';
      return `<section class="sec${def.className ? ` ${def.className}` : ''}" id="${id}"${tip}>
        <h3>${T(def.title)}</h3>
        ${def.html({ T, svg, esc, pad })}
      </section>`;
    };
    root.innerHTML = `
      <div class="edge"></div>
      <div class="sys sys-start">
        <button class="sys-btn" id="startBtn" data-cmd="start">${T('startBtn')}</button>
      </div>
      ${modules.map(sectionHtml).join('')}
      <div class="sys">
        <button class="sys-btn" id="sysBtn">${T('menu')}</button>
      </div>
      <div class="toast" id="toast"></div>`;
    const sysBtn = $('sysBtn');
    const menuRow = sysBtn.parentElement;

    // egy elem közepe a panel mentén (függőleges panelnél fentről, vízszintesnél balról)
    const alongOf = (el) => {
      if (!el) return 0;
      const r = el.getBoundingClientRect();
      return Math.round(horiz ? r.left + r.width / 2 : r.top + r.height / 2);
    };
    // egy elem helye a képernyőn: a mellé nyíló ablakok (naptár, időjárás) ehhez igazodnak
    const rectOf = (el) => {
      const r = el.getBoundingClientRect();
      return { top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right) };
    };

    let toastTimer = 0;
    const toast = (text) => {
      const t = $('toast');
      if (!horiz) t.style.bottom = `${root.clientHeight - sysBtn.offsetTop + 8}px`;
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => t.classList.remove('show'), 4500);
    };

    // Indítás (a panel elején): a CyberShell Start menüje; MENÜ (a végén): a beállítások
    $('startBtn').addEventListener('click', () => {
      if (demo) return toast('Demo: start');
      post({ type: 'cmd', name: 'start', along: alongOf($('startBtn')) });
    });
    sysBtn.addEventListener('click', () => {
      if (demo) return toast('Demo: settings');
      post({ type: 'cmd', name: 'settings' });
    });

    // ── helykihasználás ──
    // Ha a szekciók nem férnek ki (kis képernyő, laptop szekció, szóló zene), a panel
    // előbb a szekciók közti réseket szűkíti, aztán a menü fölötti díszítést is.
    const overflows = () => (horiz ? root.scrollWidth > root.clientWidth + 1 : root.scrollHeight > root.clientHeight + 1);
    let quickSig = '';
    const fitPanel = () => {
      root.classList.remove('tight', 'tighter', 'overflow');
      if (overflows()) {
        root.classList.add('tight');
        if (overflows()) root.classList.add('tighter');
        // ha így sem fér ki: görgethető, a Start és a MENÜ a helyén marad
        if (overflows()) root.classList.add('overflow');
      }
      // az Indítás és a Keresés gomb helye: a Win billentyűvel nyitott menü is ide igazodik
      const pos = { start: alongOf($('startBtn')), search: alongOf(root.querySelector('[data-cmd="search"]')) };
      const sig = JSON.stringify(pos);
      if (sig !== quickSig) { quickSig = sig; post({ type: 'quick-pos', ...pos }); }
    };
    window.addEventListener('resize', fitPanel);
    // bármelyik szekció mérete változik (pl. megjön a Claude-adat), újramérünk
    let fitQueued = false;
    const queueFit = () => {
      if (fitQueued) return;
      fitQueued = true;
      setTimeout(() => { fitQueued = false; fitPanel(); }, 30);
    };

    // ── a modulok ──
    let sections = {};
    const live = [];
    const byId = {};
    const shown = (m) => m.available && (sections[m.def.id] ?? m.def.defaultOn ?? true) !== false;
    const applyVisibility = (m) => {
      const display = shown(m) ? '' : 'none';
      if (m.el.style.display === display) return false;
      m.el.style.display = display;
      return true;
    };
    // a gombok sorrendje egy rácsban (data-k szerint), pl. az Eszköztárban és a Főkapcsolóban
    const reorder = (grid, keys) => {
      for (const k of keys || []) {
        const b = grid.querySelector(`[data-k="${k}"]`);
        if (b) grid.appendChild(b);
      }
    };
    for (const def of modules) {
      const m = { def, el: $(def.sectionId || `${def.id}Sec`), available: true, hooks: {} };
      live.push(m);
      byId[def.id] = m;
      const ctx = {
        id: def.id, el: m.el, demo, horiz, post, toast,
        fit: fitPanel,
        along: alongOf,
        rect: rectOf,
        reorder,
        sections: () => sections,
        // a modul maga rejti el magát (pl. nincs akkumulátor), a beállítástól függetlenül
        setAvailable(on) {
          m.available = !!on;
          if (applyVisibility(m)) fitPanel();
        },
        // egy másik modul nyilvános függvénye (a setup által visszaadott api-ból)
        call: (id, name, ...args) => {
          const f = byId[id] && byId[id].hooks.api && byId[id].hooks.api[name];
          return f ? f(...args) : undefined;
        },
      };
      m.hooks = def.setup ? def.setup(ctx) || {} : {};
    }

    // ── láthatóság, kompakt nézet és sorrend (a beállításokból) ──
    const applySections = () => {
      const compact = sections.compact || [];
      for (const m of live) {
        applyVisibility(m);
        if (m.def.compact) m.el.classList.toggle('compact', compact.includes(m.def.id));
      }
      // a beállított sorrendben, a MENÜ előtt (ami a listából hiányzik, a végére kerül)
      const listed = live.filter((m) => m.def.listed !== false).map((m) => m.def.id);
      const order = (sections.order || []).filter((id) => listed.includes(id));
      for (const id of listed) if (!order.includes(id)) order.push(id);
      for (const id of order) root.insertBefore(byId[id].el, menuRow);
      for (const m of live) if (m.hooks.sections) m.hooks.sections(sections);
      fitPanel();
    };

    // vízszintes panelen a görgő oldalra görget, ha nem fér ki minden (a görgőt maguk
    // használó modulok fölött – zene, hangerő – nem)
    const wheelOwners = live.filter((m) => m.def.ownsWheel).map((m) => `#${m.el.id}`).join(', ');
    root.addEventListener('wheel', (e) => {
      if (!horiz || !root.classList.contains('overflow') || e.defaultPrevented) return;
      if (wheelOwners && e.target.closest(wheelOwners)) return;
      root.scrollLeft += e.deltaY || e.deltaX;
      e.preventDefault();
    }, { passive: false });
    const fitObserver = new ResizeObserver(queueFit);
    for (const m of live) fitObserver.observe(m.el);

    // másodpercenkénti óraütés a moduloknak (óra, dátum, visszaszámláló…)
    const tick = () => {
      const d = new Date();
      for (const m of live) if (m.hooks.tick) m.hooks.tick(d);
      setTimeout(tick, 1000 - new Date().getMilliseconds() + 5);
    };
    applySections();
    tick();

    const h = (msg) => {
      if (msg.type === 'init' && msg.sections) sections = msg.sections;
      if (msg.type === 'sections') sections = msg.sections || sections;
      for (const m of live) if (m.hooks.message) m.hooks.message(msg);
      if (msg.type === 'init' || msg.type === 'sections') applySections();
      if (msg.type === 'settings-open') sysBtn.classList.toggle('open', !!msg.on);
      if (msg.type === 'toast') toast(msg.text);
      if (msg.type === 'launcher-state') $('startBtn').classList.toggle('open', !!msg.open && msg.mode === 'start');
      queueFit();
    };
    h.demo = () => {
      h({ type: 'init', activity: { minutes: 20, autoStart: false }, session: 1, claudeUsage: true, weather: { enabled: true, city: 'Budapest' },
        sections: { codex: true } });
      for (const m of live) if (m.hooks.demo) m.hooks.demo();
    };
    return h;
  }

  window.HUB.parts.right = buildRight;
})();
