//! Rendszerfigyelő háttérszál a panel Terhelés és Hálózat moduljának. Kétmásodpercenként mér:
//! processzor és memória, a videókártya terhelése (teljesítményszámlálók, mint a
//! Feladatkezelőben) és hőmérséklete (a Windows grafikus kernelje, Windows 10 2004-től),
//! valamint a hálózati forgalom (a hálózati kártyák bájtszámlálói) és a válaszidő (ping).
//! Csak akkor dolgozik, ha a modul látszik (`configure`); az eredményt a `spawn`-nak adott
//! függvény kapja meg („load” és „net” üzenet a panelnek).

use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_ADAPTER_PERFDATA, D3DKMT_ADAPTERINFO, D3DKMT_CLOSEADAPTER, D3DKMT_ENUMADAPTERS2, D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter,
    D3DKMTEnumAdapters2, D3DKMTQueryAdapterInfo, KMTQAITYPE_ADAPTERPERFDATA,
};
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, ICMP_ECHO_REPLY, IF_TYPE_SOFTWARE_LOOPBACK, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::w;

/// Mérési időköz; a ping ritkábban megy.
const STEP: Duration = Duration::from_secs(2);
const PING_EVERY: Duration = Duration::from_secs(6);

/// Mit mérjen a szál (a panel láthatósága és a beállítások szerint).
struct Want {
    load: bool,
    net: bool,
    ping_host: String,
}

static WANT: Mutex<Want> = Mutex::new(Want { load: false, net: false, ping_host: String::new() });

/// A modulok láthatósága és a ping célja (pl. „1.1.1.1” vagy „google.com”).
pub fn configure(load: bool, net: bool, ping_host: &str) {
    if let Ok(mut w) = WANT.lock() {
        *w = Want { load, net, ping_host: ping_host.to_string() };
    }
}

pub fn spawn(deliver: impl Fn(Value) + Send + 'static) {
    std::thread::spawn(move || {
        let mut load = Load::default();
        let mut net = Net::default();
        loop {
            let (want_load, want_net, host) = match WANT.lock() {
                Ok(w) => (w.load, w.net, w.ping_host.clone()),
                Err(_) => (false, false, String::new()),
            };
            if want_load {
                deliver(load.sample());
            }
            if want_net {
                deliver(net.sample(&host));
            } else {
                net = Net::default();
            }
            std::thread::sleep(STEP);
        }
    });
}

// ---- Terhelés ----

#[derive(Default)]
struct Load {
    cpu_prev: Option<(u64, u64)>,
    gpu: Option<GpuCounter>,
    gpu_failed: bool,
}

impl Load {
    fn sample(&mut self) -> Value {
        let now = crate::sys::cpu_times();
        let cpu = match (self.cpu_prev, now) {
            (Some((pi, pt)), Some((ni, nt))) if nt > pt => Some((1.0 - ni.saturating_sub(pi) as f64 / (nt - pt) as f64) * 100.0),
            _ => None,
        };
        self.cpu_prev = now;
        let (used, total) = crate::sys::memory().unwrap_or((0, 0));
        if self.gpu.is_none() && !self.gpu_failed {
            self.gpu = GpuCounter::open();
            self.gpu_failed = self.gpu.is_none();
        }
        let gpu = self.gpu.as_ref().and_then(GpuCounter::read);
        json!({
            "type": "load",
            "cpu": cpu.map(|c| c.clamp(0.0, 100.0).round()),
            "mem": (total > 0).then(|| (used as f64 / total as f64 * 100.0).round()),
            "memUsed": used,
            "memTotal": total,
            "gpu": gpu.map(|g| g.clamp(0.0, 100.0).round()),
            "gpuTemp": gpu_temperature(),
        })
    }
}

/// A videókártya 3D-motorjainak kihasználtsága (a futó programok összege, mint a
/// Feladatkezelő „3D” grafikonja).
struct GpuCounter {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl GpuCounter {
    fn open() -> Option<GpuCounter> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(None, 0, &mut query) != 0 {
                return None;
            }
            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(query, w!(r"\GPU Engine(*engtype_3D)\Utilization Percentage"), 0, &mut counter) != 0 {
                return None;
            }
            // az első gyűjtés csak viszonyítási alap
            let _ = PdhCollectQueryData(query);
            Some(GpuCounter { query, counter })
        }
    }

    fn read(&self) -> Option<f64> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) != PDH_MORE_DATA {
                return None;
            }
            // a tömb után a példányok nevei is a pufferbe kerülnek
            let items = (size as usize).div_ceil(std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>()).max(1);
            let mut buf = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); items];
            if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(buf.as_mut_ptr())) != 0 {
                return None;
            }
            Some(buf[..count as usize].iter().filter(|i| i.FmtValue.CStatus == 0).map(|i| i.FmtValue.Anonymous.doubleValue).sum())
        }
    }
}

/// A legmelegebb videókártya hőmérséklete (°C), ha a meghajtó közli (WDDM 2.x).
fn gpu_temperature() -> Option<f64> {
    unsafe {
        let mut e = D3DKMT_ENUMADAPTERS2::default();
        if D3DKMTEnumAdapters2(&mut e).is_err() || e.NumAdapters == 0 {
            return None;
        }
        let mut list = vec![D3DKMT_ADAPTERINFO::default(); e.NumAdapters as usize];
        e.pAdapters = list.as_mut_ptr();
        if D3DKMTEnumAdapters2(&mut e).is_err() {
            return None;
        }
        let mut best: Option<f64> = None;
        for a in &list[..e.NumAdapters as usize] {
            let mut perf = D3DKMT_ADAPTER_PERFDATA::default();
            let mut q = D3DKMT_QUERYADAPTERINFO {
                hAdapter: a.hAdapter,
                Type: KMTQAITYPE_ADAPTERPERFDATA,
                pPrivateDriverData: &mut perf as *mut _ as *mut _,
                PrivateDriverDataSize: std::mem::size_of::<D3DKMT_ADAPTER_PERFDATA>() as u32,
            };
            // a hőmérséklet tized fokban jön; 0: a meghajtó nem közli
            if D3DKMTQueryAdapterInfo(&mut q).is_ok() && perf.Temperature > 0 {
                let t = perf.Temperature as f64 / 10.0;
                best = Some(best.map_or(t, |b: f64| b.max(t)));
            }
            let _ = D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER { hAdapter: a.hAdapter });
        }
        best.map(|t| t.round())
    }
}

// ---- Hálózat ----

#[derive(Default)]
struct Net {
    prev: Option<(Instant, u64, u64)>,
    last_ping: Option<Instant>,
    ping: Option<u32>,
}

impl Net {
    fn sample(&mut self, host: &str) -> Value {
        let now = Instant::now();
        let (down, up) = match (self.prev, octets()) {
            (Some((t, pi, po)), Some((ni, no))) => {
                let secs = now.duration_since(t).as_secs_f64().max(0.1);
                (Some(ni.saturating_sub(pi) as f64 / secs), Some(no.saturating_sub(po) as f64 / secs))
            }
            _ => (None, None),
        };
        self.prev = octets().map(|(i, o)| (now, i, o));
        if self.last_ping.is_none_or(|t| now.duration_since(t) >= PING_EVERY) {
            self.last_ping = Some(now);
            self.ping = ping(host);
        }
        json!({ "type": "net", "down": down.map(f64::round), "up": up.map(f64::round), "ping": self.ping, "host": host })
    }
}

/// Az összes beérkezett és elküldött bájt a működő hálózati kártyákon (a szűrő- és virtuális
/// rétegek nélkül, hogy semmi ne számítson kétszer).
fn octets() -> Option<(u64, u64)> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return None;
        }
        let n = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
        let (mut rx, mut tx) = (0u64, 0u64);
        for r in rows {
            // 1. bit: hardveres csatoló, 2. bit: szűrő réteg
            let bits = r.InterfaceAndOperStatusFlags._bitfield;
            if bits & 1 == 1 && bits & 2 == 0 && r.Type != IF_TYPE_SOFTWARE_LOOPBACK && r.OperStatus == IfOperStatusUp {
                rx += r.InOctets;
                tx += r.OutOctets;
            }
        }
        FreeMibTable(table as *const _);
        Some((rx, tx))
    }
}

/// Válaszidő ezredmásodpercben (ICMP echo, legfeljebb 1 mp-ig vár); `None`, ha nincs válasz.
fn ping(host: &str) -> Option<u32> {
    let ip: Ipv4Addr = match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip,
        _ => (host, 0).to_socket_addrs().ok()?.find_map(|a| match a.ip() {
            IpAddr::V4(ip) => Some(ip),
            IpAddr::V6(_) => None,
        })?,
    };
    unsafe {
        let h = IcmpCreateFile().ok()?;
        let data = *b"CyberShell";
        let mut reply = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + data.len() + 8];
        let n = IcmpSendEcho(
            h,
            u32::from_ne_bytes(ip.octets()),
            data.as_ptr() as *const _,
            data.len() as u16,
            None,
            reply.as_mut_ptr() as *mut _,
            reply.len() as u32,
            1000,
        );
        let _ = IcmpCloseHandle(h);
        if n == 0 {
            return None;
        }
        let r = &*(reply.as_ptr() as *const ICMP_ECHO_REPLY);
        (r.Status == 0).then_some(r.RoundTripTime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Élő mérés ezen a gépen: a processzor és a memória mindig mérhető, a hálózati
    /// számlálók is olvashatók; a videókártya és a ping gépfüggő, ezeket csak kiírjuk.
    #[test]
    fn live_sample() {
        let mut l = Load::default();
        l.sample();
        std::thread::sleep(Duration::from_millis(300));
        let v = l.sample();
        assert!(v["cpu"].as_f64().is_some() && v["mem"].as_f64().is_some_and(|m| m > 0.0));
        assert!(octets().is_some());
        println!("terhelés: {v}\nping 1.1.1.1: {:?}", ping("1.1.1.1"));
    }
}
