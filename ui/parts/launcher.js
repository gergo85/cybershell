(() => {
  const { $, q, lang, locale, decimal, T, root, css, demo, post, setTheme, svg, esc, store, fmtDate, hhmm } = window.HUB;

  // ───────────── saját Start menü és kereső ─────────────
  // Egy felület, két elrendezés: "start" (az információs panel mellett: kitűzött és gyakori
  // programok, minden alkalmazás A–Z, mappák, főkapcsoló) és "search" (a képernyő közepén:
  // egyetlen keresősáv). Gépelésre mindkettő ugyanazt a találati listát mutatja: programok,
  // számológép, parancs (>), webcím vagy mappa, rendszerműveletek, beállítások, legutóbbi
  // fájlok, végül webes keresés. (Ötletek: PowerToys Run, Flow Launcher, Open-Shell, Seelen UI.)
  function buildLauncher() {
    root.className = 'lx';
    const startMode = q.get('mode');
    root.innerHTML = `
      <header class="lx-head">
        <div class="lx-brand">CYBER<span>SHELL</span><em id="lxMode"></em></div>
        <label class="lx-search">${svg('search')}<input id="lxQ" spellcheck="false" autocomplete="off" placeholder="${T('lxPlaceholder')}"><kbd id="lxEsc">ESC</kbd></label>
      </header>
      <div class="lx-body">
        <div class="lx-home" id="lxHome">
          <div class="lx-col lx-main">
            <section class="lx-sec">
              <h4>${T('lxPinned')}<small id="lxPinHint">${T('lxPinHint')}</small></h4>
              <div class="lx-grid" id="lxPins"></div>
            </section>
            <section class="lx-sec">
              <h4>${T('lxFrequent')}</h4>
              <div class="lx-list" id="lxFreq"></div>
            </section>
            <section class="lx-sec">
              <h4 id="lxRecentHead">${T('lxRecent')}</h4>
              <div class="lx-list" id="lxRecent"></div>
            </section>
          </div>
          <div class="lx-col lx-all">
            <h4>${T('lxAllApps')}<small id="lxCount"></small></h4>
            <div class="lx-filter" id="lxFilter">
              <button data-filter="all">${T('lxFilterAll')}</button><button data-filter="apps" title="${T('lxFilterAppsHint')}">${T('lxFilterApps')}</button><button data-filter="system" title="${T('lxFilterSysHint')}">${T('lxFilterSys')}</button>
            </div>
            <div class="lx-az" id="lxAz"></div>
            <div class="lx-letters" id="lxLetters"></div>
          </div>
        </div>
        <div class="lx-results" id="lxRes"></div>
      </div>
      <footer class="lx-foot">
        <div class="lx-userbox">
          <div class="lx-user"><span class="lx-avatar" id="lxAvatar">${svg('user')}</span><span id="lxUser"></span></div>
          <button class="lx-fbtn lx-acct" data-open="ms-settings:yourinfo" title="${T('lxAccount')}">${svg('settings')}</button>
          <button class="lx-fbtn lx-acct" data-open="ms-settings:signinoptions" title="${T('lxSignIn')}">${svg('shield')}</button>
        </div>
        <div class="lx-places" id="lxPlaces"></div>
        <div class="lx-power" id="lxPower"></div>
        <div class="lx-keys" id="lxKeys"></div>
      </footer>
      <div class="lx-menu" id="lxMenu"></div>
      <div class="lx-toast" id="lxToast"></div>`;

    const input = $('lxQ');
    let mode = 'start';
    let apps = [];
    let pinned = [];
    let recent = [];
    let clips = []; // vágólap-előzmények (null: kikapcsolva)
    let loading = true;

    // ── segédek ──
    // ékezet- és kisbetű-független összevetés („fenykep” → „Fényképek”)
    const norm = (s) => String(s || '').toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');
    const words = (s) => norm(s).split(/[^a-z0-9]+/).filter(Boolean);
    const initials = (s) => words(s).map((w) => w[0]).join('');
    // egy szó (vagy rész) illeszkedése: pontos, eleje, szókezdet, betűszó, tartalmazza, szórványos
    const scoreText = (name, qn, loose = true) => {
      const n = norm(name);
      if (!qn) return 0;
      if (n === qn) return 100;
      if (n.startsWith(qn)) return 92 - Math.min(10, n.length - qn.length) * 0.3;
      const ws = words(name);
      if (ws.some((w) => w.startsWith(qn))) return 78;
      if (qn.length >= 2 && initials(name).startsWith(qn)) return 72;
      const i = n.indexOf(qn);
      if (i >= 0) return 60 - Math.min(i, 20) * 0.5;
      // szórványos betűk sorrendben (pl. „vscd”): csak a névben, hosszabb keresésnél
      if (!loose || qn.length < 3) return 0;
      // legfeljebb két összefüggő darabban (pl. „phshop” → „Photoshop”), különben nem találat
      let k = 0, runs = 0, last = -2;
      for (let j = 0; j < n.length && k < qn.length; j++) {
        if (n[j] === qn[k]) { if (j !== last + 1) runs++; last = j; k++; }
      }
      return k === qn.length && runs <= 2 ? 40 - runs * 6 : 0;
    };
    // az első elem a név, a többi kulcsszó (azokban csak szókezdetre és részletre keresünk);
    // több szavas keresésnél minden szónak illeszkednie kell
    const best = (hay, qn) => Math.max(...hay.map((h, i) => (i ? scoreText(h, qn, false) * 0.9 : scoreText(h, qn))));
    const score = (hay, query) => {
      const qn = norm(query).trim();
      if (!qn) return 0;
      const whole = best(hay, qn);
      if (whole > 0) return whole;
      const parts = qn.split(/\s+/).filter(Boolean);
      if (parts.length < 2) return 0;
      let sum = 0;
      for (const p of parts) {
        const best1 = best(hay, p);
        if (!best1) return 0;
        sum += best1;
      }
      return (sum / parts.length) * 0.85;
    };

    // ── használati gyakoriság (a gyakran és nemrég indított elemek előrébb kerülnek) ──
    const USE_KEY = 'cybershell.launcher.use';
    let use = store.get(USE_KEY) || {};
    const bump = (key) => {
      const u = use[key] || { n: 0, t: 0 };
      use[key] = { n: u.n + 1, t: Date.now() };
      // a legrégebbieket elengedjük
      const keys = Object.keys(use);
      if (keys.length > 400) keys.sort((a, b) => use[a].t - use[b].t).slice(0, keys.length - 400).forEach((k) => delete use[k]);
      store.set(USE_KEY, use);
      reportFrequent();
    };
    const frecency = (key) => {
      const u = use[key];
      if (!u) return 0;
      const days = (Date.now() - u.t) / 86400000;
      return Math.min(24, Math.log2(1 + u.n) * 6) * (days < 1 ? 1 : days < 7 ? 0.8 : days < 30 ? 0.55 : 0.3);
    };
    // a gyakran használt programok (a kereső üres állapotában és a panel Mostanában moduljában)
    const frequentApps = (n) => apps.filter((a) => use[`app:${a.id}`])
      .sort((a, b) => frecency(`app:${b.id}`) - frecency(`app:${a.id}`)).slice(0, n);
    // a lista a programnak is megy (a panel Mostanában modulja ezt mutatja), csak változáskor
    let lastFrequent = '';
    const reportFrequent = () => {
      if (!apps.length) return;
      const ids = frequentApps(8).map((a) => a.id);
      const s = ids.join(',');
      if (s === lastFrequent) return;
      lastFrequent = s;
      post({ type: 'frequent', ids });
    };

    // ── ikonok ──
    const guessIcon = (hay) => {
      const s = norm(hay);
      const hit = (window.HUB_GUESS || []).find(([re]) => re.test(s));
      return hit ? hit[1] : 'app';
    };
    const appIcon = (a) => (a.img
      ? `<img src="${esc(a.img)}" alt="" loading="lazy" draggable="false" data-fallback="${esc(guessIcon(`${a.name} ${a.target || ''}`))}">`
      : svg(guessIcon(`${a.name} ${a.target || ''}`)));
    // betöltési hiba esetén a beépített vonalikon
    root.addEventListener('error', (e) => {
      const img = e.target;
      if (img.tagName === 'IMG' && img.dataset.fallback) img.outerHTML = svg(img.dataset.fallback);
    }, true);
    const FILE_ICONS = [[/\.(jpe?g|png|gif|webp|bmp|heic|tiff?|raw|cr[23]|nef|arw|dng|psd)$/, 'photo'], [/\.(mp4|mkv|mov|avi|webm)$/, 'video'],
      [/\.(mp3|flac|wav|m4a|ogg)$/, 'music'], [/\.(docx?|odt|rtf|pdf)$/, 'doc'], [/\.(xlsx?|csv|ods)$/, 'sheet'],
      [/\.(txt|md|log|json|ini|ya?ml|toml)$/, 'notes'], [/\.(js|ts|rs|py|cs|cpp|h|html|css|ps1|bat)$/, 'code'], [/\.(zip|7z|rar)$/, 'folder']];
    const fileIcon = (name) => {
      const n = name.toLowerCase();
      const hit = FILE_ICONS.find(([re]) => re.test(n));
      return hit ? hit[1] : 'file';
    };

    // ── beállítások és rendszerműveletek (keresésben) ──
    const L = (hu, en, pl, de, es) => ({ en, pl, de, es }[lang] || hu);
    const SETTINGS = [
      ['display', L('Kijelző', 'Display', 'Ekran', 'Bildschirm', 'Pantalla'), 'képernyő felbontás monitor fényerő skálázás screen resolution brightness scale'],
      ['nightlight', L('Éjszakai fény', 'Night light', 'Światło nocne', 'Nachtmodus', 'Luz nocturna'), 'kék fény blue light'],
      ['sound', L('Hang', 'Sound', 'Dźwięk', 'Sound', 'Sonido'), 'hangszóró mikrofon kimenet bemenet speaker microphone output input audio'],
      ['apps-volume', L('Hangerőkeverő', 'Volume mixer', 'Mikser głośności', 'Lautstärkemixer', 'Mezclador de volumen'), 'hangerő programonként volume per app'],
      ['notifications', L('Értesítések', 'Notifications', 'Powiadomienia', 'Benachrichtigungen', 'Notificaciones'), 'ne zavarjanak do not disturb focus'],
      ['powersleep', L('Energiagazdálkodás', 'Power & sleep', 'Zasilanie i uśpienie', 'Energie und Standby', 'Energía y suspensión'), 'alvás akkumulátor energia sleep battery power'],
      ['batterysaver', L('Akkumulátor', 'Battery', 'Bateria', 'Akku', 'Batería'), 'energiatakarékos battery saver'],
      ['storagesense', L('Tárhely', 'Storage', 'Magazyn', 'Speicher', 'Almacenamiento'), 'lemez hely meghajtó disk space drive'],
      ['multitasking', L('Többfeladatos működés', 'Multitasking', 'Wielozadaniowość', 'Multitasking', 'Multitarea'), 'ablak elrendezés snap windows'],
      ['clipboard', L('Vágólap', 'Clipboard', 'Schowek', 'Zwischenablage', 'Portapapeles'), 'másolás előzmények copy history'],
      ['about', L('Névjegy (a gép adatai)', 'About (this PC)', 'Informacje (o komputerze)', 'Info (dieser PC)', 'Información (este equipo)'), 'rendszer gépnév processzor memória system pc name specs'],
      ['bluetooth', L('Bluetooth és eszközök', 'Bluetooth & devices', 'Bluetooth i urządzenia', 'Bluetooth und Geräte', 'Bluetooth y dispositivos'), 'eszköz párosítás device pair'],
      ['printers', L('Nyomtatók és lapolvasók', 'Printers & scanners', 'Drukarki i skanery', 'Drucker und Scanner', 'Impresoras y escáneres'), 'nyomtató print scanner'],
      ['mousetouchpad', L('Egér', 'Mouse', 'Mysz', 'Maus', 'Mouse'), 'kurzor görgetés cursor scroll touchpad érintőpad'],
      ['typing', L('Gépelés', 'Typing', 'Pisanie', 'Eingabe', 'Escritura'), 'billentyűzet keyboard autocorrect'],
      ['network', L('Hálózat és internet', 'Network & internet', 'Sieć i Internet', 'Netzwerk und Internet', 'Red e Internet'), 'net ethernet kapcsolat connection'],
      ['network-wifi', L('Wi-Fi', 'Wi-Fi', 'Wi-Fi', 'WLAN', 'Wi-Fi'), 'wifi vezeték nélküli wireless'],
      ['network-vpn', L('VPN', 'VPN', 'VPN', 'VPN', 'VPN'), 'virtuális magánhálózat'],
      ['personalization', L('Személyre szabás', 'Personalization', 'Personalizacja', 'Personalisierung', 'Personalización'), 'megjelenés appearance'],
      ['personalization-background', L('Háttér', 'Background', 'Tło', 'Hintergrund', 'Fondo'), 'háttérkép wallpaper'],
      ['colors', L('Színek', 'Colors', 'Kolory', 'Farben', 'Colores'), 'sötét mód világos dark mode light accent'],
      ['lockscreen', L('Zárolási képernyő', 'Lock screen', 'Ekran blokady', 'Sperrbildschirm', 'Pantalla de bloqueo'), 'zárolás lock'],
      ['themes', L('Témák', 'Themes', 'Motywy', 'Designs', 'Temas'), 'téma theme'],
      ['fonts', L('Betűtípusok', 'Fonts', 'Czcionki', 'Schriftarten', 'Fuentes'), 'betű font'],
      ['appsfeatures', L('Telepített alkalmazások', 'Installed apps', 'Zainstalowane aplikacje', 'Installierte Apps', 'Aplicaciones instaladas'), 'program eltávolítás uninstall apps'],
      ['defaultapps', L('Alapértelmezett alkalmazások', 'Default apps', 'Aplikacje domyślne', 'Standard-Apps', 'Aplicaciones predeterminadas'), 'társítás fájltípus default association'],
      ['startupapps', L('Indítási alkalmazások', 'Startup apps', 'Aplikacje autostartu', 'Autostart-Apps', 'Aplicaciones de arranque'), 'indítás autostart startup'],
      ['yourinfo', L('Fiókadatok', 'Your info', 'Twoje informacje', 'Deine Infos', 'Tu información'), 'fiók account'],
      ['signinoptions', L('Bejelentkezési lehetőségek', 'Sign-in options', 'Opcje logowania', 'Anmeldeoptionen', 'Opciones de inicio de sesión'), 'jelszó pin hello password'],
      ['dateandtime', L('Dátum és idő', 'Date & time', 'Data i godzina', 'Datum und Uhrzeit', 'Fecha y hora'), 'óra időzóna clock time zone'],
      ['regionlanguage', L('Nyelv és régió', 'Language & region', 'Język i region', 'Sprache und Region', 'Idioma y región'), 'nyelv billentyűzetkiosztás language region keyboard layout'],
      ['gaming-gamebar', L('Game Bar', 'Game Bar', 'Game Bar', 'Game Bar', 'Game Bar'), 'játék gaming'],
      ['easeofaccess', L('Akadálymentesség', 'Accessibility', 'Ułatwienia dostępu', 'Barrierefreiheit', 'Accesibilidad'), 'nagyító felolvasó magnifier narrator'],
      ['privacy', L('Adatvédelem és biztonság', 'Privacy & security', 'Prywatność i zabezpieczenia', 'Datenschutz und Sicherheit', 'Privacidad y seguridad'), 'engedélyek permissions'],
      ['windowsdefender', L('Windows biztonság', 'Windows Security', 'Zabezpieczenia Windows', 'Windows-Sicherheit', 'Seguridad de Windows'), 'vírus védelem tűzfal virus defender firewall'],
      ['windowsupdate', L('Windows Update', 'Windows Update', 'Windows Update', 'Windows Update', 'Windows Update'), 'frissítés update'],
    ].map(([uri, name, kw]) => ({ kind: 'setting', key: `set:${uri}`, uri: `ms-settings:${uri}`, name, kw, icon: 'settings', sub: T('lxSettingSub') }));
    const TOOLS = [
      ['control', L('Vezérlőpult', 'Control Panel', 'Panel sterowania', 'Systemsteuerung', 'Panel de control'), 'control panel'],
      ['devmgmt.msc', L('Eszközkezelő', 'Device Manager', 'Menedżer urządzeń', 'Geräte-Manager', 'Administrador de dispositivos'), 'illesztőprogram driver hardver'],
      ['diskmgmt.msc', L('Lemezkezelés', 'Disk Management', 'Zarządzanie dyskami', 'Datenträgerverwaltung', 'Administración de discos'), 'partíció partition meghajtó'],
      ['services.msc', L('Szolgáltatások', 'Services', 'Usługi', 'Dienste', 'Servicios'), 'service'],
      ['regedit', L('Beállításszerkesztő', 'Registry Editor', 'Edytor rejestru', 'Registrierungs-Editor', 'Editor del Registro'), 'registry rendszerleíró'],
      ['msinfo32', L('Rendszerinformáció', 'System Information', 'Informacje o systemie', 'Systeminformationen', 'Información del sistema'), 'msinfo hardver'],
      ['eventvwr.msc', L('Eseménynapló', 'Event Viewer', 'Podgląd zdarzeń', 'Ereignisanzeige', 'Visor de eventos'), 'napló log hiba error'],
    ].map(([cmd, name, kw]) => ({ kind: 'tool', key: `tool:${cmd}`, cmd, name, kw: `${kw} ${cmd}`, icon: 'terminal', sub: T('lxToolSub', { c: cmd }) }));
    const SYSTEM = [
      ['lock', L('Zárolás', 'Lock', 'Zablokuj', 'Sperren', 'Bloquear'), 'lezár kijelentkezés lock', 'lock'],
      ['sleep', L('Alvó állapot', 'Sleep', 'Uśpij', 'Energie sparen', 'Suspender'), 'alvás felfüggesztés sleep suspend', 'moon'],
      ['restart', L('Újraindítás', 'Restart', 'Uruchom ponownie', 'Neu starten', 'Reiniciar'), 'újraindít reboot restart', 'reload'],
      ['shutdown', L('Leállítás', 'Shut down', 'Wyłącz', 'Herunterfahren', 'Apagar'), 'kikapcsol leáll shutdown power off', 'power'],
      ['settings', L('CyberShell beállítások', 'CyberShell settings', 'Ustawienia CyberShell', 'CyberShell-Einstellungen', 'Ajustes de CyberShell'), 'cybershell menü beállítás settings', 'settings'],
      ['recycle', L('Lomtár', 'Recycle Bin', 'Kosz', 'Papierkorb', 'Papelera de reciclaje'), 'kuka törölt trash recycle', 'trash'],
    ].map(([act, name, kw, icon]) => ({ kind: 'system', key: `sys:${act}`, act, name, kw, icon, sub: T('lxSystemSub') }));

    // ── számológép: + − × ÷ ^ % zárójelek, sqrt, pi (eval nélkül) ──
    const calc = (src) => {
      const s = src.replace(/^=/, '').replace(/×/g, '*').replace(/÷/g, '/').replace(/,/g, '.').replace(/\s+/g, '').toLowerCase();
      if (!s || !/[0-9]/.test(s) || !/^[0-9.+\-*/^%()a-z]+$/.test(s)) return null;
      if (!/[+\-*/^%(]/.test(s.replace(/^-/, '')) && !/^(sqrt|pi)/.test(s)) return null;
      let i = 0;
      const peek = () => s[i];
      const num = () => {
        if (s.startsWith('pi', i)) { i += 2; return Math.PI; }
        if (s.startsWith('sqrt', i)) { i += 4; return Math.sqrt(unary()); }
        if (peek() === '(') { i++; const v = expr(); if (peek() !== ')') throw 0; i++; return v; }
        const m = /^\d*\.?\d+(e[+-]?\d+)?/.exec(s.slice(i));
        if (!m) throw 0;
        i += m[0].length;
        let v = parseFloat(m[0]);
        if (peek() === '%') { i++; v /= 100; }
        return v;
      };
      const unary = () => (peek() === '-' ? (i++, -unary()) : peek() === '+' ? (i++, unary()) : num());
      const pow = () => { const b = unary(); if (peek() === '^') { i++; return b ** pow(); } return b; };
      const term = () => {
        let v = pow();
        for (;;) {
          if (peek() === '*') { i++; v *= pow(); } else if (peek() === '/') { i++; v /= pow(); } else return v;
        }
      };
      const expr = () => {
        let v = term();
        for (;;) {
          if (peek() === '+') { i++; v += term(); } else if (peek() === '-') { i++; v -= term(); } else return v;
        }
      };
      try {
        const v = expr();
        if (i !== s.length || !Number.isFinite(v)) return null;
        return v;
      } catch (_) { return null; }
    };
    const fmtNum = (v) => {
      const r = Math.abs(v) >= 1e15 || (Math.abs(v) < 1e-6 && v !== 0) ? v.toExponential(6) : String(+v.toPrecision(12));
      return decimal(r);
    };

    // ── találatok összeállítása ──
    const looksUrl = (t) => /^https?:\/\//i.test(t) || /^[\w-]+(\.[\w-]+)+(\/\S*)?$/i.test(t) && !/\s/.test(t) && /\.[a-z]{2,}(\/|$)/i.test(t);
    const looksPath = (t) => /^([a-z]:\\|\\\\|%[a-z_]+%)/i.test(t);
    const appKey = (a) => `app:${a.id}`;
    const build = (query) => {
      const t = query.trim();
      const groups = [];
      if (!t) {
        // üres kereső: a gyakran használt programok és a legutóbbi fájlok
        const freq = frequentApps(8);
        if (freq.length) groups.push({ title: T('lxFrequent'), items: freq.map(appItem), cols: 2 });
        if (clips && clips.length) groups.push({ title: T('lxClip'), items: clips.slice(0, 5).map(clipItem), clear: 'clip' });
        if (recent.length) groups.push({ title: T('lxRecent'), items: recent.slice(0, 6).map(fileItem), clear: 'recent' });
        if (!groups.length && apps.length) groups.push({ title: T('lxApps'), items: apps.slice(0, 8).map(appItem) });
        return groups;
      }
      // parancs (mint a Futtatás ablak): > után
      if (t.startsWith('>')) {
        const cmd = t.slice(1).trim();
        groups.push({ title: T('lxCommand'), items: [{ kind: 'cmd', key: 'cmd', name: cmd ? T('lxRun', { c: cmd }) : T('lxRunHint'), sub: T('lxRunSub'), icon: 'terminal', cmd }] });
        return groups;
      }
      // vágólap: v: után csak az előzmények, a további szövegre szűrve
      const cm = /^v:\s*/i.exec(t);
      if (cm) {
        const qn = norm(t.slice(cm[0].length));
        const hits = (clips || []).filter((c) => !qn || norm(c.text).includes(qn));
        const empty = { kind: 'note', key: 'clip-none', name: clips ? (qn ? T('lxNoHits') : T('lxClipEmpty')) : T('lxClipOff'), sub: '', icon: 'clipboard' };
        groups.push({ title: T('lxClip'), items: hits.length ? hits.map(clipItem) : [empty], clear: hits.length && !qn ? 'clip' : '' });
        return groups;
      }
      const val = calc(t);
      if (val != null) {
        groups.push({ title: T('lxCalc'), items: [{ kind: 'calc', key: 'calc', name: `= ${fmtNum(val)}`, sub: `${t.replace(/^=/, '')} · ${T('lxCopy')}`, icon: 'calc', value: fmtNum(val) }] });
      }
      if (looksPath(t)) groups.push({ title: T('lxOpen'), items: [{ kind: 'open', key: `open:${t}`, name: t, sub: T('lxOpenPath'), icon: 'folder', target: t }] });
      else if (looksUrl(t)) {
        const url = /^https?:\/\//i.test(t) ? t : `https://${t}`;
        groups.push({ title: T('lxOpen'), items: [{ kind: 'open', key: `open:${url}`, name: url, sub: T('lxOpenUrl'), icon: 'globe', target: url }] });
      }
      const ranked = (list, hay, limit) => list
        .map((x) => ({ x, s: score(hay(x), t) }))
        .filter((r) => r.s > 0)
        .map((r) => ({ ...r, s: r.s + frecency(r.x.key) }))
        .sort((a, b) => b.s - a.s)
        .slice(0, limit);
      const appHits = ranked(apps.map(appItem), (x) => [x.name, x.targetName || ''], mode === 'search' ? 8 : 12);
      const sysHits = ranked(SYSTEM, (x) => [x.name, x.kw], 3);
      const setHits = ranked(SETTINGS.concat(TOOLS), (x) => [x.name, x.kw], 5);
      const fileHits = ranked(recent.map(fileItem), (x) => [x.name], 5);
      // a vágólapon csak a szöveg tartalmazására keresünk: a hosszú szövegben a betűszó és a
      // szórványos betűegyezés félrevezető lenne
      const qn = norm(t);
      const clipHits = qn.length < 2 ? [] : (clips || []).filter((c) => norm(c.text).includes(qn)).slice(0, 4)
        .map((c, i) => ({ x: clipItem(c), s: 50 - i }));
      // a legjobb találat kerül előre (pl. „wifi” → beállítás, „vs” → program)
      const blocks = [
        { title: T('lxApps'), hits: appHits, w: 1.04 },
        { title: T('lxSystem'), hits: sysHits, w: 1 },
        { title: T('lxSettings'), hits: setHits, w: 0.98 },
        { title: T('lxFiles'), hits: fileHits, w: 0.96 },
        { title: T('lxClip'), hits: clipHits, w: 0.94 },
      ].filter((b) => b.hits.length);
      blocks.sort((a, b) => b.hits[0].s * b.w - a.hits[0].s * a.w);
      for (const b of blocks) groups.push({ title: b.title, items: b.hits.map((h) => h.x) });
      groups.push({ title: T('lxWeb'), items: [{ kind: 'web', key: 'web', name: T('lxWebFor', { q: t }), sub: T('lxWebSub'), icon: 'globe', q: t }] });
      return groups;
    };
    const appItem = (a) => ({
      kind: 'app', key: appKey(a), id: a.id, name: a.name, app: a, exe: !!a.exe, target: a.target,
      targetName: a.target ? a.target.replace(/^.*\\/, '').replace(/\.exe$/i, '') : '',
      sub: a.target ? a.target : (a.id.includes('!') ? T('lxStoreApp') : T('lxApp')),
    });
    // mikor másolták: „most”, „5 perce”, „2 órája”, régebben dátum és idő
    const ago = (ms) => {
      const m = Math.floor((Date.now() - ms) / 60000);
      if (m < 1) return T('lxNow');
      if (m < 60) return T('lxMinAgo', { m });
      if (m < 24 * 60) return T('lxHourAgo', { h: Math.floor(m / 60) });
      const d = new Date(ms);
      return `${fmtDate(d, { year: false })} ${hhmm(d)}`;
    };
    // a szöveg egy sorban (a szóközök összevonva), alatta a forrás, az idő és a hossz
    const clipItem = (c) => ({
      kind: 'clip', key: `clip:${c.id}`, id: c.id, icon: 'clipboard',
      name: c.text.replace(/\s+/g, ' ').trim().slice(0, 160),
      sub: T('lxClipSub', { s: c.source ? `${c.source} · ` : '', t: ago(c.at), n: c.len.toLocaleString(locale) }),
    });
    const fileItem = (f) => ({ kind: 'file', key: `file:${f.path}`, name: f.name, sub: T('lxRecentFile'), icon: fileIcon(f.name), path: f.path });

    // ── kirajzolás ──
    let flat = [];
    let sel = 0;
    let armed = null; // megerősítésre váró leállítás/újraindítás
    const itemIcon = (it) => (it.kind === 'app' ? appIcon(it.app) : svg(it.icon || 'app'));
    const delTitle = (it) => (it.kind === 'clip' ? T('lxClipDelete') : T('lxRecentDelete'));
    const itemHtml = (it, i) => `
      <button class="lx-item${i === sel ? ' sel' : ''}${armed === it.key ? ' armed' : ''}" data-i="${i}">
        <span class="lx-ico">${itemIcon(it)}</span>
        <span class="lx-txt"><b>${esc(armed === it.key ? T('lxConfirm', { a: it.name }) : it.name)}</b><small>${esc(it.sub || '')}</small></span>
        <span class="lx-go">${svg('enter')}</span>
        ${it.kind === 'clip' || it.kind === 'file' ? `<span class="lx-del" data-del="${i}" title="${delTitle(it)}">${svg('trash')}</span>` : ''}
      </button>`;
    // csoportcím, a vágólapnál és a legutóbbi fájloknál „Összes törlése” gombbal (két kattintásra)
    let clearArmed = '';
    let clearTimer = 0;
    const clearBtn = (what) => `<button class="lx-clear${clearArmed === what ? ' armed' : ''}" data-clear="${what}" title="${T(what === 'clip' ? 'lxClipClearHint' : 'lxRecentClearHint')}">${svg('trash')}<span>${T(clearArmed === what ? 'lxClearSure' : 'lxClearAll')}</span></button>`;
    const groupHead = (g) => `<h4>${esc(g.title)}${g.clear ? clearBtn(g.clear) : ''}</h4>`;
    const renderResults = () => {
      const groups = build(input.value);
      flat = groups.flatMap((g) => g.items);
      if (sel >= flat.length) sel = Math.max(0, flat.length - 1);
      let i = 0;
      $('lxRes').innerHTML = groups.map((g) => `<div class="lx-group${g.cols === 2 ? ' cols2' : ''}">${groupHead(g)}<div class="lx-items">${g.items.map((it) => itemHtml(it, i++)).join('')}</div></div>`).join('')
        || `<div class="lx-empty">${loading ? T('lxLoading') : T('lxNoHits')}</div>`;
      const cur = $('lxRes').querySelector('.lx-item.sel');
      if (cur) cur.scrollIntoView({ block: 'nearest' });
    };
    // a kétoszlopos csoport (gyakran használt) tételeinek sorszámai, ha `i` abban van
    const gridOf = (i) => {
      const g = $('lxRes').querySelector(`.lx-item[data-i="${i}"]`)?.closest('.cols2');
      return g ? [...g.querySelectorAll('.lx-item')].map((el) => Number(el.dataset.i)) : null;
    };
    const step = (dir) => {
      const g = gridOf(sel);
      if (!g) return select(sel + dir);
      const t = sel + dir * 2;
      if (g.includes(t)) return select(t);
      return select(dir > 0 ? Math.max(...g) + 1 : Math.min(...g) - 1);
    };
    const select = (i) => {
      sel = Math.max(0, Math.min(flat.length - 1, i));
      for (const el of $('lxRes').querySelectorAll('.lx-item')) el.classList.toggle('sel', Number(el.dataset.i) === sel);
      const cur = $('lxRes').querySelector('.lx-item.sel');
      if (cur) cur.scrollIntoView({ block: 'nearest' });
    };

    const tileHtml = (a) => `
      <button class="lx-tile" data-app="${esc(a.id)}" draggable="true" title="${esc(a.name)}">
        <span class="lx-ico">${appIcon(a)}</span><span class="lx-name">${esc(a.name)}</span>
      </button>`;
    const rowHtml = (a) => `
      <button class="lx-row" data-app="${esc(a.id)}" title="${esc(a.name)}">
        <span class="lx-ico">${appIcon(a)}</span><span class="lx-name">${esc(a.name)}</span>
      </button>`;
    const byId = (id) => apps.find((a) => a.id === id);
    const letterOf = (name) => {
      const c = norm(name).replace(/^[^a-z0-9]+/, '')[0] || '#';
      return /[a-z]/.test(c) ? c.toUpperCase() : '#';
    };
    // Minden alkalmazás: mind, csak a telepített programok, vagy csak a Windows részei (eszközök,
    // konzolok, mappák); a választás megmarad
    const FILTER_KEY = 'cybershell.start.filter';
    let filter = ['all', 'apps', 'system'].includes(store.get(FILTER_KEY)) ? store.get(FILTER_KEY) : 'apps';
    const shown = (a) => filter === 'all' || (filter === 'system') === !!a.system;
    $('lxFilter').addEventListener('click', (e) => {
      const b = e.target.closest('[data-filter]');
      if (!b || b.dataset.filter === filter) return;
      filter = b.dataset.filter;
      store.set(FILTER_KEY, filter);
      renderHome();
      $('lxAz').scrollTop = 0;
    });
    const renderHome = () => {
      const pins = pinned.map(byId).filter(Boolean);
      $('lxPins').innerHTML = pins.map(tileHtml).join('') || `<div class="lx-empty small">${T('lxNoPins')}</div>`;
      $('lxPinHint').style.display = pins.length ? 'none' : '';
      const freq = frequentApps(5);
      $('lxFreq').innerHTML = freq.map(rowHtml).join('') || `<div class="lx-empty small">${T('lxNoFrequent')}</div>`;
      // a Start menüben is törölhető az egész lista (mint a keresőben)
      $('lxRecentHead').innerHTML = `${T('lxRecent')}${recent.length ? clearBtn('recent') : ''}`;
      $('lxRecent').innerHTML = recent.slice(0, 5).map((f) => `
        <button class="lx-row file" data-file="${esc(f.path)}" title="${esc(f.name)}">
          <span class="lx-ico">${svg(fileIcon(f.name))}</span><span class="lx-name">${esc(f.name)}</span>
        </button>`).join('') || `<div class="lx-empty small">${T('lxNoRecent')}</div>`;
      // minden alkalmazás betűnként
      const groups = new Map();
      const listed = apps.filter(shown);
      for (const b of $('lxFilter').children) b.classList.toggle('on', b.dataset.filter === filter);
      for (const a of listed) {
        const l = letterOf(a.name);
        if (!groups.has(l)) groups.set(l, []);
        groups.get(l).push(a);
      }
      const letters = [...groups.keys()].sort((a, b) => (a === '#' ? -1 : b === '#' ? 1 : a.localeCompare(b)));
      $('lxAz').innerHTML = letters.map((l) => `<div class="lx-letter" id="lxL-${l === '#' ? 'num' : l}">${l}</div>${groups.get(l).map(rowHtml).join('')}`).join('')
        || `<div class="lx-empty">${loading ? T('lxLoading') : T('lxNoHits')}</div>`;
      $('lxLetters').innerHTML = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map((l) =>
        `<button data-letter="${l}"${groups.has(l) ? '' : ' disabled'}>${l}</button>`).join('');
      $('lxCount').textContent = apps.length ? String(listed.length) : '';
    };
    $('lxLetters').addEventListener('click', (e) => {
      const b = e.target.closest('[data-letter]');
      if (!b || b.disabled) return;
      const el = $(`lxL-${b.dataset.letter === '#' ? 'num' : b.dataset.letter}`);
      if (el) $('lxAz').scrollTo({ top: el.offsetTop - $('lxAz').offsetTop, behavior: 'smooth' });
    });

    const FOOT_PLACES = [
      ['shell:Personal', 'doc', L('Dokumentumok', 'Documents', 'Dokumenty', 'Dokumente', 'Documentos')],
      ['shell:Downloads', 'download', L('Letöltések', 'Downloads', 'Pobrane', 'Downloads', 'Descargas')],
      ['shell:My Pictures', 'photo', L('Képek', 'Pictures', 'Obrazy', 'Bilder', 'Imágenes')],
      ['shell:MyComputerFolder', 'pc', L('Ez a gép', 'This PC', 'Ten komputer', 'Dieser PC', 'Este equipo')],
      ['ms-settings:', 'settings', L('Windows beállítások', 'Windows Settings', 'Ustawienia systemu Windows', 'Windows-Einstellungen', 'Configuración de Windows')],
    ];
    $('lxPlaces').innerHTML = `<button class="lx-fbtn" data-clipview title="${T('lxClipBtn')}">${svg('clipboard')}</button>` +
      FOOT_PLACES.map(([t, icon, name]) => `<button class="lx-fbtn" data-open="${t}" title="${name}">${svg(icon)}</button>`).join('');
    // a Minden alkalmazás oszlop bal széle pontosan a lenti Képek gomb bal széléhez igazodik
    const alignAllApps = () => {
      const b = root.querySelector('[data-open="shell:My Pictures"]');
      const home = $('lxHome');
      if (!b || !b.offsetWidth || !home.offsetWidth) return;
      const w = Math.round(home.getBoundingClientRect().right - b.getBoundingClientRect().left);
      if (w > 200) home.style.setProperty('--all-w', `${w}px`);
    };
    window.addEventListener('resize', alignAllApps);
    const POWER = [['signout', 'logout', T('lxSignOut')], ['lock', 'lock', L('Zárolás', 'Lock', 'Zablokuj', 'Sperren', 'Bloquear')], ['sleep', 'moon', T('pwSleep')],
      ['restart', 'reload', L('Újraindítás', 'Restart', 'Uruchom ponownie', 'Neu starten', 'Reiniciar')], ['shutdown', 'power', L('Leállítás', 'Shut down', 'Wyłącz', 'Herunterfahren', 'Apagar')]];
    $('lxPower').innerHTML = POWER.map(([act, icon, name]) => `<button class="lx-fbtn pw-${act}" data-power="${act}" title="${name}">${svg(icon)}</button>`).join('');
    $('lxKeys').innerHTML = `<span><kbd>ENTER</kbd>${T('lxKeyOpen')}</span>` +
      `<span><kbd>CTRL+SHIFT+ENTER</kbd>${T('lxKeyAdmin')}</span><span><kbd>CTRL+ENTER</kbd>${T('lxKeyFolder')}</span><span><kbd>&gt;</kbd>${T('lxKeyCmd')}</span><span><kbd>V:</kbd>${T('lxKeyClip')}</span>`;

    // ── üzenet a gazdaprogramnak ──
    let toastTimer = 0;
    const toast = (text) => {
      const t = $('lxToast');
      t.textContent = text;
      t.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => t.classList.remove('show'), 3200);
    };
    const act = (msg) => {
      if (demo) { toast(`Demo: ${msg.type} ${msg.id || msg.target || msg.text || msg.q || msg.action || ''}`); return false; }
      post(msg);
      return true;
    };
    const powerArm = {};
    const doPower = (action, btn) => {
      // a kijelentkezés, a leállítás és az újraindítás egy második kattintással indul
      if ((action === 'shutdown' || action === 'restart' || action === 'signout') && !powerArm[action]) {
        powerArm[action] = setTimeout(() => { delete powerArm[action]; if (btn) btn.classList.remove('armed'); }, 3000);
        if (btn) btn.classList.add('armed');
        toast(`${action === 'signout' ? T('lxConfirmSignOut') : T(action === 'shutdown' ? 'pwConfirmOff' : 'pwConfirmRestart')} ${T('lxClickAgain')}`);
        return;
      }
      act({ type: 'power', action });
    };
    const copyText = async (text) => {
      try { await navigator.clipboard.writeText(text); } catch (_) {
        const ta = document.createElement('textarea');
        ta.value = text; document.body.appendChild(ta); ta.select(); document.execCommand('copy'); ta.remove();
      }
    };
    // egy találat végrehajtása; `how`: "admin" (rendszergazdaként) vagy "folder" (a helye)
    const run = (it, how) => {
      if (!it) return;
      if (it.kind === 'system' && (it.act === 'shutdown' || it.act === 'restart') && armed !== it.key) {
        armed = it.key;
        renderResults();
        return;
      }
      armed = null;
      if (it.kind === 'note') return;
      if (it.key && !['calc', 'web', 'clip'].includes(it.kind)) bump(it.key);
      switch (it.kind) {
        case 'app':
          if (how === 'folder') return it.target ? act({ type: 'open-location', id: it.id }) : toast(T('lxNoFolder'));
          return act({ type: 'run-app', id: it.id, admin: how === 'admin' });
        case 'file':
          return act({ type: 'open', target: it.path, reveal: how === 'folder' });
        case 'setting':
          return act({ type: 'open', target: it.uri });
        case 'tool':
          return act({ type: 'run-cmd', text: it.cmd, admin: how === 'admin' });
        case 'cmd':
          if (!it.cmd) return;
          return act({ type: 'run-cmd', text: it.cmd, admin: how === 'admin' });
        case 'open':
          return act({ type: 'open', target: it.target, reveal: how === 'folder' });
        case 'web':
          return act({ type: 'web', q: it.q });
        case 'clip':
          // Enter: beillesztés az előző ablakba; Ctrl+Enter: csak a vágólapra
          return act({ type: how === 'folder' ? 'clip-copy' : 'clip-paste', id: it.id });
        case 'calc':
          copyText(it.value);
          return toast(T('lxCopied', { v: it.value }));
        case 'system':
          if (it.act === 'settings') return act({ type: 'settings' });
          if (it.act === 'recycle') return act({ type: 'open', target: 'shell:RecycleBinFolder' });
          return act({ type: 'power', action: it.act });
        default:
      }
    };
    const runApp = (id, how) => {
      const a = byId(id);
      if (a) run(appItem(a), how);
    };

    // ── jobb gombos menü (kitűzés, sávra tétel, rendszergazdaként, fájl helye) ──
    const menu = $('lxMenu');
    const closeMenu = () => { menu.classList.remove('open'); menu.innerHTML = ''; };
    const openMenu = (x, y, entries, above) => {
      menu.innerHTML = entries.map(([key, icon, label]) => `<button data-m="${key}">${svg(icon)}<span>${esc(label)}</span></button>`).join('');
      menu.classList.add('open');
      const r = root.getBoundingClientRect();
      const w = menu.offsetWidth, h = menu.offsetHeight;
      menu.style.left = `${Math.max(6, Math.min(x, r.width - w - 6))}px`;
      menu.style.top = `${Math.max(6, Math.min(above ? y - h - 6 : y, r.height - h - 6))}px`;
    };
    let menuTarget = null;
    const appMenu = (a, x, y) => {
      menuTarget = { app: a };
      const isPinned = pinned.includes(a.id);
      openMenu(x, y, [
        [isPinned ? 'unpin' : 'pin', 'pin', isPinned ? T('lxUnpin') : T('lxPinIt')],
        ['add-bar', 'bar', T('lxAddBar')],
        ...(a.exe ? [['admin', 'shield', T('lxAdmin')]] : []),
        ...(a.target ? [['folder', 'folder', T('lxFolder')]] : []),
      ]);
    };
    const clipMenu = (id, x, y) => {
      menuTarget = { clip: id };
      openMenu(x, y, [['clip-paste', 'enter', T('lxClipPaste')], ['clip-copy', 'clipboard', T('lxClipCopy')],
        ['clip-delete', 'close', T('lxClipDelete')], ['clip-clear', 'trash', T('lxClipClear')]]);
    };
    const fileMenu = (path, x, y) => {
      menuTarget = { file: path };
      openMenu(x, y, [['open-file', 'file', T('lxKeyOpen')], ['folder', 'folder', T('lxFolder')], ['recent-delete', 'trash', T('lxRecentDelete')]]);
    };
    menu.addEventListener('click', (e) => {
      const b = e.target.closest('[data-m]');
      if (!b || !menuTarget) return;
      const k = b.dataset.m;
      const t = menuTarget;
      closeMenu();
      if (t.file && k === 'recent-delete') return removeItem(fileItem({ name: '', path: t.file }));
      if (t.file) return run(fileItem({ name: '', path: t.file }), k === 'folder' ? 'folder' : '');
      if (t.clip) {
        if (k === 'clip-delete') return removeItem({ kind: 'clip', id: t.clip });
        if (k === 'clip-clear') { clips = []; renderResults(); }
        return act({ type: k, id: t.clip });
      }
      const a = t.app;
      if (k === 'pin' || k === 'unpin') {
        pinned = k === 'pin' ? [...pinned.filter((p) => p !== a.id), a.id] : pinned.filter((p) => p !== a.id);
        renderHome();
        if (!demo) post({ type: k, id: a.id });
        return toast(k === 'pin' ? T('lxPinned1', { n: a.name }) : T('lxUnpinned1', { n: a.name }));
      }
      if (k === 'add-bar') return act({ type: 'add-bar', id: a.id });
      if (k === 'admin') return runApp(a.id, 'admin');
      if (k === 'folder') return runApp(a.id, 'folder');
    });
    root.addEventListener('contextmenu', (e) => {
      const el = e.target.closest('[data-app], .lx-item, [data-file]');
      if (!el) return closeMenu();
      e.preventDefault();
      const r = root.getBoundingClientRect();
      const x = e.clientX - r.left, y = e.clientY - r.top;
      if (el.dataset.app) { const a = byId(el.dataset.app); if (a) appMenu(a, x, y); return; }
      if (el.dataset.file) return fileMenu(el.dataset.file, x, y);
      const it = flat[Number(el.dataset.i)];
      if (it && it.kind === 'app') appMenu(it.app, x, y);
      else if (it && it.kind === 'file') fileMenu(it.path, x, y);
      else if (it && it.kind === 'clip') clipMenu(it.id, x, y);
    });
    document.addEventListener('pointerdown', (e) => { if (!e.target.closest('.lx-menu')) closeMenu(); });

    // ── kattintások ──
    // egy tétel törlése (vágólap: az előzményekből; fájl: a legutóbbiak listájáról, a fájl megmarad)
    const removeItem = (it) => {
      if (!it) return;
      if (it.kind === 'clip') {
        clips = clips.filter((c) => c.id !== it.id);
        act({ type: 'clip-delete', id: it.id });
      } else if (it.kind === 'file') {
        recent = recent.filter((f) => f.path !== it.path);
        act({ type: 'recent-delete', path: it.path });
        renderHome();
      }
      renderResults();
    };
    const clearAll = (what, btn) => {
      if (clearArmed !== what) {
        clearArmed = what;
        clearTimeout(clearTimer);
        clearTimer = setTimeout(() => { clearArmed = ''; renderResults(); renderHome(); }, 3000);
        renderHome();
        return renderResults();
      }
      clearArmed = '';
      clearTimeout(clearTimer);
      if (what === 'clip') { clips = []; act({ type: 'clip-clear' }); }
      else { recent = []; act({ type: 'recent-clear' }); }
      renderHome();
      toast(T(what === 'clip' ? 'lxClipCleared' : 'lxRecentCleared'));
      renderResults();
      void btn;
    };
    root.addEventListener('click', (e) => {
      if (e.target.closest('.lx-menu')) return;
      const del = e.target.closest('[data-del]');
      if (del) { e.stopPropagation(); return removeItem(flat[Number(del.dataset.del)]); }
      const clr = e.target.closest('[data-clear]');
      if (clr) return clearAll(clr.dataset.clear, clr);
      // Vágólap gomb: a vágólap-előzmények listája (a kereső „v:” előtagja)
      if (e.target.closest('[data-clipview]')) {
        $('lxQ').value = 'v: ';
        $('lxQ').dispatchEvent(new Event('input'));
        $('lxQ').focus();
        return;
      }
      const el = e.target.closest('[data-app], [data-file], .lx-item, [data-open], [data-power]');
      if (!el) return;
      if (el.dataset.app) return runApp(el.dataset.app, e.ctrlKey && e.shiftKey ? 'admin' : '');
      if (el.dataset.file) return run(fileItem({ name: '', path: el.dataset.file }));
      if (el.dataset.open) return act({ type: 'open', target: el.dataset.open });
      if (el.dataset.power) return doPower(el.dataset.power, el);
      sel = Number(el.dataset.i);
      run(flat[sel], e.ctrlKey && e.shiftKey ? 'admin' : '');
    });
    $('lxEsc').addEventListener('click', () => post({ type: 'close' }));

    // kitűzött csempék sorrendje: húzással
    let dragId = null;
    $('lxPins').addEventListener('dragstart', (e) => {
      const t = e.target.closest('[data-app]');
      if (!t) return;
      dragId = t.dataset.app;
      t.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
    });
    $('lxPins').addEventListener('dragover', (e) => {
      if (!dragId) return;
      e.preventDefault();
      const t = e.target.closest('[data-app]');
      for (const x of $('lxPins').querySelectorAll('.lx-tile')) x.classList.toggle('drop', x === t && t.dataset.app !== dragId);
    });
    $('lxPins').addEventListener('drop', (e) => {
      e.preventDefault();
      const t = e.target.closest('[data-app]');
      if (!dragId || !t || t.dataset.app === dragId) return;
      const order = pinned.filter((p) => p !== dragId);
      order.splice(order.indexOf(t.dataset.app), 0, dragId);
      pinned = order;
      renderHome();
      if (!demo) post({ type: 'pin-order', pinned });
    });
    $('lxPins').addEventListener('dragend', () => {
      dragId = null;
      for (const x of $('lxPins').querySelectorAll('.lx-tile')) x.classList.remove('dragging', 'drop');
    });

    // ── billentyűzet ──
    const update = () => {
      const has = input.value.trim().length > 0;
      root.classList.toggle('typing', has || mode === 'search');
      armed = null;
      sel = 0;
      renderResults();
    };
    input.addEventListener('input', update);
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (menu.classList.contains('open')) return closeMenu();
        if (input.value) { input.value = ''; return update(); }
        return post({ type: 'close' });
      }
      // gépelés bárhol: a keresőmezőbe kerül
      if (document.activeElement !== input && e.key.length === 1 && !e.ctrlKey && !e.altKey) input.focus();
      if (!root.classList.contains('typing')) return;
      if (e.key === 'ArrowDown') { e.preventDefault(); step(1); }
      if (e.key === 'ArrowUp') { e.preventDefault(); step(-1); }
      if ((e.key === 'ArrowRight' || e.key === 'ArrowLeft') && !input.value && gridOf(sel)) {
        e.preventDefault();
        select(sel + (e.key === 'ArrowRight' ? 1 : -1));
      }
      if (e.key === 'PageDown') { e.preventDefault(); select(sel + 6); }
      if (e.key === 'PageUp') { e.preventDefault(); select(sel - 6); }
      if (e.key === 'Enter') {
        e.preventDefault();
        run(flat[sel], e.ctrlKey && e.shiftKey ? 'admin' : e.ctrlKey ? 'folder' : '');
      }
      if (e.key === 'Tab') {
        // a következő csoport első elemére ugrik
        e.preventDefault();
        const items = [...$('lxRes').querySelectorAll('.lx-group')].map((g) => Number(g.querySelector('.lx-item').dataset.i));
        const next = e.shiftKey ? [...items].reverse().find((i) => i < sel) : items.find((i) => i > sel);
        select(next ?? (e.shiftKey ? items[items.length - 1] : items[0]) ?? 0);
      }
    });

    const open = (m) => {
      mode = m === 'search' ? 'search' : 'start';
      root.dataset.mode = mode;
      $('lxMode').textContent = mode === 'search' ? T('lxModeSearch') : T('lxModeStart');
      setTimeout(alignAllApps, 0);
      input.placeholder = T(mode === 'search' ? 'lxPlaceholderSearch' : 'lxPlaceholder');
      input.value = '';
      closeMenu();
      armed = null;
      update();
      renderHome();
      $('lxAz').scrollTop = 0;
      root.classList.remove('in');
      void root.offsetWidth;
      root.classList.add('in');
      setTimeout(() => input.focus(), 0);
    };

    const h = (msg) => {
      if (msg.theme) setTheme(msg.theme);
      if (msg.type === 'launcher-open') open(msg.mode);
      if (msg.type === 'launcher-data') {
        // megnyitáskor (és a programlista háttérbeli frissülésekor) ugyanaz az adat jön újra:
        // ilyenkor nem rajzoljuk újra a menüt, mert az ikonok újratöltése villódzást okozna
        const key = JSON.stringify([msg.apps, msg.pinned, msg.recent, msg.clips, !!msg.loading]);
        const changed = key !== lastData;
        lastData = key;
        apps = msg.apps || [];
        pinned = msg.pinned || [];
        recent = msg.recent || [];
        if (msg.clips !== undefined) clips = msg.clips;
        loading = !!msg.loading && !apps.length;
        $('lxUser').textContent = (msg.user || '').toUpperCase();
        // a Windows-fiók profilképe az ikon helyén (ha nincs, vagy nem tölthető be, marad az ikon);
        // csak egyszer töltjük be, nem minden megnyitáskor
        if (msg.userPic && !userPicLoaded) {
          userPicLoaded = true;
          const img = new Image();
          img.className = 'lx-pic';
          img.alt = '';
          img.onload = () => $('lxAvatar').replaceChildren(img);
          img.onerror = () => { userPicLoaded = false; };
          img.src = `/__userpic?${Date.now()}`;
        }
        if (changed) {
          renderHome();
          if (root.classList.contains('typing')) renderResults();
        }
        reportFrequent();
      }
      // a panel Mostanában moduljából indított program is használatnak számít
      if (msg.type === 'launcher-use' && msg.id) bump(`app:${msg.id}`);
      if (msg.type === 'launcher-clips') {
        clips = msg.clips;
        if (root.classList.contains('typing')) renderResults();
      }
      if (msg.type === 'launcher-pins') { pinned = msg.pinned || []; renderHome(); }
      if (msg.type === 'launcher-toast') toast(msg.text);
      // elrejtéskor a tartalom is eltűnik: újranyitáskor ne villanjon fel a régi állapot a beúszás előtt
      if (msg.type === 'launcher-hidden') { closeMenu(); input.value = ''; root.classList.remove('in'); }
    };
    let lastData = '';
    let userPicLoaded = false;
    h.demo = () => {
      const names = ['Firefox', 'Google Chrome', 'Visual Studio Code', 'Spotify', 'Claude', 'Adobe Lightroom Classic', 'Adobe Photoshop 2026',
        'Topaz Photo AI', 'Word', 'Excel', 'Outlook', 'OneNote', 'Discord', 'Steam', 'Számológép', 'Jegyzettömb', 'Paint', 'Fényképek',
        'Feladatkezelő', 'Parancssor', 'Terminal', 'GitHub Desktop', 'Total Commander', 'VLC media player', '7-Zip File Manager',
        'OBS Studio', 'Figma', 'Notion', 'Obsidian', 'Telegram', 'Zoom', 'DaVinci Resolve', 'Blender', 'Audacity', 'Képmetsző'];
      const list = names.map((name, i) => ({ id: `demo-${i}`, name, target: i % 3 === 0 ? `C:\\Program Files\\${name}\\${name.split(' ')[0]}.exe` : '', exe: i % 3 === 0,
        system: ['Feladatkezelő', 'Parancssor'].includes(name) }))
        .sort((a, b) => a.name.localeCompare(b.name, locale));
      use = { 'app:demo-2': { n: 30, t: Date.now() }, 'app:demo-0': { n: 22, t: Date.now() }, 'app:demo-4': { n: 15, t: Date.now() - 3600e3 },
        'app:demo-5': { n: 9, t: Date.now() - 86400e3 }, 'app:demo-3': { n: 5, t: Date.now() - 2 * 86400e3 } };
      h({ type: 'launcher-data', user: 'Demo User', pinned: ['demo-0', 'demo-2', 'demo-4', 'demo-5', 'demo-6', 'demo-3', 'demo-12', 'demo-13'], apps: list,
        recent: [{ name: 'Portfólió 2026.pdf', path: 'C:\\x\\a.pdf' }, { name: 'DSC_4821.jpg', path: 'C:\\x\\b.jpg' }, { name: 'ajánlat.docx', path: 'C:\\x\\c.docx' },
          { name: 'config.json', path: 'C:\\x\\d.json' }, { name: 'költségek.xlsx', path: 'C:\\x\\e.xlsx' }],
        clips: [
          { id: 3, text: 'https://example.com/portfolio/2026', len: 34, at: Date.now() - 40e3, source: 'Firefox' },
          { id: 2, text: 'Kedves Anna!\n\nKöszönöm a gyors választ, a jövő heti időpont megfelel.', len: 70, at: Date.now() - 9 * 60e3, source: 'Outlook' },
          { id: 1, text: 'cargo clippy --release -- -D warnings', len: 37, at: Date.now() - 3 * 3600e3, source: 'Terminál' },
        ] });
      open(startMode || 'start');
      const pre = q.get('q');
      if (pre) { input.value = pre; update(); }
    };
    return h;
  }

  window.HUB.parts.launcher = buildLauncher;
})();
