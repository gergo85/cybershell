// Zene: a Spotify asztali alkalmazás lejátszása a Windows médiavezérlőjén át (cím, előadó,
// borító, folyamatjelző, keverés, lejátszás, előző, következő). Ha a Spotify nem fut, egy
// indítógomb látszik. A görgő a zene fölött is a hangerőt állítja.
(() => {
  const { defineModule, $, T, svg, pad } = window.HUB;

  const mmss = (sec) => {
    sec = Math.max(0, Math.floor(sec || 0));
    return `${Math.floor(sec / 60)}:${pad(sec % 60)}`;
  };
  // a hosszú cím lassan körbefut
  const marquee = (el, text) => {
    const span = el.firstElementChild;
    if (span.textContent === text) return;
    span.textContent = text;
    el.classList.remove('scroll');
    el.style.removeProperty('--mq');
    requestAnimationFrame(() => {
      const over = span.scrollWidth - el.clientWidth;
      if (over > 2) { el.style.setProperty('--mq', `${-over - 12}px`); el.classList.add('scroll'); }
    });
  };

  defineModule({
    id: 'music',
    title: 'secMusic',
    name: 'sMusic',
    hint: 'sMusicHint',
    className: 'music idle',
    ownsWheel: true,
    html: () => `
      <button class="mu-launch" id="muLaunch" title="${T('muLaunchHint')}">${svg('music')}<span>SPOTIFY</span></button>
      <div class="mu-live">
        <button class="mu-art" id="muArt" title="${T('muOpen')}"><img id="muImg" alt="" draggable="false"></button>
        <i class="mu-eq"><b></b><b></b><b></b></i>
        <div class="mu-title" id="muTitle"><span></span></div>
        <div class="mu-artist" id="muArtist"></div>
        <div class="mu-bar" id="muBar"><span><i id="muFill"></i></span></div>
        <div class="mu-btns">
          <button class="ib mu-shuf" data-mu="shuffle" id="muShuf" title="${T('muShuffle')}">${svg('shuffle')}</button>
          <button class="ib mu-play" data-mu="toggle" id="muToggle">${svg('play')}</button>
          <button class="ib" data-mu="prev" title="${T('muPrev')}">${svg('prev')}</button>
          <button class="ib" data-mu="next" title="${T('muNext')}">${svg('next')}</button>
        </div>
      </div>`,
    setup(ctx) {
      const sec = ctx.el;
      // a program „media” üzenete: { running, title, artist, art, playing, shuffle, canSeek, pos, dur, at }
      let mu = { running: false };
      // a lejátszás helye most (a legutóbbi jelentés óta eltelt idővel)
      const muPos = () => (mu.playing ? (mu.pos || 0) + (Date.now() - (mu.at || Date.now())) / 1000 : (mu.pos || 0));
      const renderTime = () => {
        if (!mu.running) return;
        const dur = mu.dur || 0;
        const pos = dur ? Math.min(dur, muPos()) : 0;
        $('muFill').style.width = dur ? `${(pos / dur) * 100}%` : '0%';
        $('muBar').title = dur ? `${mmss(pos)} / ${mmss(dur)}` : '';
      };
      const render = () => {
        const wasIdle = sec.classList.contains('idle');
        sec.classList.toggle('idle', !mu.running);
        sec.classList.toggle('playing', !!mu.running && !!mu.playing);
        if (wasIdle !== !mu.running) ctx.fit();
        if (!mu.running) return;
        marquee($('muTitle'), mu.title || 'Spotify');
        $('muArtist').textContent = mu.artist || (mu.title ? '' : T('muNothing'));
        const img = $('muImg');
        if (mu.art) { if (img.getAttribute('src') !== mu.art) img.src = mu.art; sec.classList.remove('noart'); }
        else { img.removeAttribute('src'); sec.classList.add('noart'); }
        $('muToggle').innerHTML = svg(mu.playing ? 'pause' : 'play');
        $('muToggle').title = T(mu.playing ? 'muPause' : 'muPlay');
        $('muBar').classList.toggle('seek', !!mu.canSeek);
        $('muShuf').classList.toggle('on', mu.shuffle === true);
        $('muShuf').classList.toggle('unknown', mu.shuffle == null);
        $('muShuf').title = `${T('muShuffle')}: ${T(mu.shuffle ? 'muOn' : 'muOff')}`;
        sec.classList.toggle('notime', !mu.dur);
        renderTime();
      };
      const send = (action, extra) => {
        if (ctx.demo) {
          if (action === 'toggle') { mu.pos = muPos(); mu.at = Date.now(); mu.playing = !mu.playing; }
          if (action === 'seek') { mu.pos = extra.pos; mu.at = Date.now(); }
          if (action === 'shuffle') mu.shuffle = !mu.shuffle;
          if (action === 'open' || action === 'next' || action === 'prev') ctx.toast(`Demo: ${action}`);
          return render();
        }
        ctx.post({ type: 'media', action, ...(extra || {}) });
      };
      sec.querySelector('.mu-btns').addEventListener('click', (e) => {
        const b = e.target.closest('[data-mu]');
        if (!b) return;
        // a gomb azonnal mutassa az új állapotot, a Spotify visszajelzése követi
        if (b.dataset.mu === 'toggle' && !ctx.demo) { mu.pos = muPos(); mu.at = Date.now(); mu.playing = !mu.playing; render(); }
        if (b.dataset.mu === 'shuffle' && !ctx.demo && mu.shuffle != null) { mu.shuffle = !mu.shuffle; render(); }
        send(b.dataset.mu);
      });
      $('muArt').addEventListener('click', () => send('open'));
      $('muLaunch').addEventListener('click', () => send('open'));
      $('muBar').addEventListener('click', (e) => {
        if (!mu.canSeek || !mu.dur) return;
        const r = $('muBar').getBoundingClientRect();
        const pos = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * mu.dur;
        mu.pos = pos; mu.at = Date.now();
        renderTime();
        send('seek', { pos });
      });
      // görgetés a zene fölött: a hangerő változik, ahogy a hangerő szekción
      sec.addEventListener('wheel', (e) => {
        e.preventDefault();
        ctx.call('volume', 'nudge', e.deltaY < 0 ? 2 : -2);
      }, { passive: false });
      render();
      const message = (msg) => { if (msg.type === 'media') { mu = msg; render(); } };
      return {
        message,
        tick: renderTime,
        demo: () => message({ type: 'media', running: true, title: 'Neon Rain (Extended Mix)', artist: 'Night Runner', album: 'Afterglow', playing: true,
          shuffle: false, canSeek: true, pos: 74, dur: 246, at: Date.now(), art: '../wallpapers/CyberShell_background_03.jpg' }),
      };
    },
    strings: {
      hu: {
        secMusic: '// ZENE', sMusic: 'Zene (Spotify)', sMusicHint: 'A Spotify asztali alkalmazással működik.',
        muLaunchHint: 'A Spotify indítása', muOpen: 'A Spotify megnyitása', muPrev: 'Előző', muNext: 'Következő', muPlay: 'Lejátszás', muPause: 'Szünet',
        muNothing: 'Nem szól semmi', muShuffle: 'Véletlenszerű sorrend', muOn: 'be', muOff: 'ki',
      },
      en: {
        secMusic: '// MUSIC', sMusic: 'Music (Spotify)', sMusicHint: 'Works with the Spotify desktop app.',
        muLaunchHint: 'Start Spotify', muOpen: 'Open Spotify', muPrev: 'Previous', muNext: 'Next', muPlay: 'Play', muPause: 'Pause',
        muNothing: 'Nothing playing', muShuffle: 'Shuffle', muOn: 'on', muOff: 'off',
      },
      pl: {
        secMusic: '// MUZYKA', sMusic: 'Muzyka (Spotify)', sMusicHint: 'Działa z aplikacją Spotify na komputer.',
        muLaunchHint: 'Uruchom Spotify', muOpen: 'Otwórz Spotify', muPrev: 'Poprzedni', muNext: 'Następny', muPlay: 'Odtwórz', muPause: 'Pauza',
        muNothing: 'Nic nie gra', muShuffle: 'Losowa kolejność', muOn: 'wł.', muOff: 'wył.',
      },
    },
  });
})();
