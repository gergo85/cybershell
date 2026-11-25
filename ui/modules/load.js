// Terhelés: processzor, memória, videókártya (terhelés és hőmérséklet). Az adatot a program
// rendszerfigyelője küldi kétmásodpercenként (src/monitor.rs, „load” üzenet). Kattintásra a
// Feladatkezelő nyílik meg.
(() => {
  const { defineModule, $, T } = window.HUB;

  defineModule({
    id: 'load',
    title: 'secLoad',
    name: 'sLoad',
    hint: 'sLoadHint',
    defaultOn: false,
    className: 'ldmod',
    tip: 'loadTip',
    html: () => `
      <div class="ld-rows">
        ${['cpu', 'mem', 'gpu'].map((k) => `
          <div class="ld-row" id="ld-${k}">
            <div class="ld-head"><span>${T(`ld_${k}`)}</span><b>--</b></div>
            <div class="ld-bar"><i></i></div>
          </div>`).join('')}
        <div class="ld-row ld-temp" id="ld-temp"><div class="ld-head"><span>${T('ld_temp')}</span><b>--</b></div></div>
      </div>`,
    setup(ctx) {
      const sec = ctx.el;
      const level = (pct) => (pct >= 90 ? 'hot' : pct >= 70 ? 'warn' : '');
      const row = (k, pct, text) => {
        const el = $(`ld-${k}`);
        el.style.display = pct == null ? 'none' : '';
        if (pct == null) return;
        el.querySelector('b').textContent = text;
        el.className = `ld-row ${level(pct)}`;
        const bar = el.querySelector('.ld-bar i');
        if (bar) bar.style.width = `${Math.max(0, Math.min(100, pct))}%`;
      };
      const show = (m) => {
        row('cpu', m.cpu, `${m.cpu}%`);
        row('mem', m.mem, `${m.mem}%`);
        row('gpu', m.gpu, `${m.gpu}%`);
        const t = $('ld-temp');
        t.style.display = m.gpuTemp == null ? 'none' : '';
        if (m.gpuTemp != null) {
          t.querySelector('b').textContent = `${m.gpuTemp}°C`;
          t.className = `ld-row ld-temp ${m.gpuTemp >= 85 ? 'hot' : m.gpuTemp >= 75 ? 'warn' : ''}`;
        }
        // a memória gigabájtban a súgóban
        if (m.memTotal) $('ld-mem').title = `${(m.memUsed / 2 ** 30).toFixed(1)} / ${(m.memTotal / 2 ** 30).toFixed(1)} GB`;
      };
      sec.addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: taskmgr');
        ctx.post({ type: 'cmd', name: 'taskmgr' });
      });
      return {
        message: (msg) => { if (msg.type === 'load') show(msg); },
        // előnézetben mintaadat, kétmásodpercenként
        demo() {
          let n = 0;
          const fake = () => {
            n++;
            show({ cpu: Math.round(18 + 14 * Math.sin(n / 3) + Math.random() * 8), mem: 61, memUsed: 20.9 * 2 ** 30, memTotal: 32 * 2 ** 30,
              gpu: Math.round(8 + 6 * Math.sin(n / 5) + Math.random() * 4), gpuTemp: 52 });
          };
          fake();
          setInterval(fake, 2000);
        },
      };
    },
    strings: {
      hu: {
        secLoad: '// TERHELÉS', sLoad: 'Terhelés', sLoadHint: 'Processzor, memória, videókártya és a videókártya hőmérséklete',
        loadTip: 'Kattints a Feladatkezelőhöz', ld_cpu: 'CPU', ld_mem: 'MEM', ld_gpu: 'GPU', ld_temp: 'GPU °C',
      },
      en: {
        secLoad: '// LOAD', sLoad: 'Load', sLoadHint: 'Processor, memory, graphics card and its temperature',
        loadTip: 'Click for Task Manager', ld_cpu: 'CPU', ld_mem: 'MEM', ld_gpu: 'GPU', ld_temp: 'GPU °C',
      },
      pl: {
        secLoad: '// OBCIĄŻENIE', sLoad: 'Obciążenie', sLoadHint: 'Procesor, pamięć, karta graficzna i jej temperatura',
        loadTip: 'Kliknij, aby otworzyć Menedżera zadań', ld_cpu: 'CPU', ld_mem: 'RAM', ld_gpu: 'GPU', ld_temp: 'GPU °C',
      },
      de: {
        secLoad: '// LAST', sLoad: 'Last', sLoadHint: 'Prozessor, Arbeitsspeicher, Grafikkarte und ihre Temperatur',
        loadTip: 'Klicken für den Task-Manager', ld_cpu: 'CPU', ld_mem: 'RAM', ld_gpu: 'GPU', ld_temp: 'GPU °C',
      },
      es: {
        secLoad: '// CARGA', sLoad: 'Carga', sLoadHint: 'Procesador, memoria, tarjeta gráfica y su temperatura',
        loadTip: 'Haz clic para abrir el Administrador de tareas', ld_cpu: 'CPU', ld_mem: 'RAM', ld_gpu: 'GPU', ld_temp: 'GPU °C',
      },
    },
  });
})();
