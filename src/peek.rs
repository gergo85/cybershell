//! Ablakelőnézet: ha egy programnak több ablaka van nyitva, az ikonja fölött időzve
//! egymás mellett látszik az ablakok élő kisképe. A kisképeket a Windows (DWM) rajzolja
//! az előnézet-ablakba, a keretet és a címeket a felület.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
    DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

/// Az előnézet elrendezése logikai (CSS) pixelben.
pub const PAD: f64 = 14.0;
pub const HEADER: f64 = 26.0;
pub const GAP: f64 = 12.0;
pub const CARD_W: f64 = 236.0;
pub const TITLE_H: f64 = 30.0;
pub const THUMB_H: f64 = 140.0;
/// A kiskép és a kártya széle közti hely.
pub const INSET: f64 = 6.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Rectf {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rectf {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({ "x": self.x, "y": self.y, "w": self.w, "h": self.h })
    }
    pub fn to_phys(self, scale: f64) -> RECT {
        RECT {
            left: (self.x * scale).round() as i32,
            top: (self.y * scale).round() as i32,
            right: ((self.x + self.w) * scale).round() as i32,
            bottom: ((self.y + self.h) * scale).round() as i32,
        }
    }
}

/// Az ablak látható része (a Windows 11 láthatatlan átméretező szegélye nélkül),
/// az ablak saját bal felső sarkához viszonyítva.
pub fn visible_source(src: HWND) -> Option<RECT> {
    unsafe {
        let mut wr = RECT::default();
        GetWindowRect(src, &mut wr).ok()?;
        let mut ext = RECT::default();
        let got =
            DwmGetWindowAttribute(src, DWMWA_EXTENDED_FRAME_BOUNDS, &mut ext as *mut RECT as *mut _, std::mem::size_of::<RECT>() as u32);
        let r = if got.is_ok() && ext.right > ext.left && ext.bottom > ext.top {
            RECT { left: ext.left - wr.left, top: ext.top - wr.top, right: ext.right - wr.left, bottom: ext.bottom - wr.top }
        } else {
            RECT { left: 0, top: 0, right: wr.right - wr.left, bottom: wr.bottom - wr.top }
        };
        (r.right > r.left && r.bottom > r.top).then_some(r)
    }
}

/// A forrás méretarányát megtartva a dobozba illeszti (középre).
pub fn fit(boxr: Rectf, src: &RECT) -> Rectf {
    let sw = (src.right - src.left).max(1) as f64;
    let sh = (src.bottom - src.top).max(1) as f64;
    let k = (boxr.w / sw).min(boxr.h / sh);
    let (w, h) = (sw * k, sh * k);
    Rectf { x: boxr.x + (boxr.w - w) / 2.0, y: boxr.y + (boxr.h - h) / 2.0, w, h }
}

/// Élő kiskép bejegyzése: a `src` ablak képe a `dest` ablak `rect` területén.
pub fn show_thumb(dest: HWND, src: HWND, rect: RECT, source: RECT) -> Option<isize> {
    unsafe {
        let id = DwmRegisterThumbnail(dest, src).ok()?;
        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_RECTSOURCE | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: rect,
            rcSource: source,
            opacity: 255,
            fVisible: true.into(),
            fSourceClientAreaOnly: false.into(),
        };
        if DwmUpdateThumbnailProperties(id, &props).is_err() {
            let _ = DwmUnregisterThumbnail(id);
            return None;
        }
        Some(id)
    }
}

pub fn drop_thumb(id: isize) {
    unsafe {
        let _ = DwmUnregisterThumbnail(id);
    }
}

/// Egy előnézeti kártya fölött időzve előtérbe hozott ablak, és ami a visszaállításához kell.
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    pub hwnd: isize,
    /// Az ablak fölötti első (nem mindig legfelül lévő, látható) ablak; 0, ha ő volt legfelül.
    pub above: isize,
    /// Kis méretre volt téve: elengedéskor visszakerül a tálcára.
    pub iconic: bool,
}

/// Az ablakot (aktiválás nélkül) a többi fölé hozza, mint a Windows tálcáján a kiskép
/// fölötti időzés; a kis méretre tett ablakot ehhez visszaállítja.
pub fn raise(h: HWND) -> Shown {
    use windows::Win32::UI::WindowsAndMessaging::{
        GW_HWNDPREV, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, IsWindowVisible, SW_SHOWNOACTIVATE,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow, WS_EX_TOPMOST,
    };
    unsafe {
        let iconic = IsIconic(h).as_bool();
        let mut above = 0isize;
        if !iconic {
            let mut w = GetWindow(h, GW_HWNDPREV).ok();
            while let Some(p) = w {
                let topmost = (GetWindowLongPtrW(p, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0) != 0;
                if topmost {
                    break;
                }
                if IsWindowVisible(p).as_bool() {
                    above = p.0 as isize;
                    break;
                }
                w = GetWindow(p, GW_HWNDPREV).ok();
            }
        }
        if iconic {
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
        }
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
        let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
        let _ = SetWindowPos(h, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        Shown { hwnd: h.0 as isize, above, iconic }
    }
}

/// Az előtérbe hozott ablakot visszateszi a helyére (vagy újra kis méretre).
pub fn unraise(s: Shown) {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindow, SW_SHOWMINNOACTIVE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow,
    };
    let h = HWND(s.hwnd as *mut core::ffi::c_void);
    unsafe {
        if !IsWindow(Some(h)).as_bool() {
            return;
        }
        if s.iconic {
            let _ = ShowWindow(h, SW_SHOWMINNOACTIVE);
        } else if s.above != 0 {
            let a = HWND(s.above as *mut core::ffi::c_void);
            if IsWindow(Some(a)).as_bool() {
                let _ = SetWindowPos(h, Some(a), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
}
