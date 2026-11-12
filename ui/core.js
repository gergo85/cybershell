// Közös segédek minden felületnek: a cím paraméterei (melyik felület, melyik szélen, nyelv),
// szövegek, üzenetküldés a programnak, geometria és téma, ikonok, időjárás-segédek.
// Minden felületfájl a window.HUB objektumból veszi őket.
(() => {
  const q = new URLSearchParams(location.search);
  const part = q.get('part') || 'left';
  // melyik képernyőszélen van a panel (felcserélt paneleknél az ikonsáv is lehet jobbra)
  const side = q.get('side') || (part === 'right' ? 'right' : part === 'top' || part === 'bottom' ? part : 'left');
  // vízszintes sáv (fent vagy lent): a tartalom sorba rendeződik
  const horiz = side === 'top' || side === 'bottom';
  // nyelv: a szövegek az i18n.js-ben vannak
  const lang = (window.HUB_I18N || {})[q.get('lang')] ? q.get('lang') : 'hu';
  document.documentElement.lang = lang;
  const T = (key, vars) => {
    let s = (window.HUB_I18N[lang] || {})[key] ?? window.HUB_I18N.hu[key] ?? key;
    for (const [k, v] of Object.entries(vars || {})) s = s.replaceAll(`{${k}}`, v);
    return s;
  };
  const root = document.getElementById('root');
  const css = document.documentElement.style;

  // A gazdaprogram üzenetcsatornája (wry → window.ipc, tartalékként a WebView2 saját API-ja).
  const send = window.ipc && window.ipc.postMessage
    ? (s) => window.ipc.postMessage(s)
    : window.chrome && window.chrome.webview
      ? (s) => window.chrome.webview.postMessage(s)
      : null;
  const demo = !send;
  const post = (msg) => { if (send) send(JSON.stringify(msg)); };

  window.addEventListener('error', (e) => post({ type: 'log', text: `${e.message} @${e.lineno}` }));
  document.addEventListener('contextmenu', (e) => e.preventDefault());

  const setGeometry = (g) => {
    for (const k of ['l', 't', 'r', 'b', 'ia', 'ib']) if (g && g[k] != null) css.setProperty(`--${k}`, g[k]);
  };
  setGeometry({ l: q.get('l'), t: q.get('t'), r: q.get('r'), b: q.get('b'), ia: q.get('ia') ?? q.get('t') ?? 0, ib: q.get('ib') ?? q.get('b') ?? 0 });

  const setTheme = (t) => {
    if (!t) return;
    for (const k of ['main', 'alt', 'hot', 'bg', 'text']) if (t[k]) css.setProperty(`--theme-${k}`, t[k]);
    if (t.scheme) document.documentElement.dataset.scheme = t.scheme;
  };
  // sötét: Cyberpunk 2077, világos: Ghost in the Shell (a demóhoz; élesben a program küldi)
  const THEMES = {
    dark: { main: '#f3e600', alt: '#00e1ff', hot: '#ff2a6d', bg: '#06070b', text: '#c9ccd6', scheme: 'dark' },
    light: { main: '#00848a', alt: '#2c62a6', hot: '#d6452a', bg: '#e3e9ec', text: '#18262d', scheme: 'light' },
  };
  // az első kirajzolás is a jó sémával történjen
  if (q.get('scheme') === 'light') document.documentElement.dataset.scheme = 'light';
  const setAlert = (on) => document.documentElement.classList.toggle('alert', !!on);

  const svg = (name) =>
    `<svg viewBox="0 0 24 24">${(window.HUB_ICONS || {})[name] || window.HUB_ICONS.app}</svg>`;
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  const pad = (n) => String(n).padStart(2, '0');
  const $ = (id) => document.getElementById(id);
  // az ikon finom színe (config "color"), csak érvényes hex értéket fogadunk el
  const appColor = (a) => (/^#[0-9a-f]{3,8}$/i.test(a.color || '') ? ` style="--app:${a.color}"` : '');

  // helyi tároló (a felületek közösen használják, pl. a település koordinátáihoz)
  const store = {
    get(k) { try { return JSON.parse(localStorage.getItem(k) || 'null'); } catch (_) { return null; } },
    set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (_) {} },
  };

  // ── időjárás: közös segédek (Open-Meteo, kulcs nélkül) ──
  const fetchJson = async (url) => {
    const r = await fetch(url, { cache: 'no-store' });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return r.json();
  };
  // a település koordinátái (a keresés eredményét megjegyezzük)
  const resolvePlace = async (wcfg) => {
    let place = wcfg.city || '';
    if (wcfg.latitude != null && wcfg.longitude != null) return { lat: wcfg.latitude, lon: wcfg.longitude, name: place };
    const key = `cybershell.geo.${place.toLowerCase()}`;
    let geo = store.get(key);
    if (!geo) {
      const g = await fetchJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(place)}&count=1&language=${lang}`);
      const hit = g.results && g.results[0];
      if (!hit) throw new Error(T('unknownPlace', { p: place }));
      geo = { lat: hit.latitude, lon: hit.longitude, name: hit.name };
      store.set(key, geo);
    }
    return geo;
  };
  const wmo = (code) => window.HUB_WMO[code] || ['Ismeretlen', 'Unknown', 'overcast'];
  const wmoText = (code) => wmo(code)[lang === 'en' ? 1 : 0];
  const wmoIcon = (code, isDay) => {
    const icon = wmo(code)[2];
    if (!isDay && icon === 'sun') return 'moon';
    if (!isDay && icon === 'partly') return 'overcast';
    return icon;
  };

  window.HUB = {
    $, q, part, side, horiz, lang, T, root, css, send, demo, post, setGeometry, setTheme, THEMES, setAlert, svg, esc, pad, appColor, store, fetchJson, resolvePlace, wmo, wmoText, wmoIcon,
    // a felületépítők (parts/*.js) ide jegyzik be magukat
    parts: {},
  };
})();
