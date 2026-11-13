//! A sávok elhelyezése: melyik szélen mi van (Ikonok, információs panel, vékony keret), a
//! sorrend és a vastagságok, a sávok létrehozása, lefoglalása és a teljes képernyős programok kezelése.

use crate::APP_NAME;
use crate::CLEANUP;
use crate::UserEvent;
use crate::appbar;
use crate::appbar::Edge;
use crate::config::Config;
use crate::hub::Hub;
use crate::log;
use crate::send;
use crate::state_path;
use crate::subclass_proc;
use crate::web::hwnd_of;
use crate::web::serve;
use crate::web::tool_window_style;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::platform::windows::WindowBuilderExtWindows;
use tao::window::{Window, WindowBuilder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::SetWindowSubclass;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SPI_GETWORKAREA, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowPos, ShowWindow, SystemParametersInfoW,
};
use wry::http::Request;
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

/// A sávok sorrendje: a két teljes magasságú oldalpanel, aztán a felső és alsó keret.
pub(crate) const EDGES: [Edge; 4] = [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom];

/// Melyik szélen van az ikonsáv (az EDGES sorszáma). A húzáskezelő is olvassa.
pub(crate) static ICONS_EDGE: AtomicU8 = AtomicU8::new(0);

pub(crate) fn edge_from(name: &str) -> Edge {
    match name {
        "right" => Edge::Right,
        "top" => Edge::Top,
        "bottom" => Edge::Bottom,
        _ => Edge::Left,
    }
}

pub(crate) fn edge_index(edge: Edge) -> u8 {
    EDGES.iter().position(|e| *e == edge).unwrap_or(0) as u8
}

pub(crate) fn icons_edge_of(cfg: &Config) -> Edge {
    edge_from(&cfg.icons_edge)
}

pub(crate) fn info_edge_of(cfg: &Config) -> Edge {
    edge_from(&cfg.info_edge)
}

pub(crate) fn is_vertical(edge: Edge) -> bool {
    matches!(edge, Edge::Left | Edge::Right)
}

/// Az elhelyezés sorrendje: a két panel kapja a teljes hosszt (a képernyő sarkait is), a két
/// vékony keret közéjük kerül. Ha az egyik panel oldalt, a másik fent vagy lent van, az oldalsó
/// kerül előre: az a képernyő teljes magasságát kapja, a vízszintes panel mellette fut. Két
/// oldalsó vagy két vízszintes panelnél az ikonsáv az első.
pub(crate) fn edge_order(cfg: &Config) -> [Edge; 4] {
    let (icons, info) = (icons_edge_of(cfg), info_edge_of(cfg));
    let (first, second) = if !is_vertical(icons) && is_vertical(info) { (info, icons) } else { (icons, info) };
    let mut out = [first, second, Edge::Top, Edge::Bottom];
    let mut k = 2;
    for e in EDGES {
        if e != first && e != second && k < 4 {
            out[k] = e;
            k += 1;
        }
    }
    out
}

/// A sáv kért vastagsága (logikai pixel): az ikonsávé a `leftWidth`, a panelé a `rightWidth`,
/// a vékony kereteké a `frame` megfelelő értéke.
pub(crate) fn thickness_of(cfg: &Config, edge: Edge) -> u32 {
    if edge == icons_edge_of(cfg) {
        return cfg.left_width;
    }
    if edge == info_edge_of(cfg) {
        return cfg.right_width;
    }
    match edge {
        Edge::Left => cfg.frame.left,
        Edge::Top => cfg.frame.top,
        Edge::Right => cfg.frame.right,
        Edge::Bottom => cfg.frame.bottom,
    }
}

/// Melyik felület töltődik be az adott szélre: az ikonsáv ("left"), az információs panel
/// ("right") vagy vékony keret ("strip").
pub(crate) fn part_for(edge: Edge, cfg: &Config) -> &'static str {
    if edge == icons_edge_of(cfg) {
        "left"
    } else if edge == info_edge_of(cfg) {
        "right"
    } else {
        "strip"
    }
}

/// A sáv két végén lévő merőleges sávok vastagsága, ha azok a sáv hosszán belül vannak
/// (később kerültek sorra). A felület ennyivel beljebb kezdi a neonvonalat és a tartalmat.
/// Függőleges sávnál (felső, alsó), vízszintesnél (bal, jobb) sorrendben.
pub(crate) fn insets_of(cfg: &Config, edge: Edge, size: impl Fn(Edge) -> f64) -> (f64, f64) {
    let order = edge_order(cfg);
    let rank = |e: Edge| order.iter().position(|o| *o == e).unwrap_or(0);
    let (a, b) = if is_vertical(edge) { (Edge::Top, Edge::Bottom) } else { (Edge::Left, Edge::Right) };
    let pick = |p: Edge| if rank(p) > rank(edge) { size(p) } else { 0.0 };
    (pick(a), pick(b))
}

pub(crate) fn bar_url(part: &str, edge: Edge, cfg: &Config) -> String {
    let (ia, ib) = insets_of(cfg, edge, |e| thickness_of(cfg, e) as f64);
    format!(
        "cybershell://localhost/hub.html?part={part}&side={}&l={}&t={}&r={}&b={}&ia={ia}&ib={ib}&lang={}&scheme={}",
        edge.name(),
        thickness_of(cfg, Edge::Left),
        thickness_of(cfg, Edge::Top),
        thickness_of(cfg, Edge::Right),
        thickness_of(cfg, Edge::Bottom),
        cfg.language,
        cfg.scheme
    )
}

/// Felugró ablak helye egy sáv mellett. Függőleges sávnál mellette, vízszintesnél alatta vagy
/// fölötte; a sáv mentén `along` (fizikai pixel a sáv tetejétől, illetve bal szélétől) köré
/// középre igazítva, a munkaterületen belül.
pub(crate) fn popup_pos(bar_rect: RECT, edge: Edge, w: i32, h: i32, along: i32, work: RECT) -> (i32, i32) {
    if is_vertical(edge) {
        let x = if edge == Edge::Left { bar_rect.right } else { bar_rect.left - w };
        let cy = bar_rect.top + along;
        (x, (cy - h / 2).clamp(work.top + 6, (work.bottom - h - 6).max(work.top)))
    } else {
        let y = if edge == Edge::Top { bar_rect.bottom } else { bar_rect.top - h };
        let cx = bar_rect.left + along;
        ((cx - w / 2).clamp(work.left + 6, (work.right - w - 6).max(work.left)), y)
    }
}

pub(crate) struct Bar {
    pub(crate) edge: Edge,
    /// Melyik képernyőn van (0 = elsődleges); képernyőnként négy sáv.
    pub(crate) mon: usize,
    /// A képernyő DPI-szorzója (CSS-pixel → fizikai pixel).
    pub(crate) scale: f64,
    /// A betöltött felület: "left" (ikonsáv), "right" (információs panel), "top", "bottom".
    pub(crate) part: &'static str,
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) hwnd: HWND,
    pub(crate) rect: RECT,
    pub(crate) registered: bool,
}

/// Egy sáv (ablak és felület) létrehozása az adott képernyőhöz. A helyét a `layout` adja meg.
pub(crate) fn make_bar(
    target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
    web_context: &mut WebContext,
    ui_dir: &Path,
    cfg: &Config,
    edge: Edge,
    idx: usize,
    mon: usize,
) -> Bar {
    let window = WindowBuilder::new()
        // Az elsődleges képernyő bal sávja „CyberShell left” (erre keres a --quit és az őrfolyamat),
        // a többi képernyőé sorszámot kap.
        .with_title(if mon == 0 { format!("{APP_NAME} {}", edge.name()) } else { format!("{APP_NAME} {} {mon}", edge.name()) })
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_visible(false)
        .with_focused(false)
        .with_skip_taskbar(true)
        .with_undecorated_shadow(false)
        .with_drag_and_drop(false)
        .with_position(PhysicalPosition::new(0, 0))
        .with_inner_size(PhysicalSize::new(64u32, 64u32))
        .build(target)
        .expect("ablak létrehozása");
    let hwnd = hwnd_of(&window);
    tool_window_style(hwnd);
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 64, 64, SWP_NOACTIVATE | SWP_FRAMECHANGED);
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 0x4E48, 0);
    }

    let part = part_for(edge, cfg);
    let url = bar_url(part, edge, cfg);
    let bg = cfg.bg_rgba();
    let dir = ui_dir.to_path_buf();
    let page_edge = edge.name();
    let this_edge = edge;
    // Fájlhúzás: az ikonsávon az ikonokhoz kötjük, a többi sávon csak elnyeljük,
    // különben a ráejtett fájl a sáv helyén nyílna meg. (A panelek felcserélhetők.)
    let drag_handler = move |ev: wry::DragDropEvent| -> bool {
        let icons_here = edge_index(this_edge) == ICONS_EDGE.load(Ordering::Relaxed);
        if icons_here {
            let to_strings = |p: Vec<PathBuf>| p.into_iter().map(|x| x.to_string_lossy().to_string()).collect();
            match ev {
                wry::DragDropEvent::Enter { paths, position } => {
                    send(UserEvent::Drag(idx, "over", position.0, position.1, to_strings(paths)))
                }
                wry::DragDropEvent::Over { position } => send(UserEvent::Drag(idx, "over", position.0, position.1, Vec::new())),
                wry::DragDropEvent::Drop { paths, position } => {
                    send(UserEvent::Drag(idx, "drop", position.0, position.1, to_strings(paths)))
                }
                wry::DragDropEvent::Leave => send(UserEvent::Drag(idx, "leave", 0, 0, Vec::new())),
                _ => {}
            }
        }
        true
    };
    let webview = WebViewBuilder::new_with_web_context(web_context)
        .with_custom_protocol("cybershell".into(), move |_id, req| serve(&dir, &req))
        .with_on_page_load_handler(move |ev, url| {
            if matches!(ev, wry::PageLoadEvent::Finished) {
                log(format!("oldal betöltve ({page_edge}): {url}"));
            }
        })
        .with_url(url)
        .with_background_color(bg)
        .with_devtools(cfg!(debug_assertions))
        .with_default_context_menus(false)
        .with_browser_accelerator_keys(false)
        .with_ipc_handler(move |req: Request<String>| send(UserEvent::Ipc(idx, req.body().clone())))
        .with_drag_drop_handler(drag_handler)
        .build(&window)
        .expect("WebView2 létrehozása");

    Bar { edge, mon, scale: 1.0, part, window, webview, hwnd, rect: RECT::default(), registered: false }
}

impl Hub {
    pub(crate) fn own_hwnds(&self) -> Vec<HWND> {
        self.bars.iter().map(|b| b.hwnd).collect()
    }

    pub(crate) fn thickness(&self, edge: Edge) -> u32 {
        thickness_of(&self.cfg, edge)
    }

    /// Az elsődleges képernyő adott szélén lévő sáv.
    pub(crate) fn bar(&self, edge: Edge) -> Option<&Bar> {
        self.bars.iter().find(|b| b.edge == edge && b.mon == 0)
    }

    /// Ugyanazon a képernyőn lévő, adott szélű sáv.
    pub(crate) fn bar_on(&self, mon: usize, edge: Edge) -> Option<&Bar> {
        self.bars.iter().find(|b| b.edge == edge && b.mon == mon)
    }

    /// Az ikonsáv széle.
    pub(crate) fn icons_edge(&self) -> Edge {
        icons_edge_of(&self.cfg)
    }

    /// Az információs panel széle.
    pub(crate) fn info_edge(&self) -> Edge {
        info_edge_of(&self.cfg)
    }

    /// Kiszámolja és lefoglalja a négy sáv helyét. `force` nélkül csak akkor nyúl
    /// hozzájuk, ha változott valami – így a rendszer értesítései nem pörgetik körbe.
    pub(crate) fn layout(&mut self, force: bool) {
        if self.bars.is_empty() {
            return;
        }
        let mons = appbar::monitors();
        let want = if self.cfg.all_monitors { mons.len().max(1) } else { 1 };
        if want != self.want_groups {
            log(format!("képernyők: {} (sávcsoport kell: {want})", mons.len()));
            self.want_groups = want;
        }
        let groups = self.bars.len() / EDGES.len();
        let mut plan: Vec<(usize, RECT, i32)> = Vec::new();
        for g in 0..groups {
            // a már nem létező képernyő sávjai a következő újraépítésig rejtve maradnak
            let m = match mons.get(g) {
                Some(m) => *m,
                None if g == 0 => appbar::Monitor { rect: appbar::primary_monitor().0, primary: true, dpi: 96 },
                None => {
                    for i in g * EDGES.len()..(g + 1) * EDGES.len() {
                        plan.push((i, RECT::default(), 0));
                    }
                    continue;
                }
            };
            let mut base = m.rect;
            if !self.want_hidden_taskbar {
                let tb = if m.primary { appbar::taskbar_reserved() } else { appbar::secondary_taskbar_on(m.rect) };
                if let Some((edge, rc)) = tb {
                    match edge {
                        Edge::Bottom => base.bottom = rc.top,
                        Edge::Top => base.top = rc.bottom,
                        Edge::Left => base.left = rc.right,
                        Edge::Right => base.right = rc.left,
                    }
                }
            }
            let scale = m.dpi as f64 / 96.0;
            if g == 0 {
                self.scale = scale;
            }
            let px = |v: u32| ((v as f64) * scale).round() as i32;

            // Az ikonsáv, majd az információs panel kapja a szél teljes hosszát, a két vékony
            // keret a megmaradt részen, közöttük fut. Minden lefoglalt sáv szűkíti a maradékot.
            let mut rem = base;
            for edge in edge_order(&self.cfg) {
                let i = g * EDGES.len() + edge_index(edge) as usize;
                self.bars[i].scale = scale;
                let thick = px(self.thickness(edge));
                let hwnd = self.bars[i].hwnd;
                let want = match edge {
                    Edge::Left => RECT { right: rem.left + thick, ..rem },
                    Edge::Right => RECT { left: rem.right - thick, ..rem },
                    Edge::Top => RECT { bottom: rem.top + thick, ..rem },
                    Edge::Bottom => RECT { top: rem.bottom - thick, ..rem },
                };
                let rc = if thick > 0 && self.bars[i].registered { appbar::query(hwnd, edge, want, thick) } else { want };
                if thick > 0 {
                    match edge {
                        Edge::Left => rem.left = rc.right,
                        Edge::Right => rem.right = rc.left,
                        Edge::Top => rem.top = rc.bottom,
                        Edge::Bottom => rem.bottom = rc.top,
                    }
                }
                plan.push((i, rc, thick));
            }
            // az elsődleges képernyő munkaterülete (a sávok nélkül): a beállítások ezt töltik ki
            if g == 0 {
                self.work = rem;
            }
        }

        let changed = plan.iter().any(|(i, rc, _)| self.bars[*i].rect != *rc);
        if !force && !changed {
            return;
        }
        for (i, rc, thick) in plan {
            let bar = &mut self.bars[i];
            if thick <= 0 {
                if bar.registered {
                    appbar::unregister(bar.hwnd);
                    bar.registered = false;
                }
                unsafe {
                    let _ = ShowWindow(bar.hwnd, SW_HIDE);
                }
                bar.rect = rc;
                continue;
            }
            if !bar.registered {
                bar.registered = appbar::register(bar.hwnd);
            }
            bar.rect = appbar::set(bar.hwnd, bar.edge, rc);
            log(format!(
                "sáv {} #{}: {},{} – {},{}",
                bar.edge.name(),
                bar.mon,
                bar.rect.left,
                bar.rect.top,
                bar.rect.right,
                bar.rect.bottom
            ));
        }
        for m in self.fullscreen.clone() {
            self.apply_fullscreen(m, true);
        }
        let mut work = RECT::default();
        unsafe {
            let _ =
                SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut RECT as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        }
        log(format!("munkaterület: {},{} – {},{} (DPI-szorzó {:.2})", work.left, work.top, work.right, work.bottom, self.scale));
        self.suppress_until = Instant::now() + Duration::from_millis(400);
        self.save_state();
        // nyitott beállításoknál (pl. a panelek helyének váltásakor) az ablak a megváltozott
        // munkaterülethez igazodik, különben a régi panel helyén átlátszana az asztal
        self.settings_fit();
    }

    /// Kell-e a sávcsoportokat újraépíteni (képernyő került be vagy ki).
    pub(crate) fn needs_rebuild(&self) -> bool {
        !self.bars.is_empty() && self.want_groups != self.bars.len() / EDGES.len()
    }

    /// A sávcsoportok számát a képernyőkhöz igazítja (a fő ciklusból, ahol ablak hozható létre).
    pub(crate) fn rebuild_bars(
        &mut self,
        target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
        web_context: &mut WebContext,
        ui_dir: &Path,
    ) {
        let want = self.want_groups.max(1);
        let have = self.bars.len() / EDGES.len();
        log(format!("sávcsoportok: {have} → {want}"));
        self.peek_hide();
        self.fly_hide(false);
        self.launcher_hide(false);
        if want < have {
            for b in self.bars.drain(want * EDGES.len()..) {
                if b.registered {
                    appbar::unregister(b.hwnd);
                }
                unsafe {
                    let _ = ShowWindow(b.hwnd, SW_HIDE);
                }
            }
            self.fullscreen.retain(|m| *m < want);
        } else {
            for mon in have..want {
                for edge in EDGES {
                    let idx = self.bars.len();
                    let b = make_bar(target, web_context, ui_dir, &self.cfg, edge, idx, mon);
                    self.bars.push(b);
                }
            }
        }
        self.layout(true);
        for b in &self.bars {
            if b.registered {
                unsafe {
                    let _ = ShowWindow(b.hwnd, SW_SHOWNOACTIVATE);
                }
            }
        }
        self.last_state.clear();
    }

    pub(crate) fn request_layout(&mut self) {
        if Instant::now() < self.suppress_until {
            self.relayout_pending = true;
        } else {
            self.layout(false);
        }
    }

    /// Teljes képernyős program a sáv (hwnd) képernyőjén: ott a sávok hátrébb kerülnek.
    pub(crate) fn set_fullscreen(&mut self, hwnd: isize, on: bool) {
        let Some(mon) = self.bars.iter().find(|b| b.hwnd.0 as isize == hwnd).map(|b| b.mon) else { return };
        if on == self.fullscreen.contains(&mon) {
            return;
        }
        if on {
            self.fullscreen.insert(mon);
            self.peek_hide();
            self.fly_hide(false);
        } else {
            self.fullscreen.remove(&mon);
        }
        self.apply_fullscreen(mon, on);
    }

    pub(crate) fn apply_fullscreen(&self, mon: usize, on: bool) {
        for b in self.bars.iter().filter(|b| b.mon == mon) {
            unsafe {
                if on {
                    let _ = SetWindowPos(b.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    let _ = SetWindowPos(b.hwnd, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                } else {
                    let _ = SetWindowPos(b.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
        }
    }

    pub(crate) fn save_state(&self) {
        let hwnds: Vec<isize> = self.bars.iter().map(|b| b.hwnd.0 as isize).collect();
        let st = json!({ "taskbarState": self.orig_taskbar, "hwnds": hwnds });
        let _ = std::fs::write(state_path(), st.to_string());
        if let Ok(mut c) = CLEANUP.lock() {
            *c = Some((hwnds, self.orig_taskbar));
        }
    }

    pub(crate) fn on_shell_restarted(&mut self) {
        log("az Intéző újraindult – sávok újraregisztrálása");
        for b in &mut self.bars {
            b.registered = false;
        }
        if self.want_hidden_taskbar {
            appbar::hide_taskbar();
        }
        self.layout(true);
    }
}
