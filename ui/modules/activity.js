// Aktivitás: visszaszámláló (alapból 20 perc), ami lejártakor mozgásra figyelmeztet; a panel és
// a keretek rózsaszínre váltanak. A hossz a szekcióban állítható. Több képernyőnél minden
// panel ugyanazt a visszaszámlálót mutatja (a helyi tárolón át).
(() => {
  const { defineModule, $, T, svg, pad, hhmm, store, setAlert } = window.HUB;
  const KEY = 'cybershell.activity';
  const KEY_MIN = 'cybershell.activity.minutes';

  defineModule({
    id: 'activity',
    title: 'secActivity',
    name: 'sActivity',
    sectionId: 'act',
    className: 'act',
    html: () => `
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
      </div>`,
    setup(ctx) {
      const sec = ctx.el;
      let cfg = { minutes: 20, autoStart: false };
      let session = 0;
      let act = null; // { running, endAt, remaining, alert, session }
      const minutes = () => store.get(KEY_MIN) || cfg.minutes;
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
        sec.classList.toggle('alerting', on);
        setAlert(on);
        ctx.post({ type: 'alert', on });
      };
      const segs = [...$('actSegs').children];
      const render = () => {
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
        const n = Math.ceil((Math.max(0, left) / total()) * segs.length);
        segs.forEach((el, i) => el.classList.toggle('on', i < n));
        sec.classList.toggle('paused', !act.running && !act.alert);
        if (act.running) {
          const end = new Date(act.endAt);
          $('actInfo').textContent = T('actMoveAt', { t: hhmm(end) });
        } else if (!act.alert) {
          $('actInfo').textContent = T('actPaused');
        }
        $('actToggle').innerHTML = act.running ? svg('pause') : svg('play');
      };
      $('actToggle').addEventListener('click', () => {
        if (act.running) { act.remaining = Math.max(0, act.endAt - Date.now()); act.running = false; }
        else { act.endAt = Date.now() + (act.remaining > 0 ? act.remaining : total()); act.running = true; }
        save(); render();
      });
      $('actReset').addEventListener('click', () => { startCycle(); render(); });
      $('actDone').addEventListener('click', () => { startCycle(); setActAlert(false); render(); });
      $('actSnooze').addEventListener('click', () => { startCycle(5 * 60000); setActAlert(false); render(); });

      // egyedi hossz beállítása
      const input = $('actMin');
      const openEdit = () => {
        input.value = minutes();
        $('actErr').textContent = '';
        sec.classList.add('editing');
        input.focus();
        input.select();
      };
      const closeEdit = () => sec.classList.remove('editing');
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
        render();
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
        if (e.key === KEY_MIN) return render();
        if (e.key !== KEY || !e.newValue) return;
        try {
          const v = JSON.parse(e.newValue);
          if (!v || typeof v.running !== 'boolean') return;
          const wasAlert = act && act.alert;
          act = v;
          if (!!v.alert !== !!wasAlert) {
            sec.classList.toggle('alerting', !!v.alert);
            setAlert(!!v.alert);
          }
          render();
        } catch (_) {}
      });

      // A program új indulásakor (új munkamenet) a visszaszámláló nem indul magától,
      // csak ha a beállítás ezt kéri. A felület újratöltésekor viszont folytatódik.
      const start = () => {
        const saved = store.get(KEY);
        if (saved && typeof saved.running === 'boolean' && saved.session === session) {
          act = saved;
          const wasAlert = act.alert;
          act.alert = false;
          if (wasAlert) setActAlert(true);
        } else if (cfg.autoStart) {
          act = null;
          startCycle();
        } else {
          act = { running: false, endAt: 0, remaining: total(), alert: false, session };
          save();
        }
        render();
      };
      return {
        message(msg) {
          if (msg.type !== 'init') return;
          if (msg.activity) cfg = { minutes: Math.max(1, msg.activity.minutes || 20), autoStart: msg.activity.autoStart === true };
          if (msg.session) session = msg.session;
          if (!act) start();
        },
        tick: render,
      };
    },
    strings: {
      hu: {
        secActivity: '// AKTIVITÁS', sActivity: 'Aktivitás', actLength: 'HOSSZ (PERC)', actError: '1 és 240 közötti egész szám kell.',
        actAlertTitle: 'MOZDULJ MEG!', actAlertText: 'Állj fel, nyújtózz, nézz messzire.', actMoveAt: 'MOZGÁS {t}', actPaused: 'MEGÁLLÍTVA',
      },
      en: {
        secActivity: '// ACTIVITY', sActivity: 'Activity', actLength: 'LENGTH (MIN)', actError: 'Enter a whole number from 1 to 240.',
        actAlertTitle: 'GET MOVING!', actAlertText: 'Stand up, stretch, look far away.', actMoveAt: 'MOVE AT {t}', actPaused: 'PAUSED',
      },
      pl: {
        secActivity: '// AKTYWNOŚĆ', sActivity: 'Aktywność', actLength: 'DŁUGOŚĆ (MIN)', actError: 'Wymagana liczba całkowita od 1 do 240.',
        actAlertTitle: 'RUSZ SIĘ!', actAlertText: 'Wstań, przeciągnij się, popatrz w dal.', actMoveAt: 'RUCH {t}', actPaused: 'WSTRZYMANO',
      },
      de: {
        secActivity: '// AKTIVITÄT', sActivity: 'Aktivität', actLength: 'DAUER (MIN)', actError: 'Gib eine ganze Zahl von 1 bis 240 ein.',
        actAlertTitle: 'BEWEG DICH!', actAlertText: 'Steh auf, streck dich, schau in die Ferne.', actMoveAt: 'BEWEGEN UM {t}', actPaused: 'PAUSIERT',
      },
      es: {
        secActivity: '// ACTIVIDAD', sActivity: 'Actividad', actLength: 'DURACIÓN (MIN)', actError: 'Escribe un número entero del 1 al 240.',
        actAlertTitle: '¡MUÉVETE!', actAlertText: 'Levántate, estírate, mira a lo lejos.', actMoveAt: 'MOVERSE A LAS {t}', actPaused: 'EN PAUSA',
      },
    },
  });
})();
