// Hálózat: a letöltés és a feltöltés sebessége (a hálózati kártyák forgalma) és a válaszidő
// (ping, a config.json network.pingHost címére), alatta az elmúlt perc grafikonja. Az adatot a
// program rendszerfigyelője küldi (src/monitor.rs, „net” üzenet). Kattintásra a Windows
// hálózati beállításai nyílnak meg. Kompakt nézetben grafikon nélkül.
(() => {
  const { defineModule, $, T, sparkPath } = window.HUB;
  const HISTORY = 30;

  // bájt/másodperc olvasható alakban (pl. 340 KB/s, 12,4 MB/s)
  const rate = (b) => {
    if (b == null) return '--';
    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    let i = 0;
    while (b >= 1000 && i < units.length - 1) { b /= 1024; i++; }
    const n = b >= 10 || i === 0 ? Math.round(b) : Math.round(b * 10) / 10;
    return `${String(n).replace('.', T('decSep'))} ${units[i]}`;
  };

  defineModule({
    id: 'network',
    title: 'secNet',
    name: 'sNet',
    hint: 'sNetHint',
    compact: 'network',
    className: 'netmod',
    tip: 'netTip',
    html: () => `
      <div class="nm-rows">
        <div class="nm-row nm-down"><i>↓</i><b id="nmDown">--</b></div>
        <div class="nm-row nm-up"><i>↑</i><b id="nmUp">--</b></div>
        <div class="nm-row nm-ping" id="nmPingRow"><span>PING</span><b id="nmPing">--</b></div>
      </div>
      <svg class="nm-spark" viewBox="0 0 100 24" preserveAspectRatio="none"><path class="nm-up-line" d=""/><path class="nm-down-line" d=""/></svg>`,
    setup(ctx) {
      const sec = ctx.el;
      const hist = { down: [], up: [] };
      const show = (m) => {
        $('nmDown').textContent = rate(m.down);
        $('nmUp').textContent = rate(m.up);
        const ping = $('nmPingRow');
        $('nmPing').textContent = m.ping == null ? T('nmNoReply') : `${m.ping} ms`;
        ping.className = `nm-row nm-ping ${m.ping == null ? 'hot' : m.ping >= 150 ? 'hot' : m.ping >= 60 ? 'warn' : ''}`;
        ping.title = m.host ? T('nmPingTo', { h: m.host }) : '';
        for (const k of ['down', 'up']) {
          if (m[k] == null) continue;
          hist[k].push(m[k]);
          if (hist[k].length > HISTORY) hist[k].shift();
        }
        // közös skála: a perc legnagyobb forgalma (legalább 64 KB/s, hogy a csend ne ugráljon)
        const max = Math.max(65536, ...hist.down, ...hist.up);
        sec.querySelector('.nm-down-line').setAttribute('d', sparkPath(hist.down, 100, 24, 0, max));
        sec.querySelector('.nm-up-line').setAttribute('d', sparkPath(hist.up, 100, 24, 0, max));
      };
      sec.addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: network');
        ctx.post({ type: 'cmd', name: 'network' });
      });
      return {
        message: (msg) => { if (msg.type === 'net') show(msg); },
        demo() {
          let n = 0;
          const fake = () => {
            n++;
            show({ down: Math.round(2e6 + 1.6e6 * Math.sin(n / 2) + Math.random() * 5e5), up: Math.round(1.2e5 + Math.random() * 8e4), ping: 12 + Math.round(Math.random() * 6), host: '1.1.1.1' });
          };
          for (let i = 0; i < HISTORY; i++) fake();
          setInterval(fake, 2000);
        },
      };
    },
    strings: {
      hu: {
        secNet: '// HÁLÓZAT', sNet: 'Hálózat', sNetHint: 'Letöltés, feltöltés és a válaszidő (ping); a ping célja a config.json network.pingHost értéke',
        netTip: 'Kattints a hálózati beállításokhoz', nmNoReply: 'NINCS', nmPingTo: 'Válaszidő: {h}', decSep: ',',
      },
      en: {
        secNet: '// NETWORK', sNet: 'Network', sNetHint: 'Download, upload and response time (ping); the ping target is network.pingHost in config.json',
        netTip: 'Click for network settings', nmNoReply: 'NONE', nmPingTo: 'Response time: {h}', decSep: '.',
      },
      pl: {
        secNet: '// SIEĆ', sNet: 'Sieć', sNetHint: 'Pobieranie, wysyłanie i czas odpowiedzi (ping); cel pingu to network.pingHost w config.json',
        netTip: 'Kliknij, aby otworzyć ustawienia sieci', nmNoReply: 'BRAK', nmPingTo: 'Czas odpowiedzi: {h}', decSep: ',',
      },
    },
  });
})();
