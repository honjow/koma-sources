use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};

use super::{
    http_out, json_object_member, json_slice, payload_buf, selected_base, write_bytes, write_error,
    write_success_payload, FetchError, MIRROR,
};

const COOKIE: &str = "komiic-access-token";
const AUTH_FEATURE: &str = "source_native_cookie_session_v1";
const LOGIN_URLS: [&str; 2] = [
    "https://komiic.com/api/login",
    "https://komiic.cc/api/login",
];
const ACCOUNT_QUERY: &str = r#"{"query":"{ account { id } }"}"#;
const REFRESH_WINDOW_SECS: u64 = 3600;
type Result<T> = core::result::Result<T, FetchError>;

#[derive(Clone, Copy)]
enum Site {
    Com,
    Cc,
}

impl Site {
    fn selected(request: &[u8]) -> Self {
        if selected_base(request) == MIRROR {
            Self::Cc
        } else {
            Self::Com
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Com => "com",
            Self::Cc => "cc",
        }
    }

    fn domain(self) -> &'static str {
        match self {
            Self::Com => "komiic.com",
            Self::Cc => "komiic.cc",
        }
    }

    fn origin(self) -> &'static str {
        match self {
            Self::Com => "https://komiic.com",
            Self::Cc => "https://komiic.cc",
        }
    }

    fn descriptor(self) -> Value {
        json!({
            "kind": "credentials",
            "loginUrl": match self { Self::Com => LOGIN_URLS[0], Self::Cc => LOGIN_URLS[1] },
            "loginUrls": LOGIN_URLS,
            "sessionCookieNames": [COOKIE],
            "credentialStorage": "memory",
            "usernameLabel": "電子郵件",
            "passwordLabel": "密碼"
        })
    }
}

fn enabled(request: &[u8]) -> bool {
    let features = json_object_member(request, b"hostFeatures")
        .and_then(|raw| serde_json::from_slice::<Vec<String>>(raw).ok());
    [
        "source_authentication_v1",
        "source_private_session_v1",
        "source_private_session_replace_v1",
        AUTH_FEATURE,
    ]
    .iter()
    .all(|needed| {
        features
            .as_ref()
            .is_some_and(|items| items.iter().any(|item| item == needed))
    })
}

fn rpc(request: Value) -> Result<Value> {
    let mut request = request;
    request["type"] = json!("private_session");
    request["version"] = json!(1);
    let bytes = serde_json::to_vec(&request).map_err(|_| FetchError::Network)?;
    let len = super::http_request(&bytes, http_out()).map_err(|_| FetchError::Network)?;
    let response: Value =
        serde_json::from_slice(&http_out()[..len]).map_err(|_| FetchError::Network)?;
    if response["ok"] == true {
        return Ok(response);
    }
    Err(match response["error"]["code"].as_str() {
        Some("stale_version") => FetchError::StaleSession,
        Some("inactive") => FetchError::Cancelled,
        Some("credential_input_unavailable") => FetchError::AuthRequired,
        _ => FetchError::Network,
    })
}

// One Asset record owns the versioned map. Only the selected exact origin's
// access cookie is added to its requests. Login input is never part of this map.
struct Session {
    value: Value,
    tag: String,
    now: u64,
}

impl Session {
    fn token(&self, site: Site) -> Option<&str> {
        self.value[site.key()]["access"]
            .as_str()
            .filter(|value| valid_cookie(value))
    }

    fn expires(&self, site: Site) -> u64 {
        self.value[site.key()]["expiresAt"].as_u64().unwrap_or(0)
    }
}

struct Lease {
    id: String,
    now: u64,
    committed: bool,
}

impl Lease {
    fn from_begin(value: &Value) -> Result<Self> {
        if value["state"] != "owner" {
            return Err(FetchError::Network);
        }
        Ok(Self {
            id: value["lease"]
                .as_str()
                .ok_or(FetchError::Network)?
                .to_string(),
            now: now(value)?,
            committed: false,
        })
    }

    fn commit(mut self, value: Value) -> Result<Session> {
        let response = rpc(json!({"op":"commit", "lease":self.id, "value":value.to_string()}))?;
        self.committed = true;
        Ok(Session {
            value,
            tag: response["versionTag"]
                .as_str()
                .ok_or(FetchError::Network)?
                .to_string(),
            now: self.now,
        })
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if !self.committed {
            let _ = rpc(json!({"op":"abort","lease":self.id}));
        }
    }
}

enum Begin {
    Ready(Session),
    Empty(Lease),
}

fn now(value: &Value) -> Result<u64> {
    value["nowMs"]
        .as_str()
        .and_then(|ms| ms.parse::<u64>().ok())
        .map(|ms| ms / 1000)
        .ok_or(FetchError::Network)
}

fn begin() -> Result<Begin> {
    let response = rpc(json!({"op":"begin"}))?;
    if response["state"] == "owner" {
        return Ok(Begin::Empty(Lease::from_begin(&response)?));
    }
    session_from_response(&response).map(Begin::Ready)
}

fn session_from_response(response: &Value) -> Result<Session> {
    if response["state"] != "ready" {
        return Err(FetchError::Network);
    }
    let raw = response["value"].as_str().ok_or(FetchError::Network)?;
    let value: Value = serde_json::from_str(raw).map_err(|_| FetchError::Network)?;
    if value["version"] != 1 {
        return Err(FetchError::Network);
    }
    Ok(Session {
        value,
        tag: response["versionTag"]
            .as_str()
            .ok_or(FetchError::Network)?
            .to_string(),
        now: now(response)?,
    })
}

fn read() -> Result<Option<Session>> {
    let response = rpc(json!({"op":"peek"}))?;
    if response["state"] == "empty" {
        return Ok(None);
    }
    session_from_response(&response).map(Some)
}

fn replace(session: &Session) -> Result<Lease> {
    Lease::from_begin(&rpc(json!({"op":"begin","replaceVersionTag":session.tag}))?)
}

fn valid_cookie(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn jwt_expiry(token: &str) -> Option<u64> {
    let mut parts = token.split('.');
    let (_, payload, signature) = (parts.next()?, parts.next()?, parts.next()?);
    if signature.is_empty() || parts.next().is_some() {
        return None;
    }
    let raw = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice::<Value>(&raw).ok()?["exp"].as_u64()
}

fn cookie_from_response(response: &Value, site: Site, now: u64) -> Option<Value> {
    let mut current = None;
    for header in response["sessionSetCookies"].as_array()? {
        let Some(raw) = header.as_str() else { continue };
        let mut fields = raw.split(';');
        let Some((name, value)) = fields.next().and_then(|pair| pair.trim().split_once('=')) else {
            continue;
        };
        if name != COOKIE {
            continue;
        }
        let mut domain_matches = true;
        let mut root_path = false;
        let mut max_age = None;
        for field in fields {
            let Some((name, value)) = field.trim().split_once('=') else {
                continue;
            };
            if name.eq_ignore_ascii_case("Domain") {
                domain_matches &= value
                    .trim_start_matches('.')
                    .eq_ignore_ascii_case(site.domain());
            } else if name.eq_ignore_ascii_case("Path") {
                root_path = value == "/";
            } else if name.eq_ignore_ascii_case("Max-Age") {
                max_age = value.parse::<i64>().ok();
            }
        }
        if !domain_matches || !root_path {
            continue;
        }
        current = None;
        if !valid_cookie(value) || max_age.is_some_and(|age| age <= 0) {
            continue;
        }
        let Some(jwt_exp) = jwt_expiry(value) else {
            continue;
        };
        let expires = max_age
            .map(|age| now.saturating_add(age as u64).min(jwt_exp))
            .unwrap_or(jwt_exp);
        if expires > now {
            current = Some(json!({"access":value,"expiresAt":expires}));
        }
    }
    current
}

fn post(site: Site, path: &str, body: &str, token: Option<&str>) -> Result<Value> {
    let mut headers = json!({
        "Content-Type": "application/json",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Origin": site.origin(),
        "Referer": format!("{}/{}", site.origin(), if path == "/api/login" { "login" } else { "" }),
    });
    if let Some(token) = token {
        headers["Cookie"] = json!(format!("{COOKIE}={token}"));
    }
    let request = json!({
        "version": 1, "method": "POST", "url": format!("{}{path}", site.origin()),
        "nativeSession": true, "headers": headers, "bodyBase64": body,
        "timeoutMs": 15000, "responseKind": "bodyText",
    });
    let request = serde_json::to_vec(&request).map_err(|_| FetchError::Network)?;
    let len = super::http_request(&request, http_out()).map_err(|_| FetchError::Network)?;
    serde_json::from_slice(&http_out()[..len]).map_err(|_| FetchError::Network)
}

fn successful(response: &Value) -> Result<()> {
    if response["ok"] == true {
        return Ok(());
    }
    Err(match response["status"].as_u64() {
        Some(401) => FetchError::AuthRequired,
        Some(404) => FetchError::NotFound,
        Some(429) => FetchError::RateLimit,
        Some(400..=499) => FetchError::ClientError,
        Some(500..=599) => FetchError::ServerError,
        _ => FetchError::Network,
    })
}

fn confirm(site: Site, candidate: &Value) -> Result<()> {
    let access = candidate["access"]
        .as_str()
        .ok_or(FetchError::AuthRequired)?;
    let response = post(site, "/api/query", ACCOUNT_QUERY, Some(access))?;
    successful(&response)?;
    let account: Value =
        serde_json::from_str(response["bodyText"].as_str().ok_or(FetchError::Network)?)
            .map_err(|_| FetchError::Graphql)?;
    if account["data"]["account"]["id"]
        .as_str()
        .is_some_and(|id| !id.is_empty() && id != "0")
        && account["errors"].is_null()
    {
        return Ok(());
    }
    Err(FetchError::AuthRequired)
}

fn authenticate(site: Site) -> Result<Value> {
    let grant = rpc(json!({"op":"credential_input"}))?;
    let email = grant["credentials"]["username"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or(FetchError::AuthRequired)?;
    let password = grant["credentials"]["password"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or(FetchError::AuthRequired)?;
    let (lease, mut value) = match begin()? {
        Begin::Empty(lease) => (lease, json!({"version":1})),
        Begin::Ready(session) => (replace(&session)?, session.value),
    };
    let response = post(
        site,
        "/api/login",
        &json!({"email":email,"password":password}).to_string(),
        None,
    )?;
    if response["status"] == 400 || response["status"] == 403 {
        return Err(FetchError::AuthRequired);
    }
    successful(&response)?;
    let candidate =
        cookie_from_response(&response, site, lease.now).ok_or(FetchError::AuthRequired)?;
    confirm(site, &candidate)?;
    value[site.key()] = candidate;
    lease.commit(value)?;
    Ok(json!({"authenticated":true}))
}

fn renew(site: Site, mut session: Session) -> Result<Session> {
    let stale = session
        .token(site)
        .ok_or(FetchError::AuthRequired)?
        .to_string();
    let lease = match replace(&session) {
        Ok(lease) => lease,
        Err(FetchError::StaleSession) => {
            session = read()?.ok_or(FetchError::AuthRequired)?;
            if session.token(site).ok_or(FetchError::AuthRequired)? != stale {
                return Ok(session);
            }
            // A successful transaction on the other mirror changes the shared
            // generation without renewing this mirror's cookie.
            replace(&session)?
        }
        Err(error) => return Err(error),
    };
    let response = post(site, "/auth/refresh", "", Some(&stale))?;
    successful(&response)?;
    let candidate =
        cookie_from_response(&response, site, lease.now).ok_or(FetchError::AuthRequired)?;
    if candidate["access"] == stale {
        return Err(FetchError::AuthRequired);
    }
    confirm(site, &candidate)?;
    session.value[site.key()] = candidate;
    lease.commit(session.value)
}

fn graphql_result(response: Value) -> Result<&'static [u8]> {
    successful(&response)?;
    let body = response["bodyText"]
        .as_str()
        .ok_or(FetchError::Network)?
        .as_bytes();
    if let Some(errors) = json_object_member(body, b"errors") {
        let errors: Vec<Value> = serde_json::from_slice(errors).map_err(|_| FetchError::Graphql)?;
        if !errors.is_empty() {
            if errors.iter().any(super::image_quota_error) {
                return Err(FetchError::ImageQuotaExhausted);
            }
            let expired = errors.iter().any(|error| {
                matches!(
                    error["message"].as_str(),
                    Some("no token" | "token_expired")
                ) || matches!(
                    error["extensions"]["code"].as_str(),
                    Some("UNAUTHENTICATED" | "TOKEN_EXPIRED")
                )
            });
            return Err(if expired {
                FetchError::AuthRequired
            } else {
                FetchError::Graphql
            });
        }
    }
    let dst = super::json_buf();
    let mut len = 0;
    if !write_bytes(dst, &mut len, body) {
        return Err(FetchError::Network);
    }
    Ok(json_slice(len))
}

pub fn graphql(request: &[u8], body: &[u8], image_ticket: bool) -> Option<Result<&'static [u8]>> {
    if !enabled(request) {
        return None;
    }
    let site = Site::selected(request);
    let mut session = read().ok()??;
    let expires = session.expires(site);
    let mut refreshed = false;
    session.token(site)?;
    if expires <= session.now.saturating_add(REFRESH_WINDOW_SECS) {
        match renew(site, session) {
            Ok(value) => {
                session = value;
                refreshed = true;
            }
            Err(error) => {
                session = read().ok()??;
                if session.expires(site) <= session.now || matches!(error, FetchError::AuthRequired)
                {
                    return image_ticket.then_some(Err(error));
                }
            }
        }
    }
    let Ok(body) = core::str::from_utf8(body) else {
        return Some(Err(FetchError::Graphql));
    };
    let first = post(site, "/api/query", body, session.token(site)).and_then(graphql_result);
    if !matches!(first, Err(FetchError::AuthRequired)) {
        return Some(first);
    }
    if refreshed {
        return image_ticket.then_some(first);
    }
    match renew(site, session) {
        Ok(session) => {
            let result =
                post(site, "/api/query", body, session.token(site)).and_then(graphql_result);
            if !image_ticket && matches!(result, Err(FetchError::AuthRequired)) {
                None
            } else {
                Some(result)
            }
        }
        Err(FetchError::AuthRequired) if !image_ticket => None,
        Err(error) => Some(Err(error)),
    }
}

fn logout(site: Site) -> Result<Value> {
    if let Some(mut session) = read()? {
        if session.value[site.key()].is_object() {
            let lease = replace(&session)?;
            session
                .value
                .as_object_mut()
                .ok_or(FetchError::Network)?
                .remove(site.key());
            lease.commit(session.value)?;
        }
    }
    Ok(json!({"authenticated":false}))
}

pub fn operation(request: &[u8], operation: &str) -> u32 {
    if !enabled(request) {
        return write_error(
            operation,
            "unsupported_capability",
            "native session is unavailable",
        );
    }
    let site = Site::selected(request);
    let result = match operation {
        "get_authentication" => read().map(|state| json!({
            "authentication":site.descriptor(),
            "authenticated":state.is_some_and(|session| session.token(site).is_some() && session.expires(site) > session.now),
        })),
        "authenticate" => authenticate(site),
        "logout" => logout(site),
        _ => return write_error(operation, "invalid_request", "unknown session operation"),
    };
    match result {
        Ok(value) => {
            let mut len = 0;
            if write_bytes(payload_buf(), &mut len, value.to_string().as_bytes()) {
                write_success_payload(operation, len)
            } else {
                write_error(operation, "internal_error", "overflow")
            }
        }
        Err(error) => {
            let (code, message) = super::fetch_error_code(error);
            write_error(operation, code, message)
        }
    }
}
