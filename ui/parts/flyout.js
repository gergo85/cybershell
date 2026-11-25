// Az információs panel melletti oldalpanel kerete. A nézetek külön fájlokban vannak
// (fly-weather.js: részletes időjárás, fly-calendar.js: naptár és Események, fly-note.js:
// Notesz), és a window.HUB.flyViews listába jegyzik be magukat. Itt csak a közös rész van: a
// nézetek összerakása, a váltás köztük (beúszással), a bezárás és a program üzeneteinek szétosztása.
//
// Egy nézet: { html() → a nézet .fview eleme(i), setup(ctx) → { message(msg), flush(), demo(view) } }.
// A ctx: root, show(id) (a megadott .fview látszik), current() (a látható nézet azonosítója).
(() => {
  const { root, post, setTheme } = window.HUB;

  function buildFly() {
    const views = window.HUB.flyViews;
    root.className = 'wfly';
    root.innerHTML = views.map((v) => v.html()).join('');

    let curView = '';   // a látható nézet .fview azonosítója (pl. vWeather)
    let hooks = [];
    // a nézetek még nem mentett változásai (pl. a jegyzet szövege) elmennek
    const flush = () => { for (const k of hooks) if (k.flush) k.flush(); };
    const show = (id) => {
      flush();
      curView = id;
      for (const v of root.querySelectorAll('.fview')) v.classList.toggle('on', v.id === id);
      // minden megnyitáskor beúszik
      root.classList.remove('in');
      void root.offsetWidth;
      root.classList.add('in');
    };
    const ctx = { root, show, current: () => curView };
    hooks = views.map((v) => v.setup(ctx) || {});

    const close = () => { flush(); post({ type: 'close' }); };
    root.addEventListener('click', (e) => { if (e.target.closest('[data-close]')) close(); });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !e.target.closest('input')) close();
    });

    const h = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      for (const k of hooks) if (k.message) k.message(msg);
    };
    // előnézet: a cím view paramétere szerinti nézet (note, calendar, events), alapból az időjárás
    h.demo = () => {
      const view = new URLSearchParams(location.search).get('view');
      if (!hooks.some((k) => k.demo && k.demo(view))) h({ type: 'weather-open', weather: { city: 'Budapest' } });
    };
    return h;
  }

  window.HUB.parts.fly = buildFly;
})();
