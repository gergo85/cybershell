// Eszköztár: Számológép, Jegyzettömb, Keresés (a CyberShell keresője) és Képmetsző, 2×2 ikon.
// A gombok sorrendje a beállításokban húzással állítható (config.json: sections.toolsOrder).
(() => {
  const { defineModule, T, svg } = window.HUB;

  // a gombok: a Keresés a CyberShell parancsa (cmd), a többi a Windows saját eszköze (tool)
  const BUTTONS = [
    { key: 'calc', icon: 'calc', title: 'toolCalc', color: '#ffb02e', tool: 'calc' },
    { key: 'notepad', icon: 'notes', title: 'toolNotepad', color: '#5b8def', tool: 'notepad' },
    { key: 'search', icon: 'search', title: 'search', color: '#4fd1c5', cmd: 'search' },
    { key: 'snip', icon: 'snip', title: 'toolSnip', color: '#ff4fa3', tool: 'snip' },
  ];

  defineModule({
    id: 'tools',
    title: 'secTools',
    name: 'sTools',
    className: 'tools',
    // a beállítások Sorrend kártyája ebből dolgozik
    buttons: BUTTONS,
    orderKey: 'toolsOrder',
    html: () => `
      <div class="tool-grid">
        ${BUTTONS.map((b) => `<button class="tool" data-k="${b.key}" ${b.cmd ? `data-cmd="${b.cmd}"` : `data-tool="${b.tool}"`} title="${T(b.title)}" style="--app:${b.color}">${svg(b.icon)}</button>`).join('')}
      </div>`,
    setup(ctx) {
      const grid = ctx.el.querySelector('.tool-grid');
      grid.addEventListener('click', (e) => {
        const b = e.target.closest('.tool');
        if (!b) return;
        const what = b.dataset.cmd || b.dataset.tool;
        if (ctx.demo) return ctx.toast(`Demo: ${what}`);
        // a Keresés ablaka a gomb magasságába nyílik
        if (b.dataset.cmd) ctx.post({ type: 'cmd', name: b.dataset.cmd, along: ctx.along(b) });
        else ctx.post({ type: 'cmd', name: 'tool', tool: b.dataset.tool });
      });
      return {
        sections: (s) => ctx.reorder(grid, s.toolsOrder),
        message(msg) {
          // a Keresés gomb világít, amíg a kereső nyitva van
          if (msg.type === 'launcher-state') {
            for (const b of grid.querySelectorAll('[data-cmd]')) b.classList.toggle('open', !!msg.open && b.dataset.cmd === msg.mode);
          }
        },
      };
    },
    strings: {
      hu: { secTools: '// ESZKÖZTÁR', sTools: 'Eszköztár', toolCalc: 'Számológép', toolNotepad: 'Jegyzettömb', toolSnip: 'Képmetsző' },
      en: { secTools: '// TOOLBAR', sTools: 'Toolbar', toolCalc: 'Calculator', toolNotepad: 'Notepad', toolSnip: 'Snipping Tool' },
      pl: { secTools: '// NARZĘDZIA', sTools: 'Narzędzia', toolCalc: 'Kalkulator', toolNotepad: 'Notatnik', toolSnip: 'Wycinanie' },
      de: { secTools: '// WERKZEUGE', sTools: 'Werkzeuge', toolCalc: 'Rechner', toolNotepad: 'Editor', toolSnip: 'Snipping Tool' },
      es: { secTools: '// ÚTILES', sTools: 'Útiles', toolCalc: 'Calculadora', toolNotepad: 'Bloc de notas', toolSnip: 'Recortes' },
    },
  });
})();
