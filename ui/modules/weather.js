// Időjárás: hőmérséklet, leírás, szél és a település (Open-Meteo, kulcs nélkül, 15 percenként).
// Kattintásra a panel mellett kinyílik a részletes mai előrejelzés (parts/flyout.js).
(() => {
  const { defineModule, $, T, svg, fetchJson, resolvePlace, wmoText, wmoIcon } = window.HUB;

  defineModule({
    id: 'weather',
    title: 'secWeather',
    name: 'sWeather',
    hint: 'sWeatherHint',
    className: 'weather',
    tip: 'wDetails',
    html: () => `
      <div class="w-main"><span class="w-icon" id="wIcon"></span><span class="w-temp" id="wTemp">--°</span></div>
      <div class="w-desc" id="wDesc">&nbsp;</div>
      <div class="w-wind" id="wWind">&nbsp;</div>
      <div class="w-city" id="wCity">&nbsp;</div>`,
    setup(ctx) {
      const sec = ctx.el;
      let wcfg = null;
      let timer = 0;
      const fetchWeather = async () => {
        clearTimeout(timer);
        timer = setTimeout(fetchWeather, 15 * 60000);
        try {
          const geo = await resolvePlace(wcfg);
          // a hely kell az Auto témához is (napkelte, napnyugta)
          if (!ctx.demo) ctx.post({ type: 'geo', lat: geo.lat, lon: geo.lon });
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
          sec.classList.remove('err');
        } catch (e) {
          $('wDesc').textContent = T('noData');
          $('wWind').textContent = String(e.message || e).slice(0, 30);
          sec.classList.add('err');
          clearTimeout(timer);
          timer = setTimeout(fetchWeather, 2 * 60000);
        }
      };
      // a beállítás (település, koordináták, szél, be/ki) a program init és weather üzenetéből
      const setWeather = (next) => {
        const placeChanged = !wcfg || wcfg.city !== next.city || wcfg.latitude !== next.latitude || wcfg.longitude !== next.longitude;
        wcfg = next;
        // a lekérés a config.json weather.enabled értékével kapcsolható ki
        const on = wcfg.enabled !== false;
        ctx.setAvailable(on);
        // a szél sebessége és iránya kikapcsolható (a hibaüzenet ettől függetlenül látszik)
        sec.classList.toggle('no-wind', wcfg.wind === false);
        if (!on) return clearTimeout(timer);
        if (placeChanged) fetchWeather();
      };
      // kattintásra a panel mellett kinyílik a részletes mai előrejelzés
      sec.addEventListener('click', () => {
        if (!wcfg) return;
        if (ctx.demo) return ctx.toast('Demo: weather details');
        ctx.post({ type: 'weather-open', ...ctx.rect(sec) });
      });
      return {
        message(msg) {
          if (msg.type === 'init') setWeather(msg.weather || { enabled: true, city: 'Budapest' });
          if (msg.type === 'weather' && msg.weather) setWeather(msg.weather);
          if (msg.type === 'fly-state') sec.classList.toggle('open', !!msg.open && msg.kind === 'weather');
        },
      };
    },
    strings: {
      hu: { sWeather: 'Időjárás', sWeatherHint: 'Az időjárás lekérését a config.json weather.enabled értéke kapcsolja ki.', wDetails: 'Részletek' },
      en: { sWeather: 'Weather', sWeatherHint: 'Fetching the weather is turned off by weather.enabled in config.json.', wDetails: 'Details' },
      pl: { sWeather: 'Pogoda', sWeatherHint: 'Pobieranie pogody wyłącza wartość weather.enabled w config.json.', wDetails: 'Szczegóły' },
    },
  });
})();
