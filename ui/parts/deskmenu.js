(() => {
  const { root, post, setTheme, svg, esc } = window.HUB;

  // ───────────── saját asztali menü (jobb kattintás az asztal üres területén) ─────────────
  // Az elemeket a program küldi (a Windows saját asztali menüjéből kiolvasva); a felület
  // csak kirajzolja őket, és a választott elem azonosítóját küldi vissza. Az ablak az egész
  // képernyőt lefedi, de csak a menük területe látszik belőle (desk-rects): a menük helyét,
  // a képernyő szélén való megfordulást és az almenüket ezért itt számoljuk ki.
  function buildDesk() {
    root.className = 'desk';
    let seq = 0;
    // a nyitott menük: a gyökér és az almenük sorban (mindegyiknél az elemek, a DOM-elem és a kijelölt elem)
    let panels = [];
    let hoverTimer = 0;
    const SUB_DELAY = 260;

    const dpr = () => window.devicePixelRatio || 1;

    // a felirat gyorsbetűje: az „&” utáni betű aláhúzva („&&”: maga az „&” jel)
    const label = (s) => {
      let out = '', key = '';
      for (let i = 0; i < s.length; i++) {
        const c = s[i];
        if (c === '&' && i + 1 < s.length) {
          const n = s[++i];
          if (n === '&') { out += '&amp;'; continue; }
          if (!key) { key = n.toLocaleLowerCase(); out += `<u>${esc(n)}</u>`; continue; }
          out += esc(n);
          continue;
        }
        out += esc(c);
      }
      return { html: out, key };
    };

    const sendRects = () => {
      const rects = panels.map((p) => {
        const r = p.el.getBoundingClientRect();
        const x = Math.floor(r.left), y = Math.floor(r.top);
        return [x, y, Math.ceil(r.right) - x, Math.ceil(r.bottom) - y];
      });
      post({ type: 'desk-rects', seq, dpr: dpr(), rects });
    };

    const clear = () => {
      clearTimeout(hoverTimer);
      panels = [];
      root.innerHTML = '';
      root.classList.remove('kbd');
    };

    const renderPanel = (items, level) => {
      const el = document.createElement('div');
      el.className = 'dm-panel';
      el.dataset.level = level;
      el.innerHTML = items.map((it, i) => {
        if (it.sep) return '<i class="dm-sep"></i>';
        const l = label(it.label || '');
        it.mn = l.key;
        const mark = it.check === 'radio' ? '<b class="dm-radio"></b>' : it.check ? svg('check') : '';
        const icon = typeof it.icon === 'number' ? `<img src="/__deskicon/${seq}/${it.icon}" alt="">` : '';
        const cls = ['dm-item', it.disabled ? 'off' : '', it.default ? 'def' : '', it.sub ? 'has-sub' : ''].filter(Boolean).join(' ');
        return `<button class="${cls}" data-i="${i}" tabindex="-1">
            <span class="dm-mark">${mark}</span><span class="dm-icon">${icon}</span>
            <span class="dm-label">${l.html}</span>
            <span class="dm-key">${it.key ? esc(it.key) : ''}</span>
            <span class="dm-arrow">${it.sub ? svg('right') : ''}</span>
          </button>`;
      }).join('');
      root.appendChild(el);
      return el;
    };

    // a menü helye: a megadott ponttól jobbra-lefelé; ha nem fér ki, balra, illetve fölfelé
    // fordul (az almenü a szülő elem másik oldalára), végső esetben a képernyő széléhez simul
    const place = (el, x, y, altX, altY) => {
      const w = el.offsetWidth, h = el.offsetHeight;
      const W = window.innerWidth, H = window.innerHeight;
      let left = x + w <= W ? x : altX - w;
      if (left < 0) left = Math.max(0, W - w);
      let top = y + h <= H ? y : altY - h;
      if (top < 0) top = Math.max(0, H - h);
      el.style.left = `${Math.round(left)}px`;
      el.style.top = `${Math.round(top)}px`;
    };

    const closeFrom = (level) => {
      while (panels.length > level) {
        const p = panels.pop();
        p.el.remove();
      }
      const parent = panels[level - 1];
      if (parent) parent.el.querySelectorAll('.dm-item.open').forEach((b) => b.classList.remove('open'));
    };

    const setActive = (level, i) => {
      const p = panels[level];
      if (!p) return;
      p.active = i;
      p.el.querySelectorAll('.dm-item').forEach((b) => b.classList.toggle('hot', Number(b.dataset.i) === i));
    };

    const openSub = (level, i, select) => {
      const p = panels[level];
      const it = p && p.items[i];
      if (!it || !it.sub || it.disabled) return;
      if (panels[level + 1] && panels[level + 1].owner === i) {
        if (select) setActive(level + 1, firstIndex(panels[level + 1].items, 1));
        return;
      }
      closeFrom(level + 1);
      const btn = p.el.querySelector(`.dm-item[data-i="${i}"]`);
      btn.classList.add('open');
      const el = renderPanel(it.sub, level + 1);
      const r = btn.getBoundingClientRect(), pr = p.el.getBoundingClientRect();
      const pad = 5;
      const at = [pr.right - 2, r.top - pad, pr.left + 2, r.bottom + pad];
      panels.push({ el, items: it.sub, active: -1, owner: i, at });
      place(el, ...at);
      if (select) setActive(level + 1, firstIndex(it.sub, 1));
      sendRects();
      // a Windows néhány almenüt (Küldés) késleltetve tölt fel: megnyitáskor újraolvastatjuk
      if (typeof it.menu === 'number') post({ type: 'desk-sub', seq, menu: it.menu });
    };

    // egy almenü újraolvasva: ha bővült (vagy változott), a nyitott almenü is frissül
    const updateSub = (msg) => {
      if (msg.seq !== seq) return;
      for (let level = 0; level < panels.length; level++) {
        const it = panels[level].items.find((x) => x.menu === msg.menu);
        if (!it) continue;
        if (JSON.stringify(it.sub) === JSON.stringify(msg.items)) return;
        it.sub = msg.items;
        it.disabled = !msg.items.length;
        const child = panels[level + 1];
        if (!child || panels[level].items[child.owner] !== it) return;
        closeFrom(level + 2);
        const el = renderPanel(it.sub, level + 1);
        child.el.remove();
        child.el = el;
        child.items = it.sub;
        place(el, ...child.at);
        if (child.active >= 0) setActive(level + 1, Math.min(child.active, it.sub.length - 1));
        sendRects();
        return;
      }
    };

    // a következő kijelölhető elem (az elválasztókat átugorja), körbe
    const firstIndex = (items, dir, from = -1) => {
      const n = items.length;
      for (let k = 1; k <= n; k++) {
        const i = (((from + dir * k) % n) + n) % n;
        if (!items[i].sep) return i;
      }
      return -1;
    };

    const choose = (level, i) => {
      const it = panels[level] && panels[level].items[i];
      if (!it || it.sep || it.disabled) return;
      if (it.sub) return openSub(level, i, true);
      post({ type: 'desk-invoke', seq, id: it.id });
    };

    const open = (msg) => {
      try {
        if (msg.theme) setTheme(msg.theme);
        clear();
        seq = msg.seq;
        const x = msg.x / dpr(), y = msg.y / dpr();
        const el = renderPanel(msg.items || [], 0);
        panels = [{ el, items: msg.items || [], active: -1 }];
        place(el, x, y, x, y);
        sendRects();
      } catch (e) {
        post({ type: 'desk-fail', seq: msg.seq, error: String(e) });
      }
    };

    // ── egér ──
    const target = (e) => {
      const b = e.target.closest('.dm-item');
      if (!b) return null;
      const level = Number(b.closest('.dm-panel').dataset.level);
      return { b, level, i: Number(b.dataset.i) };
    };
    root.addEventListener('mouseover', (e) => {
      const t = target(e);
      if (!t) return;
      clearTimeout(hoverTimer);
      setActive(t.level, t.i);
      const it = panels[t.level].items[t.i];
      // másik elem fölött: a mélyebb almenük bezárulnak (kis késéssel, ahogy a Windowsban)
      hoverTimer = setTimeout(() => {
        if (!panels[t.level]) return;
        if (it.sub && !it.disabled) openSub(t.level, t.i, false);
        else if (panels.length > t.level + 1) { closeFrom(t.level + 1); sendRects(); }
      }, SUB_DELAY);
    });
    root.addEventListener('click', (e) => {
      const t = target(e);
      if (!t) return;
      clearTimeout(hoverTimer);
      choose(t.level, t.i);
    });
    // a jobb gomb is választ, mint a Windows menüiben
    root.addEventListener('contextmenu', (e) => {
      const t = target(e);
      if (t) choose(t.level, t.i);
    });

    // ── billentyűzet ──
    document.addEventListener('keydown', (e) => {
      if (!panels.length) return;
      root.classList.add('kbd');
      const level = panels.length - 1;
      const p = panels[level];
      const k = e.key;
      if (k === 'ArrowDown' || k === 'ArrowUp') {
        e.preventDefault();
        return setActive(level, firstIndex(p.items, k === 'ArrowDown' ? 1 : -1, p.active));
      }
      if (k === 'Home' || k === 'End') {
        e.preventDefault();
        return setActive(level, firstIndex(p.items, k === 'Home' ? 1 : -1, k === 'Home' ? -1 : p.items.length));
      }
      if (k === 'ArrowRight') {
        e.preventDefault();
        const it = p.items[p.active];
        if (it && it.sub) openSub(level, p.active, true);
        return;
      }
      if (k === 'ArrowLeft' || k === 'Escape') {
        e.preventDefault();
        if (level > 0) { closeFrom(level); return sendRects(); }
        if (k === 'Escape') post({ type: 'desk-close', seq });
        return;
      }
      if (k === 'Enter' || k === ' ') {
        e.preventDefault();
        if (p.active >= 0) choose(level, p.active);
        return;
      }
      // gyorsbetű: ha csak egy elemé, azonnal választ; ha többé, sorban kijelöli őket
      if (k.length === 1 && !e.ctrlKey && !e.altKey) {
        const key = k.toLocaleLowerCase();
        const hits = p.items.map((it, i) => (!it.sep && it.mn === key ? i : -1)).filter((i) => i >= 0);
        if (hits.length === 1) choose(level, hits[0]);
        else if (hits.length > 1) setActive(level, hits.find((i) => i > p.active) ?? hits[0]);
      }
    });

    const h = (msg) => {
      if (msg.type === 'desk-open') open(msg);
      if (msg.type === 'desk-sub') updateSub(msg);
      if (msg.type === 'desk-clear') clear();
    };
    h.demo = () => {
      document.documentElement.classList.add('desk-demo');
      open({
        seq: 1, x: 40, y: 30,
        items: [
          { id: 9, label: '&Nézet', sub: [
            { id: 16, label: '&Nagy ikonok' }, { id: 15, label: 'Kö&zepes ikonok', check: 'radio' }, { id: 14, label: 'Kis &ikonok' },
            { sep: true }, { id: 13, label: 'Ikonok &automatikus elrendezése' }, { id: 12, label: 'Ikonok &rácshoz igazítása', check: 'check' },
            { sep: true }, { id: 10, label: 'Asztali &elemek megjelenítése', check: 'check' },
          ] },
          { id: 4, label: '&Rendezés', sub: [{ id: 8, label: '&Név' }, { id: 7, label: 'Méret' }, { id: 6, label: 'Elemtípus' }, { id: 5, label: 'Módosítás dátu&ma' }] },
          { id: 3, label: 'Fris&sítés' },
          { sep: true },
          { id: 2, label: '&Beillesztés', disabled: true },
          { id: 30, label: '&Visszavonás – Törlés', key: 'Ctrl+Z' },
          { sep: true },
          { id: 18, label: 'Ú&j', sub: [{ id: 19, label: '&Mappa' }, { id: 20, label: '&Parancsikon' }, { sep: true }, { id: 27, label: 'Szöveges dokumentum' }] },
          { sep: true },
          { id: 88, label: '&Képernyő-beállítások' },
          { id: 90, label: 'S&zemélyre szabás' },
        ],
      });
    };
    return h;
  }

  window.HUB.parts.desk = buildDesk;
})();
