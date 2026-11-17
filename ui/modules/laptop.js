// Laptop: Wi-Fi, akkumulátor, hálózat és Bluetooth (2×2 gomb, a Windows saját felületét
// nyitják meg). Csak akkumulátoros gépen látszik (laptopOnly), és csak ott szerepel a
// beállítások Modulok listájában is.
(() => {
  const { defineModule, $, T, svg } = window.HUB;

  // az akkumulátor ikonja a töltöttséggel (és töltés közben villámmal)
  const batSvg = (pct, charging) => {
    const w = Math.max(0, Math.min(100, pct == null ? 0 : pct)) / 100 * 13;
    return `<svg viewBox="0 0 24 24"><path d="M3 8h16v8H3z"/><path d="M21 10.5v3" stroke-width="2"/>` +
      `<rect class="bat-fill" x="5" y="10" width="${w.toFixed(1)}" height="4"/>` +
      (charging ? '<path class="bat-bolt" d="M12.5 6.5l-3 5.5h4l-3 5.5"/>' : '') + '</svg>';
  };

  defineModule({
    id: 'laptop',
    title: 'secLaptop',
    name: 'sLaptop',
    laptopOnly: true,
    className: 'laptop',
    html: () => `
      <div class="lap-grid">
        <button class="lap" id="lapWifi" data-cmd="wifi" style="--app:#4fd1c5">${svg('wifi')}<span>${T('wifi')}</span></button>
        <button class="lap" id="lapBat" data-cmd="battery" style="--app:#7ee787"><span class="bat" id="batIcon"></span><span id="batPct">--%</span></button>
        <button class="lap" id="lapNet" data-cmd="network" style="--app:#6ea8ff" title="${T('networkHint')}">${svg('network')}<span>${T('network')}</span></button>
        <button class="lap" id="lapBt" data-cmd="bluetooth" style="--app:#8b8cff" title="${T('bluetoothHint')}">${svg('bluetooth')}<span>${T('btShort')}</span></button>
      </div>`,
    setup(ctx) {
      // amíg a program nem jelzi, hogy van akkumulátor, rejtve marad
      ctx.setAvailable(false);
      ctx.el.querySelector('.lap-grid').addEventListener('click', (e) => {
        const b = e.target.closest('.lap');
        if (!b) return;
        if (ctx.demo) return ctx.toast(`Demo: ${b.dataset.cmd}`);
        ctx.post({ type: 'cmd', name: b.dataset.cmd });
      });
      const showOnline = () => {
        for (const id of ['lapWifi', 'lapNet']) $(id).classList.toggle('offline', !navigator.onLine);
      };
      window.addEventListener('online', showOnline);
      window.addEventListener('offline', showOnline);
      showOnline();
      const message = (m) => {
        if (m.type !== 'power') return;
        ctx.setAvailable(m.laptop);
        if (!m.laptop) return;
        const pct = typeof m.pct === 'number' ? m.pct : null;
        $('batIcon').innerHTML = batSvg(pct, m.charging);
        $('batPct').textContent = pct == null ? '--%' : `${pct}%`;
        const lap = $('lapBat');
        lap.classList.toggle('charging', !!m.charging);
        lap.classList.toggle('low', pct != null && pct <= 20 && !m.charging);
      };
      return {
        message,
        demo: () => message({ type: 'power', laptop: true, pct: 76, charging: true }),
      };
    },
    strings: {
      hu: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', btShort: 'BT',
        networkHint: 'Hálózat és internet (Windows beállítások)', bluetoothHint: 'Bluetooth és eszközök (Windows beállítások)',
      },
      en: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', btShort: 'BT',
        networkHint: 'Network & internet (Windows Settings)', bluetoothHint: 'Bluetooth & devices (Windows Settings)',
      },
      pl: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', btShort: 'BT',
        networkHint: 'Sieć i internet (ustawienia Windows)', bluetoothHint: 'Bluetooth i urządzenia (ustawienia Windows)',
      },
    },
  });
})();
