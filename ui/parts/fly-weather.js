// Az oldalpanel részletes időjárás nézete (a panel Időjárás moduljára kattintva): a mostani
// idő, a következő 24 óra háromóránként, a mai adatok és a következő napok (Open-Meteo).
(() => {
  const { $, locale, decimal, T, post, svg, esc, hhmm, fetchJson, resolvePlace, wmoText, wmoIcon } = window.HUB;

  window.HUB.flyViews.push({
    html: () => `
      <div class="fview" id="vWeather">
        <header class="wf-head">
          <span class="wf-title">${T('secWeather')}<b id="wfCity"></b></span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="wf-body" id="wfBody"><div class="wf-note">${T('wLoading')}</div></div>
        <footer class="wf-foot" id="wfFoot">&nbsp;</footer>
      </div>`,

    setup({ show }) {
      let wcfg = null;
      let last = null;   // { at, geo, w }
      let busy = false;
      const dec = decimal;
      const hm = (iso) => (iso || '').slice(11, 16);
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
        $('wfFoot').textContent = `${T('wUpdated', { t: hhmm(t) })} · Open-Meteo`;
      };

      // tíz percig a legutóbbi előrejelzést mutatja, utána újra letölti
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
            '&timezone=auto&forecast_days=4' + (wcfg.unit === 'f' ? '&temperature_unit=fahrenheit' : ''));
          last = { at: Date.now(), geo, w };
          renderWeather();
        } catch (e) {
          post({ type: 'log', text: String(e.message || e) });
          if (!last) $('wfBody').innerHTML = `<div class="wf-note err">${T('noData')}<br>${esc(String(e.message || e).slice(0, 60))}</div>`;
        } finally {
          busy = false;
        }
      };

      const message = (msg) => {
        if (msg.type !== 'weather-open') return;
        const next = msg.weather || { city: 'Budapest' };
        if (wcfg && JSON.stringify(wcfg) !== JSON.stringify(next)) last = null;
        wcfg = next;
        show('vWeather');
        loadWeather(false);
      };
      return { message };
    },
  });
})();
