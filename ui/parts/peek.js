(() => {
  const { $, T, root, post, setTheme, svg, esc } = window.HUB;

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
      root.className = 'peek';
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

    // jobb kattintás egy ikonon: kis menü a program egyetlen ablakához
    let menuHwnd = 0;
    const renderMenu = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      root.className = 'peek pk-menu';
      menuHwnd = msg.hwnd;
      $('pkHead').innerHTML = `<span class="pk-name">// ${esc(msg.name || '')}</span>`;
      $('pkItems').innerHTML = (msg.items || []).map((it) =>
        `<button class="pm-item${it.danger ? ' danger' : ''}" data-cmd="${esc(it.cmd)}">${svg(it.icon)}<span>${esc(it.label)}</span></button>`).join('');
      post({ type: 'peek-ready', seq: msg.seq });
    };

    const closeCard = (card) => {
      card.classList.add('closing');
      post({ type: 'peek-close', hwnd: Number(card.dataset.hwnd) });
    };
    $('pkItems').addEventListener('click', (e) => {
      const item = e.target.closest('.pm-item');
      if (item) return post({ type: 'menu-pick', cmd: item.dataset.cmd, hwnd: menuHwnd });
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
      if (msg.type === 'menu') renderMenu(msg);
      if (msg.type === 'peek-clear') { $('pkHead').innerHTML = ''; $('pkItems').innerHTML = ''; }
    };
    h.demo = () => {
      const card = (i) => ({ x: 14 + i * 248, y: 40, w: 236, h: 170 });
      const titles = ['Új lap – Mozilla Firefox', 'GitHub · cybershell – Mozilla Firefox', 'Letöltések – Mozilla Firefox'];
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

  window.HUB.parts.peek = buildPeek;
})();
