// Laptop: Wi-Fi, akkumulátor, hálózat és Bluetooth (2×2 gomb, mint a Főkapcsolóban; a Windows
// saját felületét nyitják meg; az akkumulátor gombján a töltöttség százalékban). Csak
// akkumulátoros gépen látszik (laptopOnly), és csak ott szerepel a beállítások Modulok listájában is.
(() => {
  const { defineModule, $, T, svg } = window.HUB;

  // a gombok (title: a súgó, name: a név a beállítások Sorrend kártyáján; a sorrendjük a beállításokban húzással állítható: sections.laptopOrder);
  // az akkumulátor gombján ikon helyett a töltöttség látszik
  const BUTTONS = [
    { key: 'wifi', icon: 'wifi', title: 'wifi', name: 'wifi', color: '#4fd1c5', id: 'lapWifi' },
    { key: 'battery', icon: 'battery', title: 'batteryT', name: 'batteryT', color: '#7ee787', id: 'lapBat' },
    { key: 'network', icon: 'network', title: 'networkHint', name: 'network', color: '#6ea8ff', id: 'lapNet' },
    { key: 'bluetooth', icon: 'bluetooth', title: 'bluetoothHint', name: 'bluetooth', color: '#8b8cff', id: 'lapBt' },
  ];

  defineModule({
    id: 'laptop',
    title: 'secLaptop',
    name: 'sLaptop',
    laptopOnly: true,
    className: 'laptop',
    buttons: BUTTONS,
    orderKey: 'laptopOrder',
    html: () => `
      <div class="lap-grid">
        ${BUTTONS.map((b) => `<button class="lap" id="${b.id}" data-k="${b.key}" data-cmd="${b.key}" style="--app:${b.color}" title="${T(b.title)}">${
          b.key === 'battery' ? '<span class="bat-pct" id="batPct">--%</span>' : svg(b.icon)}</button>`).join('')}
      </div>`,
    setup(ctx) {
      // amíg a program nem jelzi, hogy van akkumulátor, rejtve marad
      ctx.setAvailable(false);
      const grid = ctx.el.querySelector('.lap-grid');
      grid.addEventListener('click', (e) => {
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
        $('batPct').textContent = pct == null ? '--%' : `${pct}%`;
        const lap = $('lapBat');
        lap.title = `${T('batteryT')}${pct == null ? '' : `: ${pct}%`}${m.charging ? ` (${T('batCharging')})` : ''}`;
        lap.classList.toggle('charging', !!m.charging);
        // alacsony töltöttség: 30% alatt sárga, 15% alatt piros (töltés közben nem)
        lap.classList.toggle('warn', pct != null && pct <= 30 && pct > 15 && !m.charging);
        lap.classList.toggle('low', pct != null && pct <= 15 && !m.charging);
      };
      return {
        message,
        sections: (s) => ctx.reorder(grid, s.laptopOrder),
        demo: () => message({ type: 'power', laptop: true, pct: 76, charging: true }),
      };
    },
    strings: {
      hu: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', batteryT: 'Akkumulátor', batCharging: 'töltődik',
        networkHint: 'Hálózat és internet (Windows beállítások)', bluetoothHint: 'Bluetooth és eszközök (Windows beállítások)',
      },
      en: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', batteryT: 'Battery', batCharging: 'charging',
        networkHint: 'Network & internet (Windows Settings)', bluetoothHint: 'Bluetooth & devices (Windows Settings)',
      },
      pl: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', batteryT: 'Bateria', batCharging: 'ładowanie',
        networkHint: 'Sieć i internet (ustawienia Windows)', bluetoothHint: 'Bluetooth i urządzenia (ustawienia Windows)',
      },
      de: {
        secLaptop: '// LAPTOP', sLaptop: 'Laptop', batteryT: 'Akku', batCharging: 'lädt',
        networkHint: 'Netzwerk und Internet (Windows-Einstellungen)', bluetoothHint: 'Bluetooth und Geräte (Windows-Einstellungen)',
      },
      es: {
        secLaptop: '// PORTÁTIL', sLaptop: 'Portátil', batteryT: 'Batería', batCharging: 'cargando',
        networkHint: 'Red e Internet (Configuración de Windows)', bluetoothHint: 'Bluetooth y dispositivos (Configuración de Windows)',
      },
    },
  });
})();
