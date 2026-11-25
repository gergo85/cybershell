//! Windows-bitképek (programikonok, a héj képei) PNG-vé alakítása a felületnek.

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDC, GetDIBits, GetObjectW, HBITMAP, HGDIOBJ, ReleaseDC,
};

/// A bitkép mérete a `BITMAP` leírása szerint: szélesség és magasság (ez negatív is lehet).
pub fn size(bmp: HBITMAP) -> (i32, i32) {
    let mut bm = BITMAP::default();
    unsafe {
        GetObjectW(HGDIOBJ(bmp.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
    }
    (bm.bmWidth, bm.bmHeight)
}

/// A bitkép képpontjai 32 bites BGRA-ként, fentről lefelé (`w` × `h`, mindkettő pozitív).
pub fn pixels(bmp: HBITMAP, w: i32, h: i32) -> Option<Vec<u8>> {
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w * h * 4) as usize];
    let lines = unsafe {
        let dc = GetDC(None);
        let lines = GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        lines
    };
    (lines != 0).then_some(px)
}

/// BGRA képpontok PNG-ként. A régi, alfa nélküli képek teljesen átlátszatlanok lesznek;
/// `premultiplied`: a kép előszorzott alfájú (a héj képei ilyenek), ezt egyenes alfára alakítjuk.
pub fn bgra_to_png(mut px: Vec<u8>, w: i32, h: i32, premultiplied: bool) -> Option<Vec<u8>> {
    let has_alpha = px.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2); // BGRA → RGBA
        if !has_alpha {
            p[3] = 255;
        } else if premultiplied && p[3] > 0 && p[3] < 255 {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&px).ok()?;
    }
    Some(out)
}
