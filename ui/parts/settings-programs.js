// A beállítások Programok füle: az ikonsáv programjainak szerkesztője. A változások egy
// vázlatban gyűlnek, és csak a Mentés gombbal lépnek életbe (a keret a bezárás előtt rákérdez).
(() => {
  const { $, T, demo, post, svg, esc } = window.HUB;

  // az ikonválasztó mátrixa (a program saját ikonja után), témák szerint csoportosítva
  const ICON_CHOICES = [
    'browser', 'globe', 'link', 'code', 'terminal', 'bug', 'robot', 'database', 'folder', 'archive', 'panels', 'grid', 'list', 'layers',
    'file', 'doc', 'sheet', 'text', 'notes', 'book', 'news', 'bookmark', 'clipboard', 'calc', 'pie', 'trend', 'translate',
    'mail', 'inbox', 'send', 'chat', 'bell', 'calendar', 'clock', 'timer', 'music', 'headphones', 'mic', 'podcast', 'video', 'film',
    'photo', 'camera', 'design', 'palette', 'brush', 'pen', 'scissors', 'enhance', 'picker', 'snip', 'cube', 'game', 'dice', 'trophy',
    'crown', 'ghost', 'smile', 'rocket', 'spark', 'star', 'heart', 'fire', 'bolt', 'bulb', 'leaf', 'coffee', 'gift', 'cart', 'tag',
    'coin', 'wallet', 'bank', 'briefcase', 'school', 'bar', 'map', 'place', 'pin', 'flag', 'compass', 'target', 'home', 'car', 'plane',
    'anchor', 'planet', 'cloud', 'download', 'server', 'pc', 'laptop', 'monitor', 'phone', 'keyboard', 'mouse', 'chip', 'gpu', 'ram',
    'disk', 'board', 'plug', 'network', 'wifi', 'bluetooth', 'printer', 'qr', 'shield', 'lock', 'key', 'eye', 'box', 'user', 'fitness',
    'settings', 'wrench', 'search', 'hash', 'start', 'github', 'sun', 'moon', 'app'];

  window.HUB.settingsPanes.push({
    id: 'programs',
    html: () => `
      <section class="set-pane" data-pane="programs" role="tabpanel">
        <main class="set-card set-main">
          <h2>${T('icons')}</h2>
          <div class="ed-head"><span>${T('colIcon')}</span><span>${T('colName')}</span><span>${T('colProgram')}</span><span>${T('colColor')}</span><span>${T('colAuto')}</span><span></span></div>
          <div class="ed-list" id="edList"></div>
          <div class="ed-add">
            <button class="sbtn" id="edBrowse">${svg('plus')}<span>${T('addProgram')}</span></button>
            <div class="ed-chips" id="edRunning" title="${esc(T('runningTitle').replace(/^\/\/\s*/, ''))}"></div>
            <footer class="ed-foot">
              <span class="ed-dirty" id="edDirty"></span>
              <button class="sbtn" id="edCancel">${svg('close')}<span>${T('discard')}</span></button>
              <button class="sbtn primary" id="edSave">${svg('check')}<span>${T('save')}</span></button>
            </footer>
          </div>
        </main>
      </section>`,

    // ctx: root, data() (a programtól kapott legutóbbi állapot), toast(text), sortable(list, rowSel, done),
    // setBadge(fül, be, súgó): jelzés a fülsávon
    setup({ root, data, toast, sortable, setBadge }) {
      let draft = [];       // a szerkesztett ikonlista
      let original = '[]';  // a legutóbb mentett (vagy kapott) lista, összehasonlításhoz

      const editable = (a) => ({
        id: a.id, name: a.name, path: a.path, args: a.args || '', icon: a.rawIcon != null ? a.rawIcon : (a.icon || ''),
        color: a.color || '', img: a.img || '', exeImg: a.exeImg || (a.icon === 'exe' ? a.img : '') || '', canExeIcon: !!a.canExeIcon,
        available: a.available !== false, hint: a.hint || '', autostart: !!a.autostart,
      });
      const strip = (a) => ({ id: a.id, name: a.name, path: a.path, args: a.args, icon: a.icon, color: a.color, autostart: a.autostart });
      const dirty = () => JSON.stringify(draft.map(strip)) !== original;
      // a vázlat a programtól kapott listából (a mentetlen változások elvesznek)
      const reset = () => {
        draft = (data().apps || []).map(editable);
        original = JSON.stringify(draft.map(strip));
        renderList();
      };

      const iconPreview = (a) => {
        if (a.icon === 'exe' && a.img) return `<img src="${esc(a.img)}" alt="">`;
        if (a.icon && window.HUB_ICONS[a.icon]) return svg(a.icon);
        const hay = `${a.name} ${a.path}`.toLowerCase();
        const hit = (window.HUB_GUESS || []).find(([re]) => re.test(hay));
        return svg(hit ? hit[1] : 'app');
      };

      // futó, de még fel nem vett programok: egy kattintással a listára kerülnek
      const renderRunning = () => {
        const have = new Set(draft.map((a) => a.path.toLowerCase()));
        const list = (data().running || []).filter((r) => !have.has(r.path.toLowerCase()));
        $('edRunning').innerHTML = list.length
          ? list.map((r, i) => `<button class="run-chip" data-run="${i}" title="${esc(T('runningTitle').replace(/^\/\/\s*/, ''))}"><img src="${esc(r.img)}" alt=""><span>${esc(r.name)}</span>${svg('plus')}</button>`).join('')
          : `<span class="ed-none">${T('noRunning')}</span>`;
        $('edRunning').dataset.list = JSON.stringify(list);
      };

      const renderList = () => {
        $('edList').innerHTML = draft.map((a, i) => {
          const color = /^#[0-9a-f]{6}$/i.test(a.color) ? a.color : '#8a8f9c';
          return `
            <div class="ed-item" data-i="${i}">
            <span class="grip" title="${T('dragHint')}">${svg('grip')}</span>
            <div class="ed-row${a.available ? '' : ' missing'}" data-i="${i}">
              <button class="ed-icon ed-pick" data-act="pick" title="${T('pickIcon')}: ${pickLabel(a)}"${a.color ? ` style="--app:${esc(a.color)}"` : ''}>${iconPreview(a)}</button>
              <input class="ed-name" data-field="name" value="${esc(a.name)}" maxlength="24" spellcheck="false">
              <div class="ed-path">${esc(a.path)}${a.available ? '' : `<b>${T('notInstalled')}</b>`}</div>
              <input class="ed-color" type="color" data-field="color" value="${color}">
              <button class="ib auto${a.autostart ? ' on' : ''}" data-act="auto">${svg('power')}</button>
              <button class="ib danger" data-act="del">${svg('trash')}</button>
            </div>
            </div>`;
        }).join('') || `<div class="ed-none">${T('noIcons')}</div>`;
        renderRunning();
        updateDirty();
      };

      // ── ikonválasztó: az ikonra kattintva nyíló mátrix, az első elem a program saját ikonja (ha nincs neki: automatikus) ──
      const pickLabel = (a) => T(a.icon === 'exe' ? 'programIcon' : !a.icon ? 'automatic' : window.HUB_ICONS[a.icon] ? 'builtinIcon' : 'imageIcon');
      const icoPop = document.createElement('div');
      icoPop.className = 'ico-pop';
      icoPop.hidden = true;
      root.appendChild(icoPop);
      let picking = -1;
      const closePicker = () => {
        icoPop.hidden = true;
        picking = -1;
        for (const b of $('edList').querySelectorAll('.ed-pick.open')) b.classList.remove('open');
      };
      const openPicker = (i, btn) => {
        if (picking === i) return closePicker();
        closePicker();
        const a = draft[i];
        const first = a.canExeIcon
          ? { v: 'exe', html: a.exeImg ? `<img src="${esc(a.exeImg)}" alt="">` : svg('app'), title: T('programIcon') }
          : { v: '', html: iconPreview({ ...a, icon: '' }), title: T('automatic') };
        const cells = [first, ...ICON_CHOICES.map((n) => ({ v: n, html: svg(n), title: n }))];
        icoPop.innerHTML = `<div class="ico-head">${esc(a.name)}</div><div class="ico-grid" style="--app:${esc(a.color || 'var(--c-main)')}">${cells.map((c, k) =>
          `<button class="ico-cell${k === 0 ? ' own' : ''}${a.icon === c.v ? ' on' : ''}" data-icon="${c.v}" title="${esc(c.title)}">${c.html}</button>`).join('')}</div>`;
        icoPop.hidden = false;
        picking = i;
        btn.classList.add('open');
        // az ikon alá (ha ott nem fér el, fölé), a képernyőn belül
        const r = btn.getBoundingClientRect(), w = icoPop.offsetWidth, h = icoPop.offsetHeight;
        const left = Math.max(8, Math.min(r.left, innerWidth - w - 8));
        const top = r.bottom + 6 + h <= innerHeight - 8 ? r.bottom + 6 : Math.max(8, r.top - 6 - h);
        Object.assign(icoPop.style, { left: `${left}px`, top: `${top}px` });
      };
      icoPop.addEventListener('click', (e) => {
        const c = e.target.closest('[data-icon]');
        if (!c || picking < 0) return;
        const a = draft[picking];
        a.icon = c.dataset.icon;
        if (a.icon === 'exe') a.img = a.exeImg;
        closePicker();
        renderList();
      });
      document.addEventListener('pointerdown', (e) => {
        if (!icoPop.hidden && !icoPop.contains(e.target) && !e.target.closest('.ed-pick')) closePicker();
      }, true);
      document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !icoPop.hidden) { e.stopPropagation(); closePicker(); } }, true);
      window.addEventListener('resize', closePicker);
      $('edList').addEventListener('scroll', closePicker);

      const updateDirty = () => {
        const d = dirty();
        $('edDirty').textContent = d ? T('unsaved') : '';
        $('edSave').disabled = !d;
        $('edCancel').disabled = !d;
        // a Programok fülön jelzés, ha mentetlen változás van
        setBadge('programs', d, d ? T('unsaved') : '');
      };

      // lista szerkesztése
      $('edList').addEventListener('click', (e) => {
        const row = e.target.closest('.ed-row');
        if (!row) return;
        const i = Number(row.dataset.i);
        const b = e.target.closest('button');
        if (!b) return;
        const act = b.dataset.act;
        if (act === 'pick') return openPicker(i, b);
        if (act === 'del') draft.splice(i, 1);
        if (act === 'auto') {
          const a = draft[i];
          a.autostart = !a.autostart;
          toast(a.autostart && !data().autostart ? T('autoHint') : T(a.autostart ? 'autoOn' : 'autoOff', { n: a.name }));
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
          color: '', img: app.img || '', exeImg: app.img || '', canExeIcon: !!app.canExeIcon, available: true, hint: '', autostart: false,
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
      $('edCancel').addEventListener('click', reset);

      return {
        // új adat a programtól: ha épp szerkesztesz, a listádat nem írjuk felül
        load() {
          if (!dirty() || !draft.length) reset();
          else renderRunning();
        },
        message(msg) {
          // a programválasztó (Program hozzáadása) eredménye
          if (msg.type === 'picked' && msg.app) addApp(msg.app);
        },
        dirty,
        discard: reset,
      };
    },
  });
})();
