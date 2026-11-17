//! Claude-használat: a claude.ai belépőablak, a lekérdezés és a háttérszál eredményének közzététele.

use crate::UserEvent;
use crate::config::Config;
use crate::hub::Hub;
use crate::web::push;
use crate::{appbar, log, send};
use serde_json::json;
use std::time::{Duration, Instant};
use tao::dpi::PhysicalPosition;
use tao::window::{Window, WindowBuilder};
use wry::http::Request;
use wry::{WebContext, WebView, WebViewBuilder};

/// A claude.ai ablak üresjárati oldala: könnyű, de a claude.ai-hoz tartozik,
/// így innen elérhetők a saját fiókod adatai (a sütik a CyberShell saját böngészőprofiljában vannak).
pub(crate) const CLAUDE_IDLE_URL: &str = "https://claude.ai/robots.txt";

pub(crate) const CLAUDE_LOGIN_URL: &str = "https://claude.ai/login";

/// A teljes claude.ai alkalmazás: ha a munkamenet „kötése” (lásd `ClaudeWeb::revived`)
/// hosszabb kikapcsolás alatt lejárt, ennek betöltése megújítja.
pub(crate) const CLAUDE_APP_URL: &str = "https://claude.ai/new";

/// A claude.ai oldalon futó lekérdezés: a bejelentkezett fiók szervezete, majd annak
/// használati adatai. Ugyanazt az adatot kéri le, amit a claude.ai beállításai is mutatnak.
pub(crate) const CLAUDE_FETCH_JS: &str = r#"(async () => {
  const post = window.ipc ? (s) => window.ipc.postMessage(s) : (s) => window.chrome.webview.postMessage(s);
  const send = (m) => post(JSON.stringify(Object.assign({ type: 'claude-web' }, m)));
  if (location.origin !== 'https://claude.ai') return send({ status: 'skip' });
  try {
    const r = await fetch('/api/organizations', { credentials: 'include', headers: { accept: 'application/json' } });
    if (r.status === 401 || r.status === 403) return send({ status: 'login', code: r.status });
    if (!r.ok) return send({ status: 'error', code: r.status });
    const orgs = await r.json();
    if (!Array.isArray(orgs) || !orgs.length) return send({ status: 'login', code: 0 });
    const caps = (o) => o.capabilities || [];
    const org = orgs.find((o) => caps(o).some((c) => /claude_(pro|max)/.test(c)))
      || orgs.find((o) => caps(o).includes('chat')) || orgs[0];
    const u = await fetch(`/api/organizations/${org.uuid}/usage`, { credentials: 'include', headers: { accept: 'application/json' } });
    if (!u.ok) return send({ status: 'error', code: u.status });
    send({ status: 'ok', usage: await u.json(), caps: caps(org) });
  } catch (e) {
    send({ status: 'error', message: String(e).slice(0, 120) });
  }
})();"#;

/// A claude.ai belépő- és lekérdezőablak.
pub(crate) struct ClaudeWeb {
    pub(crate) window: Window,
    pub(crate) webview: WebView,
    pub(crate) visible: bool,
    pub(crate) next_fetch: Instant,
    /// A legutóbbi sikeres lekérés ideje és eredménye.
    pub(crate) ok: Option<(Instant, String)>,
    /// A belépés sütijeit ebben a futásban már tartóssá tettük-e.
    pub(crate) persisted: bool,
    /// A claude.ai a belépést az eszközhöz köti (`__Host-sessionBinding`, néhány percig
    /// érvényes, futás közben folyamatosan megújul). Ha a program hosszabb ideig nem futott,
    /// a kötés lejár, és a puszta lekérdezés „nincs belépve” választ kap, pedig a belépés
    /// (sessionKey) még érvényes. Ilyenkor egyszer betöltjük a teljes claude.ai oldalt,
    /// ami megújítja a kötést, és csak ha ezután sem megy, kérünk újra belépést.
    pub(crate) revived: bool,
    /// Mit mondott legutóbb a claude.ai: "pending" (még nem válaszolt), "ok", "login"
    /// (valóban nincs belépve) vagy "error" (nem érhető el, pl. induláskor még nincs hálózat).
    /// Belépést csak "login" esetén kérünk.
    pub(crate) state: &'static str,
}

/// A claude.ai ablak: rejtve indul, csak belépéskor jelenik meg. A sütik a CyberShell saját
/// böngészőprofiljában maradnak, az asztali Claude apphoz nem nyúlunk.
pub(crate) fn make_claude_window(
    target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
    web_context: &mut WebContext,
    cfg: &Config,
) -> Option<ClaudeWeb> {
    let (mon, _) = appbar::primary_monitor();
    let window = WindowBuilder::new()
        .with_title(match cfg.language.as_str() {
            "en" => "CyberShell – sign in to claude.ai",
            "pl" => "CyberShell – logowanie do claude.ai",
            _ => "CyberShell – belépés a claude.ai-ra",
        })
        // a tálcán az exe saját ikonja látszódjon (assets/cybershell.rc, 1-es azonosító)
        .with_window_icon({
            use tao::platform::windows::IconExtWindows;
            tao::window::Icon::from_resource(1, None).ok()
        })
        .with_visible(false)
        .with_inner_size(tao::dpi::LogicalSize::new(480.0, 760.0))
        .with_position(PhysicalPosition::new((mon.left + mon.right) / 2 - 300, (mon.top + mon.bottom) / 2 - 470))
        .build(target)
        .ok();
    window.and_then(|window| {
        let webview = WebViewBuilder::new_with_web_context(web_context)
            .with_url(CLAUDE_IDLE_URL)
            .with_background_color(cfg.bg_rgba())
            .with_ipc_handler(|req: Request<String>| send(UserEvent::ClaudeWeb(req.body().clone())))
            // A Google-belépés felugró ablakban zajlik; alapból ezt a komponens letiltaná,
            // és a claude.ai „There was an error logging you in” hibát adna.
            .with_new_window_req_handler(|url: String, _features| {
                log(format!("claude.ai felugró ablak: {}", url.split('?').next().unwrap_or("")));
                if url.starts_with("https://") { wry::NewWindowResponse::Allow } else { wry::NewWindowResponse::Deny }
            })
            .with_on_page_load_handler(|ev, url| {
                if matches!(ev, wry::PageLoadEvent::Finished) {
                    send(UserEvent::ClaudeLoaded(url));
                }
            })
            .build(&window)
            .map_err(|e| log(format!("claude.ai ablak nem hozható létre: {e}")))
            .ok()?;
        Some(ClaudeWeb {
            window,
            webview,
            visible: false,
            next_fetch: Instant::now() + Duration::from_secs(300),
            ok: None,
            persisted: false,
            revived: false,
            state: "pending",
        })
    })
}

/// A claude.ai munkamenet-sütijeit (amelyek a program bezárásakor elvesznének)
/// 30 napig érvényesre állítja, hogy újraindítás után se kelljen újra belépni.
/// A sütik a CyberShell saját böngészőprofiljában maradnak; értéküket nem írjuk ki.
pub(crate) fn persist_claude_cookies(webview: &WebView) -> usize {
    let Ok(cookies) = webview.cookies_for_url("https://claude.ai") else {
        return 0;
    };
    let mut n = 0;
    for mut c in cookies {
        let session = matches!(c.expires(), None | Some(wry::cookie::Expiration::Session));
        // a Cloudflare rövid életű sütijeit nem bántjuk
        if session && !c.name().starts_with("__cf") && !c.name().starts_with("cf_") {
            c.set_max_age(wry::cookie::time::Duration::days(30));
            if webview.set_cookie(&c).is_ok() {
                n += 1;
            }
        }
    }
    n
}

/// A claude.ai használati válaszából a panel üzenete (ugyanaz a forma, mint a háttérszálé).
pub(crate) fn claude_web_payload(v: &serde_json::Value) -> serde_json::Value {
    let u = &v["usage"];
    let caps: Vec<String> =
        v["caps"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(str::to_lowercase)).collect()).unwrap_or_default();
    let plan = if caps.iter().any(|c| c.contains("claude_max")) {
        "max"
    } else if caps.iter().any(|c| c.contains("claude_pro")) {
        "pro"
    } else {
        ""
    };
    let keys = [
        ("five_hour", "5 órás", true),
        ("seven_day", "Heti", true),
        ("seven_day_opus", "Heti · Opus", false),
        ("seven_day_sonnet", "Heti · Sonnet", false),
    ];
    let mut limits = Vec::new();
    for (key, label, always) in keys {
        let l = &u[key];
        let pct = l["utilization"].as_f64().or_else(|| l["used_percentage"].as_f64());
        if let Some(pct) = pct
            && (always || pct > 0.0)
        {
            limits.push(json!({ "key": key, "label": label, "pct": pct, "resetsAt": l["resets_at"] }));
        }
    }
    if limits.is_empty() {
        return json!({ "type": "usage", "status": "error", "plan": plan, "message": "Ismeretlen válasz a claude.ai-tól." });
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    json!({ "type": "usage", "status": "ok", "plan": plan, "limits": limits, "fetchedAt": now, "source": "web" })
}

impl Hub {
    /// A háttérszál eredménye (állapotsor vagy közvetlen lekérés).
    pub(crate) fn on_usage(&mut self, payload: String) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
            // csak állapotváltozáskor naplózunk, hogy ne teljen meg a napló
            let changed = self.thread_usage.as_ref().map(|t| t["status"] != v["status"] || t["message"] != v["message"]).unwrap_or(true);
            if changed {
                log(format!("Claude-használat (háttér): {} {}", v["status"].as_str().unwrap_or("?"), v["message"].as_str().unwrap_or("")));
            }
            self.thread_usage = Some(v);
        }
        self.publish_usage();
    }

    /// A legjobb elérhető adat kiküldése a panelnek: a claude.ai-ról frissen lekért,
    /// ha nincs, a háttérszálé, ha az sincs, belépésre hívó üzenet.
    pub(crate) fn publish_usage(&mut self) {
        let fresh_web = self
            .claude
            .as_ref()
            .and_then(|c| c.ok.as_ref())
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(15 * 60))
            .map(|(_, p)| p.clone());
        let payload = if let Some(p) = fresh_web {
            p
        } else if let Some(t) = self.thread_usage.as_ref().filter(|t| t["status"] == "ok") {
            t.to_string()
        } else if let Some(state) = self.claude.as_ref().map(|c| c.state).filter(|s| *s != "login") {
            // a claude.ai még nem válaszolt, vagy nem érhető el: ez nem kijelentkezés
            json!({ "type": "usage", "status": if state == "error" { "offline" } else { "wait" }, "plan": "" }).to_string()
        } else {
            json!({ "type": "usage", "status": "login", "plan": "" }).to_string()
        };
        if self.last_usage.as_deref() != Some(payload.as_str()) {
            let info = self.info_edge();
            for b in self.bars.iter().filter(|b| b.edge == info) {
                push(&b.webview, &payload);
            }
            self.last_usage = Some(payload);
        }
    }

    pub(crate) fn claude_fetch(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            c.next_fetch = Instant::now() + Duration::from_secs(5 * 60);
            if c.state == "error" {
                // ha az oldal sem töltődött be (nem volt hálózat), előbb azt töltjük újra;
                // a betöltés végén magától lekérdez
                let _ = c.webview.load_url(CLAUDE_IDLE_URL);
            } else {
                let _ = c.webview.evaluate_script(CLAUDE_FETCH_JS);
            }
        }
    }

    pub(crate) fn claude_show(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            log("claude.ai belépőablak megnyitva");
            let _ = c.webview.load_url(CLAUDE_LOGIN_URL);
            c.window.set_visible(true);
            c.window.set_focus();
            c.visible = true;
        }
    }

    pub(crate) fn claude_hide(&mut self) {
        if let Some(c) = self.claude.as_mut() {
            c.window.set_visible(false);
            c.visible = false;
            // a nehéz claude.ai alkalmazás helyett egy üres oldalon várakozunk
            let _ = c.webview.load_url(CLAUDE_IDLE_URL);
        }
    }

    /// A claude.ai ablakban betöltődött egy oldal: ha a claude.ai-on vagyunk, lekérdezünk.
    /// A teljes alkalmazás betöltése után (munkamenet megújítása) hagyunk időt az oldalnak.
    pub(crate) fn on_claude_loaded(&mut self, url: &str) {
        if !url.starts_with("https://claude.ai/") {
            return;
        }
        if let Some(c) = self.claude.as_mut()
            && !c.visible
            && url.starts_with(CLAUDE_APP_URL)
        {
            c.next_fetch = Instant::now() + Duration::from_secs(6);
            return;
        }
        self.claude_fetch();
    }

    pub(crate) fn on_claude_web(&mut self, raw: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else { return };
        let status = v["status"].as_str().unwrap_or("");
        if status == "skip" {
            // nem a claude.ai oldalon vagyunk (pl. hibaoldal, mert nem volt hálózat):
            // fél perc múlva újratöltjük az oldalt
            if let Some(c) = self.claude.as_mut()
                && !c.visible
                && c.state != "ok"
            {
                c.state = "error";
                c.next_fetch = Instant::now() + Duration::from_secs(30);
                self.publish_usage();
            }
            return;
        }
        log(format!(
            "Claude-használat (claude.ai): {status} {}{}",
            v["code"].as_i64().map(|c| format!("HTTP {c} ")).unwrap_or_default(),
            v["message"].as_str().unwrap_or("")
        ));
        match status {
            "ok" => {
                let payload = claude_web_payload(&v);
                if payload["status"] != "ok" {
                    // ismeretlen formátum: a kulcsneveket naplózzuk (használati adat, nem titok)
                    let keys: Vec<&str> = v["usage"].as_object().map(|o| o.keys().map(|k| k.as_str()).collect()).unwrap_or_default();
                    log(format!("ismeretlen válasz, kulcsok: {keys:?}"));
                }
                if let Some(c) = self.claude.as_mut()
                    && payload["status"] == "ok"
                {
                    c.state = "ok";
                    c.ok = Some((Instant::now(), payload.to_string()));
                    if !c.persisted {
                        c.persisted = true;
                        let n = persist_claude_cookies(&c.webview);
                        log(format!("claude.ai belépés megjegyezve ({n} süti)"));
                    }
                }
                if self.claude.as_ref().is_some_and(|c| c.visible) {
                    self.claude_hide();
                } else if let Some(c) = self.claude.as_mut()
                    && c.revived
                {
                    c.revived = false;
                    log("claude.ai: a munkamenet megújult");
                    let _ = c.webview.load_url(CLAUDE_IDLE_URL);
                }
                // egy későbbi lejáratnál újra megpróbálhatjuk a megújítást
                if let Some(c) = self.claude.as_mut() {
                    c.revived = false;
                }
            }
            "login" => {
                if let Some(c) = self.claude.as_mut() {
                    // először megpróbáljuk megújítani a munkamenetet (lásd `revived`)
                    if !c.revived && !c.visible {
                        c.revived = true;
                        log("claude.ai: a munkamenet lejárt kötését a teljes oldal betöltésével megújítom");
                        c.next_fetch = Instant::now() + Duration::from_secs(60);
                        let _ = c.webview.load_url(CLAUDE_APP_URL);
                        return;
                    }
                    c.ok = None;
                    c.state = "login";
                }
            }
            // nem érhető el (hálózat, szerverhiba): hamarabb újrapróbáljuk
            _ => {
                if let Some(c) = self.claude.as_mut() {
                    if c.state != "ok" || c.ok.is_none() {
                        c.state = "error";
                    }
                    c.next_fetch = Instant::now() + Duration::from_secs(30);
                }
            }
        }
        self.publish_usage();
    }
}
