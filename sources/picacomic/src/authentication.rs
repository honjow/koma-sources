use super::*;
fn rpc(v: Value) -> R<Value> {
    let mut v = v;
    v["type"] = json!("private_session");
    v["version"] = json!(1);
    let b = serde_json::to_vec(&v).map_err(|_| E::from("request_too_large"))?;
    let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
    serde_json::from_slice(&http_out()[..n]).map_err(|_| "network_error".into())
}
fn rpc_ok(v: Value) -> R<Value> {
    let v = rpc(v)?;
    if v["ok"] != true {
        return Err(if v["error"]["code"] == "credential_input_unavailable" {
            "auth_required"
        } else if v["error"]["code"] == "inactive" {
            "cancelled"
        } else {
            "network_error"
        }
        .into());
    }
    Ok(v)
}
fn descriptor() -> Value {
    json!({"kind":"credentials","loginUrl":"https://picaapi.picacomic.com/auth/sign-in","usernameLabel":"用户名","passwordLabel":"密码","credentialStorage":"private"})
}
pub struct Session {
    value: Value,
    tag: String,
}
impl Session {
    fn from_begin(v: &Value) -> R<Self> {
        let s: Value =
            serde_json::from_str(field(v, "value")?).map_err(|_| E::from("parse_error"))?;
        if s["v"] != 1 {
            return Err("parse_error".into());
        }
        for k in ["username", "password", "token"] {
            let _ = field(&s, k)?;
        }
        Ok(Self {
            value: s,
            tag: field(v, "versionTag")?.to_string(),
        })
    }
    fn token(&self) -> R<&str> {
        field(&self.value, "token")
    }
    pub fn existing() -> R<Self> {
        let b = rpc_ok(json!({"op":"begin"}))?;
        match field(&b, "state")? {
            "ready" => {
                let s = Self::from_begin(&b)?;
                if s.token()?.is_empty()
                    && (field(&s.value, "username")?.is_empty()
                        || field(&s.value, "password")?.is_empty())
                {
                    return Err("auth_required".into());
                }
                Ok(s)
            }
            "owner" => {
                rpc_ok(json!({"op":"abort","lease":field(&b,"lease")?}))?;
                Err("auth_required".into())
            }
            _ => Err("network_error".into()),
        }
    }
}
pub fn status(r: &[u8]) -> R<Value> {
    require(r)?;
    let b = rpc_ok(json!({"op":"begin"}))?;
    let authenticated = match field(&b, "state")? {
        "ready" => !Session::from_begin(&b)?.token()?.is_empty(),
        "owner" => {
            rpc_ok(json!({"op":"abort","lease":field(&b,"lease")?}))?;
            false
        }
        _ => return Err("network_error".into()),
    };
    Ok(json!({"authentication":descriptor(),"authenticated":authenticated}))
}
pub fn logout(r: &[u8]) -> R<Value> {
    require(r)?;
    let b = rpc_ok(json!({"op":"begin"}))?;
    match field(&b, "state")? {
        "ready" => {
            rpc_ok(json!({"op":"invalidate","versionTag":field(&b,"versionTag")?}))?;
        }
        "owner" => {
            rpc_ok(json!({"op":"abort","lease":field(&b,"lease")?}))?;
        }
        _ => return Err("network_error".into()),
    }
    Ok(json!({"authenticated":false}))
}
fn replace_lease(tag: &str) -> R<Value> {
    rpc(json!({"op":"begin","replaceVersionTag":tag}))
}
pub fn authenticate(r: &[u8], v: &Value) -> R<Value> {
    require(r)?;
    let mut options = Options::new(v)?;
    options.require_transport(r)?;
    // Only the explicit Native authenticate grant can return these values. No args
    // or ordinary setting, Web Cookie, log, or operation result ever carries them.
    let input = rpc_ok(json!({"op":"credential_input"}))?;
    let user = field(&input["credentials"], "username")?;
    let password = field(&input["credentials"], "password")?;
    if user.is_empty() || password.is_empty() {
        return Err("auth_required".into());
    }
    options.transport(r)?;
    let b = rpc_ok(json!({"op":"begin"}))?;
    let (lease, prior) = match field(&b, "state")? {
        "owner" => (field(&b, "lease")?.to_string(), String::new()),
        "ready" => {
            let old = Session::from_begin(&b)?;
            let next = replace_lease(&old.tag)?;
            if next["ok"] != true || next["state"] != "owner" {
                return Err("network_error".into());
            }
            (field(&next, "lease")?.to_string(), old.token()?.to_string())
        }
        _ => return Err("network_error".into()),
    };
    let result = (|| {
        let token = fetch_token(user, password, &prior, &options)?;
        rpc_ok(
            json!({"op":"commit","lease":lease,"value":serde_json::to_string(&json!({"v":1,"username":user,"password":password,"token":token})).map_err(|_|E::from("result_too_large"))?}),
        )?;
        Ok(json!({"authenticated":true}))
    })();
    if result.is_err() {
        let _ = rpc(json!({"op":"abort","lease":lease}));
    }
    result
}
fn refresh(stale: &str, options: &Options) -> R<Session> {
    let current = Session::existing()?;
    if current.token()? != stale {
        return Ok(current);
    }
    let user = field(&current.value, "username")?;
    let password = field(&current.value, "password")?;
    if user.is_empty() || password.is_empty() {
        return Err("auth_required".into());
    }
    let b = replace_lease(&current.tag)?;
    if b["ok"] != true {
        // A concurrent refresh may already have published a new token. Adopt that
        // exact current generation; never log in a second time for the stale one.
        if b["error"]["code"] == "stale_version" {
            let next = Session::existing()?;
            if next.token()? != stale {
                return Ok(next);
            }
        }
        return Err("network_error".into());
    }
    if b["state"] != "owner" {
        return Err("network_error".into());
    }
    let lease = field(&b, "lease")?.to_string();
    let result = (|| {
        let token = fetch_token(user, password, stale, options)?;
        let value = json!({"v":1,"username":user,"password":password,"token":token});
        let done = rpc_ok(
            json!({"op":"commit","lease":lease,"value":serde_json::to_string(&value).map_err(|_|E::from("result_too_large"))?}),
        )?;
        Ok(Session {
            value,
            tag: field(&done, "versionTag")?.to_string(),
        })
    })();
    if result.is_err() {
        let _ = rpc(json!({"op":"abort","lease":lease}));
    }
    result
}
fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for c in b {
        s.push(b"0123456789abcdef"[(c >> 4) as usize] as char);
        s.push(b"0123456789abcdef"[(c & 15) as usize] as char)
    }
    s
}
fn hmac(raw: &[u8]) -> String {
    // RFC2104 HMAC with the platform-independent RustCrypto SHA256 primitive.
    let key = if HMAC_KEY.len() > 64 {
        Sha256::digest(HMAC_KEY.as_bytes()).to_vec()
    } else {
        HMAC_KEY.as_bytes().to_vec()
    };
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for (i, b) in key.iter().enumerate() {
        inner[i] ^= *b;
        outer[i] ^= *b
    }
    let mut a = Sha256::new();
    a.update(inner);
    a.update(raw);
    let digest = a.finalize();
    let mut b = Sha256::new();
    b.update(outer);
    b.update(digest);
    hex(&b.finalize())
}
fn stamp_nonce() -> R<(String, String)> {
    let mut time = None;
    let mut nonce = String::new();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    while nonce.len() < 32 {
        if host::check_cancel() {
            return Err("cancelled".into());
        }
        let clock = rpc_ok(json!({"op":"clock"}))?;
        if time.is_none() {
            let ms = field(&clock, "nowMs")?
                .parse::<u64>()
                .map_err(|_| E::from("network_error"))?;
            time = Some((ms / 1000).to_string())
        }
        let random = field(&clock, "randomHex")?;
        if random.len() != 32 || !random.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("network_error".into());
        }
        for i in (0..random.len()).step_by(2) {
            let b =
                u8::from_str_radix(&random[i..i + 2], 16).map_err(|_| E::from("network_error"))?;
            if b < 248 {
                nonce.push(CHARS[(b % 62) as usize] as char);
                if nonce.len() == 32 {
                    break;
                }
            }
        }
    }
    Ok((time.ok_or_else(|| E::from("network_error"))?, nonce))
}
fn prepare(
    path: &str,
    method: &str,
    body: Option<Value>,
    token: &str,
    options: &Options,
) -> R<Value> {
    if !path.starts_with('/') || path.starts_with("//") || token.chars().any(char::is_control) {
        return Err("invalid_request".into());
    }
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let (time, nonce) = stamp_nonce()?;
    let signature = hmac(
        format!(
            "{}{}{}{}{}",
            path.trim_start_matches('/'),
            time,
            nonce,
            method,
            KEY
        )
        .to_lowercase()
        .as_bytes(),
    );
    let mut req = json!({"version":1,"url":format!("{}{}",API,path),"method":method,"responseKind":"bodyText","headers":{
    "api-key":KEY,"app-channel":options.channel,"app-version":"2.2.1.3.3.4","app-uuid":"defaultUuid","app-platform":"android","app-build-version":"44","User-Agent":"okhttp/3.8.1","accept":"application/vnd.picacomic.com.v1+json","image-quality":options.quality,"Content-Type":"application/json; charset=UTF-8","time":time,"nonce":nonce,"signature":signature,"Authorization":token
    }});
    // Despite its historical name the Host's bodyBase64 field carries raw UTF-8
    // body text. Existing SDK/CLI/App all use this contract; do not base64 it.
    if let Some(body) = body {
        req["bodyBase64"] =
            json!(serde_json::to_string(&body).map_err(|_| E::from("request_too_large"))?)
    }
    options.apply_transport(&mut req);
    Ok(req)
}
fn send(req: &Value) -> R<Value> {
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let b = serde_json::to_vec(&req).map_err(|_| E::from("request_too_large"))?;
    let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
    let n = protocol::body(&http_out()[..n], body_buf()).map_err(E::from)?;
    let body = &body_buf()[..n];
    let data = protocol::direct(body, b"data")
        .map_err(|_| E::from("parse_error"))?
        .ok_or_else(|| E::from("parse_error"))?;
    for key in [b"token".as_slice(), b"comic", b"comics", b"eps", b"pages"] {
        let _ = protocol::direct(data, key).map_err(|_| E::from("parse_error"))?;
    }
    serde_json::from_slice(body).map_err(|_| "parse_error".into())
}
fn wire(path: &str, method: &str, body: Option<Value>, token: &str, options: &Options) -> R<Value> {
    let req = prepare(path, method, body, token, options)?;
    send(&req)
}
fn fetch_token(user: &str, password: &str, old: &str, options: &Options) -> R<String> {
    let data = wire(
        "/auth/sign-in",
        "POST",
        Some(json!({"email":user,"password":password})),
        old,
        options,
    )?;
    let token = data["data"]["token"]
        .as_str()
        .filter(|s| !s.is_empty() && !s.chars().any(char::is_control))
        .ok_or_else(|| E {
            code: "auth_required",
            message: "登录响应缺少有效账号凭据，请核对账号或稍后重试。".to_string(),
        })?;
    Ok(token.to_string())
}
pub fn request_api(
    session: &mut Session,
    options: &Options,
    path: &str,
    method: &str,
    body: Option<Value>,
) -> R<Value> {
    let mut req = prepare(path, method, body, session.token()?, options)?;
    if session.token()?.is_empty() {
        *session = refresh("", options)?;
        req["headers"]["Authorization"] = json!(session.token()?);
    }
    let sent = session.token()?.to_string();
    match send(&req) {
        Err(e) if e.code == "auth_required" => {
            *session = refresh(&sent, options)?;
            // Upstream retries the same signed request, replacing only its
            // Authorization header. Do not regenerate nonce/time/body here.
            req["headers"]["Authorization"] = json!(session.token()?);
            send(&req)
        }
        result => result,
    }
}
