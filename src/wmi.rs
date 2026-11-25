//! A Windows WMI-adatbázisának lekérdezése közvetlenül, COM-on át (IWbemLocator,
//! IWbemServices::ExecQuery), külső program (PowerShell) indítása nélkül.

use serde_json::{Map, Value};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Variant::{VARIANT, VT_BOOL, VT_BSTR, VT_I2, VT_I4, VT_I8, VT_R4, VT_R8, VT_UI1, VariantClear};
use windows::Win32::System::Wmi::{
    CIM_CHAR16, CIM_SINT64, CIM_UINT16, CIM_UINT32, CIM_UINT64, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WbemLocator,
};
use windows::core::{BSTR, HSTRING, PCWSTR};

/// Windows-hitelesítés (RPC_C_AUTHN_WINNT) és jogosultság-szolgáltatás nélkül (RPC_C_AUTHZ_NONE).
const AUTHN_WINNT: u32 = 10;
const AUTHZ_NONE: u32 = 0;
/// Egy sorra legfeljebb ennyit várunk (ms), hogy egy beragadt szolgáltató ne akassza meg a szálat.
const ROW_TIMEOUT_MS: i32 = 10_000;

/// Kapcsolat egy WMI-névtérhez (pl. `root\cimv2`).
pub struct Wmi(IWbemServices);

impl Wmi {
    pub fn connect(namespace: &str) -> Option<Wmi> {
        unsafe {
            let loc: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).ok()?;
            let empty = BSTR::new();
            let svc = loc.ConnectServer(&BSTR::from(namespace), &empty, &empty, &empty, 0, &empty, None).ok()?;
            // a szolgáltatók a hívó nevében olvasnak; enélkül némelyik üres választ ad
            let _ = CoSetProxyBlanket(
                &svc,
                AUTHN_WINNT,
                AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            );
            Some(Wmi(svc))
        }
    }

    /// Egy osztály példányai a kért tulajdonságokkal, a feltétellel (WQL WHERE, lehet üres)
    /// szűrve. A hiányzó vagy üres érték `null`. Hibánál üres lista.
    pub fn query(&self, class: &str, props: &[&str], filter: &str) -> Vec<Map<String, Value>> {
        let mut wql = format!("SELECT {} FROM {class}", props.join(", "));
        if !filter.is_empty() {
            wql.push_str(" WHERE ");
            wql.push_str(filter);
        }
        let mut rows = Vec::new();
        unsafe {
            let Ok(en) =
                self.0.ExecQuery(&BSTR::from("WQL"), &BSTR::from(wql), WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY, None)
            else {
                return rows;
            };
            loop {
                let mut objs = [None];
                let mut n = 0u32;
                if en.Next(ROW_TIMEOUT_MS, &mut objs, &mut n).is_err() || n == 0 {
                    break;
                }
                let Some(obj) = objs[0].take() else { break };
                let mut row = Map::new();
                for p in props {
                    let mut v = VARIANT::default();
                    let mut cim = 0i32;
                    let val = match obj.Get(&HSTRING::from(*p), 0, &mut v, Some(&mut cim), None) {
                        Ok(()) => convert(scalar(&v), cim),
                        Err(_) => Value::Null,
                    };
                    let _ = VariantClear(&mut v);
                    row.insert(p.to_string(), val);
                }
                rows.push(row);
            }
        }
        rows
    }
}

/// Egy VARIANT egyszerű értéke, a COM-típusok nélkül.
#[derive(Debug, PartialEq)]
enum Scalar {
    Null,
    Str(String),
    Int(i64),
    I32(i32),
    I16(i16),
    Bool(bool),
    Float(f64),
}

fn scalar(v: &VARIANT) -> Scalar {
    unsafe {
        let inner = &v.Anonymous.Anonymous;
        let d = &inner.Anonymous;
        match inner.vt {
            VT_BSTR => Scalar::Str(d.bstrVal.to_string()),
            VT_I4 => Scalar::I32(d.lVal),
            VT_I2 => Scalar::I16(d.iVal),
            VT_UI1 => Scalar::Int(d.bVal as i64),
            VT_I8 => Scalar::Int(d.llVal),
            VT_BOOL => Scalar::Bool(d.boolVal.0 != 0),
            VT_R4 => Scalar::Float(d.fltVal as f64),
            VT_R8 => Scalar::Float(d.dblVal),
            // VT_NULL, VT_EMPTY, tömbök és objektumok: ezekre nincs szükség
            _ => Scalar::Null,
        }
    }
}

/// A WMI a típust a CIM-típusban adja meg, a VARIANT csak hordozza az értéket: a 64 bites
/// egészek szövegként, az előjel nélküli 32 és 16 bites egészek előjelesként, a karakterek
/// (pl. meghajtóbetű) 16 bites egészként érkeznek. Ezeket visszaalakítjuk.
fn convert(v: Scalar, cim: i32) -> Value {
    match v {
        Scalar::Null => Value::Null,
        Scalar::Str(s) if cim == CIM_UINT64.0 => s.parse::<u64>().map(Value::from).unwrap_or(Value::String(s)),
        Scalar::Str(s) if cim == CIM_SINT64.0 => s.parse::<i64>().map(Value::from).unwrap_or(Value::String(s)),
        Scalar::Str(s) => Value::String(s),
        Scalar::I32(i) if cim == CIM_UINT32.0 || cim == CIM_UINT16.0 => Value::from(i as u32),
        Scalar::I32(i) => Value::from(i),
        Scalar::I16(i) if cim == CIM_CHAR16.0 => match char::from_u32(i as u16 as u32).filter(|c| *c != '\0') {
            Some(c) => Value::String(c.to_string()),
            None => Value::Null,
        },
        Scalar::I16(i) if cim == CIM_UINT16.0 => Value::from(i as u16),
        Scalar::I16(i) => Value::from(i),
        Scalar::Int(i) => Value::from(i),
        Scalar::Bool(b) => Value::Bool(b),
        Scalar::Float(f) => serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cim_types() {
        // uint64: szövegként jön (pl. a memóriamodul mérete)
        assert_eq!(convert(Scalar::Str("8589934592".into()), CIM_UINT64.0), json!(8589934592u64));
        assert_eq!(convert(Scalar::Str("-5".into()), CIM_SINT64.0), json!(-5));
        // uint32 a 2 GB fölötti értékeknél negatívként érkezik (pl. AdapterRAM)
        assert_eq!(convert(Scalar::I32(-1_048_576), CIM_UINT32.0), json!(4293918720u32));
        assert_eq!(convert(Scalar::I32(3600), CIM_UINT32.0), json!(3600));
        // char16: a meghajtóbetű; a 0 azt jelenti, hogy nincs
        assert_eq!(convert(Scalar::I16('C' as i16), CIM_CHAR16.0), json!("C"));
        assert_eq!(convert(Scalar::I16(0), CIM_CHAR16.0), Value::Null);
        // a szöveg szöveg marad, ha a típusa nem szám
        assert_eq!(convert(Scalar::Str("P10.31".into()), 8), json!("P10.31"));
        assert_eq!(convert(Scalar::Null, CIM_UINT64.0), Value::Null);
        assert_eq!(convert(Scalar::Bool(true), 11), json!(true));
    }

    #[test]
    fn variant_values() {
        let v = VARIANT::default();
        assert_eq!(scalar(&v), Scalar::Null);
    }

    /// Élő lekérdezés ezen a gépen: a processzornak mindig van neve.
    #[test]
    fn live_query() {
        let _com = crate::com::ComThread::init();
        let wmi = Wmi::connect(r"root\cimv2").expect("WMI-kapcsolat");
        let rows = wmi.query("Win32_Processor", &["Name", "NumberOfCores"], "");
        assert!(!rows.is_empty());
        assert!(rows[0]["Name"].as_str().is_some_and(|s| !s.trim().is_empty()));
        assert!(rows[0]["NumberOfCores"].as_u64().is_some_and(|n| n > 0));
    }
}
