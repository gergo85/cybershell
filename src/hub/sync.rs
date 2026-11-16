//! Igazodás a Windowshoz: a téma átvezetése a Windowsra és a Windows Terminalra, a Windows
//! módjának követése, és a neonkeret más programok ablakain (a megvalósítás: `winsync`).

use crate::hub::Hub;
use crate::log;
use crate::neon;
use crate::winsync;
use crate::wintrack::WinInfo;
use std::collections::HashSet;
use windows::Win32::Foundation::HWND;

impl Hub {
    /// A bekapcsolt szinkronok átvezetése (a kikapcsoltaké visszaáll az eredetire), háttérben.
    pub(crate) fn sync_windows(&self) {
        let c = &self.cfg;
        winsync::apply(c.windows_sync.clone(), c.appearance == "windows", c.scheme == "light", c.theme.clone(), c.theme_light.clone());
    }

    /// A Windows színbeállítása megváltozott: a „Windows” megjelenés azonnal követi.
    pub(crate) fn on_system_theme(&mut self) {
        if self.cfg.appearance == "windows" && self.cfg.resolve_scheme() {
            log(format!("megjelenés (windows): {}", self.cfg.scheme));
            self.push_theme();
            self.send_settings();
        }
    }

    /// Neonkeret: az aktív ablak kerete a téma fő színét kapja, „all” módban a többi ablaké
    /// egy tompább árnyalatot. Csak a változásokat küldjük el a Windowsnak.
    pub(crate) fn update_borders(&mut self, wins: &[WinInfo]) {
        if self.cfg.neon_border == "off" {
            self.reset_borders();
            return;
        }
        let all = self.cfg.neon_border == "all";
        let (main, dim) = winsync::border_colors(self.cfg.active_theme());
        // A keret az utolsó valódi programablakon marad: ha kattintáskor egy pillanatra menü,
        // felugró elem vagy más, nem programablak kerül előtérbe, nem tűnik el és jön vissza.
        let last = self.last_fg.map(|h| h.0 as isize);
        if let Some(k) = last
            && wins.iter().any(|w| w.hwnd.0 as isize == k && !w.iconic)
        {
            self.neon_target = Some(k);
        }
        if self.neon_target.is_some_and(|k| !wins.iter().any(|w| w.hwnd.0 as isize == k && !w.iconic)) {
            self.neon_target = None;
        }
        let fg = self.neon_target;
        let mut live = HashSet::new();
        for w in wins {
            let k = w.hwnd.0 as isize;
            live.insert(k);
            let want = if Some(k) == fg && !w.iconic {
                main
            } else if all {
                dim
            } else {
                winsync::BORDER_DEFAULT
            };
            let have = self.borders.get(&k).copied().unwrap_or(winsync::BORDER_DEFAULT);
            if want != have {
                winsync::set_border(w.hwnd, want);
                if want == winsync::BORDER_DEFAULT {
                    self.borders.remove(&k);
                } else {
                    self.borders.insert(k, want);
                }
            }
        }
        self.borders.retain(|k, _| live.contains(k));
        // a fénylő keret-réteg az aktív ablak körül (teljes méretű ablakon is látszik)
        let target = wins.iter().find(|w| Some(w.hwnd.0 as isize) == fg).map(|w| w.hwnd);
        let color = winsync::parse_hex(&self.cfg.active_theme().main).unwrap_or((243, 230, 0));
        neon::set_target(target, color);
    }

    /// Minden átszínezett keret vissza a Windows sajátjára (kikapcsoláskor és kilépéskor).
    pub(crate) fn reset_borders(&mut self) {
        self.neon_target = None;
        neon::set_target(None, (0, 0, 0));
        for (k, _) in self.borders.drain() {
            winsync::set_border(HWND(k as *mut core::ffi::c_void), winsync::BORDER_DEFAULT);
        }
    }
}
