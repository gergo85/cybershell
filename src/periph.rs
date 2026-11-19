//! A Bluetooth-eszközök (egér, billentyűzet, fejhallgató…) akkumulátor-töltöttsége a Windows
//! eszközkezelőjéből (CfgMgr32). A töltöttséget jelző eszközök a `{104EA319-…} 2` tulajdonságba
//! írják (ugyanezt mutatja a Windows Bluetooth-beállítása). Egy fizikai eszközhöz több eszközcsomópont
//! is tartozhat (pl. a fejhallgató sztereó és kihangosító része): ezeket a tároló (ContainerId)
//! szerint vonjuk össze.

use std::collections::BTreeMap;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_GETIDLIST_FILTER_PRESENT, CM_Get_DevNode_PropertyW, CM_Get_Device_ID_List_SizeW, CM_Get_Device_ID_ListW, CM_LOCATE_DEVNODE_NORMAL,
    CM_Locate_DevNodeW, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_ContainerId, DEVPKEY_Device_FriendlyName, DEVPKEY_NAME, DEVPROP_TYPE_BOOLEAN, DEVPROP_TYPE_BYTE, DEVPROP_TYPE_GUID,
    DEVPROP_TYPE_STRING, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{GUID, PCWSTR};

/// A Bluetooth-eszköz töltöttsége (0–100, egy bájt).
const DEVPKEY_BLUETOOTH_BATTERY: DEVPROPKEY = DEVPROPKEY { fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5), pid: 2 };
/// Csatlakoztatva van-e most az eszköz (a párosított, de kikapcsolt eszköz is „jelen van”).
const DEVPKEY_DEVICE_IS_CONNECTED: DEVPROPKEY = DEVPROPKEY { fmtid: GUID::from_u128(0x83da6326_97a6_4088_9453_a1923f573b29), pid: 15 };

/// Egy tulajdonság nyers értéke és típusa.
fn prop(dev: u32, key: &DEVPROPKEY) -> Option<(DEVPROPTYPE, Vec<u8>)> {
    let mut ty = DEVPROPTYPE::default();
    let mut size = 0u32;
    unsafe {
        let _ = CM_Get_DevNode_PropertyW(dev, key, &mut ty, None, &mut size, 0);
    }
    if size == 0 || size > 4096 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    let r = unsafe { CM_Get_DevNode_PropertyW(dev, key, &mut ty, Some(buf.as_mut_ptr()), &mut size, 0) };
    (r == CR_SUCCESS).then(|| {
        buf.truncate(size as usize);
        (ty, buf)
    })
}

fn prop_string(dev: u32, key: &DEVPROPKEY) -> Option<String> {
    let (ty, buf) = prop(dev, key)?;
    if ty != DEVPROP_TYPE_STRING {
        return None;
    }
    let wide: Vec<u16> = buf.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).take_while(|&c| c != 0).collect();
    let s = String::from_utf16_lossy(&wide).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// A jelen lévő eszközök azonosítói (a lista közben bővülhet: akkor még egyszer próbáljuk).
fn device_ids() -> Vec<String> {
    for _ in 0..2 {
        let mut len = 0u32;
        if unsafe { CM_Get_Device_ID_List_SizeW(&mut len, PCWSTR::null(), CM_GETIDLIST_FILTER_PRESENT) } != CR_SUCCESS || len == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u16; len as usize + 1024];
        if unsafe { CM_Get_Device_ID_ListW(PCWSTR::null(), &mut buf, CM_GETIDLIST_FILTER_PRESENT) } == CR_SUCCESS {
            return buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect();
        }
    }
    Vec::new()
}

/// Egy fizikai eszköz adatai (a csomópontjaiból összevonva).
#[derive(Default)]
struct Dev {
    /// Az eszköz saját csomópontjának neve (BTHENUM\DEV_…, BTHLE\DEV_…).
    main_name: Option<String>,
    /// A többi csomópont nevei közül a legrövidebb.
    other_name: Option<String>,
    battery: Option<u8>,
    /// None: egyik csomópont sem jelzi a kapcsolatot (ilyenkor csatlakoztatottnak vesszük).
    connected: Option<bool>,
}

/// A szolgáltatáscsomópontok nevének végződései („Fejhallgató Hands-Free AG”).
const SUFFIXES: [&str; 5] = [" Hands-Free AG", " Hands-Free", " Avrcp Transport", " Stereo", " LE"];

/// A csatlakoztatott, töltöttséget jelző Bluetooth-eszközök: (név, töltöttség %), név szerint.
pub fn list() -> Vec<(String, u8)> {
    let mut devs: BTreeMap<String, Dev> = BTreeMap::new();
    for id in device_ids() {
        let up = id.to_ascii_uppercase();
        if !(up.starts_with("BTHENUM\\") || up.starts_with("BTHLE\\") || up.starts_with("BTHLEDEVICE\\")) {
            continue;
        }
        let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        let mut node = 0u32;
        if unsafe { CM_Locate_DevNodeW(&mut node, PCWSTR(wide.as_ptr()), CM_LOCATE_DEVNODE_NORMAL) } != CR_SUCCESS {
            continue;
        }
        // a tároló azonosítója fogja össze egy eszköz csomópontjait
        let container = match prop(node, &DEVPKEY_Device_ContainerId) {
            Some((ty, b)) if ty == DEVPROP_TYPE_GUID && b.len() == 16 => b.iter().map(|x| format!("{x:02x}")).collect::<String>(),
            _ => up.clone(),
        };
        let d = devs.entry(container).or_default();
        if let Some((ty, b)) = prop(node, &DEVPKEY_BLUETOOTH_BATTERY)
            && ty == DEVPROP_TYPE_BYTE
            && let Some(&v) = b.first()
            && v <= 100
        {
            d.battery = Some(d.battery.map_or(v, |old| old.min(v)));
        }
        if let Some((ty, b)) = prop(node, &DEVPKEY_DEVICE_IS_CONNECTED)
            && ty == DEVPROP_TYPE_BOOLEAN
            && let Some(&v) = b.first()
        {
            d.connected = Some(d.connected.unwrap_or(false) || v != 0);
        }
        let name = prop_string(node, &DEVPKEY_Device_FriendlyName).or_else(|| prop_string(node, &DEVPKEY_NAME));
        if let Some(name) = name {
            if up.contains("\\DEV_") {
                d.main_name.get_or_insert(name);
            } else if d.other_name.as_ref().is_none_or(|o| name.len() < o.len()) {
                d.other_name = Some(name);
            }
        }
    }
    let mut out: Vec<(String, u8)> = devs
        .into_values()
        .filter(|d| d.connected != Some(false))
        .filter_map(|d| {
            let pct = d.battery?;
            let name = d.main_name.or_else(|| {
                d.other_name.map(|mut n| {
                    for s in SUFFIXES {
                        if let Some(stripped) = n.strip_suffix(s) {
                            n = stripped.to_string();
                        }
                    }
                    n
                })
            })?;
            Some((name, pct))
        })
        .collect();
    out.sort_by_key(|a| a.0.to_lowercase());
    out.dedup_by(|a, b| a.0 == b.0);
    out
}
