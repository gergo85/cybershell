(() => {
  const { $, T, root, post, setTheme, svg } = window.HUB;

  // ───────────── Egészség: szünetre figyelmeztető sáv (a munkaterület tetején, középen) ─────────────
  function buildHealth() {
    root.className = 'hb';
    root.innerHTML = `
      <span class="hb-ico">${svg('alert')}</span>
      <div class="hb-txt"><b>${T('hTitle')}</b><span id="hbMsg"></span></div>
      <button class="hb-x" id="hbClose" title="${T('hCloseHint')}">${svg('close')}<span>${T('hClose')}</span></button>`;
    $('hbClose').addEventListener('click', () => post({ type: 'close' }));
    const show = (minutes) => {
      const m = Number(minutes) || 90;
      $('hbMsg').textContent = T('hText', { t: m % 60 ? T('hMin', { m }) : T('hHour', { h: m / 60 }) });
      root.classList.remove('in');
      void root.offsetWidth;
      root.classList.add('in');
    };
    const h = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      if (msg.type === 'health-show') show(msg.minutes);
    };
    h.demo = () => show(90);
    return h;
  }

  window.HUB.parts.health = buildHealth;
})();
