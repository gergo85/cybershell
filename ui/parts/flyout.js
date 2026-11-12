(() => {
  const { $, q, part, lang, T, root, demo, post, setTheme, svg, esc, pad, fetchJson, resolvePlace, wmoText, wmoIcon } = window.HUB;

  // ───────────── oldalpanel: részletes időjárás és naptár (az információs panel mellett) ─────────────
  function buildFly() {
    root.className = 'wfly';
    root.innerHTML = `
      <div class="fview" id="vWeather">
        <header class="wf-head">
          <span class="wf-title">${T('secWeather')}<b id="wfCity"></b></span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="wf-body" id="wfBody"><div class="wf-note">${T('wLoading')}</div></div>
        <footer class="wf-foot" id="wfFoot">&nbsp;</footer>
      </div>
      <div class="fview" id="vCal">
        <header class="wf-head">
          <span class="wf-title">${T('secCalendar')}<b id="calTitle"></b></span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="cal-nav">
          <button class="cal-arrow" id="calPrev">${svg('left')}</button>
          <span class="cal-month" id="calMonth"></span>
          <button class="cal-today" id="calToday">${T('calToday')}</button>
          <button class="cal-arrow" id="calNext">${svg('right')}</button>
        </div>
        <div class="cal-grid" id="calGrid"></div>
        <div class="cal-day" id="calDay"></div>
        <footer class="cal-foot" id="calFoot"></footer>
      </div>`;
    const close = () => post({ type: 'close' });
    root.addEventListener('click', (e) => { if (e.target.closest('[data-close]')) close(); });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !e.target.closest('input')) close();
    });
    const showView = (id) => {
      for (const v of root.querySelectorAll('.fview')) v.classList.toggle('on', v.id === id);
      // minden megnyitáskor beúszik
      root.classList.remove('in');
      void root.offsetWidth;
      root.classList.add('in');
    };
    const locale = lang === 'en' ? 'en-GB' : 'hu-HU';
    const hm = (iso) => (iso || '').slice(11, 16);

    // ════════════ időjárás ════════════
    let wcfg = null;
    let last = null;   // { at, geo, w }
    let busy = false;
    const dec = (v) => (lang === 'en' ? v : v.replace('.', ','));
    const uvText = (u) => (u < 3 ? T('wUvLow') : u < 6 ? T('wUvMid') : u < 8 ? T('wUvHigh') : T('wUvVeryHigh'));
    const dayName = (iso) => new Intl.DateTimeFormat(locale, { weekday: 'short' })
      .format(new Date(`${iso}T12:00:00`)).replace('.', '').toUpperCase();

    const renderWeather = () => {
      const { geo, w, at } = last;
      const c = w.current, d = w.daily, hr = w.hourly;
      $('wfCity').textContent = (geo.name || '').toUpperCase();
      // a következő 24 óra háromóránként, a mostani órától
      const nowHour = c.time.slice(0, 13);
      let i0 = hr.time.findIndex((t) => t.slice(0, 13) === nowHour);
      if (i0 < 0) i0 = 0;
      const hours = [];
      for (let i = i0; i < hr.time.length && hours.length < 8; i += 3) hours.push(i);
      const temps = hours.map((i) => hr.temperature_2m[i]);
      const tMin = Math.min(...temps), tMax = Math.max(...temps);
      const yOf = (t) => (tMax === tMin ? 50 : 80 - ((t - tMin) / (tMax - tMin)) * 60);
      const xOf = (k) => (k + 0.5) * (100 / hours.length);
      const line = temps.map((t, k) => `${k ? 'L' : 'M'}${xOf(k).toFixed(2)},${yOf(t).toFixed(2)}`).join(' ');
      const prob = (i) => hr.precipitation_probability[i] ?? 0;
      const uv = d.uv_index_max[0];
      const tiles = [
        ['thermo', T('wFeels'), `${Math.round(c.apparent_temperature)}°`, ''],
        ['drop', T('wHumidity'), `${Math.round(c.relative_humidity_2m)}%`, ''],
        ['wind', T('wWindGust'), `${Math.round(c.wind_speed_10m)}/${Math.round(c.wind_gusts_10m)}`, 'km/h'],
        ['rain', T('wPrecip'), dec((d.precipitation_sum[0] || 0).toFixed(1)), `mm · ${d.precipitation_probability_max[0] ?? 0}%`],
        ['uv', T('wUv'), `${Math.round(uv ?? 0)}`, uv == null ? '' : uvText(uv)],
        ['sunrise', T('wSun'), hm(d.sunrise[0]), hm(d.sunset[0])],
      ];
      const days = [1, 2, 3].filter((k) => d.time[k]);
      const dMin = Math.min(...days.map((k) => d.temperature_2m_min[k]));
      const dMax = Math.max(...days.map((k) => d.temperature_2m_max[k]));
      const pct = (t) => ((t - dMin) / Math.max(1, dMax - dMin)) * 100;
      $('wfBody').innerHTML = `
        <section class="wf-hero">
          <span class="wf-icon">${svg(wmoIcon(c.weather_code, c.is_day))}</span>
          <div class="wf-now">
            <b>${Math.round(c.temperature_2m)}°</b>
            <span>${esc(wmoText(c.weather_code).toUpperCase())}</span>
            <em>${T('wMinMax', { min: Math.round(d.temperature_2m_min[0]), max: Math.round(d.temperature_2m_max[0]) })}</em>
          </div>
        </section>
        <section class="wf-sec">
          <h4>${T('wHourly')}</h4>
          <div class="wf-hours" style="--n:${hours.length}">
            <div class="wf-plot">
              <svg class="wf-curve" viewBox="0 0 100 100" preserveAspectRatio="none"><path d="${line}"/></svg>
              ${hours.map((i, k) => `<b class="wf-hv" style="left:${xOf(k).toFixed(2)}%;top:${yOf(temps[k]).toFixed(1)}%">${Math.round(temps[k])}°</b>`).join('')}
            </div>
            ${hours.map((i, k) => `
              <div class="wf-h${k === 0 ? ' now' : ''}">
                <span class="wf-hi">${svg(wmoIcon(hr.weather_code[i], hr.is_day[i]))}</span>
                <span class="wf-hp${prob(i) >= 50 ? ' wet' : ''}"><i style="height:${prob(i)}%"></i></span>
                <span class="wf-hpv">${prob(i)}%</span>
                <span class="wf-ht">${k === 0 ? T('wNow') : hm(hr.time[i])}</span>
              </div>`).join('')}
          </div>
        </section>
        <section class="wf-tiles">
          ${tiles.map(([ic, label, val, sub]) => `
            <div class="wf-tile">
              <span class="wf-tl">${svg(ic)}<span>${label}</span></span>
              <b>${val}${sub ? `<small>${esc(sub)}</small>` : ''}</b>
            </div>`).join('')}
        </section>
        <section class="wf-sec">
          <h4>${T('wDays')}</h4>
          <div class="wf-days">
            ${days.map((k) => `
              <div class="wf-day">
                <span class="wf-dn">${dayName(d.time[k])}</span>
                <span class="wf-di">${svg(wmoIcon(d.weather_code[k], 1))}</span>
                <span class="wf-dmin">${Math.round(d.temperature_2m_min[k])}°</span>
                <span class="wf-dbar"><i style="left:${pct(d.temperature_2m_min[k]).toFixed(1)}%;right:${(100 - pct(d.temperature_2m_max[k])).toFixed(1)}%"></i></span>
                <span class="wf-dmax">${Math.round(d.temperature_2m_max[k])}°</span>
              </div>`).join('')}
          </div>
        </section>`;
      const t = new Date(at);
      $('wfFoot').textContent = `${T('wUpdated', { t: `${pad(t.getHours())}:${pad(t.getMinutes())}` })} · Open-Meteo`;
    };

    const loadWeather = async (force) => {
      if (!wcfg || busy) return;
      if (last && !force && Date.now() - last.at < 10 * 60000) return renderWeather();
      busy = true;
      if (!last) $('wfBody').innerHTML = `<div class="wf-note">${T('wLoading')}</div>`;
      try {
        const geo = await resolvePlace(wcfg);
        const w = await fetchJson(`https://api.open-meteo.com/v1/forecast?latitude=${geo.lat}&longitude=${geo.lon}` +
          '&current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,is_day,wind_speed_10m,wind_gusts_10m' +
          '&hourly=temperature_2m,weather_code,precipitation_probability,is_day' +
          '&daily=weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset,uv_index_max,precipitation_sum,precipitation_probability_max' +
          '&timezone=auto&forecast_days=4');
        last = { at: Date.now(), geo, w };
        renderWeather();
      } catch (e) {
        post({ type: 'log', text: String(e.message || e) });
        if (!last) $('wfBody').innerHTML = `<div class="wf-note err">${T('noData')}<br>${esc(String(e.message || e).slice(0, 60))}</div>`;
      } finally {
        busy = false;
      }
    };

    // ════════════ naptár ════════════
    // Az eseményeket a program tölti le (iCal), itt csak értelmezzük és kirajzoljuk.
    const LOCAL_TZ = Intl.DateTimeFormat().resolvedOptions().timeZone;
    let feeds = [];          // csatolt naptárak: { name, color }
    let calEvents = [];      // értelmezett események
    let calStatus = [];      // letöltési eredmények: { name, color, ok, error }
    let calAt = 0;
    let calLoading = false;
    let addOpen = false;
    let view = new Date();   // a mutatott hónap
    let sel = new Date();    // a kiválasztott nap
    view = new Date(view.getFullYear(), view.getMonth(), 1);
    sel = new Date(sel.getFullYear(), sel.getMonth(), sel.getDate());

    const sameDay = (a, b) => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
    const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
    const isoWeek = (d) => {
      const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
      const day = t.getUTCDay() || 7;
      t.setUTCDate(t.getUTCDate() + 4 - day);
      return Math.ceil(((t - new Date(Date.UTC(t.getUTCFullYear(), 0, 1))) / 86400000 + 1) / 7);
    };

    // ── iCal értelmezése ──
    const unescapeIcs = (v) => v.replace(/\\n/gi, '\n').replace(/\\([,;\\])/g, '$1');
    const splitProp = (line) => {
      // az első idézőjelen kívüli kettőspont választja el a nevet és a paramétereket az értéktől
      let q = false;
      for (let i = 0; i < line.length; i++) {
        const ch = line[i];
        if (ch === '"') q = !q;
        else if (ch === ':' && !q) return [line.slice(0, i), line.slice(i + 1)];
      }
      return null;
    };
    const fmtCache = {};
    const tzOffset = (tz, t) => {
      const f = fmtCache[tz] || (fmtCache[tz] = new Intl.DateTimeFormat('en-US', {
        timeZone: tz, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
      }));
      const p = Object.fromEntries(f.formatToParts(new Date(t)).map((x) => [x.type, x.value]));
      return Date.UTC(+p.year, +p.month - 1, +p.day, +p.hour % 24, +p.minute, +p.second) - t;
    };
    const zoned = (y, mo, d, h, mi, s, tz) => {
      try {
        const guess = Date.UTC(y, mo, d, h, mi, s);
        const off = tzOffset(tz, guess);
        let t = guess - off;
        const off2 = tzOffset(tz, t);
        if (off2 !== off) t = guess - off2;
        return new Date(t);
      } catch (_) {
        // ismeretlen időzóna-név (pl. Windows-név): helyi időnek vesszük
        return new Date(y, mo, d, h, mi, s);
      }
    };
    const parseDt = (o) => {
      if (!o) return null;
      const v = o.v.trim();
      if (o.p.VALUE === 'DATE' || /^\d{8}$/.test(v)) {
        return { t: new Date(+v.slice(0, 4), +v.slice(4, 6) - 1, +v.slice(6, 8)), allDay: true };
      }
      const m = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})(Z?)$/.exec(v);
      if (!m) return null;
      const [y, mo, d, h, mi, s] = m.slice(1, 7).map(Number);
      if (m[7]) return { t: new Date(Date.UTC(y, mo - 1, d, h, mi, s)), allDay: false };
      const tz = o.p.TZID;
      if (!tz || tz === LOCAL_TZ) return { t: new Date(y, mo - 1, d, h, mi, s), allDay: false };
      return { t: zoned(y, mo - 1, d, h, mi, s, tz), allDay: false };
    };
    const parseDuration = (v) => {
      const m = /^([+-])?P(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$/.exec(v || '');
      if (!m) return 0;
      const ms = ((+m[2] || 0) * 7 * 86400 + (+m[3] || 0) * 86400 + (+m[4] || 0) * 3600 + (+m[5] || 0) * 60 + (+m[6] || 0)) * 1000;
      return m[1] === '-' ? -ms : ms;
    };
    const parseIcs = (text, feed) => {
      const lines = text.replace(/\r?\n[ \t]/g, '').split(/\r?\n/);
      const raw = [];
      let cur = null;
      let depth = 0; // a VEVENT-en belüli blokkok (pl. VALARM) kihagyása
      for (const line of lines) {
        if (line === 'BEGIN:VEVENT') { cur = { EXDATE: [] }; depth = 0; continue; }
        if (line === 'END:VEVENT') { if (cur) raw.push(cur); cur = null; continue; }
        if (!cur) continue;
        if (line.startsWith('BEGIN:')) { depth++; continue; }
        if (line.startsWith('END:')) { depth--; continue; }
        if (depth > 0) continue;
        const kv = splitProp(line);
        if (!kv) continue;
        const [head, val] = kv;
        const [name, ...params] = head.split(';');
        const p = {};
        for (const x of params) {
          const i = x.indexOf('=');
          if (i > 0) p[x.slice(0, i).toUpperCase()] = x.slice(i + 1).replace(/^"|"$/g, '');
        }
        const N = name.toUpperCase();
        if (N === 'EXDATE') for (const v of val.split(',')) cur.EXDATE.push({ v, p });
        else cur[N] = { v: val, p };
      }
      const out = [];
      // Névnap: évente ismétlődő, egész napos bejegyzés, aminek a címe csak keresztnév vagy
      // keresztnevek felsorolása („Rózsa”, „Konrád, Zelmira”), vagy a címében benne van, hogy névnap.
      // Ezek nem kapnak pöttyöt a naptárban, és a nap listájában külön sorban látszanak.
      const NAMES_RE = /^\p{Lu}\p{Ll}+(?:\s*,\s*\p{Lu}\p{Ll}+)*$/u;
      const isNameDay = (summary, allDay, rrule) =>
        /n[ée]vnap|name ?day/i.test(summary) || (allDay && !!rrule && rrule.FREQ === 'YEARLY' && NAMES_RE.test(summary.trim()));
      for (const r of raw) {
        if (r.STATUS && r.STATUS.v.toUpperCase() === 'CANCELLED') continue;
        const s = parseDt(r.DTSTART);
        if (!s) continue;
        const e = parseDt(r.DTEND);
        let dur = e ? e.t - s.t : r.DURATION ? parseDuration(r.DURATION.v) : s.allDay ? 86400000 : 0;
        if (dur < 0) dur = 0;
        let rrule = null;
        if (r.RRULE) rrule = Object.fromEntries(r.RRULE.v.split(';').map((x) => x.split('=')).map(([k, v]) => [k.toUpperCase(), v]));
        const summary = r.SUMMARY ? unescapeIcs(r.SUMMARY.v) : '—';
        out.push({
          uid: r.UID ? r.UID.v : '',
          nameday: isNameDay(summary, s.allDay, rrule),
          rid: r['RECURRENCE-ID'] ? parseDt(r['RECURRENCE-ID']) : null,
          summary,
          location: r.LOCATION ? unescapeIcs(r.LOCATION.v) : '',
          start: s.t, allDay: s.allDay, dur, rrule,
          exdates: r.EXDATE.map(parseDt).filter(Boolean).map((x) => +x.t),
          feed,
        });
      }
      // az ismétlődő esemény egyedileg módosított alkalmai: a sorozatból ezeket kihagyjuk
      const overridden = {};
      for (const ev of out) if (ev.rid) (overridden[ev.uid] || (overridden[ev.uid] = new Set())).add(+ev.rid.t);
      for (const ev of out) if (!ev.rid && ev.rrule) ev.overridden = overridden[ev.uid] || new Set();
      const nameUids = new Set(out.filter((ev) => ev.nameday && !ev.rid).map((ev) => ev.uid));
      for (const ev of out) if (ev.rid && nameUids.has(ev.uid)) ev.nameday = true;
      return out;
    };

    // ── ismétlődés kibontása egy időszakra (DAILY, WEEKLY, MONTHLY, YEARLY; INTERVAL, COUNT, UNTIL, BYDAY, BYMONTHDAY) ──
    const WD = { SU: 0, MO: 1, TU: 2, WE: 3, TH: 4, FR: 5, SA: 6 };
    const expand = (ev, from, to) => {
      const out = [];
      const push = (t) => {
        if (ev.exdates.includes(+t) || (ev.overridden && ev.overridden.has(+t))) return;
        const end = new Date(+t + ev.dur);
        if ((end > from || +end === +t && t >= from) && t < to) out.push({ ev, s: t, e: end });
      };
      if (!ev.rrule) { push(ev.start); return out; }
      const r = ev.rrule;
      const start = ev.start;
      const interval = Math.max(1, +(r.INTERVAL || 1));
      const count = r.COUNT ? +r.COUNT : Infinity;
      const until = r.UNTIL ? (parseDt({ v: r.UNTIL, p: {} }) || {}).t : null;
      const hh = start.getHours(), mm = start.getMinutes(), ss = start.getSeconds();
      const at = (y, m, d) => new Date(y, m, d, hh, mm, ss);
      const days = r.BYDAY ? r.BYDAY.split(',') : null;
      const nth = (y, m, spec) => {
        const mm2 = /^([+-]?\d+)?(SU|MO|TU|WE|TH|FR|SA)$/.exec(spec);
        if (!mm2) return [];
        const wd = WD[mm2[2]];
        const all = [];
        const first = new Date(y, m, 1);
        for (let d = new Date(first); d.getMonth() === first.getMonth(); d.setDate(d.getDate() + 1)) {
          if (d.getDay() === wd) all.push(at(d.getFullYear(), d.getMonth(), d.getDate()));
        }
        if (!mm2[1]) return all;
        const n = +mm2[1];
        const hit = n > 0 ? all[n - 1] : all[all.length + n];
        return hit ? [hit] : [];
      };
      // hosszú, darabszám nélküli sorozatoknál a mutatott időszak elé ugrunk
      let i0 = 0;
      const periodDays = { DAILY: 1, WEEKLY: 7 }[r.FREQ];
      if (periodDays && count === Infinity && from > start) {
        i0 = Math.max(0, Math.floor((from - start) / (periodDays * interval * 86400000)) - 1);
      }
      let n = 0;
      for (let i = i0; i < i0 + 3000 && n < count; i++) {
        let cands = [];
        if (r.FREQ === 'DAILY') {
          cands = [at(start.getFullYear(), start.getMonth(), start.getDate() + i * interval)];
        } else if (r.FREQ === 'WEEKLY') {
          const base = at(start.getFullYear(), start.getMonth(), start.getDate() + i * 7 * interval);
          if (!days) cands = [base];
          else {
            const mon = addDays(base, -((base.getDay() + 6) % 7));
            cands = days.map((d) => WD[d.slice(-2)]).filter((w) => w != null)
              .map((w) => at(mon.getFullYear(), mon.getMonth(), mon.getDate() + ((w + 6) % 7)))
              .sort((a, b) => a - b);
          }
        } else if (r.FREQ === 'MONTHLY') {
          const first = new Date(start.getFullYear(), start.getMonth() + i * interval, 1);
          const y = first.getFullYear(), m = first.getMonth();
          if (days) cands = days.flatMap((d) => nth(y, m, d));
          else if (r.BYMONTHDAY) {
            cands = r.BYMONTHDAY.split(',').map(Number)
              .map((d) => (d > 0 ? at(y, m, d) : at(y, m + 1, d + 1)))
              .filter((t) => t.getMonth() === m);
          } else {
            const t = at(y, m, start.getDate());
            if (t.getMonth() === m) cands = [t];
          }
          cands.sort((a, b) => a - b);
        } else if (r.FREQ === 'YEARLY') {
          const t = at(start.getFullYear() + i * interval, start.getMonth(), start.getDate());
          if (t.getMonth() === start.getMonth()) cands = [t];
        } else {
          push(start);
          break;
        }
        let stop = false;
        for (const t of cands) {
          if (t < start) continue;
          if ((until && t > until) || n >= count || t >= to) { stop = true; break; }
          n++;
          push(t);
        }
        if (stop) break;
      }
      return out;
    };

    // ── kirajzolás ──
    const gridStart = () => addDays(view, -((view.getDay() + 6) % 7));
    const occurrences = (from, to) => calEvents.flatMap((ev) => expand(ev, from, to));
    const dayOcc = (occ, d) => {
      const a = d, b = addDays(d, 1);
      return occ.filter((o) => o.s < b && (o.e > a || (+o.e === +o.s && o.s >= a)))
        .sort((x, y) => (y.ev.allDay - x.ev.allDay) || (x.s - y.s));
    };
    const timeOf = (d) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

    const renderCal = () => {
      const g0 = gridStart();
      const g1 = addDays(g0, 42);
      const occ = occurrences(g0, g1);
      const today = new Date();
      $('calMonth').textContent = new Intl.DateTimeFormat(locale, { year: 'numeric', month: 'long' }).format(view).toUpperCase();
      const wdNames = [...Array(7)].map((_, k) => new Intl.DateTimeFormat(locale, { weekday: 'narrow' })
        .format(new Date(2024, 0, 1 + k)).toUpperCase());
      let html = `<span class="cg-wk cg-h">${T('calWeekShort')}</span>` + wdNames.map((n, k) => `<span class="cg-h${k > 4 ? ' we' : ''}">${n}</span>`).join('');
      for (let w = 0; w < 6; w++) {
        const monday = addDays(g0, w * 7);
        html += `<span class="cg-wk">${isoWeek(monday)}</span>`;
        for (let k = 0; k < 7; k++) {
          const d = addDays(monday, k);
          const list = dayOcc(occ, d);
          const real = list.filter((o) => !o.ev.nameday);
          const colors = real.slice(0, 6).map((o) => o.ev.feed.color);
          const cls = ['cg-d'];
          if (d.getMonth() !== view.getMonth()) cls.push('out');
          if (k > 4) cls.push('we');
          if (sameDay(d, today)) cls.push('today');
          if (sameDay(d, sel)) cls.push('sel');
          const tip = real.length ? ` title="${real.length} ${T('calEventsN')}"` : '';
          html += `<button class="${cls.join(' ')}" data-day="${+d}"${tip}><span>${d.getDate()}</span>` +
            `<i class="cg-dots">${colors.map((c) => `<b style="--c:${esc(c)}"></b>`).join('')}</i></button>`;
        }
      }
      $('calGrid').innerHTML = html;

      // a kiválasztott nap eseményei
      const head = new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric', weekday: 'long' }).format(sel).toUpperCase();
      const dayAll = dayOcc(occurrences(sel, addDays(sel, 1)), sel);
      const names = [...new Set(dayAll.filter((o) => o.ev.nameday).map((o) => o.ev.summary.replace(/\s*n[ée]vnap(ja)?\s*/ig, ' ').trim()))];
      const list = dayAll.filter((o) => !o.ev.nameday);
      const nameHtml = names.length ? `<div class="cal-names">${svg('user')}<span>${T('calNameDay')}</span><b>${esc(names.join(', '))}</b></div>` : '';
      let body;
      if (!feeds.length) {
        body = connectHtml();
      } else if (calLoading && !calAt) {
        body = `<div class="wf-note">${T('wLoading')}</div>`;
      } else if (!list.length) {
        body = `${nameHtml}<div class="cal-empty">${T('calNoEvents')}</div>`;
      } else {
        body = `${nameHtml}<div class="cal-events">${list.map((o) => {
          const ev = o.ev;
          const when = ev.allDay ? T('calAllDay') : `${timeOf(o.s)}${ev.dur ? `<em>${timeOf(o.e)}</em>` : ''}`;
          const now = new Date();
          const live = !ev.allDay && o.s <= now && o.e > now;
          return `<div class="cal-ev${live ? ' live' : ''}" style="--c:${esc(ev.feed.color)}">
              <span class="cal-when">${when}</span>
              <span class="cal-what"><b>${esc(ev.summary)}</b>${ev.location ? `<small>${esc(ev.location)}</small>` : ''}</span>
            </div>`;
        }).join('')}</div>`;
      }
      $('calDay').innerHTML = `<h4 class="cal-dh">// ${esc(head)}</h4>${body}${feeds.length && addOpen ? connectHtml() : ''}`;
      renderFoot();
    };

    const connectHtml = () => `
      <div class="cal-connect">
        <div class="cal-c-title">${svg('calendar')}<b>${T(feeds.length ? 'calAddTitle' : 'calConnectTitle')}</b></div>
        <ol class="cal-steps">${T('calSteps')}</ol>
        <div class="cal-form">
          <input id="calUrl" type="text" spellcheck="false" placeholder="https://calendar.google.com/calendar/ical/…/basic.ics">
          <button class="cal-btn" id="calAdd">${T('calConnect')}</button>
        </div>
        <div class="cal-err" id="calErr"></div>
        <button class="cal-link" id="calHelp">${svg('globe')}<span>${T('calOpenGoogle')}</span></button>
      </div>`;

    const renderFoot = () => {
      if (!feeds.length) { $('calFoot').innerHTML = `<span class="cal-src">${T('calNoFeeds')}</span>`; return; }
      const chips = feeds.map((f, i) => {
        const st = calStatus.find((x) => x.name === f.name);
        const bad = st && !st.ok;
        return `<span class="cal-chip${bad ? ' bad' : ''}" style="--c:${esc(f.color)}" title="${bad ? esc(T('calFeedError', { e: st.error })) : ''}">` +
          `<i></i>${esc(f.name)}<button data-remove="${i}" title="${T('calRemove')}">${svg('close')}</button></span>`;
      }).join('');
      const t = calAt ? new Date(calAt) : null;
      $('calFoot').innerHTML = `<div class="cal-chips">${chips}<button class="cal-plus${addOpen ? ' on' : ''}" id="calPlus" title="${T('calAddTitle')}">${svg('plus')}</button></div>
        <button class="cal-refresh${calLoading ? ' busy' : ''}" id="calRefresh">${svg('reload')}<span>${t ? `${pad(t.getHours())}:${pad(t.getMinutes())}` : '--:--'}</span></button>`;
    };

    // kattintások a naptárnézetben
    $('vCal').addEventListener('click', (e) => {
      const day = e.target.closest('[data-day]');
      if (day) {
        sel = new Date(Number(day.dataset.day));
        if (sel.getMonth() !== view.getMonth()) view = new Date(sel.getFullYear(), sel.getMonth(), 1);
        return renderCal();
      }
      const rm = e.target.closest('[data-remove]');
      if (rm) {
        // második kattintásra törli (véletlen törlés ellen)
        if (rm.classList.contains('armed')) return post({ type: 'cal-remove', i: Number(rm.dataset.remove) });
        rm.classList.add('armed');
        setTimeout(() => rm.isConnected && rm.classList.remove('armed'), 2500);
        return;
      }
      const id = (e.target.closest('button') || {}).id;
      if (id === 'calPrev') { view = new Date(view.getFullYear(), view.getMonth() - 1, 1); renderCal(); }
      if (id === 'calNext') { view = new Date(view.getFullYear(), view.getMonth() + 1, 1); renderCal(); }
      if (id === 'calToday') {
        const t = new Date();
        sel = new Date(t.getFullYear(), t.getMonth(), t.getDate());
        view = new Date(t.getFullYear(), t.getMonth(), 1);
        renderCal();
      }
      if (id === 'calRefresh') { calLoading = true; renderFoot(); post({ type: 'cal-fetch', force: true }); }
      if (id === 'calPlus') { addOpen = !addOpen; renderCal(); if (addOpen) $('calUrl').focus(); }
      if (id === 'calHelp') post({ type: 'cal-help' });
      if (id === 'calAdd') submitUrl();
    });
    $('vCal').addEventListener('keydown', (e) => { if (e.key === 'Enter' && e.target.id === 'calUrl') submitUrl(); });
    // görgővel lapozás a hónapok között
    $('calGrid').addEventListener('wheel', (e) => {
      e.preventDefault();
      view = new Date(view.getFullYear(), view.getMonth() + (e.deltaY > 0 ? 1 : -1), 1);
      renderCal();
    }, { passive: false });
    const submitUrl = () => {
      const v = ($('calUrl').value || '').trim();
      if (!/^(https|webcal):\/\//i.test(v)) { $('calErr').textContent = T('calErrUrl'); return; }
      $('calErr').textContent = '';
      calLoading = true;
      if (demo) return toastDemo();
      post({ type: 'cal-add', url: v });
    };
    const toastDemo = () => { $('calErr').textContent = 'Demo'; };

    const onCalData = (list) => {
      calLoading = false;
      calAt = Date.now();
      calStatus = (list || []).map(({ name, color, ok, error }) => ({ name, color, ok, error }));
      calEvents = [];
      for (const f of list || []) {
        if (!f.ok) continue;
        try { calEvents.push(...parseIcs(f.ics, { name: f.name, color: f.color })); } catch (e) { post({ type: 'log', text: `iCal: ${e.message}` }); }
      }
      if (calStatus.some((x) => x.ok)) addOpen = false;
      renderCal();
    };

    const h = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      if (msg.type === 'weather-open') {
        const next = msg.weather || { city: 'Budapest' };
        if (wcfg && JSON.stringify(wcfg) !== JSON.stringify(next)) last = null;
        wcfg = next;
        showView('vWeather');
        loadWeather(false);
      }
      if (msg.type === 'calendar-open') {
        feeds = msg.feeds || [];
        if (!feeds.length) { calEvents = []; calStatus = []; calAt = 0; }
        const t = new Date();
        sel = new Date(t.getFullYear(), t.getMonth(), t.getDate());
        view = new Date(t.getFullYear(), t.getMonth(), 1);
        addOpen = false;
        showView('vCal');
        renderCal();
        if (feeds.length) { calLoading = true; renderFoot(); post({ type: 'cal-fetch', force: false }); }
      }
      if (msg.type === 'cal-feeds') {
        feeds = msg.feeds || [];
        if (!feeds.length) { calEvents = []; calStatus = []; calAt = 0; }
        renderCal();
      }
      if (msg.type === 'cal-data') onCalData(msg.feeds);
      if (msg.type === 'cal-error') {
        calLoading = false;
        const el = $('calErr');
        if (el) el.textContent = T(msg.error === 'dup' ? 'calErrDup' : 'calErrUrl');
        renderFoot();
      }
    };
    h.demo = () => {
      const part = new URLSearchParams(location.search).get('view');
      if (part !== 'calendar') return h({ type: 'weather-open', weather: { city: 'Budapest' } });
      h({ type: 'calendar-open', feeds: [{ name: 'Google Naptár', color: '#00e1ff' }, { name: 'Munka', color: '#f3e600' }] });
      const d = new Date();
      const ymd = (x) => `${x.getFullYear()}${pad(x.getMonth() + 1)}${pad(x.getDate())}`;
      const ics1 = ['BEGIN:VCALENDAR', 'BEGIN:VEVENT', 'UID:a', `DTSTART:${ymd(d)}T090000`, `DTEND:${ymd(d)}T093000`, 'SUMMARY:Napi egyeztetés',
        'RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:b', `DTSTART;VALUE=DATE:${ymd(d)}`, `DTEND;VALUE=DATE:${ymd(addDays(d, 1))}`, 'SUMMARY:Szülinap – Anna', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:n', `DTSTART;VALUE=DATE:${ymd(d)}`, `DTEND;VALUE=DATE:${ymd(addDays(d, 1))}`, 'RRULE:FREQ=YEARLY', 'SUMMARY:Jenő\, Ágota', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:c', `DTSTART:${ymd(addDays(d, 2))}T140000`, `DTEND:${ymd(addDays(d, 2))}T153000`, 'SUMMARY:Fotózás', 'LOCATION:Budapest\\, Margitsziget', 'END:VEVENT',
        'END:VCALENDAR'].join('\r\n');
      const ics2 = ['BEGIN:VCALENDAR', 'BEGIN:VEVENT', 'UID:d', `DTSTART:${ymd(d)}T130000`, `DTEND:${ymd(d)}T140000`, 'SUMMARY:Ügyféltalálkozó', 'LOCATION:Iroda', 'END:VEVENT',
        'BEGIN:VEVENT', 'UID:e', `DTSTART:${ymd(addDays(d, 5))}T100000`, `DTEND:${ymd(addDays(d, 5))}T110000`, 'SUMMARY:Havi riport', 'RRULE:FREQ=MONTHLY', 'END:VEVENT',
        'END:VCALENDAR'].join('\r\n');
      setTimeout(() => onCalData([
        { name: 'Google Naptár', color: '#00e1ff', ok: true, ics: ics1 },
        { name: 'Munka', color: '#f3e600', ok: true, ics: ics2 },
      ]), 300);
    };
    return h;
  }

  window.HUB.parts.fly = buildFly;
})();
