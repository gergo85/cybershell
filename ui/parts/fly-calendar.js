// Az oldalpanel naptár nézetei: a havi naptár (a panel Dátum moduljára kattintva) és a következő
// hét eseményei (az Események modulra kattintva). Mindkettő a csatolt iCal-naptárakból dolgozik:
// a letöltést a program végzi, itt csak értelmezzük (ical.js) és kirajzoljuk.
(() => {
  const { $, locale, T, demo, post, svg, esc, pad, hhmm } = window.HUB;

  window.HUB.flyViews.push({
    html: () => `
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
      </div>
      <div class="fview" id="vEvents">
        <header class="wf-head">
          <span class="wf-title">${T('secEvents')}</span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="cal-day" id="evBody"></div>
        <footer class="cal-foot" id="evFoot"></footer>
      </div>`,

    setup({ show, current }) {
      const { parseIcs, expand, addDays } = window.HUB.ical;
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
      const isoWeek = (d) => {
        const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
        const day = t.getUTCDay() || 7;
        t.setUTCDate(t.getUTCDate() + 4 - day);
        return Math.ceil(((t - new Date(Date.UTC(t.getUTCFullYear(), 0, 1))) / 86400000 + 1) / 7);
      };

      // ── kirajzolás ──
      const gridStart = () => addDays(view, -((view.getDay() + 6) % 7));
      const occurrences = (from, to) => calEvents.flatMap((ev) => expand(ev, from, to));
      const dayOcc = (occ, d) => {
        const a = d, b = addDays(d, 1);
        return occ.filter((o) => o.s < b && (o.e > a || (+o.e === +o.s && o.s >= a)))
          .sort((x, y) => (y.ev.allDay - x.ev.allDay) || (x.s - y.s));
      };
      const timeOf = (d) => hhmm(d);

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
            const tip = real.length ? ` title="${T('calEventsN', { n: real.length })}"` : '';
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
          <button class="cal-refresh${calLoading ? ' busy' : ''}" id="calRefresh">${svg('reload')}<span>${t ? hhmm(t) : '--:--'}</span></button>`;
      };

      // ── a következő egy hét eseményei időrendben (az Események modulra kattintva) ──
      const renderEvents = () => {
        const t0 = new Date();
        const today = new Date(t0.getFullYear(), t0.getMonth(), t0.getDate());
        let html = '';
        if (!feeds.length) {
          html = `<div class="cal-empty">${T('evNoFeeds')}</div>`;
        } else if (calLoading && !calAt) {
          html = `<div class="wf-note">${T('wLoading')}</div>`;
        } else {
          const occ = calEvents.flatMap((ev) => expand(ev, today, addDays(today, 7))).filter((o) => !o.ev.nameday);
          for (let i = 0; i < 7; i++) {
            const d = addDays(today, i);
            const day = dayOcc(occ, d);
            if (!day.length) continue;
            const head = new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric', weekday: 'long' }).format(d).toUpperCase();
            html += `<h4 class="cal-dh">// ${esc(head)}${i < 2 ? ` · <span class="ev-today">${T(i === 0 ? 'evToday' : 'evTomorrow')}</span>` : ''}</h4><div class="cal-events">${day.map((o) => {
              const ev = o.ev;
              const when = ev.allDay ? T('calAllDay') : `${timeOf(o.s)}${ev.dur ? `<em>${timeOf(o.e)}</em>` : ''}`;
              const live = !ev.allDay && o.s <= t0 && o.e > t0;
              const past = !ev.allDay && o.e <= t0;
              return `<div class="cal-ev${live ? ' live' : ''}${past ? ' past' : ''}" style="--c:${esc(ev.feed.color)}">
                  <span class="cal-when">${when}</span>
                  <span class="cal-what"><b>${esc(ev.summary)}</b>${ev.location ? `<small>${esc(ev.location)}</small>` : ''}</span>
                </div>`;
            }).join('')}</div>`;
          }
          if (!html) html = `<div class="cal-empty">${T('evNone')}</div>`;
        }
        $('evBody').innerHTML = html;
        if (!feeds.length) {
          $('evFoot').innerHTML = '';
        } else {
          const t = calAt ? new Date(calAt) : null;
          $('evFoot').innerHTML = `<span class="cal-src">${T('secEvents')}</span>
            <button class="cal-refresh${calLoading ? ' busy' : ''}" id="evRefresh">${svg('reload')}<span>${t ? hhmm(t) : '--:--'}</span></button>`;
        }
        fitEvents();
      };
      // az ablak magassága a listához igazodik (kevés eseménynél ne maradjon üres hely alul; sok
      // eseménynél a teljes magasság, a lista görgethető): a tartalom természetes magassága megy a programnak
      const fitEvents = () => {
        if (demo || current() !== 'vEvents') return;
        const body = $('evBody');
        body.style.flex = 'none';
        const need = [...$('vEvents').children].reduce((s, el) => s + el.offsetHeight, 0) + 2;
        body.style.flex = '';
        post({ type: 'fly-fit', h: need });
      };
      $('vEvents').addEventListener('click', (e) => {
        if ((e.target.closest('button') || {}).id === 'evRefresh') { calLoading = true; renderEvents(); post({ type: 'cal-fetch', force: true }); }
      });

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
        if (demo) { $('calErr').textContent = 'Demo'; return; }
        post({ type: 'cal-add', url: v });
      };

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
        if (current() === 'vEvents') renderEvents();
      };

      // a csatolt naptárak listája; ha nincs egy sem, a korábbi letöltés is érvénytelen
      const setFeeds = (list) => {
        feeds = list || [];
        if (!feeds.length) { calEvents = []; calStatus = []; calAt = 0; }
      };

      const message = (msg) => {
        if (msg.type === 'calendar-open') {
          setFeeds(msg.feeds);
          const t = new Date();
          sel = new Date(t.getFullYear(), t.getMonth(), t.getDate());
          view = new Date(t.getFullYear(), t.getMonth(), 1);
          addOpen = false;
          show('vCal');
          renderCal();
          if (feeds.length) { calLoading = true; renderFoot(); post({ type: 'cal-fetch', force: false }); }
        }
        if (msg.type === 'events-open') {
          setFeeds(msg.feeds);
          show('vEvents');
          if (feeds.length) { calLoading = true; post({ type: 'cal-fetch', force: false }); }
          renderEvents();
        }
        if (msg.type === 'cal-feeds') {
          setFeeds(msg.feeds);
          renderCal();
          if (current() === 'vEvents') renderEvents();
        }
        if (msg.type === 'cal-data') onCalData(msg.feeds);
        if (msg.type === 'cal-error') {
          calLoading = false;
          const el = $('calErr');
          if (el) el.textContent = T(msg.error === 'dup' ? 'calErrDup' : 'calErrUrl');
          renderFoot();
        }
      };

      // előnézet: két kitalált naptár néhány eseménnyel (ismétlődő, egész napos, névnap)
      const demoOpen = (which) => {
        if (which !== 'calendar' && which !== 'events') return false;
        message({ type: which === 'events' ? 'events-open' : 'calendar-open', feeds: [{ name: 'Google Naptár', color: '#00e1ff' }, { name: 'Munka', color: '#f3e600' }] });
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
        return true;
      };
      return { message, demo: demoOpen };
    },
  });
})();
