// Indító: a cím "part" paramétere szerint felépíti a felületet (parts/*.js), és fogadja a
// program üzeneteit (window.hub.receive).
(() => {
  const { part, lang, post, demo, setTheme, setGeometry, setAlert, parts: p } = window.HUB;
  const parts = { left: p.left, right: p.right, settings: p.settings, peek: p.peek, fly: p.fly, launcher: p.launcher, health: p.health, desk: p.desk, strip: p.strip };
  const handler = (parts[part] || p.strip)();

  window.hub = {
    receive(msg) {
      // ha a beállított nyelv eltér attól, amivel az oldal betöltődött, átváltunk
      if (msg.language && msg.language !== lang && window.HUB_I18N[msg.language]) {
        const u = new URL(location.href);
        u.searchParams.set('lang', msg.language);
        post({ type: 'log', text: `nyelvváltás: ${lang} → ${msg.language}` });
        location.replace(u.toString());
        return;
      }
      if (msg.type === 'init') { setTheme(msg.theme); setGeometry(msg.geometry); }
      if (msg.type === 'theme') setTheme(msg.theme);
      if (msg.type === 'alert') setAlert(msg.on);
      if (handler) handler(msg);
    },
  };
  if (!demo) post({ type: 'ready' });
  else if (handler && handler.demo) handler.demo();
})();
