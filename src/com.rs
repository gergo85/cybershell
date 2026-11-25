//! COM a háttérszálakon (WMI-lekérdezés, WinRT-hívások, háttérkép-kezelő).

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoIncrementMTAUsage, CoInitializeEx, CoUninitialize};

/// COM a hívó szálon (többszálú módban); megszűnésekor lezárja, ha ő indította el.
/// Előbb ezt hozd létre, és csak utána a COM-objektumokat, hogy azok előbb szűnjenek meg.
pub struct ComThread(bool);

impl ComThread {
    pub fn init() -> ComThread {
        // A többszálú COM-tér (MTA) a program végéig éljen, akkor is, ha épp egy szál sincs
        // benne. Különben az utolsó ilyen szál lezárásakor megszűnne, és vele a windows crate
        // által eltárolt WinRT-gyárak (pl. StartupTask) is: a következő hívás összeomlana.
        static KEEP_MTA: std::sync::Once = std::sync::Once::new();
        KEEP_MTA.call_once(|| {
            let _ = unsafe { CoIncrementMTAUsage() };
        });
        // S_FALSE: már el volt indítva (azt is le kell zárni); RPC_E_CHANGED_MODE: más módban fut, maradhat
        ComThread(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}

impl Drop for ComThread {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}
