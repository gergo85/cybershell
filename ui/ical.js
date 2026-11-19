// Közös iCal-értelmező: a .ics szövegből eseményeket készít, és az ismétlődő eseményeket
// kibontja egy időszakra. A naptár (parts/flyout.js) és az Események modul (modules/events.js)
// is ezt használja. Használat: const { parseIcs, expand } = window.HUB.ical;
(() => {
  const LOCAL_TZ = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

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

  window.HUB.ical = { parseIcs, expand, addDays };
})();
