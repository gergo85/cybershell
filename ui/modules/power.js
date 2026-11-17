// Főkapcsoló: zárolás, alvó állapot, leállítás és újraindítás (2×2 gomb). A leállítás és az
// újraindítás megerősítést kér. A gombok sorrendje a beállításokban húzással állítható
// (config.json: sections.powerOrder).
(() => {
  const { defineModule, $, T, svg } = window.HUB;

  // label: a gomb felirata (vízszintes panelen), title: a teljes név (súgó, beállítások)
  const BUTTONS = [
    { key: 'lock', icon: 'lock', label: 'pwLock', title: 'pwLockT', color: '#4fd1c5' },
    { key: 'sleep', icon: 'moon', label: 'pwSleep', title: 'pwSleepT', color: '#8b6cff' },
    { key: 'shutdown', icon: 'power', label: 'pwOff', title: 'pwOffT', color: '#ff2a6d', confirm: 'pwConfirmOff' },
    { key: 'restart', icon: 'reload', label: 'pwRestart', title: 'pwRestartT', color: '#ffb02e', confirm: 'pwConfirmRestart' },
  ];

  defineModule({
    id: 'power',
    title: 'secPower',
    name: 'sPower',
    className: 'power',
    buttons: BUTTONS,
    orderKey: 'powerOrder',
    html: () => `
      <div class="pw-panes">
        <div class="pw-grid">
          ${BUTTONS.map((b) => `<button class="pw" data-k="${b.key}" data-act="${b.key}" style="--app:${b.color}" title="${T(b.title)}">${svg(b.icon)}<span>${T(b.label)}</span></button>`).join('')}
        </div>
        <div class="pw-confirm" id="pwConfirm">
          <div class="pw-q" id="pwQ"></div>
          <div class="act-btns">
            <button class="ib danger" id="pwYes">${svg('check')}</button>
            <button class="ib" id="pwNo">${svg('close')}</button>
          </div>
        </div>
      </div>`,
    setup(ctx) {
      const sec = ctx.el;
      const grid = sec.querySelector('.pw-grid');
      const run = (act) => {
        if (ctx.demo) return ctx.toast(`Demo: ${act}`);
        ctx.post({ type: 'cmd', name: 'power', action: act });
      };
      // a leállítás és az újraindítás megerősítést kér (8 másodpercig vár)
      let pending = null;
      let timer = 0;
      const closeConfirm = () => {
        pending = null;
        clearTimeout(timer);
        sec.classList.remove('confirming');
      };
      grid.addEventListener('click', (e) => {
        const b = e.target.closest('.pw');
        if (!b) return;
        const def = BUTTONS.find((x) => x.key === b.dataset.act);
        if (!def || !def.confirm) return run(b.dataset.act);
        pending = def.key;
        $('pwQ').textContent = T(def.confirm);
        sec.classList.add('confirming');
        clearTimeout(timer);
        timer = setTimeout(closeConfirm, 8000);
      });
      $('pwYes').addEventListener('click', () => {
        const act = pending;
        closeConfirm();
        if (act) run(act);
      });
      $('pwNo').addEventListener('click', closeConfirm);
      return { sections: (s) => ctx.reorder(grid, s.powerOrder) };
    },
    strings: {
      hu: {
        secPower: '// FŐKAPCSOLÓ', sPower: 'Főkapcsoló', pwLock: 'Zárol', pwOff: 'Leáll', pwRestart: 'Újra',
        pwLockT: 'Zárolás', pwSleepT: 'Alvó állapot', pwOffT: 'Leállítás', pwRestartT: 'Újraindítás',
      },
      en: {
        secPower: '// POWER', sPower: 'Power', pwLock: 'Lock', pwOff: 'Off', pwRestart: 'Reboot',
        pwLockT: 'Lock', pwSleepT: 'Sleep', pwOffT: 'Shut down', pwRestartT: 'Restart',
      },
      pl: {
        secPower: '// ZASILANIE', sPower: 'Zasilanie', pwLock: 'Blokuj', pwOff: 'Wyłącz', pwRestart: 'Restart',
        pwLockT: 'Zablokuj', pwSleepT: 'Uśpij', pwOffT: 'Wyłącz', pwRestartT: 'Uruchom ponownie',
      },
    },
  });
})();
