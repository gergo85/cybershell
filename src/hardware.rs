//! A gép alkatrészei a beállítások Rendszer fülének: processzor, alaplap (BIOS, foglalatok),
//! memóriamodulok, videókártya, háttértárak, Windows-változat. A Windows WMI-adatbázisából
//! (wmi.rs), a videomemória a registryből; a WMI-ben nem szereplő adatokat (memóriatípus,
//! buszszélesség, órajel) egy táblázat adja az ismert videókártyákhoz.

use crate::reg;

/// A gép alkatrészei: alaplap (BIOS, foglalatok), processzor, memóriamodulok, videókártya,
/// Windows-változat. Közvetlenül a Windows WMI-adatbázisából, de így is lassú (fél–egy mp):
/// háttérszálon hívd. A szálon a COM-ot maga indítja el és zárja le.
pub fn query() -> Option<serde_json::Value> {
    Some(summary(&raw()?))
}

/// A nyers adatok, a `summary` bemenete: osztályonként a WMI-tulajdonságok nevével.
fn raw() -> Option<serde_json::Value> {
    use crate::com::ComThread;
    use crate::wmi::Wmi;
    use serde_json::{Value, json};
    // a COM elsőként indul és utolsóként zárul, a WMI-kapcsolatok előtte szűnnek meg
    let _com = ComThread::init();
    let cim = Wmi::connect(r"root\cimv2")?;
    let rows = |class: &str, props: &[&str], filter: &str| -> Vec<Value> {
        cim.query(class, props, filter).into_iter().map(Value::Object).collect()
    };
    let first = |v: Vec<Value>| v.into_iter().next().unwrap_or(Value::Null);
    let cpu = first(rows("Win32_Processor", &["Name", "NumberOfCores", "NumberOfLogicalProcessors", "MaxClockSpeed"], ""));
    let board = first(rows("Win32_BaseBoard", &["Manufacturer", "Product"], ""));
    let bios = first(rows("Win32_BIOS", &["SMBIOSBIOSVersion", "ReleaseDate"], ""));
    let bios = json!({
        "SMBIOSBIOSVersion": bios["SMBIOSBIOSVersion"],
        "date": bios["ReleaseDate"].as_str().and_then(cim_date),
    });
    // 3: a rendszermemória (nem a videókártyáé vagy a gyorsítótáré)
    let slots = first(rows("Win32_PhysicalMemoryArray", &["MemoryDevices", "MaxCapacityEx"], "Use = 3"));
    let mem = rows(
        "Win32_PhysicalMemory",
        &["Capacity", "SMBIOSMemoryType", "Speed", "ConfiguredClockSpeed", "Manufacturer", "PartNumber", "DeviceLocator", "BankLabel"],
        "",
    );
    let gpu = rows("Win32_VideoController", &["Name", "AdapterRAM"], "");
    let os = first(rows("Win32_OperatingSystem", &["Caption", "BuildNumber"], ""));
    let vols = rows("Win32_LogicalDisk", &["DeviceID", "Size", "FreeSpace", "VolumeName"], "DriveType = 3");
    // a fizikai lemezek és a partíciók a tárolókezelő saját névteréből (ha elérhető)
    let (mut disks, mut parts) = (Vec::new(), Vec::new());
    if let Some(st) = Wmi::connect(r"root\Microsoft\Windows\Storage") {
        for mut d in st.query("MSFT_PhysicalDisk", &["DeviceId", "FriendlyName", "MediaType", "BusType", "Size"], "") {
            let media = d["MediaType"].as_u64().map(disk_media).unwrap_or("Unspecified");
            let bus = d["BusType"].as_u64().map(disk_bus).unwrap_or("Unknown");
            d.insert("MediaType".into(), media.into());
            d.insert("BusType".into(), bus.into());
            disks.push(Value::Object(d));
        }
        parts = st
            .query("MSFT_Partition", &["DiskNumber", "DriveLetter"], "")
            .into_iter()
            .filter_map(|p| Some(json!({ "DiskNumber": p["DiskNumber"], "L": p["DriveLetter"].as_str()? })))
            .collect();
    }
    Some(json!({
        "cpu": cpu, "board": board, "bios": bios, "slots": slots, "mem": mem, "gpu": gpu,
        "gpumem": gpu_memory(), "os": os, "disks": disks, "parts": parts, "vols": vols,
    }))
}

/// A WMI dátuma („20240822000000.000000+000”) éééé-hh-nn alakban.
fn cim_date(s: &str) -> Option<String> {
    let d = s.get(..8).filter(|d| d.bytes().all(|b| b.is_ascii_digit()))?;
    Some(format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..8]))
}

/// A lemez fajtája (MSFT_PhysicalDisk.MediaType), ahogy a Windows is írja.
fn disk_media(n: u64) -> &'static str {
    match n {
        3 => "HDD",
        4 => "SSD",
        5 => "SCM",
        _ => "Unspecified",
    }
}

/// A lemez csatlakozása (MSFT_PhysicalDisk.BusType), ahogy a Windows is írja.
fn disk_bus(n: u64) -> &'static str {
    match n {
        1 => "SCSI",
        2 => "ATAPI",
        3 => "ATA",
        4 => "1394",
        5 => "SSA",
        6 => "Fibre Channel",
        7 => "USB",
        8 => "RAID",
        9 => "iSCSI",
        10 => "SAS",
        11 => "SATA",
        12 => "SD",
        13 => "MMC",
        15 => "File Backed Virtual",
        16 => "Storage Spaces",
        17 => "NVMe",
        18 => "SCM",
        19 => "UFS",
        _ => "Unknown",
    }
}

/// A videókártyák pontos memóriamérete a meghajtójuk beállításaiból (a WMI 4 GB-nál levág):
/// a kijelzőosztály kulcsai alatt (0000, 0001…) a `HardwareInformation.qwMemorySize` érték.
fn gpu_memory() -> Vec<serde_json::Value> {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    const CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
    let mut out = Vec::new();
    // a „Properties” és a „Configuration” nem eszköz
    for sub in reg::subkeys(HKEY_LOCAL_MACHINE, CLASS).into_iter().filter(|s| s.starts_with('0')) {
        let key = format!(r"{CLASS}\{sub}");
        // a méret REG_QWORD, egyes meghajtóknál 8 (régebben 4) bájtos REG_BINARY
        let mem = match reg::get_raw(HKEY_LOCAL_MACHINE, &key, "HardwareInformation.qwMemorySize").as_deref() {
            Some(&[a, b, c, d, e, f, g, h]) => u64::from_le_bytes([a, b, c, d, e, f, g, h]),
            Some(&[a, b, c, d]) => u32::from_le_bytes([a, b, c, d]) as u64,
            _ => 0,
        };
        if mem == 0 {
            continue;
        }
        let desc = reg::get_string(HKEY_LOCAL_MACHINE, &key, "DriverDesc").unwrap_or_default();
        out.push(serde_json::json!({ "DriverDesc": desc, "mem": mem }));
    }
    out
}

/// Ismert videókártyák adatai, amit a Windows nem ad meg: memóriatípus, buszszélesség (bit),
/// a memória sebessége (Gbps) és a grafikus processzor boost-órajele (MHz). Az asztali
/// változatokra; a hosszabb (pontosabb) név előrébb, hogy pl. az „RX 6600 XT” ne „RX 6600”
/// legyen.
const GPU_SPECS: &[(&str, &str, u32, f32, u32)] = &[
    // AMD
    ("RX 9070 XT", "GDDR6", 256, 20.0, 2970),
    ("RX 9070", "GDDR6", 256, 20.0, 2520),
    ("RX 9060 XT", "GDDR6", 128, 20.0, 3130),
    ("RX 7900 XTX", "GDDR6", 384, 20.0, 2500),
    ("RX 7900 GRE", "GDDR6", 256, 18.0, 2245),
    ("RX 7900 XT", "GDDR6", 320, 20.0, 2400),
    ("RX 7800 XT", "GDDR6", 256, 19.5, 2430),
    ("RX 7700 XT", "GDDR6", 192, 18.0, 2544),
    ("RX 7600 XT", "GDDR6", 128, 18.0, 2755),
    ("RX 7600", "GDDR6", 128, 18.0, 2655),
    ("RX 6950 XT", "GDDR6", 256, 18.0, 2310),
    ("RX 6900 XT", "GDDR6", 256, 16.0, 2250),
    ("RX 6800 XT", "GDDR6", 256, 16.0, 2250),
    ("RX 6800", "GDDR6", 256, 16.0, 2105),
    ("RX 6750 XT", "GDDR6", 192, 18.0, 2600),
    ("RX 6700 XT", "GDDR6", 192, 16.0, 2581),
    ("RX 6700", "GDDR6", 160, 16.0, 2450),
    ("RX 6650 XT", "GDDR6", 128, 17.5, 2635),
    ("RX 6600 XT", "GDDR6", 128, 16.0, 2589),
    ("RX 6600", "GDDR6", 128, 14.0, 2491),
    ("RX 6500 XT", "GDDR6", 64, 18.0, 2815),
    ("RX 6400", "GDDR6", 64, 16.0, 2321),
    ("RX 5700 XT", "GDDR6", 256, 14.0, 1905),
    ("RX 5700", "GDDR6", 256, 14.0, 1725),
    ("RX 5600 XT", "GDDR6", 192, 14.0, 1560),
    ("RX 5500 XT", "GDDR6", 128, 14.0, 1845),
    ("RX 590", "GDDR5", 256, 8.0, 1545),
    ("RX 580", "GDDR5", 256, 8.0, 1340),
    ("RX 570", "GDDR5", 256, 7.0, 1244),
    // NVIDIA
    ("RTX 5090", "GDDR7", 512, 28.0, 2407),
    ("RTX 5080", "GDDR7", 256, 30.0, 2617),
    ("RTX 5070 TI", "GDDR7", 256, 28.0, 2452),
    ("RTX 5070", "GDDR7", 192, 28.0, 2512),
    ("RTX 5060 TI", "GDDR7", 128, 28.0, 2572),
    ("RTX 5060", "GDDR7", 128, 28.0, 2497),
    ("RTX 4090", "GDDR6X", 384, 21.0, 2520),
    ("RTX 4080 SUPER", "GDDR6X", 256, 23.0, 2550),
    ("RTX 4080", "GDDR6X", 256, 22.4, 2505),
    ("RTX 4070 TI SUPER", "GDDR6X", 256, 21.0, 2610),
    ("RTX 4070 TI", "GDDR6X", 192, 21.0, 2610),
    ("RTX 4070 SUPER", "GDDR6X", 192, 21.0, 2475),
    ("RTX 4070", "GDDR6X", 192, 21.0, 2475),
    ("RTX 4060 TI", "GDDR6", 128, 18.0, 2535),
    ("RTX 4060", "GDDR6", 128, 17.0, 2460),
    ("RTX 3090 TI", "GDDR6X", 384, 21.0, 1860),
    ("RTX 3090", "GDDR6X", 384, 19.5, 1695),
    ("RTX 3080 TI", "GDDR6X", 384, 19.0, 1665),
    ("RTX 3080", "GDDR6X", 320, 19.0, 1710),
    ("RTX 3070 TI", "GDDR6X", 256, 19.0, 1770),
    ("RTX 3070", "GDDR6", 256, 14.0, 1725),
    ("RTX 3060 TI", "GDDR6", 256, 14.0, 1665),
    ("RTX 3060", "GDDR6", 192, 15.0, 1777),
    ("RTX 3050", "GDDR6", 128, 14.0, 1777),
    ("RTX 2080 TI", "GDDR6", 352, 14.0, 1545),
    ("RTX 2080 SUPER", "GDDR6", 256, 15.5, 1815),
    ("RTX 2080", "GDDR6", 256, 14.0, 1710),
    ("RTX 2070 SUPER", "GDDR6", 256, 14.0, 1770),
    ("RTX 2070", "GDDR6", 256, 14.0, 1620),
    ("RTX 2060 SUPER", "GDDR6", 256, 14.0, 1650),
    ("RTX 2060", "GDDR6", 192, 14.0, 1680),
    ("GTX 1660 TI", "GDDR6", 192, 12.0, 1770),
    ("GTX 1660 SUPER", "GDDR6", 192, 14.0, 1785),
    ("GTX 1660", "GDDR5", 192, 8.0, 1785),
    ("GTX 1080 TI", "GDDR5X", 352, 11.0, 1582),
    ("GTX 1080", "GDDR5X", 256, 10.0, 1733),
    ("GTX 1070", "GDDR5", 256, 8.0, 1683),
    ("GTX 1060", "GDDR5", 192, 8.0, 1708),
    // Intel
    ("ARC B580", "GDDR6", 192, 19.0, 2670),
    ("ARC B570", "GDDR6", 160, 19.0, 2500),
    ("ARC A770", "GDDR6", 256, 17.5, 2400),
    ("ARC A750", "GDDR6", 256, 16.0, 2400),
    ("ARC A580", "GDDR6", 256, 16.0, 2000),
    ("ARC A380", "GDDR6", 96, 15.5, 2000),
];

/// Egy videókártya ismert adatai a neve alapján (laptopos változatnál nincs: ott mások).
fn gpu_spec(name: &str) -> Option<(&'static str, u32, f32, u32)> {
    let up = name.to_uppercase();
    if up.contains("LAPTOP") || up.contains("MOBILE") || up.contains("MAX-Q") {
        return None;
    }
    let mut specs: Vec<&(&str, &str, u32, f32, u32)> = GPU_SPECS.iter().collect();
    specs.sort_by_key(|s| std::cmp::Reverse(s.0.len()));
    specs
        .into_iter()
        .find(|s| {
            up.match_indices(s.0).any(|(i, _)| {
                let after = &up[i + s.0.len()..];
                // a név itt véget ér (pl. „RX 6600” után nem „0” jön)
                !after.starts_with(|c: char| c.is_ascii_alphanumeric())
            })
        })
        .map(|s| (s.1, s.2, s.3, s.4))
}

/// Az alaplap lapkakészlete a típusnevéből (pl. „B450M Pro4” → B450, „X670E Taichi” → X670E).
fn chipset_of(product: &str) -> Option<String> {
    product.split(|c: char| !c.is_ascii_alphanumeric()).find_map(|tok| {
        let t = tok.to_uppercase();
        let b = t.as_bytes();
        if b.len() < 4 || !b"ABXHZQ".contains(&b[0]) || !b[1..4].iter().all(u8::is_ascii_digit) {
            return None;
        }
        match &t[4..] {
            "" | "M" => Some(t[..4].to_string()),
            "E" | "EM" => Some(format!("{}E", &t[..4])),
            _ => None,
        }
    })
}

/// A felületnek szánt összefoglaló a nyers adatokból (a beállítások Rendszer fülén ez látszik).
fn summary(raw: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;
    let s = |v: &serde_json::Value| v.as_str().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
    let list = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        match v {
            serde_json::Value::Array(a) => a.clone(),
            serde_json::Value::Null => Vec::new(),
            o => vec![o.clone()],
        }
    };
    let gb = |b: f64| {
        let g = b / 1_073_741_824.0;
        if g >= 10.0 || (g - g.round()).abs() < 0.05 { format!("{:.0} GB", g) } else { format!("{:.1} GB", g) }
    };
    let c = &raw["cpu"];
    let cpu_name = s(&c["Name"]);
    let cpu = json!({
        "name": cpu_name,
        "cores": c["NumberOfCores"],
        "threads": c["NumberOfLogicalProcessors"],
        "mhz": c["MaxClockSpeed"],
    });
    // alaplap: gyártó és típus, lapkakészlet, BIOS, memóriafoglalatok
    let product = s(&raw["board"]["Product"]);
    let vendor = if cpu_name.to_uppercase().contains("INTEL") {
        "Intel"
    } else if cpu_name.to_uppercase().contains("AMD") {
        "AMD"
    } else {
        ""
    };
    let chipset = chipset_of(&product).map(|c| format!("{vendor} {c}").trim().to_string());
    let slots_total = raw["slots"]["MemoryDevices"].as_u64().unwrap_or(0);
    let max_mem = raw["slots"]["MaxCapacityEx"].as_f64().map(|kb| gb(kb * 1024.0)).unwrap_or_default();
    let board = json!({
        "name": format!("{} {}", s(&raw["board"]["Manufacturer"]), product).trim(),
        "chipset": chipset,
        "bios": s(&raw["bios"]["SMBIOSBIOSVersion"]),
        "biosDate": s(&raw["bios"]["date"]),
        "slots": slots_total,
        "maxMem": max_mem,
    });
    // memória: modulonként (méret, típus, sebesség, gyártó, típusszám, foglalat)
    let kind = |t: u64| match t {
        20 => "DDR",
        21 | 22 => "DDR2",
        24 => "DDR3",
        26 => "DDR4",
        27 => "LPDDR",
        28 => "LPDDR2",
        29 => "LPDDR3",
        30 => "LPDDR4",
        34 => "DDR5",
        35 => "LPDDR5",
        _ => "",
    };
    let mut total = 0f64;
    let mut modules = Vec::new();
    for m in list(&raw["mem"]) {
        let cap = m["Capacity"].as_f64().unwrap_or(0.0);
        total += cap;
        let maker = s(&m["Manufacturer"]);
        let maker = if maker.eq_ignore_ascii_case("unknown") || maker.starts_with("0000") { String::new() } else { maker };
        modules.push(json!({
            "size": gb(cap),
            "type": kind(m["SMBIOSMemoryType"].as_u64().unwrap_or(0)),
            "speed": m["Speed"].as_u64().unwrap_or(0),
            "configured": m["ConfiguredClockSpeed"].as_u64().unwrap_or(0),
            "maker": maker,
            "part": s(&m["PartNumber"]),
            "slot": s(&m["DeviceLocator"]),
        }));
    }
    let types: Vec<&str> = modules.iter().filter_map(|m| m["type"].as_str()).filter(|t| !t.is_empty()).collect();
    let mem_type = if !types.is_empty() && types.iter().all(|t| *t == types[0]) { types[0].to_string() } else { String::new() };
    // a modulok a leglassabb sebességén járnak együtt
    let run = modules
        .iter()
        .filter_map(|m| m["configured"].as_u64().filter(|v| *v > 0).or(m["speed"].as_u64()))
        .filter(|v| *v > 0)
        .min()
        .unwrap_or(0);
    let same = modules.windows(2).all(|w| w[0]["size"] == w[1]["size"] && w[0]["maker"] == w[1]["maker"] && w[0]["part"] == w[1]["part"]);
    // videókártyák: a pontos memóriaméret a registryből (a WMI 4 GB-nál levág), a többi a táblából
    let vram: Vec<(String, f64)> = list(&raw["gpumem"]).iter().map(|g| (s(&g["DriverDesc"]), g["mem"].as_f64().unwrap_or(0.0))).collect();
    let gpus: Vec<serde_json::Value> = list(&raw["gpu"])
        .iter()
        .map(|g| {
            let name = s(&g["Name"]);
            let mem = vram.iter().find(|(n, _)| *n == name).map(|(_, m)| *m).or(g["AdapterRAM"].as_f64()).unwrap_or(0.0);
            let spec = gpu_spec(&name);
            json!({
                "name": name,
                "vram": if mem > 0.0 { gb(mem) } else { String::new() },
                "memType": spec.map(|x| x.0),
                "bus": spec.map(|x| x.1),
                "gbps": spec.map(|x| x.2),
                "boost": spec.map(|x| x.3),
            })
        })
        .filter(|g| !g["name"].as_str().unwrap_or("").is_empty())
        .collect();
    let os = format!("{} ({})", s(&raw["os"]["Caption"]).trim_start_matches("Microsoft "), s(&raw["os"]["BuildNumber"]));
    // háttértárak: a gyártói (tízes) méret, és a rajtuk lévő meghajtók szabad helye (mint az Intézőben)
    let dec = |b: f64| {
        if b >= 1e12 { format!("{} TB", format!("{:.1}", b / 1e12).trim_end_matches(".0")) } else { format!("{:.0} GB", b / 1e9) }
    };
    let vols = list(&raw["vols"]);
    let disks: Vec<serde_json::Value> = list(&raw["disks"])
        .iter()
        .map(|d| {
            let id = d["DeviceId"].as_str().map(str::to_string).or_else(|| d["DeviceId"].as_u64().map(|n| n.to_string())).unwrap_or_default();
            let letters: Vec<String> = list(&raw["parts"])
                .iter()
                .filter(|p| p["DiskNumber"].as_u64().map(|n| n.to_string()) == Some(id.clone()))
                .filter_map(|p| p["L"].as_str().map(|l| format!("{l}:")))
                .collect();
            let volumes: Vec<serde_json::Value> = letters
                .iter()
                .filter_map(|l| vols.iter().find(|v| v["DeviceID"].as_str() == Some(l.as_str())))
                .map(|v| {
                    json!({
                        "letter": s(&v["DeviceID"]),
                        "label": s(&v["VolumeName"]),
                        "free": gb(v["FreeSpace"].as_f64().unwrap_or(0.0)),
                        "total": gb(v["Size"].as_f64().unwrap_or(0.0)),
                        "pct": v["Size"].as_f64().filter(|t| *t > 0.0).map(|t| (100.0 * (1.0 - v["FreeSpace"].as_f64().unwrap_or(0.0) / t)).round()),
                    })
                })
                .collect();
            let text = |v: &serde_json::Value| match v {
                serde_json::Value::String(x) if x != "Unspecified" => x.clone(),
                _ => String::new(),
            };
            json!({
                "name": s(&d["FriendlyName"]),
                "media": text(&d["MediaType"]),
                "bus": text(&d["BusType"]),
                "size": dec(d["Size"].as_f64().unwrap_or(0.0)),
                "volumes": volumes,
            })
        })
        .collect();
    json!({
        "board": board,
        "cpu": cpu,
        "mem": {
            "total": if total > 0.0 { gb(total) } else { String::new() },
            "type": mem_type,
            "speed": run,
            "slots": slots_total,
            "same": same,
            "modules": modules,
        },
        "gpus": gpus,
        "disks": disks,
        "os": os,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_names() {
        assert_eq!(gpu_spec("AMD Radeon RX 6600").map(|s| s.1), Some(128));
        assert_eq!(gpu_spec("AMD Radeon RX 6600 XT").map(|s| s.3), Some(2589));
        assert_eq!(gpu_spec("NVIDIA GeForce RTX 4070 Ti SUPER").map(|s| s.1), Some(256));
        assert_eq!(gpu_spec("NVIDIA GeForce RTX 4070 Laptop GPU"), None);
        assert_eq!(gpu_spec("Microsoft Basic Display Adapter"), None);
    }

    #[test]
    fn chipsets() {
        assert_eq!(chipset_of("B450M Pro4").as_deref(), Some("B450"));
        assert_eq!(chipset_of("X670E Taichi").as_deref(), Some("X670E"));
        assert_eq!(chipset_of("ROG STRIX Z790-E GAMING WIFI").as_deref(), Some("Z790"));
        assert_eq!(chipset_of("PRIME"), None);
    }

    #[test]
    fn wmi_values() {
        assert_eq!(cim_date("20240822000000.000000+000").as_deref(), Some("2024-08-22"));
        assert_eq!(cim_date("2024"), None);
        assert_eq!(cim_date("ismeretlen"), None);
        assert_eq!((disk_media(4), disk_media(3), disk_media(0)), ("SSD", "HDD", "Unspecified"));
        assert_eq!((disk_bus(17), disk_bus(11), disk_bus(7), disk_bus(99)), ("NVMe", "SATA", "USB", "Unknown"));
    }

    /// Egy valódi gép nyers adataiból a felületnek szánt összefoglaló.
    #[test]
    fn summary_of_sample() {
        let raw = serde_json::json!({
            "cpu": {"Name": "AMD Ryzen 5 3600 6-Core Processor   ", "NumberOfCores": 6, "NumberOfLogicalProcessors": 12, "MaxClockSpeed": 3600},
            "board": {"Manufacturer": "ASRock", "Product": "B450M Pro4"},
            "bios": {"SMBIOSBIOSVersion": "P10.31", "date": "2024-08-22"},
            "slots": {"MemoryDevices": 4, "MaxCapacityEx": 134217728u64},
            "mem": [
                {"Capacity": 8589934592u64, "SMBIOSMemoryType": 26, "Speed": 3200, "ConfiguredClockSpeed": 3200, "Manufacturer": "Kingston", "PartNumber": "KF3200C16D4/8GX  ", "DeviceLocator": "DIMM 0"},
                {"Capacity": 8589934592u64, "SMBIOSMemoryType": 26, "Speed": 3200, "ConfiguredClockSpeed": 2933, "Manufacturer": "Kingston", "PartNumber": "KF3200C16D4/8GX  ", "DeviceLocator": "DIMM 1"},
            ],
            "gpu": [{"Name": "AMD Radeon RX 6600", "AdapterRAM": 4293918720u64}],
            "gpumem": [{"DriverDesc": "AMD Radeon RX 6600", "mem": 8573157376u64}],
            "os": {"Caption": "Microsoft Windows 11 Pro", "BuildNumber": "26300"},
            "disks": [{"DeviceId": "3", "FriendlyName": "Samsung SSD 970 EVO Plus 500GB", "MediaType": "SSD", "BusType": "NVMe", "Size": 500107862016u64}],
            "parts": [{"DiskNumber": 3, "L": "C"}, {"DiskNumber": 1, "L": "D"}],
            "vols": [{"DeviceID": "C:", "Size": 499016265728u64, "FreeSpace": 33819176960u64, "VolumeName": ""}],
        });
        let s = summary(&raw);
        assert_eq!(s["cpu"]["name"], "AMD Ryzen 5 3600 6-Core Processor");
        assert_eq!(s["board"]["name"], "ASRock B450M Pro4");
        assert_eq!(s["board"]["chipset"], "AMD B450");
        assert_eq!(s["board"]["maxMem"], "128 GB");
        assert_eq!(s["mem"]["total"], "16 GB");
        assert_eq!(s["mem"]["type"], "DDR4");
        assert_eq!(s["mem"]["speed"], 2933);
        assert_eq!(s["mem"]["modules"][0]["part"], "KF3200C16D4/8GX");
        assert_eq!(s["gpus"][0]["vram"], "8 GB");
        assert_eq!(s["gpus"][0]["bus"], 128);
        assert_eq!(s["os"], "Windows 11 Pro (26300)");
        let d = &s["disks"][0];
        assert_eq!((d["media"].as_str(), d["bus"].as_str(), d["size"].as_str()), (Some("SSD"), Some("NVMe"), Some("500 GB")));
        assert_eq!(d["volumes"][0]["letter"], "C:");
        assert_eq!(d["volumes"][0]["pct"], 93.0);
    }

    /// Élő lekérdezés ezen a gépen (COM nélküli szálon, ahogy a beállítások hívják).
    #[test]
    fn live_hardware() {
        let h = std::thread::spawn(query).join().unwrap().expect("WMI-lekérdezés");
        assert!(h["cpu"]["name"].as_str().is_some_and(|n| !n.is_empty()));
        assert!(h["cpu"]["threads"].as_u64().is_some_and(|n| n > 0));
        assert!(h["os"].as_str().is_some_and(|o| o.starts_with("Windows")));
        assert!(!h["mem"]["total"].as_str().unwrap_or("").is_empty());
    }
}
