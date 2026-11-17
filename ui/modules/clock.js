// Idő: óra és perc nagyban, alatta a perc másodperceit 12 szakaszban mutató sáv.
(() => {
  const { defineModule, $, pad } = window.HUB;

  defineModule({
    id: 'clock',
    title: 'secClock',
    name: 'sClock',
    // kompakt nézet: a másodperc-sáv nélkül
    compact: 'clock',
    className: 'clock',
    html: () => `
      <div class="clock-row"><span id="hh">--</span><span class="colon">:</span><span id="mm">--</span></div>
      <div class="sec-line"><div class="segs" id="segs">${'<i></i>'.repeat(12)}</div><span class="ss" id="ss">--</span></div>`,
    setup() {
      const segs = [...$('segs').children];
      return {
        tick(d) {
          const s = d.getSeconds();
          $('hh').textContent = pad(d.getHours());
          $('mm').textContent = pad(d.getMinutes());
          $('ss').textContent = pad(s);
          // 5 másodpercenként egy szakasz; az éppen futó kiemelve
          const cur = Math.floor(s / 5);
          segs.forEach((el, i) => { el.classList.toggle('on', i < cur); el.classList.toggle('now', i === cur); });
        },
      };
    },
    strings: {
      hu: { secClock: '// IDŐ', sClock: 'Idő' },
      en: { secClock: '// TIME', sClock: 'Time' },
      pl: { secClock: '// CZAS', sClock: 'Czas' },
    },
  });
})();
