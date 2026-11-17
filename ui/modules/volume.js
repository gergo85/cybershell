// Hangerő: a Windows alapértelmezett lejátszóeszköze. A csúszka húzható és kattintható, a
// szekció fölött görgetni is lehet; a hangszóró némít, a százalék a Windows hangerőkeverőjét
// nyitja meg. Más modulok a nudge(lépés) függvénnyel állíthatják (pl. a zene görgője).
(() => {
  const { defineModule, $, T, svg } = window.HUB;

  defineModule({
    id: 'volume',
    title: 'secVolume',
    name: 'sVolume',
    sectionId: 'volSec',
    className: 'vol',
    tip: 'volHint',
    ownsWheel: true,
    html: () => `
      <div class="vol-row">
        <button class="vol-mute" id="volMute">${svg('volume')}</button>
        <button class="vol-pct" id="volPct">--</button>
      </div>
      <div class="vol-bar" id="volBar"><span><i id="volFill"></i></span></div>`,
    setup(ctx) {
      const sec = ctx.el;
      // a program „volume” üzenete: { ok, pct, muted }
      let vol = ctx.demo ? { ok: true, pct: 42, muted: false } : { ok: false, pct: 0, muted: false };
      let drag = false, hold = 0, sent = 0;
      const render = () => {
        sec.classList.toggle('off', !vol.ok);
        sec.classList.toggle('muted', vol.ok && vol.muted);
        // a % jel kisebb, háromjegyű értéknél (100) a szám is: így a panel szélén sem lóg ki
        $('volPct').innerHTML = vol.ok ? `${vol.pct}<small>%</small>` : '—';
        $('volPct').classList.toggle('full', vol.ok && vol.pct >= 100);
        $('volFill').style.width = `${vol.ok ? vol.pct : 0}%`;
        $('volMute').innerHTML = svg(vol.ok && (vol.muted || vol.pct === 0) ? 'mute' : 'volume');
        $('volMute').title = T(vol.muted ? 'volUnmute' : 'volMuteT');
      };
      const setVol = (pct) => {
        pct = Math.max(0, Math.min(100, Math.round(pct)));
        if (!vol.ok) return;
        vol.pct = pct; vol.muted = false;
        hold = Date.now() + 800; // a közben érkező régebbi állapot ne ugrassza vissza
        render();
        if (ctx.demo) return;
        // húzás közben nem küldünk minden egyes mozdulatot
        const now = Date.now();
        if (now - sent > 40 || !drag) { sent = now; ctx.post({ type: 'volume-set', pct }); }
      };
      const bar = $('volBar');
      const fromX = (x) => {
        const r = bar.getBoundingClientRect();
        return ((x - r.left) / r.width) * 100;
      };
      bar.addEventListener('pointerdown', (e) => {
        if (!vol.ok) return;
        drag = true;
        bar.setPointerCapture(e.pointerId);
        setVol(fromX(e.clientX));
      });
      bar.addEventListener('pointermove', (e) => { if (drag) setVol(fromX(e.clientX)); });
      const end = (e) => {
        if (!drag) return;
        drag = false;
        setVol(fromX(e.clientX));
        if (!ctx.demo) ctx.post({ type: 'volume-set', pct: vol.pct });
      };
      bar.addEventListener('pointerup', end);
      bar.addEventListener('pointercancel', end);
      sec.addEventListener('wheel', (e) => {
        e.preventDefault();
        setVol(vol.pct + (e.deltaY < 0 ? 2 : -2));
      }, { passive: false });
      $('volMute').addEventListener('click', () => {
        if (!vol.ok) return;
        vol.muted = !vol.muted;
        hold = Date.now() + 800;
        render();
        if (!ctx.demo) ctx.post({ type: 'volume-mute', on: vol.muted });
      });
      $('volPct').addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: volume mixer');
        ctx.post({ type: 'volume-open' });
      });
      render();
      return {
        message(msg) {
          if (msg.type !== 'volume' || drag || Date.now() < hold) return;
          vol = { ok: !!msg.ok, pct: msg.pct | 0, muted: !!msg.muted };
          render();
        },
        api: { nudge: (step) => setVol(vol.pct + step) },
      };
    },
    strings: {
      hu: {
        secVolume: '// HANGERŐ', sVolume: 'Hangerő', volMuteT: 'Némítás', volUnmute: 'Némítás feloldása',
        volHint: 'Görgess a hangerő állításához; a csúszka húzható. A százalékra kattintva a Windows hangerőkeverője nyílik meg.',
      },
      en: {
        secVolume: '// VOLUME', sVolume: 'Volume', volMuteT: 'Mute', volUnmute: 'Unmute',
        volHint: 'Scroll to change the volume; the slider can be dragged. Click the percentage for the Windows volume mixer.',
      },
      pl: {
        secVolume: '// GŁOŚNOŚĆ', sVolume: 'Głośność', volMuteT: 'Wycisz', volUnmute: 'Wyłącz wyciszenie',
        volHint: 'Przewiń, aby zmienić głośność; suwak można przeciągać. Kliknięcie procentów otwiera mikser głośności Windows.',
      },
    },
  });
})();
