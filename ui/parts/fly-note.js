// Az oldalpanel Notesz nézete (a panel Jegyzet moduljára kattintva): két fül, Jegyzet (szöveg)
// és Teendők (lista). A szöveg gépelés közben fél másodpercenként, illetve bezáráskor és
// fókuszvesztéskor mentődik; a választott fül megmarad (config.json: note.tab).
(() => {
  const { $, T, demo, post, svg, esc } = window.HUB;

  window.HUB.flyViews.push({
    html: () => `
      <div class="fview" id="vNote">
        <header class="wf-head">
          <span class="wf-title">${T('secNotebook')}</span>
          <button class="wf-x" data-close title="${T('wClose')}">${svg('close')}</button>
        </header>
        <div class="nt-ftabs" id="ntFTabs">
          <button data-tab="note">${svg('notes')}<span>${T('noteTabNote')}</span></button>
          <button data-tab="tasks">${svg('check')}<span>${T('noteTabTasks')}</span><b id="ntFCount"></b></button>
        </div>
        <textarea class="nt-edit" id="ntEdit" maxlength="2000" spellcheck="false" placeholder="${T('noteHint')}"></textarea>
        <div class="nt-tpane">
          <div class="nt-add">
            <input id="ntAdd" type="text" maxlength="200" spellcheck="false" autocomplete="off" placeholder="${T('taskAdd')}">
            <button id="ntAddGo" title="${T('taskAddBtn')}">${svg('plus')}</button>
          </div>
          <div class="nt-list" id="ntList"></div>
        </div>
        <footer class="cal-foot"><span class="cal-src" id="ntCount"></span><span class="cal-src" id="ntState"></span></footer>
      </div>`,

    setup({ root, show, current }) {
      const ntEdit = $('ntEdit');
      let noteSaved = '';
      let noteTimer = 0;
      let tasks = [];
      let tab = 'note';
      // előnézetben a panel modulja ugyanebből a tárolóból olvas
      const demoSave = () => window.HUB.store.set('cybershell.note', { text: noteSaved, tasks, tab });
      const noteCount = () => {
        const done = tasks.filter((t) => t.done).length;
        $('ntFCount').textContent = tasks.length ? `${done}/${tasks.length}` : '';
        if (tab === 'note') {
          $('ntCount').textContent = T('noteChars', { n: ntEdit.value.length });
          $('ntState').textContent = ntEdit.value === noteSaved ? T('noteSaved') : '…';
        } else {
          $('ntCount').textContent = tasks.length ? T('tasksCount', { d: done, n: tasks.length }) : '';
          $('ntState').innerHTML = done ? `<button class="nt-clear" id="ntClear">${T('tasksClear')}</button>` : '';
        }
      };
      // a még nem mentett szöveg mentése (nézetváltáskor és bezáráskor a keret is hívja)
      const noteFlush = () => {
        clearTimeout(noteTimer);
        noteTimer = 0;
        if (current() !== 'vNote' || ntEdit.value === noteSaved) return;
        noteSaved = ntEdit.value;
        if (demo) demoSave();
        else post({ type: 'note-set', text: noteSaved });
        noteCount();
      };
      ntEdit.addEventListener('input', () => { noteCount(); clearTimeout(noteTimer); noteTimer = setTimeout(noteFlush, 500); });
      ntEdit.addEventListener('blur', noteFlush);
      window.addEventListener('blur', noteFlush);

      // ── fülek ──
      const showTab = (next, focus) => {
        tab = next === 'tasks' ? 'tasks' : 'note';
        $('vNote').dataset.tab = tab;
        for (const b of $('ntFTabs').children) b.classList.toggle('on', b.dataset.tab === tab);
        noteCount();
        if (!focus) return;
        setTimeout(() => {
          if (tab === 'tasks') return $('ntAdd').focus();
          ntEdit.focus();
          ntEdit.setSelectionRange(ntEdit.value.length, ntEdit.value.length);
          ntEdit.scrollTop = ntEdit.scrollHeight;
        }, 30);
      };
      $('ntFTabs').addEventListener('click', (e) => {
        const b = e.target.closest('[data-tab]');
        if (!b || b.dataset.tab === tab) return;
        noteFlush();
        showTab(b.dataset.tab, true);
        if (demo) demoSave();
        else post({ type: 'note-tab', tab });
      });

      // ── teendők: hozzáadás (fent, Enterrel), pipálás, átírás, törlés, sorrend húzással ──
      const tasksSave = () => {
        renderTasks();
        if (demo) demoSave();
        else post({ type: 'tasks-set', tasks });
      };
      const renderTasks = () => {
        $('ntList').innerHTML = tasks.length ? tasks.map((t, i) => `
          <div class="nt-item${t.done ? ' done' : ''}" data-i="${i}">
            <span class="nt-grip" title="${T('taskDrag')}">${svg('grip')}</span>
            <i class="nt-box" title="${T(t.done ? 'noteUndo' : 'noteDone')}"></i>
            <input type="text" maxlength="200" spellcheck="false" value="${esc(t.text)}">
            <button class="nt-del" title="${T('taskDel')}">${svg('close')}</button>
          </div>`).join('') : `<div class="nt-empty">${T('tasksEmpty')}</div>`;
        noteCount();
      };
      const addTask = () => {
        const text = $('ntAdd').value.trim();
        if (!text) return;
        if (tasks.length >= 100) return;
        tasks.push({ text, done: false });
        $('ntAdd').value = '';
        tasksSave();
        $('ntList').scrollTop = $('ntList').scrollHeight;
      };
      $('ntAdd').addEventListener('keydown', (e) => { if (e.key === 'Enter') addTask(); });
      $('ntAddGo').addEventListener('click', () => { addTask(); $('ntAdd').focus(); });
      const itemOf = (el) => {
        const it = el.closest('.nt-item');
        return it ? Number(it.dataset.i) : -1;
      };
      $('ntList').addEventListener('click', (e) => {
        const i = itemOf(e.target);
        if (i < 0) return;
        if (e.target.closest('.nt-box')) {
          tasks[i] = { ...tasks[i], done: !tasks[i].done };
          tasksSave();
        } else if (e.target.closest('.nt-del')) {
          tasks.splice(i, 1);
          tasksSave();
        }
      });
      // átírás: Enterre vagy a mezőből kilépve; üresre törölve a teendő megszűnik
      $('ntList').addEventListener('change', (e) => {
        const i = itemOf(e.target);
        if (i < 0 || e.target.tagName !== 'INPUT') return;
        const text = e.target.value.trim();
        if (text === tasks[i].text) return;
        if (text) tasks[i] = { ...tasks[i], text };
        else tasks.splice(i, 1);
        tasksSave();
      });
      $('ntList').addEventListener('keydown', (e) => {
        if (e.target.tagName !== 'INPUT') return;
        if (e.key === 'Enter') e.target.blur();
        if (e.key === 'Escape') {
          e.stopPropagation();
          e.target.value = tasks[itemOf(e.target)].text;
          e.target.blur();
        }
      });
      root.addEventListener('click', (e) => {
        if (!e.target.closest('#ntClear')) return;
        tasks = tasks.filter((t) => !t.done);
        tasksSave();
      });
      // sorrend: a bal oldali jelet megfogva húzható; a többi elem menet közben félreáll
      $('ntList').addEventListener('pointerdown', (e) => {
        const grip = e.target.closest('.nt-grip');
        if (!grip || e.button !== 0) return;
        e.preventDefault();
        const item = grip.closest('.nt-item');
        const list = $('ntList');
        const from = Number(item.dataset.i);
        let lastY = e.clientY;
        let offset = 0;
        item.classList.add('dragging');
        grip.setPointerCapture(e.pointerId);
        const move = (ev) => {
          offset += ev.clientY - lastY;
          lastY = ev.clientY;
          // a szomszéd felénél elhaladva helyet cserél vele
          const prev = item.previousElementSibling, next = item.nextElementSibling;
          if (next && offset > next.offsetHeight / 2) {
            next.after(item);
            offset -= next.offsetHeight + 3;
          } else if (prev && offset < -prev.offsetHeight / 2) {
            prev.before(item);
            offset += prev.offsetHeight + 3;
          }
          item.style.transform = `translateY(${offset}px)`;
          // a lista széléhez érve görget
          const r = list.getBoundingClientRect();
          if (ev.clientY < r.top + 16) list.scrollTop -= 8;
          if (ev.clientY > r.bottom - 16) list.scrollTop += 8;
        };
        const end = () => {
          grip.removeEventListener('pointermove', move);
          grip.removeEventListener('pointerup', end);
          grip.removeEventListener('pointercancel', end);
          item.classList.remove('dragging');
          item.style.transform = '';
          const order = [...list.querySelectorAll('.nt-item')].map((el) => Number(el.dataset.i));
          if (order.indexOf(from) === from && order.every((v, k) => v === k)) return;
          tasks = order.map((k) => tasks[k]);
          tasksSave();
        };
        grip.addEventListener('pointermove', move);
        grip.addEventListener('pointerup', end);
        grip.addEventListener('pointercancel', end);
      });
      // a program küldte állapot (megnyitáskor, illetve a panelen történt változás után)
      const setNote = (n, opening) => {
        const next = n && typeof n === 'object' ? n : {};
        if (opening || ntEdit.value === noteSaved) {
          noteSaved = typeof next.text === 'string' ? next.text : '';
          if (ntEdit.value !== noteSaved) ntEdit.value = noteSaved;
        }
        tasks = Array.isArray(next.tasks) ? next.tasks.map((t) => ({ text: String(t.text || ''), done: !!t.done })) : [];
        // ha épp egy teendőt írnak át, a lista nem rajzolódik újra alóla
        if (opening || !$('ntList').contains(document.activeElement)) renderTasks();
        showTab(next.tab, opening);
      };

      const message = (msg) => {
        // a panelen történt változás (kipipált teendő, fülváltás): a szerkesztő is átveszi
        if (msg.type === 'note-sync') setNote(msg.note, false);
        if (msg.type === 'note-open') {
          noteFlush();
          show('vNote');
          // a kurzor a szöveg végére, illetve a teendők beviteli mezőjébe kerül
          setNote(msg.note, true);
        }
      };
      const demoOpen = (view) => {
        if (view !== 'note') return false;
        const saved = window.HUB.store.get('cybershell.note');
        message({ type: 'note-open', note: saved && typeof saved === 'object' ? saved : {
          text: T('noteDemo'), tab: 'tasks', tasks: T('tasksDemo').split('|').map((text, i) => ({ text, done: i === 0 })),
        } });
        return true;
      };
      return { message, flush: noteFlush, demo: demoOpen };
    },
  });
})();
