(() => {
  const { part, side, horiz, root } = window.HUB;

  // ───────────── vékony keretek (felső, alsó) ─────────────
  function buildStrip() {
    // fent és lent vízszintes, oldalt (ha a panelek fent/lent vannak) függőleges keret
    const pos = ['top', 'bottom', 'left', 'right'].includes(side) ? side : part;
    root.className = `strip ${horiz ? pos : `v${pos}`}`;
    const deco = {
      top: ['ticks', 'line', 'sweep', 'notch', 'chip c1', 'chip c2', 'chip c3'],
      bottom: ['ticks', 'line', 'sweep', 'notch', 'hazard-s', 'chip c1', 'chip c3'],
    }[pos === 'top' || pos === 'left' ? 'top' : 'bottom'];
    root.innerHTML = deco.map((c) => `<div class="${c}"></div>`).join('');
    return null;
  }

  window.HUB.parts.strip = buildStrip;
})();
