// A beállítások Rendszer füle: általános kapcsolók (indulás a Windowszal, súgó szövegek, nyelv),
// a config.json gombjai, a gép alkatrészei és a rendszerterhelés görbéje (a fülsáv alján kicsiben is).
// A be/ki kapcsolókat és a parancsgombokat a keret közös kezelője kezeli (data-toggle, data-bool,
// data-cmd), a fül többi része itt van.
(() => {
  const { $, T, lang, demo, post, svg, esc } = window.HUB;

  // a választható nyelvek, a nevük szerint ábécérendben (a gombon a kód, a súgóban a név:
  // öt teljes név nem fér el a sorban)
  const LANGS = [['de', 'Deutsch'], ['en', 'English'], ['es', 'Español'], ['hu', 'Magyar'], ['pl', 'Polski']].sort((a, b) => a[1].localeCompare(b[1]));
  // a terhelésgörbe hossza: ennyi másodperc
  const HIST = 60;

  window.HUB.settingsPanes.push({
    id: 'system',
    // h: a keret HTML-segédei (toggle: be/ki kapcsoló)
    html: ({ toggle }) => `
      <section class="set-pane pane-system" data-pane="system" role="tabpanel">
        <section class="set-card c-general">
          <h2>${T('generalCard')}</h2>
          <div class="set-grid two">
            <button class="sbtn toggle" data-toggle="autostart">${svg('power')}<span>${T('startWithWindows')}</span><b class="switch">–</b></button>
            ${toggle('tooltips', 'help', T('tooltips'), T('tooltipsHint'))}
            <div class="lang-row wide">${svg('globe')}<span>${T('language')}</span>
              <div class="seg" id="langSeg">${LANGS.map(([k, n]) => `<button data-lang="${k}" title="${n}">${k.toUpperCase()}</button>`).join('')}</div>
            </div>
          </div>
        </section>
        <section class="set-card c-config">
          <h2>${T('configCard')}</h2>
          <div class="set-grid two">
            <button class="sbtn" data-cmd="config">${svg('notes')}<span>${T('configFile')}</span></button>
            <button class="sbtn" data-cmd="reload" title="${T('reloadHint')}">${svg('reload')}<span>${T('reload')}</span></button>
            <button class="sbtn danger" id="resetAll" title="${T('resetAllHint')}">${svg('reload')}<span>${T('resetAll')}</span></button>
            <button class="sbtn danger" data-cmd="exit">${svg('close')}<span>${T('exit')}</span></button>
          </div>
        </section>
        <section class="set-card c-hw">
          <h2>${T('hwCard')}</h2>
          <div class="hw-list" id="hwList"><div class="ed-none">${T('hwLoading')}</div></div>
        </section>
        <section class="set-card load">
          <h2>${T('systemLoad')}</h2>
          <div class="load-pair">
            <div class="meter" id="cpuMeter">
              <div class="meter-head"><span>${T('cpu')}</span><b id="cpuVal">--<small>%</small></b></div>
              <svg class="spark" viewBox="0 0 120 40" preserveAspectRatio="none"><path class="spark-area" id="cpuArea"/><path class="spark-line" id="cpuLine"/></svg>
              <div class="meter-sub"><span id="cpuCores">&nbsp;</span><span id="cpuAvg">&nbsp;</span></div>
            </div>
            <div class="meter mem" id="memMeter">
              <div class="meter-head"><span>${T('memory')}</span><b id="memVal">--<small>%</small></b></div>
              <svg class="spark" viewBox="0 0 120 40" preserveAspectRatio="none"><path class="spark-area" id="memArea"/><path class="spark-line" id="memLine"/></svg>
              <div class="meter-sub"><span id="memSub">&nbsp;</span><span id="memFree">&nbsp;</span></div>
            </div>
          </div>
        </section>
      </section>`,

    // ctx: data() (a programtól kapott legutóbbi állapot), toast(text), railLoad(cpu|mem, %, szint):
    // a terhelés kis kijelzője a fülsáv alján
    setup({ data, toast, railLoad }) {
      // ── rendszerkomponensek (a program háttérben kérdezi le, az első megnyitáskor) ──
      const renderHardware = () => {
        const hw = data().hardware;
        if (!hw) return;
        const join = (...xs) => xs.filter(Boolean).join(' · ');
        const row = (icon, label, value, subs = [], extra = '', cls = '') => `
          <div class="hw-row${cls}">${svg(icon)}<span class="hw-label">${label}</span>
            <div class="hw-val"><b>${esc(value || '—')}</b>${extra}${subs.filter(Boolean).map((x) => `<small>${esc(x)}</small>`).join('')}</div></div>`;
        const num = (v, d = 1) => Number(v).toFixed(d).replace(/\.0+$/, '').replace('.', T('gbSep'));
        // alaplap
        const b = typeof hw.board === 'object' && hw.board ? hw.board : { name: hw.board };
        const boardSub = join(b.bios && `BIOS ${b.bios}${b.biosDate ? ` (${b.biosDate})` : ''}`);
        // processzor
        const c = hw.cpu || {};
        const cpuSub = c.cores ? T('hwCores', { c: c.cores, t: c.threads || c.cores, g: c.mhz ? num(c.mhz / 1000) : '?' }) : '';
        // memória szövegesen: azonos moduloknál „4 × 8 GB”, eltérőknél csoportonként
        // („2 × 8 GB + 1 × 4 GB”), alatta csoportonként a gyártó, a típusszám és a sebesség
        const mem = hw.mem || {};
        const mods = mem.modules || [];
        const groups = [];
        for (const m of mods) {
          const key = [m.size, m.type, m.speed, m.maker, m.part].join('|');
          const g = groups.find((x) => x.key === key);
          if (g) g.n++; else groups.push({ key, n: 1, m });
        }
        const speeds = new Set(mods.map((m) => m.speed));
        const empty = Math.max(0, (mem.slots || 0) - mods.length);
        const sizes = groups.map((g) => `${g.n} × ${g.m.size}`).join(' + ');
        const memMain = join(mem.total, sizes, mem.type || (mods.length ? T('hwMixedType') : ''), mem.speed ? `${mem.speed} MT/s` : '');
        const memSubs = groups.map((g) => join(groups.length > 1 ? `${g.n} × ${g.m.size}` : '', g.m.maker, g.m.part,
          speeds.size > 1 && g.m.speed ? `${g.m.speed} MT/s` : ''));
        if (empty) memSubs.push(T('hwEmptySlots', { n: empty }));
        // videókártya
        const gpus = hw.gpus || [];
        const gpuRow = (g, i) => row('gpu', i ? '' : T('hwGpu'), g.name, [join(
          join(g.vram, g.memType).replace(' · ', ' '),
          g.bus && `${g.bus} bit`,
          g.gbps && `${num(g.gbps)} Gbps`)]);
        // háttértárak: meghajtónként a típus, a méret és a rajta lévő meghajtók szabad helye
        const disks = hw.disks || [];
        const lvl = (p) => (p >= 90 ? ' hot' : p >= 75 ? ' warn' : '');
        const diskRows = disks.flatMap((d) => {
          const vols = d.volumes && d.volumes.length ? d.volumes : [null];
          return vols.map((v, i) => `
            <span class="hd-name"${i ? '' : ` title="${esc(d.name)}"`}>${i ? '' : esc(d.name)}</span>
            <span>${i ? '' : esc(d.bus || '—')}</span><span>${i ? '' : esc(d.media || '—')}</span><span class="hd-num">${i ? '' : esc(d.size)}</span>
            <span class="hd-vol">${v ? esc(v.letter) : '—'}${v && v.label ? ` <em>${esc(v.label)}</em>` : ''}</span>
            <span class="hd-num">${v ? `${esc(v.free)} / ${esc(v.total)}` : ''}</span>
            <span class="hd-use${v && typeof v.pct === 'number' ? lvl(v.pct) : ''}">${v && typeof v.pct === 'number' ? `<i><b style="width:${v.pct}%"></b></i><em>${v.pct}%</em>` : ''}</span>`);
        }).join('');
        const diskTable = disks.length ? `<div class="hd-table">
            <span class="hd-h">${T('hdName')}</span><span class="hd-h">${T('hdBus')}</span><span class="hd-h">${T('hdType')}</span><span class="hd-h hd-num">${T('hdSize')}</span>
            <span class="hd-h">${T('hdVol')}</span><span class="hd-h hd-num">${T('hdFree')}</span><span class="hd-h">${T('hdUse')}</span>${diskRows}</div>` : '';
        $('hwList').innerHTML =
          row('board', T('hwBoard'), b.name, [boardSub]) +
          row('chip', T('hwCpu'), c.name, [cpuSub]) +
          row('ram', T('hwMem'), memMain, memSubs) +
          (gpus.length ? gpus.map(gpuRow).join('') : row('gpu', T('hwGpu'), '')) +
          (disks.length ? `<div class="hw-row wide">${svg('disk')}<span class="hw-label">${T('hwStorage')}</span>${diskTable}</div>` : '');
      };

      // a felület nyelve (váltáskor a program minden felületet újratölt az új nyelvvel)
      const renderLang = () => {
        for (const b of $('langSeg').children) b.classList.toggle('on', b.dataset.lang === (data().language || lang));
      };
      $('langSeg').addEventListener('click', (e) => {
        const b = e.target.closest('[data-lang]');
        if (!b || !data() || b.dataset.lang === (data().language || lang)) return;
        if (demo) { location.search = `?part=settings&lang=${b.dataset.lang}`; return; }
        post({ type: 'language', lang: b.dataset.lang });
      });

      // Minden alaphelyzetbe: második kattintásra
      let resetAllArm = 0;
      const disarmResetAll = () => {
        clearTimeout(resetAllArm);
        resetAllArm = 0;
        $('resetAll').classList.remove('armed');
        $('resetAll').querySelector('span').textContent = T('resetAll');
      };
      $('resetAll').addEventListener('click', () => {
        if (!resetAllArm) {
          $('resetAll').classList.add('armed');
          $('resetAll').querySelector('span').textContent = T('resetAllSure');
          resetAllArm = setTimeout(disarmResetAll, 4000);
          return;
        }
        disarmResetAll();
        if (demo) return toast('Demo: reset all');
        post({ type: 'cmd', name: 'reset-all' });
      });

      // ── rendszerterhelés: az utolsó 60 másodperc görbéje (és kicsiben a fülsáv alján) ──
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
          railLoad('cpu', s.cpu, level(s.cpu));
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
          $('memMeter').className = `meter mem ${level(pct)}`;
          $('memSub').textContent = `${gb(s.memUsed)} / ${gb(s.memTotal)} GB`;
          $('memFree').textContent = T('free', { v: gb(s.memTotal - s.memUsed) });
          railLoad('mem', pct, level(pct));
        }
      };

      // előnézet: kitalált terhelés (véletlen bolyongás)
      const demoLoad = () => {
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

      return {
        render() { renderLang(); renderHardware(); },
        message(msg) { if (msg.type === 'sys') showSys(msg); },
        demo: demoLoad,
      };
    },
  });
})();
