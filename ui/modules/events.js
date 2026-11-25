// Események: a következő 5 nap (kompakt nézetben 3; vízszintes panelen a magasságtól függően 3–4, illetve normál nézetben 2×3–2×4) soronként, a nap száma és rövid neve („05 SZE”), és az aznapi események száma. Kattintásra
// az információs panel mellett kinyílik a következő egy hét eseményeinek időrendi listája
// (parts/flyout.js, "events" nézet). Az adatot a csatolt naptárakból (iCal) a program tölti le
// és küldi (cal-data); az értelmezés a közös modulban van (ical.js).
(() => {
  const { defineModule, T, esc, locale, lang } = window.HUB;

  // a nap rövid neve, nagybetűvel; magyarul egységesen hárombetűs (a gyári rövidítés „P”, „V”, „SZE” vegyesen)
  const HU_DAYS = ['VAS', 'HÉT', 'KED', 'SZE', 'CSÜ', 'PÉN', 'SZO'];
  const dayName = (d) => (lang === 'hu' ? HU_DAYS[d.getDay()]
    : new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(d).replace('.', '').toUpperCase());
  const startOfDay = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate());

  defineModule({
    id: 'events',
    title: 'secEvents',
    name: 'sEvents',
    hint: 'sEventsHint',
    tip: 'evOpen',
    compact: 'calendar',
    defaultOn: false,
    className: 'events',
    html: () => '<div class="ev-list" id="evList"></div>',
    setup(ctx) {
      const { parseIcs, expand, addDays } = window.HUB.ical;
      const list = ctx.el.querySelector('#evList');
      let state = 'loading';   // loading | none (nincs csatolt naptár) | error (egyik sem töltődött le) | ready
      let events = [];
      let shownDay = '';
      let compact = false;
      // vízszintes panelen a kiférő sorok száma (a panel magasságától függően 3 vagy 4): kompakt
      // nézetben ennyi nap egy oszlopban, normálban kétszer ennyi két oszlopban
      let rows = 3;
      const daysNow = () => (ctx.horiz ? (compact ? rows : rows * 2) : compact ? 3 : 5);

      // egy nap eseményei (a névnapok nem számítanak)
      const countOn = (occ, d) => {
        const a = d, b = addDays(d, 1);
        return occ.filter((o) => !o.ev.nameday && o.s < b && (o.e > a || (+o.e === +o.s && o.s >= a))).length;
      };

      const render = () => {
        const today = startOfDay(new Date());
        shownDay = String(+today);
        if (state === 'none' || state === 'error') {
          list.innerHTML = `<div class="ev-note">${esc(T(state === 'none' ? 'evNoFeedsShort' : 'evOffline'))}</div>`;
          return;
        }
        const days = daysNow();
        const occ = state === 'ready' ? events.flatMap((ev) => expand(ev, today, addDays(today, days))) : [];
        list.style.setProperty('--rows', rows);
        list.innerHTML = [...Array(days)].map((_, i) => {
          const d = addDays(today, i);
          const n = state === 'ready' ? countOn(occ, d) : null;
          return `<div class="ev-row${i === 0 ? ' today' : ''}${n === 0 ? ' zero' : ''}">` +
            `<span class="ev-d"><i class="ev-dn">${String(d.getDate()).padStart(2, '0')}</i> ${esc(dayName(d))}</span><b class="ev-n">${n == null ? '…' : n}</b></div>`;
        }).join('');
        fitRows();
      };
      // vízszintes panelen: hány sor fér ki teljesen a szekció aljáig (legfeljebb 4)
      const fitRows = () => {
        const first = list.querySelector('.ev-row');
        // rejtett szekciónál (még nincs mérete) nem mérünk
        if (!ctx.horiz || !first || !first.offsetHeight) return;
        const avail = ctx.el.getBoundingClientRect().bottom - 4 - list.getBoundingClientRect().top;
        const step = first.offsetHeight + 3;
        const want = Math.max(1, Math.min(4, Math.floor((avail + 3) / step)));
        if (want !== rows) { rows = want; render(); ctx.fit(); }
      };
      window.addEventListener('resize', () => { if (ctx.horiz) fitRows(); });

      const onData = (feeds) => {
        feeds = feeds || [];
        // egyik naptár sem töltődött le (pl. nincs internet): a korábbi adat marad, ha volt,
        // különben jelezzük, hogy nem elérhető (ne tűnjön úgy, hogy nincs esemény)
        if (feeds.length > 0 && !feeds.some((f) => f.ok)) {
          if (state === 'ready') return;
          state = 'error';
          render();
          ctx.fit();
          return;
        }
        events = [];
        for (const f of feeds) {
          if (!f.ok) continue;
          try { events.push(...parseIcs(f.ics, { name: f.name, color: f.color })); } catch (e) { ctx.post({ type: 'log', text: `iCal: ${e.message}` }); }
        }
        state = feeds.length > 0 ? 'ready' : 'none';
        render();
        ctx.fit();
      };

      ctx.el.addEventListener('click', () => {
        if (ctx.demo) return ctx.toast('Demo: events');
        ctx.post({ type: 'events-open', ...ctx.rect(ctx.el) });
      });
      render();

      return {
        message(msg) {
          if (msg.type === 'cal-data') onData(msg.feeds);
          // kiemelve, amíg a lista nyitva van
          if (msg.type === 'fly-state') ctx.el.classList.toggle('open', !!msg.open && msg.kind === 'events');
        },
        sections(s) {
          const c = (s.compact || []).includes('events');
          // (a bekapcsolt szekció is itt kap először méretet)
          if (c !== compact) { compact = c; render(); } else fitRows();
        },
        // éjfélkor a sorok a következő napra lépnek
        tick(now) {
          if (state !== 'loading' && shownDay !== String(+startOfDay(now))) render();
        },
        demo() {
          const d = new Date();
          const ymd = (x) => `${x.getFullYear()}${String(x.getMonth() + 1).padStart(2, '0')}${String(x.getDate()).padStart(2, '0')}`;
          const ev = (uid, day, h, name) => ['BEGIN:VEVENT', `UID:${uid}`, `DTSTART:${ymd(addDays(d, day))}T${h}0000`, `DTEND:${ymd(addDays(d, day))}T${h}3000`, `SUMMARY:${name}`, 'END:VEVENT'];
          const ics = ['BEGIN:VCALENDAR', ...ev('1', 0, '09', 'Napi egyeztetés'), ...ev('2', 0, '13', 'Ügyfélhívás'), ...ev('3', 2, '14', 'Fotózás'), ...ev('4', 3, '10', 'Havi riport'), ...ev('5', 3, '16', 'Edzés'), ...ev('6', 3, '18', 'Vacsora'), 'END:VCALENDAR'].join('\r\n');
          onData([{ name: 'Demo', color: '#00e1ff', ok: true, ics }]);
        },
      };
    },
    strings: {
      hu: {
        secEvents: '// ESEMÉNYEK', sEvents: 'Események', evOpen: 'A következő hét eseményei',
        sEventsHint: 'A következő 5 nap (kompakt nézetben 3) eseményeinek száma soronként; kattintásra a következő egy hét eseményei időrendben. Csatolt naptárat igényel (a Dátum naptárában csatolható).',
        evNoFeedsShort: 'NINCS NAPTÁR', evOffline: 'NEM ELÉRHETŐ',evNoFeeds: 'Nincs csatolt naptár. A Dátum szekció naptárában csatolhatod a Google Naptáradat.',
        evNone: 'Nincs esemény a következő héten.', evToday: 'MA', evTomorrow: 'HOLNAP',
      },
      en: {
        secEvents: '// EVENTS', sEvents: 'Events', evOpen: 'Events in the next week',
        sEventsHint: 'The number of events on each of the next 5 days (3 in compact view); click for the events of the next week in order. Needs a connected calendar (connect one in the Date calendar).',
        evNoFeedsShort: 'NO CALENDAR', evOffline: 'UNAVAILABLE',evNoFeeds: 'No calendar connected. You can connect your Google Calendar in the calendar of the Date section.',
        evNone: 'No events in the next week.', evToday: 'TODAY', evTomorrow: 'TOMORROW',
      },
      pl: {
        secEvents: '// WYDARZENIA', sEvents: 'Wydarzenia', evOpen: 'Wydarzenia w najbliższym tygodniu',
        sEventsHint: 'Liczba wydarzeń w każdym z najbliższych 5 dni (3 w widoku kompaktowym); po kliknięciu lista wydarzeń na najbliższy tydzień. Wymaga połączonego kalendarza (połącz go w kalendarzu sekcji Data).',
        evNoFeedsShort: 'BRAK KALENDARZA', evOffline: 'NIEDOSTĘPNE',evNoFeeds: 'Brak połączonego kalendarza. Kalendarz Google możesz połączyć w kalendarzu sekcji Data.',
        evNone: 'Brak wydarzeń w najbliższym tygodniu.', evToday: 'DZIŚ', evTomorrow: 'JUTRO',
      },
      de: {
        secEvents: '// TERMINE', sEvents: 'Termine', evOpen: 'Termine der nächsten Woche',
        sEventsHint: 'Die Zahl der Termine an jedem der nächsten 5 Tage (3 in der Kompaktansicht); ein Klick zeigt die Termine der nächsten Woche der Reihe nach. Braucht einen verbundenen Kalender (im Kalender des Datums verbinden).',
        evNoFeedsShort: 'KEIN KALENDER', evOffline: 'NICHT VERFÜGBAR', evNoFeeds: 'Kein Kalender verbunden. Deinen Google Kalender kannst du im Kalender des Datum-Abschnitts verbinden.',
        evNone: 'Keine Termine in der nächsten Woche.', evToday: 'HEUTE', evTomorrow: 'MORGEN',
      },
      es: {
        secEvents: '// EVENTOS', sEvents: 'Eventos', evOpen: 'Eventos de la próxima semana',
        sEventsHint: 'El número de eventos de cada uno de los próximos 5 días (3 en vista compacta); al hacer clic, los eventos de la próxima semana en orden. Requiere un calendario conectado (se conecta en el calendario de Fecha).',
        evNoFeedsShort: 'SIN CALENDARIO', evOffline: 'NO DISPONIBLE', evNoFeeds: 'No hay ningún calendario conectado. Puedes conectar tu Google Calendar en el calendario de la sección Fecha.',
        evNone: 'No hay eventos la próxima semana.', evToday: 'HOY', evTomorrow: 'MAÑANA',
      },
    },
  });
})();
