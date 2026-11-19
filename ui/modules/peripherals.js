// Perifériák: a Bluetooth-eszközök (egér, billentyűzet, fejhallgató…) akkumulátor-töltöttsége.
// A programtól kapja az adatot (periph), és csak akkor látszik, ha van legalább egy eszköz, amely
// jelzi a töltöttségét. 30% alatt sárga, 15% alatt piros (mint a Laptop szekcióban). Csak
// akkumulátoros gépen (laptopon) van rá szükség: asztali gépen rejtve marad, és a beállítások
// Modulok listájában sem szerepel (laptopOnly), a program le sem kérdezi az eszközöket. Laptopon is
// csak akkor szerepel a listában, ha van csatlakoztatott, töltöttséget jelző eszköz (listedIf).
(() => {
  const { defineModule, T, esc } = window.HUB;

  defineModule({
    id: 'peripherals',
    title: 'secPeriph',
    name: 'sPeriph',
    hint: 'sPeriphHint',
    className: 'periph',
    laptopOnly: true,
    // a beállítások Modulok listájában is csak akkor szerepel, ha van töltöttséget jelző eszköz
    listedIf: (data) => (data.periph || 0) > 0,
    html: () => '<div class="pr-list" id="prList"></div>',
    setup(ctx) {
      // amíg a program nem jelzi, hogy van eszköz, rejtve marad
      ctx.setAvailable(false);
      const list = ctx.el.querySelector('#prList');
      let laptop = false;
      let count = 0;

      const render = (devices) => {
        count = devices.length;
        ctx.setAvailable(laptop && count > 0);
        list.innerHTML = devices.map((d) => {
          const pct = Math.max(0, Math.min(100, Math.round(d.pct)));
          const lvl = pct <= 15 ? ' low' : pct <= 30 ? ' warn' : '';
          return `<div class="pr-dev${lvl}" title="${esc(d.name)}: ${pct}%">` +
            `<span class="pr-name">${esc(d.name)}</span>` +
            `<span class="pr-row"><span class="pr-bar"><i style="width:${pct}%"></i></span><b class="pr-pct">${pct}%</b></span></div>`;
        }).join('');
        ctx.fit();
      };

      return {
        message(msg) {
          if (msg.type === 'periph') render(Array.isArray(msg.devices) ? msg.devices : []);
          // akkumulátoros gép-e (a Laptop modul is ebből tudja)
          if (msg.type === 'power') { laptop = !!msg.laptop; ctx.setAvailable(laptop && count > 0); }
        },
        demo() { laptop = true; render([{ name: 'MX Master 3', pct: 76 }, { name: 'MX Keys', pct: 24 }, { name: 'WH-1000XM4', pct: 12 }]); },
      };
    },
    strings: {
      hu: {
        secPeriph: '// PERIFÉRIÁK', sPeriph: 'Perifériák',
        sPeriphHint: 'A Bluetooth-eszközök (egér, billentyűzet, fejhallgató) akkumulátor-töltöttsége. Csak laptopon, és csak akkor látszik, ha van töltöttséget jelző eszköz csatlakoztatva.',
      },
      en: {
        secPeriph: '// DEVICES', sPeriph: 'Peripherals',
        sPeriphHint: 'The battery level of Bluetooth devices (mouse, keyboard, headset). Laptops only, and it only shows when a device that reports its battery is connected.',
      },
      pl: {
        secPeriph: '// URZĄDZENIA', sPeriph: 'Urządzenia',
        sPeriphHint: 'Poziom baterii urządzeń Bluetooth (mysz, klawiatura, słuchawki). Tylko na laptopie i tylko wtedy, gdy podłączone jest urządzenie podające poziom baterii.',
      },
    },
  });
})();
