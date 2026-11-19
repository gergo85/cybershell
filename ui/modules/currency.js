// Valuta: átváltó a panelen. Az összeget beírod, a két listában választod a pénznemeket, a ⇅
// gomb megfordítja az irányt; alatta az árfolyam, az elmúlt 30 nap változása és grafikonja
// (kompakt nézetben csak az eredmény). Az árfolyamok az Európai Központi Bank napi adatai (Frankfurter,
// kulcs nélkül, óránként frissítve). A választható pénznemek a beállításokban állíthatók
// (config.json: currency.list); a legutóbbi összeg és irány megmarad.
(() => {
  const { defineModule, $, T, svg, esc, lang, locale, store, fetchJson, sparkPath } = window.HUB;
  const API = 'https://api.frankfurter.dev/v1';
  const KEY = 'cybershell.currency';
  // az EKB által közölt pénznemek (ennyi közül választható a beállításokban)
  const ALL = ['AUD', 'BRL', 'CAD', 'CHF', 'CNY', 'CZK', 'DKK', 'EUR', 'GBP', 'HKD', 'HUF', 'IDR', 'ILS', 'INR', 'ISK', 'JPY', 'KRW',
    'MXN', 'MYR', 'NOK', 'NZD', 'PHP', 'PLN', 'RON', 'SEK', 'SGD', 'THB', 'TRY', 'USD', 'ZAR'];
  const fullName = (c) => { try { return new Intl.DisplayNames([locale], { type: 'currency' }).of(c); } catch (_) { return c; } };
  const fmt = (n, digits) => new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(n);
  // az átváltott összeg: nagy számnál egész, kicsinél két tizedes
  const fmtAmount = (n) => fmt(n, Math.abs(n) >= 1000 ? 0 : 2);
  // a beírt összeg (szóközzel tagolva, vesszővel vagy ponttal is)
  const parseAmount = (s) => Number(String(s).replace(/\s/g, '').replace(',', '.'));
  const ymd = (d) => d.toISOString().slice(0, 10);

  // árfolyamok alapdevizánként (óránként újrakérve) és az elmúlt 30 nap párosonként (6 óránként)
  const latest = {};
  const history = {};
  const getRates = async (base) => {
    const c = latest[base];
    if (c && Date.now() - c.at < 3600e3) return c;
    const r = await fetchJson(`${API}/latest?base=${base}`);
    return (latest[base] = { at: Date.now(), date: r.date, rates: { ...r.rates, [base]: 1 } });
  };
  const getHistory = async (from, to) => {
    const k = `${from}-${to}`;
    const c = history[k];
    if (c && Date.now() - c.at < 6 * 3600e3) return c.values;
    const start = ymd(new Date(Date.now() - 30 * 86400e3));
    const r = await fetchJson(`${API}/${start}..?base=${from}&symbols=${to}`);
    const values = Object.keys(r.rates).sort().map((d) => r.rates[d][to]);
    history[k] = { at: Date.now(), values };
    return values;
  };

  defineModule({
    id: 'currency',
    title: 'secCur',
    name: 'sCur',
    hint: 'sCurHint',
    compact: 'coin',
    defaultOn: false,
    className: 'cur',
    html: () => `
      <input class="cur-amount" id="curAmount" type="text" inputmode="decimal" spellcheck="false" autocomplete="off">
      <div class="cur-pair">
        <select class="cur-sel" id="curFrom"></select>
        <button class="cur-swap" id="curSwap" title="${T('curSwap')}">${svg('swap')}</button>
        <select class="cur-sel" id="curTo"></select>
      </div>
      <div class="cur-result" id="curResult">--</div>
      <div class="cur-rate" id="curRate"><span id="curRateVal">&nbsp;</span><em id="curTrend"></em></div>
      <svg class="cur-spark" viewBox="0 0 100 24" preserveAspectRatio="none"><path d=""/></svg>`,
    setup(ctx) {
      const sec = ctx.el;
      let list = ['EUR', 'USD', 'HUF', 'GBP', 'CHF'];
      const saved = store.get(KEY) || {};
      let st = { from: saved.from || 'EUR', to: saved.to || ({ hu: 'HUF', pl: 'PLN' }[lang] || 'USD'), amount: saved.amount || '100' };
      const save = () => store.set(KEY, st);
      let seq = 0;

      const fill = () => {
        // a lista csak a beállított pénznemeket kínálja (a választott akkor is megmarad, ha kikerült)
        if (!list.includes(st.from)) st.from = list[0];
        if (!list.includes(st.to)) st.to = list.find((c) => c !== st.from) || list[0];
        for (const [id, v] of [['curFrom', st.from], ['curTo', st.to]]) {
          $(id).innerHTML = list.map((c) => `<option value="${c}" title="${esc(fullName(c))}">${c}</option>`).join('');
          $(id).value = v;
        }
        $('curAmount').value = st.amount;
      };
      const compute = async () => {
        const my = ++seq;
        const amount = parseAmount(st.amount);
        sec.classList.toggle('bad', !(amount >= 0));
        try {
          const r = await getRates(st.from);
          if (my !== seq) return;
          const rate = r.rates[st.to];
          if (rate == null) throw new Error('?');
          // csak a szám (a pénznem fölötte, a listában látszik); hosszú számnál kisebb betűvel
          const text = amount >= 0 ? fmtAmount(amount * rate) : '--';
          $('curResult').textContent = text;
          $('curResult').title = `${text} ${st.to}`;
          $('curResult').dataset.len = text.length > 9 ? 'xl' : text.length > 7 ? 'l' : '';
          // az árfolyam (1 egység ára), a súgóban a teljes alak és a dátum
          $('curRateVal').textContent = fmt(rate, rate >= 100 ? 2 : 4);
          $('curRate').title = `1 ${st.from} = ${fmt(rate, 6)} ${st.to} · ${T('curDate', { d: r.date })}`;
          sec.classList.remove('err');
        } catch (e) {
          if (my !== seq) return;
          $('curResult').textContent = '--';
          $('curRateVal').textContent = T('noData');
          $('curTrend').textContent = '';
          sec.classList.add('err');
        }
        // az elmúlt 30 nap (csak normál nézetben kell, de a lekérés olcsó és tárolt)
        const path = sec.querySelector('.cur-spark path');
        const trend = $('curTrend');
        if (st.from === st.to) { trend.textContent = ''; return path.setAttribute('d', ''); }
        try {
          const values = await getHistory(st.from, st.to);
          if (my !== seq) return;
          path.setAttribute('d', sparkPath(values, 100, 24));
          const first = values[0], last = values[values.length - 1];
          const ch = first ? (last / first - 1) * 100 : 0;
          const dir = ch > 0.05 ? 'up' : ch < -0.05 ? 'down' : '';
          sec.querySelector('.cur-spark').dataset.trend = dir;
          // a változás az elmúlt 30 napban (a grafikon színével)
          trend.textContent = `${ch > 0 ? '+' : ''}${fmt(ch, 1)}%`;
          trend.dataset.trend = dir;
          trend.title = T('curTrend', { p: trend.textContent });
        } catch (_) {
          path.setAttribute('d', '');
          trend.textContent = '';
        }
      };

      $('curAmount').addEventListener('input', () => { st.amount = $('curAmount').value; save(); compute(); });
      $('curAmount').addEventListener('keydown', (e) => { if (e.key === 'Escape') $('curAmount').blur(); });
      $('curAmount').addEventListener('focus', () => $('curAmount').select());
      $('curFrom').addEventListener('change', () => { st.from = $('curFrom').value; save(); compute(); });
      $('curTo').addEventListener('change', () => { st.to = $('curTo').value; save(); compute(); });
      $('curSwap').addEventListener('click', () => {
        st = { ...st, from: st.to, to: st.from };
        save();
        fill();
        compute();
      });
      fill();
      // az árfolyam óránként újrakérődik
      setInterval(compute, 3600e3);

      const setList = (l) => {
        if (Array.isArray(l) && l.length >= 2) list = l;
        fill();
        compute();
      };
      // vízszintes panelen normál nézetben: ha a pénznemek alatt van hely, az árfolyam oda kerül
      // (a szám és a változás egy sorban), különben a pénznemek mellett marad
      const fitRate = () => {
        if (!ctx.horiz || !sec.offsetHeight) return;
        sec.classList.remove('rate-below');
        if (sec.classList.contains('compact')) return;
        sec.classList.add('rate-below');
        const rate = $('curRate').getBoundingClientRect();
        if (rate.bottom > sec.getBoundingClientRect().bottom + 0.5) sec.classList.remove('rate-below');
        ctx.fit();
      };
      window.addEventListener('resize', fitRate);
      return {
        message(msg) {
          if (msg.type === 'init' || msg.type === 'currency') setList(msg.currency && msg.currency.list);
        },
        sections: () => requestAnimationFrame(fitRate),
        demo: () => setList(['EUR', 'USD', 'HUF', 'GBP', 'CHF']),
      };
    },

    // a beállítások Funkciók fülén: a választható pénznemek (hozzáadás, eltávolítás)
    settings: {
      title: 'curCard',
      html: () => `
        <div class="cur-chips" id="curChips"></div>
        <div class="cal-add cur-add">
          <label class="ws-city">${svg('coin')}<span class="ws-field"><select id="curAddSel"></select></span></label>
          <button class="sbtn mini" id="curAddGo">${svg('plus')}<span>${T('curAdd')}</span></button>
        </div>`,
      setup(sx) {
        let list = [];
        const send = () => {
          sx.update({ currency: { list } });
          if (!sx.demo) sx.post({ type: 'currency', list });
          render();
        };
        const render = () => {
          sx.$('curChips').innerHTML = list.map((c) => `
            <span class="cur-chip" title="${esc(fullName(c))}"><b>${c}</b><small>${esc(fullName(c))}</small>
              <button data-del="${c}" title="${T('curRemove')}"${list.length <= 2 ? ' disabled' : ''}>${svg('close')}</button></span>`).join('');
          const rest = ALL.filter((c) => !list.includes(c));
          sx.$('curAddSel').innerHTML = rest.map((c) => `<option value="${c}">${c} · ${esc(fullName(c))}</option>`).join('');
          sx.$('curAddGo').disabled = !rest.length;
        };
        sx.$('curChips').addEventListener('click', (e) => {
          const b = e.target.closest('[data-del]');
          if (!b || list.length <= 2) return;
          list = list.filter((c) => c !== b.dataset.del);
          send();
        });
        sx.$('curAddGo').addEventListener('click', () => {
          const c = sx.$('curAddSel').value;
          if (!c || list.includes(c)) return;
          list = [...list, c];
          send();
        });
        return {
          render(data) {
            list = ((data.currency || {}).list || ['EUR', 'USD', 'HUF', 'GBP', 'CHF']).slice();
            render();
          },
        };
      },
    },

    strings: {
      hu: {
        secCur: '// VALUTA', sCur: 'Valuta', sCurHint: 'Átváltó az EKB napi árfolyamaival; a választható pénznemek a Funkciók fülön',
        curSwap: 'Megfordítás', curDate: 'Az EKB árfolyama, {d}', curTrend: 'Az elmúlt 30 nap: {p}',
        curCard: '// VALUTA', curAdd: 'Hozzáadás', curRemove: 'Eltávolítás',
      },
      en: {
        secCur: '// CURRENCY', sCur: 'Currency', sCurHint: 'Converter with the ECB daily rates; choose the currencies on the Features tab',
        curSwap: 'Swap', curDate: 'ECB rate, {d}', curTrend: 'Last 30 days: {p}',
        curCard: '// CURRENCY', curAdd: 'Add', curRemove: 'Remove',
      },
      pl: {
        secCur: '// WALUTY', sCur: 'Waluty', sCurHint: 'Przelicznik z dziennymi kursami EBC; dostępne waluty na karcie Funkcje',
        curSwap: 'Zamień', curDate: 'Kurs EBC, {d}', curTrend: 'Ostatnie 30 dni: {p}',
        curCard: '// WALUTY', curAdd: 'Dodaj', curRemove: 'Usuń',
      },
    },
  });
})();
