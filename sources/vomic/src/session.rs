use super::*;

fn rpc(query: Value) -> Result<Value> {
    let mut query = query;
    query["version"] = json!(1);
    query["type"] = json!("private_session");
    let bytes = serde_json::to_vec(&query).map_err(|_| "internal_error")?;
    let size = host::http_request(&bytes, http_out()).map_err(|_| "network_error")?;
    let response: Value =
        serde_json::from_slice(&http_out()[..size]).map_err(|_| "network_error")?;
    if response["ok"] != true {
        return Err(match response["error"]["code"].as_str() {
            Some("stale_version") => "stale_version",
            Some("inactive") => "cancelled",
            Some("credential_input_unavailable") => "auth_required",
            _ => "network_error",
        });
    }
    Ok(response)
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or("parse_error")
}

pub(super) fn clock_seconds() -> Result<String> {
    let response = rpc(json!({"op":"clock"}))?;
    let millis = field(&response, "nowMs")?
        .parse::<u64>()
        .map_err(|_| "network_error")?;
    Ok((millis / 1000).to_string())
}

fn require(request: &[u8]) -> Result<()> {
    check_feature(request, b"source_private_session_v1")?;
    check_feature(request, b"source_authentication_v1")?;
    check_feature(request, b"source_native_cookie_session_v1")?;
    check_feature(request, b"source_private_session_replace_v1")
}

fn descriptor() -> Value {
    json!({"kind":"credentials","loginUrl":LOGIN,"usernameLabel":"邮箱","passwordLabel":"密码","credentialStorage":"private"})
}

pub(super) struct Session {
    email: String,
    password: String,
    token: String,
    tag: String,
}

impl Session {
    pub(super) fn token(&self) -> &str {
        &self.token
    }
}

fn prior(response: &Value) -> Result<Session> {
    let value: Value =
        serde_json::from_str(field(response, "value")?).map_err(|_| "parse_error")?;
    if value["v"] != 1 {
        return Err("parse_error");
    }
    Ok(Session {
        email: field(&value, "email")?.to_string(),
        password: field(&value, "password")?.to_string(),
        token: field(&value, "token")?.to_string(),
        tag: field(response, "versionTag")?.to_string(),
    })
}

pub(super) fn current() -> Result<Option<Session>> {
    // This path does not initialize the store or hold a write lease. The
    // published value is retained while a replacement is in progress.
    let response = rpc(json!({"op":"peek"}))?;
    match field(&response, "state")? {
        "ready" => prior(&response).map(Some),
        "empty" => Ok(None),
        _ => Err("network_error"),
    }
}

fn claim(tag: Option<&str>) -> Result<String> {
    let mut request = json!({"op":"begin"});
    if let Some(tag) = tag {
        request["replaceVersionTag"] = json!(tag);
    }
    let response = rpc(request)?;
    match field(&response, "state")? {
        "owner" => Ok(field(&response, "lease")?.to_string()),
        "ready" => Err("stale_version"),
        _ => Err("network_error"),
    }
}

fn replace(lease: &str, email: &str, password: &str) -> Result<Session> {
    let result = (|| {
        let response = request_api(
            &endpoint("login")?,
            Some(json!({"identity":email,"password":password})),
            None,
        )?;
        let token = field(&response, "token").map_err(|_| "auth_required")?;
        if token.chars().any(char::is_control) {
            return Err("parse_error");
        }
        let value = json!({"v":1,"email":email,"password":password,"token":token}).to_string();
        let response = rpc(json!({"op":"commit","lease":lease,"value":value}))?;
        Ok(Session {
            email: email.to_string(),
            password: password.to_string(),
            token: token.to_string(),
            tag: field(&response, "versionTag")?.to_string(),
        })
    })();
    if result.is_err() {
        let _ = rpc(json!({"op":"abort","lease":lease}));
    }
    result
}

pub(super) fn status(request: &[u8]) -> Result<Value> {
    require(request)?;
    Ok(json!({"authentication":descriptor(),"authenticated":current()?.is_some()}))
}

pub(super) fn authenticate(request: &[u8]) -> Result<Value> {
    require(request)?;
    let input = rpc(json!({"op":"credential_input"}))?;
    let email = field(&input["credentials"], "username").map_err(|_| "auth_required")?;
    let password = field(&input["credentials"], "password").map_err(|_| "auth_required")?;
    let previous = current()?;
    let lease = claim(previous.as_ref().map(|session| session.tag.as_str()))?;
    replace(&lease, email, password)?;
    Ok(json!({"authenticated":true}))
}

pub(super) fn logout(request: &[u8]) -> Result<Value> {
    require(request)?;
    if let Some(session) = current()? {
        rpc(json!({"op":"invalidate","versionTag":session.tag}))?;
    }
    Ok(json!({"authenticated":false}))
}

pub(super) fn refresh(stale: &Session) -> Result<Session> {
    let current = current()?.ok_or("auth_required")?;
    if current.tag != stale.tag {
        return Ok(current);
    }
    match claim(Some(&stale.tag)) {
        Ok(lease) => replace(&lease, &current.email, &current.password),
        Err("stale_version") => self::current()?
            .filter(|session| session.tag != stale.tag)
            .ok_or("auth_required"),
        Err(error) => Err(error),
    }
}
