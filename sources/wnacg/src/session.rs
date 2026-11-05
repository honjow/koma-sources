use super::*;
const HARDCODED: &str = "https://www.wnacg.buzz,https://www.wnacg.site";
fn rpc(mut v: Value) -> R<Value> {
    v["type"] = json!("private_session");
    v["version"] = json!(1);
    let b = serde_json::to_vec(&v).map_err(|_| E::from("request_too_large"))?;
    let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
    let v: Value =
        serde_json::from_slice(&http_out()[..n]).map_err(|_| E::from("network_error"))?;
    if v["ok"] != true {
        return Err(E {
            code: if v["error"]["code"] == "inactive" {
                "cancelled"
            } else {
                "network_error"
            },
            message: "书源私有会话读取或更新失败，请稍后重试。".to_string(),
        });
    }
    Ok(v)
}
fn feature(v: &Value, key: &str) -> bool {
    v["hostFeatures"]
        .as_array()
        .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(key)))
}
fn empty() -> Value {
    json!({"v":1,"hardcoded":HARDCODED,"processTag":"","mirrors":DEFAULTS,"extra":[],"selectionToken":"0","autoAlias":"0","launchOldBase":null})
}
fn state(raw: &str) -> R<Value> {
    let mut s: Value = serde_json::from_str(raw).map_err(|_| E::from("parse_error"))?;
    if s["v"] != 1 {
        return Err("parse_error".into());
    }
    if s["hardcoded"] != HARDCODED {
        return Ok(empty());
    }
    let a = s["mirrors"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or_else(|| E::from("parse_error"))?;
    for u in a {
        if origin(u.as_str().ok_or_else(|| E::from("parse_error"))?).as_deref() != u.as_str() {
            return Err("parse_error".into());
        }
    }
    if !s["extra"].is_array() {
        s["extra"] = json!([])
    }
    if !s["autoAlias"].is_string() || !s["selectionToken"].is_string() {
        return Err("parse_error".into());
    }
    Ok(s)
}
fn origin(raw: &str) -> Option<String> {
    let u = Url::parse(raw).ok()?;
    if !matches!(u.scheme(), "http" | "https")
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return None;
    }
    Some(u.origin().ascii_serialization())
}
fn mirrors(s: &Value) -> Vec<String> {
    s["mirrors"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_else(|| DEFAULTS.iter().map(|s| s.to_string()).collect())
}
fn values(v: &Value) -> &Value {
    v["args"]
        .get("settingValues")
        .filter(|x| x.is_object())
        .unwrap_or(&v["settings"])
}
fn token(v: &Value) -> R<String> {
    Ok(setting(values(v), "preferred_mirror", "0")?.to_string())
}
fn url_token(token: &str) -> Option<&str> {
    token
        .strip_prefix("u:")
        .or_else(|| token.strip_prefix("r:"))
}
fn selected(s: &Value, token: &str) -> String {
    let m = mirrors(s);
    let first = m
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULTS[0].to_string());
    if token == "0" || s["autoAlias"].as_str() == Some(token) {
        return first;
    }
    url_token(token)
        .filter(|u| m.iter().any(|x| x == u))
        .map(ToString::to_string)
        .unwrap_or(first)
}
fn reconcile(s: &mut Value, list: Vec<String>, token: &str) {
    let old = selected(s, token);
    let auto = token == "0" || s["autoAlias"].as_str() == Some(token);
    let first = list
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULTS[0].to_string());
    // Public values are opaque option IDs. A per-install alias preserves the
    // reference's automatic-first selection without mutating Host public settings.
    let alias = if auto || old == first || !list.contains(&old) {
        token
    } else {
        "0"
    };
    s["mirrors"] = json!(list);
    s["autoAlias"] = json!(alias);
    s["selectionToken"] = json!(token);
}
pub struct Context {
    pub base: String,
    s: Value,
    tag: Option<String>,
    token: String,
}
impl Context {
    pub fn metadata(v: &Value) -> R<Self> {
        let token = token(v)?;
        let mut s = empty();
        let mut tag = None;
        if feature(v, "source_private_session_v1") {
            let b = rpc(json!({"op":"begin"}))?;
            match field(&b, "state")? {
                "ready" => {
                    s = state(field(&b, "value")?)?;
                    tag = Some(field(&b, "versionTag")?.to_string())
                }
                "owner" => {
                    rpc(json!({"op":"abort","lease":field(&b,"lease")?}))?;
                }
                _ => return Err("network_error".into()),
            }
        }
        // Metadata reflects the effective/clamped selection without writing it.
        let list = mirrors(&s);
        reconcile(&mut s, list, &token);
        let base = selected(&s, &token);
        Ok(Self {
            base,
            s,
            tag,
            token,
        })
    }
    pub fn network(r: &[u8], v: &Value) -> R<Self> {
        // No RPC/HTTP before all required Host capabilities have been established.
        for f in [
            b"source_private_session_v1".as_slice(),
            b"source_private_session_process_v1",
            b"source_private_session_replace_v1",
            b"source_rate_limit_wnacg_v1",
        ] {
            if !protocol::feature(r, f) {
                return Err("unsupported".into());
            }
        }
        let token = token(v)?;
        let process = rpc(json!({"op":"process_scope"}))?;
        let process = field(&process, "processTag")?.to_string();
        let b = rpc(json!({"op":"begin"}))?;
        let (mut s, old, mut lease) = match field(&b, "state")? {
            "ready" => (
                state(field(&b, "value")?)?,
                Some(field(&b, "versionTag")?.to_string()),
                None,
            ),
            "owner" => (empty(), None, Some(field(&b, "lease")?.to_string())),
            _ => return Err("network_error".into()),
        };
        if s["processTag"].as_str() != Some(&process) {
            if lease.is_none() {
                let b = rpc(json!({"op":"begin","replaceVersionTag":old}))?;
                if b["state"] != "owner" {
                    return Err("network_error".into());
                }
                lease = Some(field(&b, "lease")?.to_string())
            }
            let l = lease.as_deref().ok_or_else(|| E::from("network_error"))?;
            let result = (|| {
                // Upstream catches failed/empty domain discovery and keeps the previous
                // mirrors for this launch. No retries, alternate discovery URL, or TTL.
                let found = match http(DOMAIN_LIST, &selected(&s, &token)) {
                    Ok(bytes) => {
                        let t = core::str::from_utf8(&bytes).unwrap_or("");
                        let mut a = Vec::new();
                        for raw in t.split(',') {
                            if let Some(o) = origin(raw) {
                                if !a.contains(&o) {
                                    a.push(o)
                                }
                            }
                        }
                        a
                    }
                    Err(e) if e.code == "cancelled" => return Err(e),
                    Err(_) => {
                        host::log_info(b"wnacg mirror_discovery_unavailable");
                        Vec::new()
                    }
                };
                let before = selected(&s, &token);
                let list = if found.is_empty() { mirrors(&s) } else { found };
                reconcile(&mut s, list, &token);
                let after = selected(&s, &token);
                s["launchOldBase"] = if before != after {
                    json!(before)
                } else {
                    Value::Null
                };
                s["processTag"] = json!(process);
                let done = rpc(
                    json!({"op":"commit","lease":l,"value":serde_json::to_string(&s).map_err(|_|E::from("result_too_large"))?}),
                )?;
                Ok(field(&done, "versionTag")?.to_string())
            })();
            match result {
                Ok(tag) => {
                    let base = selected(&s, &token);
                    return Ok(Self {
                        base,
                        s,
                        tag: Some(tag),
                        token,
                    });
                }
                Err(e) => {
                    let _ = rpc(json!({"op":"abort","lease":l}));
                    return Err(e);
                }
            }
        }
        if let Some(l) = lease {
            let _ = rpc(json!({"op":"abort","lease":l}));
            return Err("network_error".into());
        }
        let base = selected(&s, &token);
        Ok(Self {
            base,
            s,
            tag: old,
            token,
        })
    }
    pub fn moved_image_uri(&self, raw: &str) -> R<String> {
        let Some(old) = self.s["launchOldBase"].as_str() else {
            return Ok(raw.to_string());
        };
        let old = Url::parse(old).map_err(|_| E::from("parse_error"))?;
        let mut url = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
        if old.host_str() != url.host_str() {
            return Ok(raw.to_string());
        }
        let base = Url::parse(&self.base).map_err(|_| E::from("parse_error"))?;
        url.set_scheme(base.scheme())
            .map_err(|_| E::from("parse_error"))?;
        url.set_host(base.host_str())
            .map_err(|_| E::from("parse_error"))?;
        url.set_port(base.port())
            .map_err(|_| E::from("parse_error"))?;
        Ok(url.to_string())
    }
    pub fn categories(&self) -> Vec<(String, String)> {
        self.s["extra"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        Some((
                            p.get(0)?.as_str()?.to_string(),
                            p.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn mirror_options(&self) -> R<Vec<Value>> {
        let m = mirrors(&self.s);
        let alias = self.s["autoAlias"].as_str().unwrap_or("0");
        let mut options = Vec::new();
        for (i, u) in m.iter().enumerate() {
            let value = if i == 0 {
                alias.to_string()
            } else {
                let mut t = format!("u:{}", u);
                if t == alias {
                    t = format!("r:{}", u)
                }
                t
            };
            let label = Url::parse(u)
                .ok()
                .and_then(|u| u.host_str().map(ToString::to_string))
                .unwrap_or_else(|| u.clone());
            options.push(json!({"label":label,"value":value}))
        }
        Ok(options)
    }
    pub fn save_categories(&mut self, d: &Node) -> R<()> {
        let mut extra = Vec::<(String, String)>::new();
        let re = rx(r"-cate-(\d+)\.html")?;
        for item in d.all("#album_tabs > li")? {
            // scraper does not support Jsoup's leading child combinator. :scope > a is
            // standard CSS; descendants from the dropdown must not become the parent.
            let parent = item.one(":scope > a").or_else(|| item.one("a"));
            let Some(parent) = parent else { continue };
            let label = parent.text()?;
            let mut links = alloc::vec![(label.clone(), parent)];
            for n in item.all(".drop_dwon a")? {
                links.push((format!("{}-{}", label, n.text()?), n))
            }
            for (label, n) in links {
                let Some(h) = n.attr("href")? else { continue };
                if let Some(c) = re.captures(&h) {
                    let id = &c[1];
                    if !CATEGORIES.iter().any(|(_, x)| *x == id) {
                        extra.push((label, id.to_string()))
                    }
                }
            }
        }
        let next = json!(extra);
        if next == self.s["extra"] {
            return Ok(());
        }
        let tag = self
            .tag
            .as_deref()
            .ok_or_else(|| E::from("network_error"))?;
        let b = rpc(json!({"op":"begin","replaceVersionTag":tag}))?;
        if b["state"] != "owner" {
            return Err("network_error".into());
        }
        let l = field(&b, "lease")?.to_string();
        let mut s = self.s.clone();
        s["extra"] = next;
        let list = mirrors(&s);
        reconcile(&mut s, list, &self.token);
        let result = (|| {
            let done = rpc(
                json!({"op":"commit","lease":l,"value":serde_json::to_string(&s).map_err(|_|E::from("result_too_large"))?}),
            )?;
            Ok(field(&done, "versionTag")?.to_string())
        })();
        match result {
            Ok(tag) => {
                self.s = s;
                self.tag = Some(tag);
                Ok(())
            }
            Err(e) => {
                let _ = rpc(json!({"op":"abort","lease":l}));
                Err(e)
            }
        }
    }
}
