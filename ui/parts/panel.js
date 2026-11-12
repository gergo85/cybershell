(() => {
  const { $, q, side, horiz, T, root, demo, post, setAlert, svg, esc, pad, appColor, store, fetchJson, resolvePlace, wmoText, wmoIcon } = window.HUB;

  // ───────────── információs panel (alapból jobbra) ─────────────
  function buildRight() {
    root.className = `right side-${side}${horiz ? ' horiz' : ''}`;
    root.innerHTML = `
      <div class="edge"></div>
      <div class="quick">
        <div class="quick-apps" id="quickApps"></div>
        <button class="qbtn" data-cmd="search" style="--app:#4fd1c5"><span class="qtile">${svg('search')}</span><span class="qlabel">${T('search')}</span></button>
        <button class="qbtn" data-cmd="start" style="--app:#3b9eff"><span class="qtile">${svg('start')}</span><span class="qlabel">${T('start')}</span></button>
      </div>
      <section class="sec" id="claudeSec">
        <h3>${T('secClaude')}</h3>
        <div id="usage"><div class="note">…</div></div>
        <div class="usage-foot">
          <button class="refresh" id="usageRefresh">${svg('reload')}<span id="usageTime">${T('refresh')}</span></button>
          <span class="plan" id="plan"></span>
        </div>
      </section>
      <section class="sec act" id="act">
        <h3>${T('secActivity')}</h3>
        <div class="act-panes">
        <div class="act-normal">
          <div class="act-time" id="actTime">--:--</div>
          <div class="act-segs" id="actSegs">${'<i></i>'.repeat(10)}</div>
          <div class="act-info" id="actInfo">&nbsp;</div>
          <div class="act-btns">
            <button class="ib" id="actToggle"></button>
            <button class="ib" id="actReset">${svg('reload')}</button>
            <button class="ib" id="actSet">${svg('settings')}</button>
          </div>
        </div>
        <div class="act-edit">
          <label class="act-edit-label" for="actMin">${T('actLength')}</label>
          <input id="actMin" type="number" min="1" max="240" step="1" inputmode="numeric">
          <div class="act-err" id="actErr"></div>
          <div class="act-btns">
            <button class="ib" id="actSave">${svg('check')}</button>
            <button class="ib" id="actCancel">${svg('close')}</button>
          </div>
        </div>
        </div>
        <div class="act-alert">
          <div class="act-alert-title">${T('actAlertTitle')}</div>
          <div class="act-alert-text">${T('actAlertText')}</div>
          <div class="act-btns">
            <button class="ib" id="actDone">${svg('check')}</button>
            <button class="ib" id="actSnooze">+5</button>
          </div>
        </div>
      </section>
      <section class="sec clock" id="clockSec">
        <h3>${T('secClock')}</h3>
        <div class="clock-row"><span id="hh">--</span><span class="colon">:</span><span id="mm">--</span></div>
        <div class="sec-line"><div class="segs" id="segs">${'<i></i>'.repeat(12)}</div><span class="ss" id="ss">--</span></div>
      </section>
      <section class="sec date" id="dateSec" title="${T('calOpen')}">
        <h3>${T('secDate')}</h3>
        <div class="date-stack"><span id="dy">----</span><span id="dmd">--</span></div>
        <div class="week" id="week">&nbsp;</div>
      </section>
      <section class="sec laptop" id="laptopSec" style="display:none">
        <h3>${T('secLaptop')}</h3>
        <div class="lap-grid">
          <button class="lap" id="lapWifi" data-cmd="wifi" style="--app:#4fd1c5">${svg('wifi')}<span>${T('wifi')}</span></button>
          <button class="lap" id="lapBat" data-cmd="battery" style="--app:#7ee787"><span class="bat" id="batIcon"></span><span id="batPct">--%</span></button>
        </div>
      </section>
      <section class="sec weather" id="weatherSec" title="${T('wDetails')}">
        <h3>${T('secWeather')}</h3>
        <div class="w-main"><span class="w-icon" id="wIcon"></span><span class="w-temp" id="wTemp">--°</span></div>
        <div class="w-desc" id="wDesc">&nbsp;</div>
        <div class="w-wind" id="wWind">&nbsp;</div>
        <div class="w-city" id="wCity">&nbsp;</div>
      </section>
      <section class="sec music idle" id="musicSec">
        <h3>${T('secMusic')}</h3>
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
        </div>
      </section>
      <section class="sec vol" id="volSec" title="${T('volHint')}">
        <h3>${T('secVolume')}</h3>
        <div class="vol-row">
          <button class="vol-mute" id="volMute">${svg('volume')}</button>
          <button class="vol-pct" id="volPct">--</button>
        </div>
        <div class="vol-bar" id="volBar"><span><i id="volFill"></i></span></div>
      </section>
      <section class="sec power" id="powerSec">
        <h3>${T('secPower')}</h3>
        <div class="pw-panes">
        <div class="pw-grid">
          <button class="pw" data-act="lock" style="--app:#4fd1c5" title="${T('pwLockT')}">${svg('lock')}<span>${T('pwLock')}</span></button>
          <button class="pw" data-act="sleep" style="--app:#8b6cff" title="${T('pwSleepT')}">${svg('moon')}<span>${T('pwSleep')}</span></button>
          <button class="pw" data-act="shutdown" style="--app:#ff2a6d" title="${T('pwOffT')}">${svg('power')}<span>${T('pwOff')}</span></button>
          <button class="pw" data-act="restart" style="--app:#ffb02e" title="${T('pwRestartT')}">${svg('reload')}<span>${T('pwRestart')}</span></button>
        </div>
        <div class="pw-confirm" id="pwConfirm">
          <div class="pw-q" id="pwQ"></div>
          <div class="act-btns">
            <button class="ib danger" id="pwYes">${svg('check')}</button>
            <button class="ib" id="pwNo">${svg('close')}</button>
          </div>
        </div>
        </div>
      </section>
      <div class="sys">
        <button class="sys-btn" id="sysBtn">${T('menu')}</button>
      </div>
      <div class="toast" id="toast"></div>`;

    // Keresés és Indítás: a Windows saját keresője és Start menüje; fölöttük a
    // jobb oldalra tett programok (config "place": "right")
    root.querySelector('.quick').addEventListener('click', (e) => {
      const b = e.target.closest('.qbtn');
      if (!b) return;
      if (b.dataset.id != null) post({ type: 'launch', id: Number(b.dataset.id), newInstance: e.shiftKey });
      else post({ type: 'cmd', name: b.dataset.cmd, along: alongOf(b) });
    });
    // egy elem közepe a panel mentén (függőleges panelnél fentről, vízszintesnél balról)
    const alongOf = (el) => {
      const r = el.getBoundingClientRect();
      return Math.round(horiz ? r.left + r.width / 2 : r.top + r.height / 2);
    };
    const renderQuickApps = (apps) => {
      $('quickApps').innerHTML = (apps || []).map((a) => {
        const icon = a.img ? `<img src="${esc(a.img)}" alt="">` : svg(window.HUB_ICONS[a.icon] ? a.icon : 'app');
        return `<button class="qbtn qapp" data-id="${a.id}"${appColor(a)}><span class="qtile">${icon}</span><span class="qlabel">${esc(a.name)}</span></button>`;
      }).join('');
    };
    const applyQuickState = (running, active) => {
      const run = new Set(running || []);
      for (const el of root.querySelectorAll('.qapp')) {
        const id = Number(el.dataset.id);
        el.classList.toggle('running', run.has(id));
        el.classList.toggle('active', id === active);
      }
    };

    // MENÜ: a beállításokat nyitja meg
    const sysBtn = $('sysBtn');
    sysBtn.addEventListener('click', () => {
      if (demo) return toast('Demo: settings');
      post({ type: 'cmd', name: 'settings' });
    });

    let toastTimer = 0;
    const toast = (text) => {
      const t = $('toast');
      if (!horiz) t.style.bottom = `${root.clientHeight - sysBtn.offsetTop + 8}px`;
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => t.classList.remove('show'), 4500);
    };

    // ── főkapcsoló: a leállítás és az újraindítás megerősítést kér ──
    const POWER_Q = { shutdown: T('pwConfirmOff'), restart: T('pwConfirmRestart') };
    let pendingPower = null;
    let pwTimer = 0;
    const closeConfirm = () => {
      pendingPower = null;
      clearTimeout(pwTimer);
      $('powerSec').classList.remove('confirming');
    };
    root.querySelector('.pw-grid').addEventListener('click', (e) => {
      const b = e.target.closest('.pw');
      if (!b) return;
      const act = b.dataset.act;
      if (POWER_Q[act]) {
        pendingPower = act;
        $('pwQ').textContent = POWER_Q[act];
        $('powerSec').classList.add('confirming');
        clearTimeout(pwTimer);
        pwTimer = setTimeout(closeConfirm, 8000);
        return;
      }
      if (demo) return toast(`Demo: ${act}`);
      post({ type: 'cmd', name: 'power', action: act });
    });
    $('pwYes').addEventListener('click', () => {
      const act = pendingPower;
      closeConfirm();
      if (!act) return;
      if (demo) return toast(`Demo: ${act}`);
      post({ type: 'cmd', name: 'power', action: act });
    });
    $('pwNo').addEventListener('click', closeConfirm);

    // Ha a szekciók nem férnek ki (kis képernyő, laptop szekció, szóló zene), a panel
    // előbb a szekciók közti réseket szűkíti, aztán a menü fölötti díszítést is.
    const overflows = () => (horiz ? root.scrollWidth > root.clientWidth + 1 : root.scrollHeight > root.clientHeight + 1);
    let quickSig = '';
    const fitPanel = () => {
      root.classList.remove('tight', 'tighter');
      if (overflows()) {
        root.classList.add('tight');
        if (overflows()) root.classList.add('tighter');
      }
      // az Indítás és a Keresés gomb helye: a Win billentyűvel nyitott menü is ide igazodik
      const pos = { start: alongOf(root.querySelector('[data-cmd="start"]')), search: alongOf(root.querySelector('[data-cmd="search"]')) };
      const sig = JSON.stringify(pos);
      if (sig !== quickSig) { quickSig = sig; post({ type: 'quick-pos', ...pos }); }
    };
    window.addEventListener('resize', fitPanel);
    // bármelyik szekció mérete változik (pl. megjön a Claude-adat), újramérünk
    let fitQueued = false;
    const queueFit = () => {
      if (fitQueued) return;
      fitQueued = true;
      setTimeout(() => { fitQueued = false; fitPanel(); }, 30);
    };
    const fitObserver = new ResizeObserver(queueFit);
    for (const el of root.querySelectorAll('.sec, .quick')) fitObserver.observe(el);

    // ── szekciók láthatósága és sorrendje (a beállításokból) ──
    let claudeEnabled = true;
    let sections = { claude: true, activity: true, clock: true, date: true, weather: true, music: true, volume: true,
      order: ['claude', 'activity', 'date', 'clock', 'weather', 'music', 'volume'] };
    const SECTION_EL = { claude: 'claudeSec', activity: 'act', clock: 'clockSec', date: 'dateSec', weather: 'weatherSec', music: 'musicSec', volume: 'volSec' };
    let weatherOn = true;
    const applySections = () => {
      $('claudeSec').style.display = claudeEnabled && sections.claude !== false ? '' : 'none';
      $('act').style.display = sections.activity !== false ? '' : 'none';
      $('clockSec').style.display = sections.clock !== false ? '' : 'none';
      $('dateSec').style.display = sections.date !== false ? '' : 'none';
      $('volSec').style.display = sections.volume !== false ? '' : 'none';
      $('musicSec').style.display = sections.music !== false ? '' : 'none';
      $('weatherSec').style.display = weatherOn && sections.weather !== false ? '' : 'none';
      // a szekciók a beállított sorrendben, a Főkapcsoló előtt (ami a listából hiányzik, a végére kerül)
      const anchor = $('powerSec');
      const order = [...(sections.order || [])];
      for (const k of Object.keys(SECTION_EL)) if (!order.includes(k)) order.push(k);
      for (const k of order) {
        const el = $(SECTION_EL[k]);
        if (el) root.insertBefore(el, anchor);
      }
      // a laptop szekció (Wi-Fi, akkumulátor) mindig az időjárás előtt van
      root.insertBefore($('laptopSec'), $('weatherSec'));
      fitPanel();
    };

    // ── óra, dátum és a hét sorszáma (ISO 8601: a hét hétfőn kezdődik) ──
    const isoWeek = (d) => {
      const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
      const day = t.getUTCDay() || 7;
      t.setUTCDate(t.getUTCDate() + 4 - day);
      const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
      return Math.ceil(((t - yearStart) / 86400000 + 1) / 7);
    };
    const segs = [...$('segs').children];
    const tick = () => {
      const d = new Date();
      const s = d.getSeconds();
      $('hh').textContent = pad(d.getHours());
      $('mm').textContent = pad(d.getMinutes());
      $('ss').textContent = pad(s);
      const cur = Math.floor(s / 5);
      segs.forEach((el, i) => { el.classList.toggle('on', i < cur); el.classList.toggle('now', i === cur); });
      $('dy').textContent = T('dateYear', { y: d.getFullYear() });
      $('dmd').textContent = T('dateMonthDay', { m: pad(d.getMonth() + 1), d: pad(d.getDate()) });
      $('week').textContent = T('week', { n: isoWeek(d) });
      actTick();
      if (s % 30 === 0) renderUsage();
      setTimeout(tick, 1000 - new Date().getMilliseconds() + 5);
    };

    // ── Claude-használat ──
    let usage = null;
    const rel = (iso) => {
      if (!iso) return '';
      const ms = new Date(iso).getTime() - Date.now();
      if (!(ms > 0)) return T('soon');
      const m = Math.round(ms / 60000);
      const dd = Math.floor(m / 1440), hh = Math.floor((m % 1440) / 60), mm = m % 60;
      if (dd > 0) return T('relDH', { d: dd, h: hh });
      if (hh > 0) return T('relHM', { h: hh, m: pad(mm) });
      return T('relM', { m: mm });
    };
    const renderUsage = () => {
      const box = $('usage');
      if (!usage) return;
      $('plan').textContent = (usage.plan || '').toUpperCase();
      // a claude.ai még nem válaszolt, vagy épp nem érhető el: nem kell újra belépni
      if (usage.status === 'wait' || usage.status === 'offline') {
        box.innerHTML = `<div class="note ${usage.status}">${T(usage.status === 'wait' ? 'usageWait' : 'usageOffline')}</div>`;
        return;
      }
      if (usage.status !== 'ok') {
        const text = usage.status === 'error' ? `${T('usageError')} ${esc(usage.message || '')}` : T('usageLogin');
        box.innerHTML = `<div class="note ${usage.status}">${text}</div>
          <button class="ib login" id="claudeLogin">${T('login')}</button>`;
        $('claudeLogin').addEventListener('click', () => post({ type: 'cmd', name: 'claude-login' }));
        return;
      }
      box.innerHTML = usage.limits.map((l) => {
        // ha az ablak azóta újraindult, az érték ismeretlen, amíg újra nem jön adat
        const known = typeof l.pct === 'number';
        const pct = known ? Math.max(0, Math.min(100, Math.round(l.pct))) : 0;
        const lvl = pct >= 85 ? 'hot' : pct >= 60 ? 'warn' : '';
        const label = l.key === 'five_hour' ? T('limit5h') : l.key === 'seven_day' ? T('limitWeek') : esc(l.label.toUpperCase());
        return `
          <div class="lim ${lvl}">
            <div class="lim-head"><span>${label}</span><b>${known ? `${pct}%` : '—'}</b></div>
            <div class="lim-bar"><i style="width:${pct}%"></i></div>
            <div class="lim-reset">${l.resetsAt ? `↻ ${rel(l.resetsAt)}` : '&nbsp;'}</div>
          </div>`;
      }).join('');
    };
    $('usageRefresh').addEventListener('click', () => {
      $('usageTime').textContent = `${T('refresh')}…`;
      post({ type: 'usage-refresh' });
    });

    // ── aktivitás-visszaszámláló ──
    const KEY = 'cybershell.activity';
    const KEY_MIN = 'cybershell.activity.minutes';
    let cfgAct = { minutes: 20, autoStart: false };
    let session = 0;
    let act = null; // { running, endAt, remaining, alert, session }
    const minutes = () => store.get(KEY_MIN) || cfgAct.minutes;
    const total = () => minutes() * 60000;
    const save = () => store.set(KEY, act);
    const remaining = () => (act.running ? act.endAt - Date.now() : act.remaining);
    // az alert állapotot a setActAlert kapcsolja, itt csak megőrizzük
    const startCycle = (ms = total()) => {
      act = { running: true, endAt: Date.now() + ms, remaining: ms, alert: act ? act.alert : false, session };
      save();
    };
    const setActAlert = (on) => {
      if (act.alert === on) return;
      act.alert = on;
      save();
      $('act').classList.toggle('alerting', on);
      setAlert(on);
      post({ type: 'alert', on });
    };
    const actSegs = [...$('actSegs').children];
    function actTick() {
      if (!act) return;
      let left = remaining();
      if (act.running && left <= 0) {
        act.running = false;
        act.remaining = 0;
        left = 0;
        setActAlert(true);
      }
      const s = Math.ceil(Math.max(0, left) / 1000);
      $('actTime').textContent = `${pad(Math.floor(s / 60))}:${pad(s % 60)}`;
      const n = Math.ceil((Math.max(0, left) / total()) * actSegs.length);
      actSegs.forEach((el, i) => el.classList.toggle('on', i < n));
      $('act').classList.toggle('paused', !act.running && !act.alert);
      if (act.running) {
        const end = new Date(act.endAt);
        $('actInfo').textContent = T('actMoveAt', { t: `${pad(end.getHours())}:${pad(end.getMinutes())}` });
      } else if (!act.alert) {
        $('actInfo').textContent = T('actPaused');
      }
      $('actToggle').innerHTML = act.running ? svg('pause') : svg('play');
    }
    $('actToggle').addEventListener('click', () => {
      if (act.running) { act.remaining = Math.max(0, act.endAt - Date.now()); act.running = false; }
      else { act.endAt = Date.now() + (act.remaining > 0 ? act.remaining : total()); act.running = true; }
      save(); actTick();
    });
    $('actReset').addEventListener('click', () => { startCycle(); actTick(); });
    $('actDone').addEventListener('click', () => { startCycle(); setActAlert(false); actTick(); });
    $('actSnooze').addEventListener('click', () => { startCycle(5 * 60000); setActAlert(false); actTick(); });

    // egyedi hossz beállítása
    const actEl = $('act');
    const input = $('actMin');
    const openEdit = () => {
      input.value = minutes();
      $('actErr').textContent = '';
      actEl.classList.add('editing');
      input.focus();
      input.select();
    };
    const closeEdit = () => actEl.classList.remove('editing');
    const saveEdit = () => {
      const v = Number(input.value);
      if (!Number.isInteger(v) || v < 1 || v > 240) {
        $('actErr').textContent = T('actError');
        input.focus();
        return;
      }
      store.set(KEY_MIN, v);
      closeEdit();
      startCycle();
      actTick();
    };
    $('actSet').addEventListener('click', openEdit);
    $('actSave').addEventListener('click', saveEdit);
    $('actCancel').addEventListener('click', closeEdit);
    input.addEventListener('input', () => { $('actErr').textContent = ''; });
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') saveEdit();
      if (e.key === 'Escape') closeEdit();
    });

    // Több képernyőnél minden információs panel ugyanazt a visszaszámlálót mutatja:
    // ha egy másik panelen változott, átvesszük.
    window.addEventListener('storage', (e) => {
      if (e.key !== KEY || !e.newValue) return;
      try {
        const v = JSON.parse(e.newValue);
        if (!v || typeof v.running !== 'boolean') return;
        const wasAlert = act && act.alert;
        act = v;
        if (!!v.alert !== !!wasAlert) {
          $('act').classList.toggle('alerting', !!v.alert);
          setAlert(!!v.alert);
        }
        actTick();
      } catch (_) {}
    });
    window.addEventListener('storage', (e) => { if (e.key === KEY_MIN) actTick(); });

    // A program új indulásakor (új munkamenet) a visszaszámláló nem indul magától,
    // csak ha a beállítás ezt kéri. A felület újratöltésekor viszont folytatódik.
    const initActivity = () => {
      const saved = store.get(KEY);
      if (saved && typeof saved.running === 'boolean' && saved.session === session) {
        act = saved;
        const wasAlert = act.alert;
        act.alert = false;
        if (wasAlert) setActAlert(true);
      } else if (cfgAct.autoStart) {
        act = null;
        startCycle();
      } else {
        act = { running: false, endAt: 0, remaining: total(), alert: false, session };
        save();
      }
      actTick();
    };

    // ── időjárás (Open-Meteo, kulcs nélkül) ──
    let wcfg = null;
    let wTimer = 0;
    const fetchWeather = async () => {
      clearTimeout(wTimer);
      wTimer = setTimeout(fetchWeather, 15 * 60000);
      try {
        const geo = await resolvePlace(wcfg);
        // a hely kell az Auto témához is (napkelte, napnyugta)
        if (!demo) post({ type: 'geo', lat: geo.lat, lon: geo.lon });
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${geo.lat}&longitude=${geo.lon}` +
          '&current=temperature_2m,weather_code,is_day,wind_speed_10m,wind_direction_10m&timezone=auto&forecast_days=1');
        const c = w.current;
        $('wIcon').innerHTML = svg(wmoIcon(c.weather_code, c.is_day));
        $('wTemp').textContent = `${Math.round(c.temperature_2m)}°`;
        $('wDesc').textContent = wmoText(c.weather_code).toUpperCase();
        // a nyíl arra mutat, amerre a szél fúj (a jelentett irány az, ahonnan fúj)
        $('wWind').innerHTML = `<i class="w-arrow" style="transform:rotate(${(c.wind_direction_10m + 180) % 360}deg)">↑</i>` +
          `<b>${Math.round(c.wind_speed_10m)}</b><span>km/h</span>`;
        $('wCity').textContent = (geo.name || '').toUpperCase();
        $('weatherSec').classList.remove('err');
      } catch (e) {
        $('wDesc').textContent = T('noData');
        $('wWind').textContent = String(e.message || e).slice(0, 30);
        $('weatherSec').classList.add('err');
        clearTimeout(wTimer);
        wTimer = setTimeout(fetchWeather, 2 * 60000);
      }
    };
    // kattintásra a panel mellett kinyílik a részletes mai előrejelzés
    $('weatherSec').addEventListener('click', () => {
      if (!wcfg) return;
      const r = $('weatherSec').getBoundingClientRect();
      if (demo) return toast('Demo: weather details');
      post({ type: 'weather-open', top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right) });
    });

    // ── hangerő: a csúszka húzható és kattintható, a szekció fölött görgetni is lehet ──
    let vol = { ok: false, pct: 0, muted: false };
    let volDrag = false, volHold = 0, volSent = 0;
    const renderVol = () => {
      const sec = $('volSec');
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
      volHold = Date.now() + 800; // a közben érkező régebbi állapot ne ugrassza vissza
      renderVol();
      if (demo) return;
      // húzás közben nem küldünk minden egyes mozdulatot
      const now = Date.now();
      if (now - volSent > 40 || !volDrag) { volSent = now; post({ type: 'volume-set', pct }); }
    };
    const volFromX = (x) => {
      const r = $('volBar').getBoundingClientRect();
      return ((x - r.left) / r.width) * 100;
    };
    $('volBar').addEventListener('pointerdown', (e) => {
      if (!vol.ok) return;
      volDrag = true;
      $('volBar').setPointerCapture(e.pointerId);
      setVol(volFromX(e.clientX));
    });
    $('volBar').addEventListener('pointermove', (e) => { if (volDrag) setVol(volFromX(e.clientX)); });
    const volEnd = (e) => {
      if (!volDrag) return;
      volDrag = false;
      setVol(volFromX(e.clientX));
      if (!demo) post({ type: 'volume-set', pct: vol.pct });
    };
    $('volBar').addEventListener('pointerup', volEnd);
    $('volBar').addEventListener('pointercancel', volEnd);
    $('volSec').addEventListener('wheel', (e) => {
      e.preventDefault();
      setVol(vol.pct + (e.deltaY < 0 ? 2 : -2));
    }, { passive: false });
    $('volMute').addEventListener('click', () => {
      if (!vol.ok) return;
      vol.muted = !vol.muted;
      volHold = Date.now() + 800;
      renderVol();
      if (!demo) post({ type: 'volume-mute', on: vol.muted });
    });
    $('volPct').addEventListener('click', () => {
      if (demo) return toast('Demo: volume mixer');
      post({ type: 'volume-open' });
    });
    if (demo) { vol = { ok: true, pct: 42, muted: false }; }
    renderVol();

    // ── zene: a Spotify alkalmazás lejátszása (a Windows médiavezérlőjén át) ──
    let mu = { running: false };
    const mmss = (sec) => {
      sec = Math.max(0, Math.floor(sec || 0));
      return `${Math.floor(sec / 60)}:${pad(sec % 60)}`;
    };
    const muPos = () => (mu.playing ? (mu.pos || 0) + (Date.now() - (mu.at || Date.now())) / 1000 : (mu.pos || 0));
    const renderMuTime = () => {
      if (!mu.running) return;
      const dur = mu.dur || 0;
      const pos = dur ? Math.min(dur, muPos()) : 0;
      $('muFill').style.width = dur ? `${(pos / dur) * 100}%` : '0%';
      $('muBar').title = dur ? `${mmss(pos)} / ${mmss(dur)}` : '';
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
    const renderMusic = () => {
      const sec = $('musicSec');
      const wasIdle = sec.classList.contains('idle');
      sec.classList.toggle('idle', !mu.running);
      sec.classList.toggle('playing', !!mu.running && !!mu.playing);
      if (wasIdle !== !mu.running) fitPanel();
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
      renderMuTime();
    };
    const muSend = (action, extra) => {
      if (demo) {
        if (action === 'toggle') { mu.pos = muPos(); mu.at = Date.now(); mu.playing = !mu.playing; }
        if (action === 'seek') { mu.pos = extra.pos; mu.at = Date.now(); }
        if (action === 'shuffle') mu.shuffle = !mu.shuffle;
        if (action === 'open' || action === 'next' || action === 'prev') toast(`Demo: ${action}`);
        return renderMusic();
      }
      post({ type: 'media', action, ...(extra || {}) });
    };
    root.querySelector('.mu-btns').addEventListener('click', (e) => {
      const b = e.target.closest('[data-mu]');
      if (!b) return;
      // a gomb azonnal mutassa az új állapotot, a Spotify visszajelzése követi
      if (b.dataset.mu === 'toggle' && !demo) { mu.pos = muPos(); mu.at = Date.now(); mu.playing = !mu.playing; renderMusic(); }
      if (b.dataset.mu === 'shuffle' && !demo && mu.shuffle != null) { mu.shuffle = !mu.shuffle; renderMusic(); }
      muSend(b.dataset.mu);
    });
    $('muArt').addEventListener('click', () => muSend('open'));
    $('muLaunch').addEventListener('click', () => muSend('open'));
    $('muBar').addEventListener('click', (e) => {
      if (!mu.canSeek || !mu.dur) return;
      const r = $('muBar').getBoundingClientRect();
      const pos = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * mu.dur;
      mu.pos = pos; mu.at = Date.now();
      renderMuTime();
      muSend('seek', { pos });
    });
    $('musicSec').addEventListener('wheel', (e) => {
      // görgetés a zene fölött: a hangerő változik, ahogy a hangerő szekción
      e.preventDefault();
      setVol(vol.pct + (e.deltaY < 0 ? 2 : -2));
    }, { passive: false });
    setInterval(renderMuTime, 1000);
    renderMusic();

    // a dátumra kattintva a naptár nyílik ki a panel mellett
    $('dateSec').addEventListener('click', () => {
      const r = $('dateSec').getBoundingClientRect();
      if (demo) return toast('Demo: calendar');
      post({ type: 'calendar-open', top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right) });
    });

    // ── laptop: Wi-Fi és akkumulátor (a Windows saját felületét nyitják meg) ──
    root.querySelector('.lap-grid').addEventListener('click', (e) => {
      const b = e.target.closest('.lap');
      if (!b) return;
      if (demo) return toast(`Demo: ${b.dataset.cmd}`);
      post({ type: 'cmd', name: b.dataset.cmd });
    });
    const batSvg = (pct, charging) => {
      const w = Math.max(0, Math.min(100, pct == null ? 0 : pct)) / 100 * 13;
      return `<svg viewBox="0 0 24 24"><path d="M3 8h16v8H3z"/><path d="M21 10.5v3" stroke-width="2"/>` +
        `<rect class="bat-fill" x="5" y="10" width="${w.toFixed(1)}" height="4"/>` +
        (charging ? '<path class="bat-bolt" d="M12.5 6.5l-3 5.5h4l-3 5.5"/>' : '') + '</svg>';
    };
    const showPower = (m) => {
      const was = $('laptopSec').style.display;
      $('laptopSec').style.display = m.laptop ? '' : 'none';
      if (was !== $('laptopSec').style.display) fitPanel();
      if (!m.laptop) return;
      const pct = typeof m.pct === 'number' ? m.pct : null;
      $('batIcon').innerHTML = batSvg(pct, m.charging);
      $('batPct').textContent = pct == null ? '--%' : `${pct}%`;
      const lap = $('lapBat');
      lap.classList.toggle('charging', !!m.charging);
      lap.classList.toggle('low', pct != null && pct <= 20 && !m.charging);
    };
    const showOnline = () => $('lapWifi').classList.toggle('offline', !navigator.onLine);
    window.addEventListener('online', showOnline);
    window.addEventListener('offline', showOnline);
    showOnline();

    tick();

    const h = (msg) => {
      if (msg.type === 'init') {
        if (msg.activity) cfgAct = { minutes: Math.max(1, msg.activity.minutes || 20), autoStart: msg.activity.autoStart === true };
        if (msg.session) session = msg.session;
        claudeEnabled = msg.claudeUsage !== false;
        if (msg.sections) sections = msg.sections;
        applySections();
        if (!act) initActivity();
        renderQuickApps(msg.apps);
        wcfg = msg.weather || { enabled: true, city: 'Budapest' };
        weatherOn = wcfg.enabled !== false;
        $('weatherSec').style.display = weatherOn && sections.weather !== false ? '' : 'none';
        if (wcfg.enabled !== false) fetchWeather();
        fitPanel();
      }
      if (msg.type === 'sections') { sections = msg.sections || sections; applySections(); }
      if (msg.type === 'settings-open') sysBtn.classList.toggle('open', !!msg.on);
      if (msg.type === 'state') applyQuickState(msg.running, msg.active);
      if (msg.type === 'toast') toast(msg.text);
      if (msg.type === 'power') showPower(msg);
      if (msg.type === 'volume') {
        if (volDrag || Date.now() < volHold) return;
        vol = { ok: !!msg.ok, pct: msg.pct | 0, muted: !!msg.muted };
        renderVol();
      }
      if (msg.type === 'media') { mu = msg; renderMusic(); }
      queueFit();
      if (msg.type === 'launcher-state') {
        for (const b of root.querySelectorAll('.qbtn[data-cmd]')) b.classList.toggle('open', !!msg.open && b.dataset.cmd === msg.mode);
      }
      if (msg.type === 'fly-state') {
        $('weatherSec').classList.toggle('open', !!msg.open && msg.kind === 'weather');
        $('dateSec').classList.toggle('open', !!msg.open && msg.kind === 'calendar');
      }
      if (msg.type === 'usage') {
        usage = msg;
        // az adat saját időbélyege (állapotsorból érkezve ez lehet régebbi is)
        const t = new Date(msg.status === 'ok' && msg.fetchedAt ? msg.fetchedAt : Date.now());
        $('usageTime').textContent = `${pad(t.getHours())}:${pad(t.getMinutes())}`;
        renderUsage();
      }
    };
    h.demo = () => {
      h({ type: 'init', activity: { minutes: 20, autoStart: false }, session: 1, claudeUsage: true, weather: { enabled: true, city: 'Budapest' },
        apps: [{ id: 13, name: 'Cyberpunk', icon: 'game', color: '#ff3b5c' }] });
      h({ type: 'power', laptop: true, pct: 76, charging: true });
      h({ type: 'media', running: true, title: 'Neon Rain (Extended Mix)', artist: 'Night Runner', album: 'Afterglow', playing: true,
        shuffle: false, canSeek: true, pos: 74, dur: 246, at: Date.now(), art: '../wallpapers/04-neon-street.jpg' });
      const inH = (hrs) => new Date(Date.now() + hrs * 3600000).toISOString();
      h({ type: 'usage', status: 'ok', plan: 'pro', limits: [
        { key: 'five_hour', label: '5 órás', pct: 34, resetsAt: inH(2.3) },
        { key: 'seven_day', label: 'Heti', pct: 71, resetsAt: inH(76) },
      ] });
    };
    return h;
  }

  window.HUB.parts.right = buildRight;
})();
