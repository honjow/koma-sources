use crate::types::{HttpErrorEnvelope, HttpRequest, HttpResponse};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use wasmtime::*;

/// Per-source settings store (key → value). Shared across WASM invocations.
static SETTINGS: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Set a source setting value.
pub fn set_setting(key: &str, value: &str) {
    if let Ok(mut s) = SETTINGS.lock() {
        s.insert(key.to_string(), value.to_string());
    }
}

/// A selected descriptor points into its original document; it is never reparsed.
#[derive(Clone, Copy)]
struct HtmlHandle { owner: usize, node: Option<ego_tree::NodeId> }

/// State stored inside the wasmtime Store
pub struct HostState {
    pub logs: Vec<String>,
    html_docs: Vec<Option<scraper::Html>>,
    html_handles: Vec<Option<HtmlHandle>>,
}

impl HostState {
    fn new() -> Self { Self { logs: Vec::new(), html_docs: vec![None], html_handles: vec![None] } }
    fn parse_html(&mut self, html: &str) -> i32 {
        let owner = self.html_docs.len();
        self.html_docs.push(Some(scraper::Html::parse_document(html)));
        self.add_handle(HtmlHandle { owner, node: None })
    }
    fn add_handle(&mut self, handle: HtmlHandle) -> i32 {
        let id = self.html_handles.len(); self.html_handles.push(Some(handle)); id as i32
    }
    fn handle(&self, id: i32) -> Option<HtmlHandle> {
        if id <= 0 { return None; } self.html_handles.get(id as usize)?.as_ref().copied()
    }
    fn select_ids(&self, id: i32, selector: &scraper::Selector) -> Option<(usize, Vec<ego_tree::NodeId>)> {
        let h = self.handle(id)?; let doc = self.html_docs.get(h.owner)?.as_ref()?;
        let root = match h.node { Some(node) => doc.tree.get(node)?, None => doc.tree.root() };
        let scope = match h.node { Some(_) => Some(scraper::ElementRef::wrap(root)?), None => None };
        // Malformed source HTML can cause ego-tree's traversal iterator to
        // leave the selected scope early. Follow only the original child links.
        let mut stack: Vec<_> = root.children().collect(); stack.reverse();
        let mut seen = std::collections::HashSet::new();
        let mut ids = Vec::new();
        while let Some(current) = stack.pop() {
            if !seen.insert(current.id()) { continue; }
            if let Some(el) = scraper::ElementRef::wrap(current) {
                if selector.matches_with_scope(&el, scope) { ids.push(el.id()); }
            }
            let mut children: Vec<_> = current.children().collect(); children.reverse(); stack.extend(children);
        }
        Some((h.owner, ids))
    }
    fn attr(&self, id: i32, name: &str) -> Option<String> {
        let h = self.handle(id)?; let doc = self.html_docs.get(h.owner)?.as_ref()?;
        let el = scraper::ElementRef::wrap(doc.tree.get(h.node?)?)?; Some(el.value().attr(name)?.to_owned())
    }
    fn text(&self, id: i32) -> Option<String> {
        let h = self.handle(id)?; let doc = self.html_docs.get(h.owner)?.as_ref()?;
        let root = match h.node { Some(node) => doc.tree.get(node)?, None => doc.tree.root() };
        let mut stack = vec![root]; let mut seen = std::collections::HashSet::new(); let mut text = String::new();
        while let Some(current) = stack.pop() {
            if !seen.insert(current.id()) { continue; }
            if let scraper::Node::Text(value) = current.value() { text.push_str(value); }
            let mut children: Vec<_> = current.children().collect(); children.reverse(); stack.extend(children);
        }
        Some(text)
    }
    fn close(&mut self, id: i32) -> bool {
        let Some(h) = self.handle(id) else { return false; };
        if h.node.is_none() {
            self.html_docs[h.owner] = None;
            for slot in &mut self.html_handles { if slot.as_ref().is_some_and(|x| x.owner == h.owner) { *slot = None; } }
        } else { self.html_handles[id as usize] = None; }
        true
    }
}

fn read_guest_bytes(memory: &Memory, store: &impl AsContext<Data = HostState>, ptr: u32, len: u32) -> Result<Vec<u8>> {
    let data = memory.data(store);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        anyhow::bail!("guest memory out of bounds");
    }
    Ok(data[start..end].to_vec())
}

fn write_guest_bytes(memory: &Memory, store: &mut impl AsContextMut<Data = HostState>, ptr: u32, bytes: &[u8]) -> Result<usize> {
    let data = memory.data_mut(store);
    let start = ptr as usize;
    let end = start + bytes.len();
    if end > data.len() {
        anyhow::bail!("guest memory out of bounds for write");
    }
    data[start..end].copy_from_slice(bytes);
    Ok(bytes.len())
}

/// Execute real HTTP request
fn do_http_request(req: &HttpRequest) -> HttpResponse {
    let url_lower = req.url.to_lowercase();
    if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
        return HttpResponse {
            ok: false, status: None, headers: None, body_text: None, body_json: None,
            final_url: None, network_performed: false,
            error: Some(HttpErrorEnvelope {
                code: "scheme_not_allowed".into(),
                message: format!("Only http/https allowed"),
                phase: "url".into(), retryable: false,
            }),
        };
    }

    let method_upper = req.method.to_uppercase();
    if matches!(method_upper.as_str(), "CONNECT" | "TRACE") {
        return HttpResponse {
            ok: false, status: None, headers: None, body_text: None, body_json: None,
            final_url: None, network_performed: false,
            error: Some(HttpErrorEnvelope {
                code: "method_not_allowed".into(),
                message: format!("Method {} not allowed", method_upper),
                phase: "method".into(), retryable: false,
            }),
        };
    }

    let mut ureq_req = ureq::request(&method_upper, &req.url);
    ureq_req = ureq_req.timeout(std::time::Duration::from_millis(req.timeout_ms));

    for (key, value) in &req.headers {
        ureq_req = ureq_req.set(key, value);
    }

    if !req.headers.keys().any(|k| k.to_lowercase() == "user-agent") {
        ureq_req = ureq_req.set("User-Agent", "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36");
    }

    let result = if let Some(body) = req.effective_body() {
        ureq_req.send_string(body)
    } else {
        ureq_req.call()
    };

    match result {
        Ok(resp) => build_success_response(&req, resp),
        Err(ureq::Error::Status(_, resp)) => build_success_response(&req, resp),
        Err(e) => {
            eprintln!("[dev-runner] HTTP error: {:?}", e);
            HttpResponse {
                ok: false, status: None, headers: None, body_text: None, body_json: None,
                final_url: None, network_performed: true,
                error: Some(HttpErrorEnvelope {
                    code: "network_error".into(),
                    message: format!("{}", e),
                    phase: "transport".into(), retryable: true,
                }),
            }
        },
    }
}

// Opt-in diagnostic guard: persist only a digest of the complete request
// identity. 400 is deliberately excluded because it can be a legal fallback.
fn coverage_request_key(req: &HttpRequest) -> Result<String> {
    let mut headers: Vec<(String, String)> = req.headers.iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.clone())).collect();
    headers.sort();
    // Duplicate case-insensitive names can produce order-dependent wire headers;
    // refuse the ambiguous diagnostic request instead of hashing a guessed identity.
    if headers.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        anyhow::bail!("ambiguous case-insensitive request headers");
    }
    let identity = serde_json::to_vec(&(req.method.to_ascii_uppercase(), &req.url, req.effective_body(), headers))?;
    Ok(format!("{:x}", Sha256::digest(identity)))
}
fn coverage_guard_error(code: &str) -> HttpResponse {
    HttpResponse { ok:false,status:None,headers:None,body_text:None,body_json:None,
        final_url:None,network_performed:false,error:Some(HttpErrorEnvelope {
            code:code.into(),message:"diagnostic coverage guard prevented network request".into(),
            phase:"diagnostic".into(),retryable:false,
        }) }
}
fn guarded_http_request<F>(req: &HttpRequest, dir: Option<&Path>, mut send: F) -> HttpResponse
where F: FnMut(&HttpRequest) -> HttpResponse {
    let Some(dir) = dir else { return send(req); };
    if std::fs::create_dir_all(dir).is_err() { eprintln!("[dev-runner] coverage guard unavailable"); return coverage_guard_error("coverage_guard_unavailable"); }
    let Ok(key) = coverage_request_key(req) else { eprintln!("[dev-runner] coverage guard unavailable"); return coverage_guard_error("coverage_guard_unavailable"); };
    let marker = dir.join(key);
    if marker.exists() {
        eprintln!("[dev-runner] coverage request suppressed");
        return coverage_guard_error("coverage_request_suppressed");
    }
    // Fail before networking if this scope cannot persist a future denial.
    static PROBE: AtomicU64 = AtomicU64::new(0);
    let probe = dir.join(format!(".probe-{}-{}", std::process::id(), PROBE.fetch_add(1, Ordering::Relaxed)));
    if std::fs::OpenOptions::new().write(true).create_new(true).open(&probe).is_err()
        || std::fs::remove_file(&probe).is_err() {
        eprintln!("[dev-runner] coverage guard unavailable");
        return coverage_guard_error("coverage_guard_unavailable");
    }
    let response = send(req);
    if matches!(response.status, Some(401 | 403 | 429)) {
        if std::fs::OpenOptions::new().write(true).create_new(true).open(marker).is_err() {
            // AlreadyExists is safe when a concurrent identical request recorded it.
            if !dir.join(coverage_request_key(req).unwrap_or_default()).exists() {
                eprintln!("[dev-runner] coverage guard unavailable");
            }
        }
    }
    response
}

// Match the host's batch envelope while executing child calls serially for
// reproducible, rate-conscious source coverage. Private state is never emulated.
fn encode_http_envelope<F>(input: &[u8], mut send: F) -> Result<Vec<u8>>
where F: FnMut(&HttpRequest, bool) -> HttpResponse {
    let value: serde_json::Value = serde_json::from_slice(input)?;
    // This CLI has no RCP DNS adapter or Host-issued process proof. Never
    // silently discard the opt-in plan and send via system DNS/channel 1.
    let needs_pica_dns = |v: &serde_json::Value| {
        v.get("type").and_then(|x| x.as_str()) == Some("pica_channel_init") || v.get("dnsOverride").is_some()
    };
    if needs_pica_dns(&value) || value.get("requests").and_then(|v| v.as_array())
        .is_some_and(|requests| requests.iter().any(needs_pica_dns)) {
        return Ok(serde_json::to_vec(&serde_json::json!({"ok":false,"networkPerformed":false,
            "error":{"code":"unsupported","message":"DNS-preserving channel transport unavailable in this host","retryable":false}}))?);
    }
    if value.get("type").and_then(|v| v.as_str()) == Some("private_session") {
        eprintln!("[dev-runner] unsupported host feature private_session");
        return Ok(serde_json::to_vec(&serde_json::json!({"ok":false,"networkPerformed":false,"error":{"code":"host_feature_required","message":"private session unsupported","retryable":false}}))?);
    }
    if value.get("type").and_then(|v| v.as_str()) == Some("java_regex") {
        eprintln!("[dev-runner] unsupported host feature java_regex");
        return Ok(serde_json::to_vec(&serde_json::json!({"ok":false,"networkPerformed":false,"error":{"code":"host_feature_required","message":"java regex unsupported","retryable":false}}))?);
    }
    if let Some(group) = value.get("requests") {
        let invalid = || serde_json::json!({"ok":false,"networkPerformed":false,"error":{"code":"invalid_request","message":"network request failed","retryable":false}});
        let too_large = || serde_json::json!({"ok":false,"networkPerformed":false,"error":{"code":"request_too_large","message":"network request failed","retryable":false}});
        let Some(requests) = group.as_array() else { return Ok(serde_json::to_vec(&invalid())?); };
        if requests.is_empty() || requests.len() > 5 { return Ok(serde_json::to_vec(&invalid())?); }
        let mut validated = Vec::with_capacity(requests.len());
        for item in requests {
            let Some(obj) = item.as_object() else { return Ok(serde_json::to_vec(&invalid())?); };
            if obj.contains_key("requests") { return Ok(serde_json::to_vec(&invalid())?); }
            let Some(url) = obj.get("url").and_then(|v|v.as_str()).map(str::trim) else { return Ok(serde_json::to_vec(&invalid())?); };
            if !(url.starts_with("https://") || url.starts_with("http://")) { return Ok(serde_json::to_vec(&invalid())?); }
            let Some(method) = obj.get("method").and_then(|v|v.as_str()).map(|v|v.trim().to_ascii_uppercase()) else { return Ok(serde_json::to_vec(&invalid())?); };
            if !(3..=12).contains(&method.len()) || !method.bytes().all(|c|c.is_ascii_uppercase()) { return Ok(serde_json::to_vec(&invalid())?); }
            let body = obj.get("bodyBase64").and_then(|v|v.as_str()).or_else(||obj.get("body").and_then(|v|v.as_str())).unwrap_or("");
            if body.encode_utf16().count() > 1024*1024 { return Ok(serde_json::to_vec(&too_large())?); }
            let mut normalized=item.clone();
            normalized["version"]=serde_json::json!(1);
            normalized["url"]=serde_json::json!(url);
            normalized["method"]=serde_json::json!(method);
            // The production host ignores malformed/oversized headers instead
            // of rejecting an otherwise valid group. Normalize those inputs
            // before handing the child to the existing development transport.
            let mut headers = serde_json::Map::new();
            if let Some(values) = obj.get("headers").and_then(|v|v.as_object()) {
                for (name,value) in values {
                    if let Some(raw)=value.as_str() {
                        if !name.trim().is_empty() && name.encode_utf16().count()<=256 && raw.encode_utf16().count()<=8192 {
                            headers.insert(name.clone(),serde_json::json!(raw));
                        }
                    }
                }
            }
            normalized["headers"]=serde_json::Value::Object(headers);
            let timeout = obj.get("timeoutMs").and_then(|v| v.as_f64().or_else(||v.as_str().and_then(|v|v.parse::<f64>().ok())))
                .filter(|v|v.is_finite() && *v>0.0).map(|v|v.floor().clamp(1000.0,60000.0) as u64).unwrap_or(15000);
            normalized["timeoutMs"]=serde_json::json!(timeout);
            if obj.get("bodyBase64").and_then(|v|v.as_str()).is_none() {
                if obj.get("body").and_then(|v|v.as_str()).is_some() { normalized["bodyBase64"]=serde_json::json!(body); }
                else if let Some(object)=normalized.as_object_mut() { object.remove("bodyBase64"); }
            }
            if obj.get("responseKind").and_then(|v|v.as_str()).is_none() {
                if let Some(object)=normalized.as_object_mut() { object.remove("responseKind"); }
            }
            let Ok(req) = serde_json::from_value::<HttpRequest>(normalized) else { return Ok(serde_json::to_vec(&invalid())?); };
            validated.push(req);
        }
        let responses: Vec<HttpResponse> = validated.iter().map(|req|send(req,true)).collect();
        return Ok(serde_json::to_vec(&serde_json::json!({"ok":true,"responses":responses}))?);
    }
    let req: HttpRequest = serde_json::from_value(value)?;
    Ok(serde_json::to_vec(&send(&req,false))?)
}

fn build_success_response(req: &HttpRequest, resp: ureq::Response) -> HttpResponse {
    let status = resp.status();
    let mut resp_headers = std::collections::HashMap::new();
    for name in resp.headers_names() {
        if let Some(val) = resp.header(&name) {
            resp_headers.insert(name, val.to_string());
        }
    }
    // A successful status is not a complete response: preserve body-read
    // failure instead of silently turning a truncated response into empty HTML.
    let body_text = match resp.into_string() {
        Ok(body) => body,
        Err(error) => return HttpResponse {
            ok: false, status: Some(status), headers: Some(resp_headers),
            body_text: None, body_json: None, final_url: Some(req.url.clone()),
            network_performed: true,
            error: Some(HttpErrorEnvelope {
                code: "network_error".into(), message: error.to_string(),
                phase: "response_body".into(), retryable: true,
            }),
        },
    };
    let body_json = if req.response_kind == "bodyJson" {
        serde_json::from_str(&body_text).ok()
    } else { None };
    HttpResponse {
        ok: true, status: Some(status), headers: Some(resp_headers),
        body_text: Some(body_text), body_json,
        final_url: Some(req.url.clone()), network_performed: true, error: None,
    }
}

pub fn run_source_info(wasm_path: &Path) -> Result<serde_json::Value> {
    let (mut store, instance) = instantiate_source(wasm_path)?;

    let source_info = instance
        .get_typed_func::<(), u32>(&mut store, "koma_source_info")
        .context("Missing export koma_source_info")?;

    let result_ptr = source_info.call(&mut store, ())?;
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("no memory export"))?;
    let result = read_result_buffer(&memory, &store, result_ptr)?;

    let logs = &store.data().logs;
    for log in logs {
        eprintln!("[source log] {}", log);
    }

    Ok(result)
}

pub fn run_operation(wasm_path: &Path, op: &str, request_json: &str) -> Result<serde_json::Value> {
    let (mut store, instance) = instantiate_source(wasm_path)?;

    let export_name = match op {
        "search" => "koma_source_search",
        "get_manga" => "koma_source_get_manga",
        "get_chapters" => "koma_source_get_chapters",
        "get_pages" => "koma_source_get_pages",
        "get_listings" => "koma_source_get_listings",
        "get_manga_list" => "koma_source_get_manga_list",
        "get_home" => "koma_source_get_home",
        "get_filters" => "koma_source_get_filters",
        "get_settings" => "koma_source_get_settings",
        "get_authentication" => "koma_source_get_authentication",
        "authenticate" => "koma_source_authenticate",
        "logout" => "koma_source_logout",
        "get_image_request" => "koma_source_get_image_request",
        "image_request" | "modify_image_request" => "koma_source_modify_image_request",
        _ => anyhow::bail!("Unknown operation: {}", op),
    };

    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("no memory export"))?;

    // Auto-inject "operation" field into request JSON if missing
    let enriched_request = {
        let mut parsed: serde_json::Value = serde_json::from_str(request_json)
            .with_context(|| format!("Invalid request JSON: {}", request_json))?;
        if let Some(obj) = parsed.as_object_mut() {
            if !obj.contains_key("operation") {
                obj.insert("operation".into(), serde_json::Value::String(op.to_string()));
            }
        }
        serde_json::to_string(&parsed)?
    };
    let request_bytes = enriched_request.as_bytes();

    // Write request into guest memory
    let (req_ptr, req_len) = if let Ok(alloc) = instance.get_typed_func::<u32, u32>(&mut store, "koma_source_alloc") {
        let ptr = alloc.call(&mut store, request_bytes.len() as u32)?;
        write_guest_bytes(&memory, &mut store, ptr, request_bytes)?;
        (ptr, request_bytes.len() as u32)
    } else {
        // Fallback: write at high offset
        let offset = 0x10000_u32;
        write_guest_bytes(&memory, &mut store, offset, request_bytes)?;
        (offset, request_bytes.len() as u32)
    };

    let op_func = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, export_name)
        .with_context(|| format!("Missing export {}", export_name))?;

    let result_ptr = op_func.call(&mut store, (req_ptr, req_len))?;
    let result = read_result_buffer(&memory, &store, result_ptr)?;

    let logs = &store.data().logs;
    for log in logs {
        eprintln!("[source log] {}", log);
    }

    Ok(result)
}

fn read_result_buffer(memory: &Memory, store: &impl AsContext<Data = HostState>, ptr: u32) -> Result<serde_json::Value> {
    let data = memory.data(store);
    let base = ptr as usize;

    // Result buffer layout: magic(4) + flags(4) + payload_len(4) + reserved(4) + payload
    const HEADER_LEN: usize = 16;
    const KOMA_MAGIC: u32 = 0x4B4F4D41;

    if base + HEADER_LEN > data.len() {
        anyhow::bail!("result pointer out of bounds: ptr={} memsize={}", ptr, data.len());
    }

    let magic = u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
    if magic != KOMA_MAGIC {
        anyhow::bail!("invalid result buffer magic: expected 0x{:08X}, got 0x{:08X}", KOMA_MAGIC, magic);
    }

    let _flags = u32::from_le_bytes([data[base + 4], data[base + 5], data[base + 6], data[base + 7]]);
    let payload_len = u32::from_le_bytes([data[base + 8], data[base + 9], data[base + 10], data[base + 11]]) as usize;

    let json_start = base + HEADER_LEN;
    let json_end = json_start + payload_len;
    if json_end > data.len() {
        anyhow::bail!("result buffer JSON out of bounds: start={} len={} memsize={}", json_start, payload_len, data.len());
    }

    let json_bytes = &data[json_start..json_end];
    let value: serde_json::Value = serde_json::from_slice(json_bytes)
        .with_context(|| {
            // Save raw JSON to file for debugging
            if let Ok(dir) = std::env::var("KOMA_DEBUG_DIR") {
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(format!("{}/last_result.json", dir), json_bytes);
            }
            let preview_len = json_bytes.len().min(4000);
            let preview = String::from_utf8_lossy(&json_bytes[..preview_len]);
            let hex_tail: String = json_bytes[json_bytes.len().saturating_sub(50)..]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            format!("Failed to parse result JSON (len={}): {}\nHEX tail: {}", payload_len, preview, hex_tail)
        })?;
    Ok(value)
}

fn instantiate_source(wasm_path: &Path) -> Result<(Store<HostState>, Instance)> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)
        .with_context(|| format!("Failed to load WASM from {:?}", wasm_path))?;

    let mut store = Store::new(&engine, HostState::new());
    let mut linker = Linker::new(&engine);

    register_host_imports(&mut linker)?;

    let instance = linker.instantiate(&mut store, &module)
        .context("Failed to instantiate WASM module")?;

    Ok((store, instance))
}

fn register_host_imports(linker: &mut Linker<HostState>) -> Result<()> {
    // Existing source ABI: exact little-endian epoch milliseconds, no fallback.
    linker.func_wrap("koma_host", "unix_time_ms", |mut caller: Caller<'_, HostState>, out_ptr: i32, out_cap: i32| -> i32 {
        if out_cap != 8 { return -1; }
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(memory) => memory,
            None => return -1,
        };
        let start = out_ptr as u32 as usize;
        let end = match start.checked_add(8) {
            Some(end) if end <= memory.data_size(&caller) => end,
            _ => return -1,
        };
        let millis = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => match u64::try_from(duration.as_millis()) {
                Ok(millis) if millis != 0 => millis,
                _ => return -2,
            },
            Err(_) => return -2,
        };
        memory.data_mut(&mut caller)[start..end].copy_from_slice(&millis.to_le_bytes());
        8
    })?;

    // Use OS entropy, bounded by the same 256-byte ABI as the App host.
    linker.func_wrap("koma_host", "random_bytes", |mut caller: Caller<'_, HostState>, out_ptr: i32, out_cap: i32| -> i32 {
        if !(1..=256).contains(&out_cap) { return -1; }
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(memory) => memory,
            None => return -1,
        };
        let start = out_ptr as u32 as usize;
        let count = out_cap as usize;
        let end = match start.checked_add(count) {
            Some(end) if end <= memory.data_size(&caller) => end,
            _ => return -1,
        };
        let mut bytes = [0_u8; 256];
        if getrandom::getrandom(&mut bytes[..count]).is_err() { return -2; }
        memory.data_mut(&mut caller)[start..end].copy_from_slice(&bytes[..count]);
        out_cap
    })?;

    // koma_host.log
    linker.func_wrap("koma_host", "log", |mut caller: Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
        let memory = caller.get_export("memory").and_then(|e| e.into_memory());
        if let Some(memory) = memory {
            let data = memory.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end <= data.len() {
                let msg = String::from_utf8_lossy(&data[start..end]).to_string();
                caller.data_mut().logs.push(format!("[level={}] {}", level, msg));
            }
        }
    })?;

    // koma_host.check_cancel
    linker.func_wrap("koma_host", "check_cancel", |_caller: Caller<'_, HostState>| -> i32 {
        0
    })?;

    // koma_host.http_request
    linker.func_wrap("koma_host", "http_request", |mut caller: Caller<'_, HostState>, req_ptr: i32, req_len: i32, out_ptr: i32, out_cap: i32| -> i32 {
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return -8,
        };

        let data = memory.data(&caller);
        let start = req_ptr as usize;
        let end = start + req_len as usize;
        if end > data.len() { return -2; }
        let req_bytes = data[start..end].to_vec();

        let response_json = match encode_http_envelope(&req_bytes, |http_req, batch| {
            // Preserve existing ordinary-request logs. Batch children only emit
            // status, so opaque URLs and credentials are not newly logged.
            let guard = std::env::var_os("KOMA_COVERAGE_DENIAL_DIR").filter(|v| !v.is_empty());
            if !batch && guard.is_none() { eprintln!("[dev-runner] HTTP {} {}", http_req.method, http_req.url); }
            let response = guarded_http_request(http_req, guard.as_deref().map(Path::new), do_http_request);
            if let Some(status) = response.status { eprintln!("[dev-runner] HTTP status {}", status); }
            response
        }) {
            Ok(j) => j,
            Err(e) => { eprintln!("[dev-runner] failed to parse HTTP request: {}", e); return -2; }
        };

        if response_json.len() > out_cap as usize { return -3; }

        let out_start = out_ptr as usize;
        let out_end = out_start + response_json.len();
        let data = memory.data_mut(&mut caller);
        if out_end > data.len() { return -8; }
        data[out_start..out_end].copy_from_slice(&response_json);
        response_json.len() as i32
    })?;

    // DOM descriptors retain the original tree and node identity; closing its
    // document invalidates all descendants, as in the production host.
    linker.func_wrap("koma_host", "html_parse", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
        let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else { return -1; };
        if ptr < 0 || len <= 0 { return -1; }
        let Ok(bytes) = read_guest_bytes(&memory, &caller, ptr as u32, len as u32) else { return -1; };
        caller.data_mut().parse_html(&String::from_utf8_lossy(&bytes))
    })?;
    linker.func_wrap("koma_host", "html_select", |mut caller: Caller<'_, HostState>, id: i32, ptr: i32, len: i32| -> i32 {
        let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else { return -1; };
        if ptr < 0 || len <= 0 { return -1; }
        let Ok(bytes) = read_guest_bytes(&memory, &caller, ptr as u32, len as u32) else { return -1; };
        let Ok(selector) = scraper::Selector::parse(&String::from_utf8_lossy(&bytes)) else { return -1; };
        let Some((owner, ids)) = caller.data().select_ids(id, &selector) else { return -1; };
        match ids.first() { Some(node) => caller.data_mut().add_handle(HtmlHandle { owner, node: Some(*node) }), None => -1 }
    })?;
    linker.func_wrap("koma_host", "html_select_all", |mut caller: Caller<'_, HostState>, id: i32, ptr: i32, len: i32, out: i32, cap: i32| -> i32 {
        let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else { return -1; };
        if ptr < 0 || len <= 0 || out < 0 || cap < 0 { return -1; }
        let Ok(bytes) = read_guest_bytes(&memory, &caller, ptr as u32, len as u32) else { return -1; };
        let Ok(selector) = scraper::Selector::parse(&String::from_utf8_lossy(&bytes)) else { return -1; };
        let Some((owner, ids)) = caller.data().select_ids(id, &selector) else { return -1; };
        let count = ids.len(); let written = count.min(cap as usize / 4);
        if (out as usize).checked_add(written * 4).map_or(true, |end| end > memory.data_size(&caller)) { return -1; }
        for (i, node) in ids.into_iter().take(written).enumerate() {
            let desc = caller.data_mut().add_handle(HtmlHandle { owner, node: Some(node) });
            if write_guest_bytes(&memory, &mut caller, out as u32 + (i * 4) as u32, &desc.to_le_bytes()).is_err() { return -1; }
        }
        count as i32
    })?;
    linker.func_wrap("koma_host", "html_attr", |mut caller: Caller<'_, HostState>, id: i32, ptr: i32, len: i32, out: i32, cap: i32| -> i32 {
        let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else { return -1; };
        if ptr < 0 || len <= 0 || out < 0 || cap < 0 { return -1; }
        let Ok(bytes) = read_guest_bytes(&memory, &caller, ptr as u32, len as u32) else { return -1; };
        let Some(value) = caller.data().attr(id, &String::from_utf8_lossy(&bytes)) else { return -1; };
        let bytes = value.as_bytes(); let n = bytes.len().min(cap as usize);
        if write_guest_bytes(&memory, &mut caller, out as u32, &bytes[..n]).is_err() { return -1; } n as i32
    })?;
    linker.func_wrap("koma_host", "html_text", |mut caller: Caller<'_, HostState>, id: i32, out: i32, cap: i32| -> i32 {
        let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else { return -1; };
        if out < 0 || cap < 0 { return -1; }
        let Some(value) = caller.data().text(id) else { return -1; };
        let bytes = value.as_bytes(); let n = bytes.len().min(cap as usize);
        if write_guest_bytes(&memory, &mut caller, out as u32, &bytes[..n]).is_err() { return -1; } n as i32
    })?;
    linker.func_wrap("koma_host", "html_close", |mut caller: Caller<'_, HostState>, id: i32| -> i32 {
        if caller.data_mut().close(id) { 0 } else { -1 }
    })?;

    // koma_host.get_setting
    linker.func_wrap(
        "koma_host",
        "get_setting",
        |mut caller: Caller<'_, HostState>,
         key_ptr: i32,
         key_len: i32,
         out_ptr: i32,
         out_cap: i32|
         -> i32 {
            let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return -1,
            };
            let data = memory.data(&caller);
            let k_start = key_ptr as usize;
            let k_end = k_start + key_len as usize;
            if k_end > data.len() { return -1; }
            let key = String::from_utf8_lossy(&data[k_start..k_end]).to_string();

            let value = SETTINGS
                .lock()
                .ok()
                .and_then(|s| s.get(&key).cloned())
                .unwrap_or_default();

            let val_bytes = value.as_bytes();
            if val_bytes.len() > out_cap as usize { return -3; }

            let out_start = out_ptr as usize;
            let out_end = out_start + val_bytes.len();
            let mem = memory.data_mut(&mut caller);
            if out_end > mem.len() { return -1; }
            mem[out_start..out_end].copy_from_slice(val_bytes);
            val_bytes.len() as i32
        },
    )?;

    Ok(())
}

#[cfg(test)]
mod html_descriptor_tests {
    use super::*;

    #[test]
    fn unclosed_anchor_does_not_hide_selected_article_descendants() {
        let mut host = HostState::new();
        let doc = host.parse_html("<p><a href='social'><img></p><div id='area'><div id='box'><div class='article' id='article-1'><ul class='article-head'><li class='title' data-article-link='/1'><h2>Title</h2></li></ul><div class='article-content-inner'><img alt='#漫畫'></div></div></div></div>");
        let (owner, rows) = host.select_ids(doc, &scraper::Selector::parse("#area > #box > div.article[id]").unwrap()).unwrap();
        assert_eq!(rows.len(), 1);
        let row = host.add_handle(HtmlHandle { owner, node: Some(rows[0]) });
        assert_eq!(host.select_ids(row, &scraper::Selector::parse("ul.article-head > li.title[data-article-link]").unwrap()).unwrap().1.len(), 1);
        assert_eq!(host.select_ids(row, &scraper::Selector::parse("div.article-content-inner img").unwrap()).unwrap().1.len(), 1);
    }

    #[test]
    fn selected_nodes_keep_owner_scope_and_document_lifetime() {
        let mut host = HostState::new();
        let doc = host.parse_html("<div id='outside' title='outer'><article id='a'><ul><li title='inner'>X</li></ul></article><article id='b'></article></div>");
        let (owner, rows) = host.select_ids(doc, &scraper::Selector::parse("article").unwrap()).unwrap();
        assert_eq!(rows.len(), 2);
        let first = host.add_handle(HtmlHandle { owner, node: Some(rows[0]) });
        let second = host.add_handle(HtmlHandle { owner, node: Some(rows[1]) });
        assert_eq!(host.attr(first, "title"), None);
        let (_, children) = host.select_ids(first, &scraper::Selector::parse("ul > li[title]").unwrap()).unwrap();
        assert_eq!(children.len(), 1);
        let child = host.add_handle(HtmlHandle { owner, node: Some(children[0]) });
        assert_eq!(host.attr(child, "title").as_deref(), Some("inner"));
        assert_eq!(host.text(child).as_deref(), Some("X"));
        assert!(host.close(first));
        assert_eq!(host.text(child).as_deref(), Some("X"));
        assert!(host.close(doc));
        assert!(host.text(child).is_none());
        assert!(host.text(second).is_none());
        assert!(!host.close(child));
    }
}

#[cfg(test)]
mod coverage_batch_tests {
    use super::*;

    #[test]
    fn pica_dns_plan_is_rejected_before_any_http_in_this_non_rcp_host() {
        for value in [
            serde_json::json!({"type":"pica_channel_init","version":1}),
            serde_json::json!({"version":1,"method":"POST","url":"https://picaapi.picacomic.com/auth/sign-in",
                "dnsOverride":{"kind":"pica_channel_v1","proof":"fixture-only"}}),
            serde_json::json!({"requests":[{"version":1,"method":"GET","url":"https://picaapi.picacomic.com/",
                "dnsOverride":{"kind":"pica_channel_v1","proof":"fixture-only"}}]})
        ] {
            let output=encode_http_envelope(value.to_string().as_bytes(),|_,_|panic!("must not send without DNS support")).unwrap();
            let response:serde_json::Value=serde_json::from_slice(&output).unwrap();
            assert_eq!(response["error"]["code"],"unsupported");
            assert_eq!(response["networkPerformed"],false);
        }
    }

    fn result(status: u16) -> HttpResponse { HttpResponse { ok:true,status:Some(status),headers:None,body_text:Some(status.to_string()),body_json:None,final_url:None,network_performed:true,error:None } }
    #[test]
    fn serial_batch_preserves_order_and_partial_http_failure() {
        let mut seen=Vec::new();
        let input=br#"{"version":1,"requests":[{"method":"GET","url":"https://example.test/a"},{"method":"POST","url":"https://example.test/b"}]}"#;
        let output=encode_http_envelope(input,|r,batch| { assert!(batch);seen.push(r.url.clone()); result(if seen.len()==1 {200}else{403}) }).unwrap();
        let v:serde_json::Value=serde_json::from_slice(&output).unwrap();
        assert_eq!(seen,["https://example.test/a","https://example.test/b"]);
        assert_eq!(v["ok"],true);assert_eq!(v["responses"][0]["status"],200);assert_eq!(v["responses"][1]["status"],403);
    }
    #[test]
    fn reject_invalid_entire_group_before_sending() {
        for raw in [r#"{"requests":[]}"#,r#"{"requests":[{"method":"GET","url":"https://example.test"},{"requests":[]}]}"#,r#"{"requests":[{"method":"GET","url":"ftp://example.test"}]}"#] {
            let mut calls=0;let output=encode_http_envelope(raw.as_bytes(),|_,_|{calls+=1;result(200)}).unwrap();
            let v:serde_json::Value=serde_json::from_slice(&output).unwrap();assert_eq!(calls,0);assert_eq!(v["error"]["code"],"invalid_request");
        }
    }
    #[test]
    fn limits_and_transport_failure_remain_per_child() {
        let too_many=serde_json::json!({"requests":vec![serde_json::json!({"method":"GET","url":"https://example.test"});6]});
        let mut calls=0;let output=encode_http_envelope(too_many.to_string().as_bytes(),|_,_|{calls+=1;result(200)}).unwrap();
        let v:serde_json::Value=serde_json::from_slice(&output).unwrap();assert_eq!(calls,0);assert_eq!(v["error"]["code"],"invalid_request");
        let large=serde_json::json!({"requests":[{"method":"POST","url":"https://example.test","body":"x".repeat(1024*1024+1)}]});
        let output=encode_http_envelope(large.to_string().as_bytes(),|_,_|{panic!("must validate before sending")}).unwrap();
        let v:serde_json::Value=serde_json::from_slice(&output).unwrap();assert_eq!(v["error"]["code"],"request_too_large");
        let input=br#"{"requests":[{"method":"GET","url":"https://example.test/a"},{"method":"GET","url":"https://example.test/b"}]}"#;
        let mut n=0;let output=encode_http_envelope(input,|_,_|{n+=1;if n==1 { result(200) } else { HttpResponse {ok:false,status:None,headers:None,body_text:None,body_json:None,final_url:None,network_performed:true,error:Some(HttpErrorEnvelope {code:"network_error".into(),message:"failed".into(),phase:"transport".into(),retryable:true})} }}).unwrap();
        let v:serde_json::Value=serde_json::from_slice(&output).unwrap();assert_eq!(v["ok"],true);assert_eq!(v["responses"][0]["status"],200);assert_eq!(v["responses"][1]["error"]["code"],"network_error");
    }
    #[test]
    fn ordinary_body_aliases_match_app_and_guard_identity() {
        let cases = [
            (serde_json::json!({"body":"body only"}), Some("body only")),
            (serde_json::json!({"bodyBase64":"primary only"}), Some("primary only")),
            (serde_json::json!({"bodyBase64":"preferred","body":"ignored"}), Some("preferred")),
            (serde_json::json!({"bodyBase64":42,"body":"fallback"}), Some("fallback")),
            (serde_json::json!({"bodyBase64":null,"body":"fallback"}), Some("fallback")),
            (serde_json::json!({"bodyBase64":false,"body":{}}), None),
        ];
        for (fields, expected) in cases {
            let mut value = serde_json::json!({"version":1,"method":"POST","url":"https://example.test/body"});
            value.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
            let request: HttpRequest = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(request.effective_body(), expected);
            let reference: HttpRequest = serde_json::from_value(serde_json::json!({
                "version":1,"method":"POST","url":"https://example.test/body","bodyBase64":expected,
            })).unwrap();
            assert_eq!(coverage_request_key(&request).unwrap(), coverage_request_key(&reference).unwrap());
            let mut calls = 0;
            encode_http_envelope(value.to_string().as_bytes(), |sent, batch| {
                calls += 1;
                assert!(!batch);
                assert_eq!(sent.effective_body(), expected);
                result(200)
            }).unwrap();
            assert_eq!(calls,1);
        }
    }
    #[test]
    fn opt_in_denial_guard_persists_exact_identity_without_credentials() {
        let dir=std::env::temp_dir().join(format!("koma-coverage-guard-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir);
        let make=|method:&str,body:&str,token:&str| -> HttpRequest {
            serde_json::from_value(serde_json::json!({"version":1,"method":method,"url":"https://example.test/resource", "bodyBase64":body,"headers":{"Authorization":token}})).unwrap()
        };
        let first=make("GET","a","secret-a");let mut count=0;
        let r=guarded_http_request(&first,Some(&dir),|_|{count+=1;result(403)});assert_eq!(r.status,Some(403));
        let blocked=guarded_http_request(&first,Some(&dir),|_|{count+=1;result(200)});assert_eq!(count,1);assert!(!blocked.network_performed);assert_eq!(blocked.error.unwrap().code,"coverage_request_suppressed");
        for different in [make("POST","a","secret-a"),make("GET","b","secret-a"),make("GET","a","secret-b")] {
            let r=guarded_http_request(&different,Some(&dir),|_|{count+=1;result(200)});assert_eq!(r.status,Some(200));
        }
        assert_eq!(count,4);
        let ambiguous:HttpRequest=serde_json::from_value(serde_json::json!({"version":1,"method":"GET","url":"https://example.test/resource","headers":{"Authorization":"one","authorization":"two"}})).unwrap();
        assert_eq!(guarded_http_request(&ambiguous,Some(&dir),|_|panic!("ambiguous headers must not be sent")).error.unwrap().code,"coverage_guard_unavailable");
        let bad400=make("GET","c","secret-a");for _ in 0..2 {guarded_http_request(&bad400,Some(&dir),|_|{count+=1;result(400)});}assert_eq!(count,6);
        for (status, body) in [(401,"d"),(429,"e")] {
            let denied=make("GET",body,"secret-a");
            assert_eq!(guarded_http_request(&denied,Some(&dir),|_|result(status)).status,Some(status));
            assert_eq!(guarded_http_request(&denied,Some(&dir),|_|panic!("denial must be cached")).error.unwrap().code,"coverage_request_suppressed");
        }
        for _ in 0..2 {guarded_http_request(&first,None,|_|{count+=1;result(403)});}assert_eq!(count,8);
        let files:Vec<_>=std::fs::read_dir(&dir).unwrap().collect();assert_eq!(files.len(),3);
        let entry=files[0].as_ref().unwrap();assert_eq!(entry.file_name().to_string_lossy().len(),64);assert_eq!(std::fs::read(entry.path()).unwrap(),b"");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn ordinary_request_keeps_existing_transport_shape() {
        let mut calls=0;let input=br#"{"version":1,"method":"GET","url":"https://example.test/a"}"#;
        let output=encode_http_envelope(input,|r,batch|{calls+=1;assert!(!batch);assert_eq!(r.url,"https://example.test/a");result(200)}).unwrap();
        let v:serde_json::Value=serde_json::from_slice(&output).unwrap();assert_eq!(calls,1);assert_eq!(v["ok"],true);assert_eq!(v["status"],200);assert!(v.get("responses").is_none());
    }
}

#[cfg(test)]
mod response_body_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn response(status: u16, declared: usize, body: &'static [u8]) -> HttpResponse {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request);
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n").unwrap();
            stream.write_all(body).unwrap();
        });
        let request: HttpRequest = serde_json::from_value(serde_json::json!({"version":1, "method":"GET", "url":format!("http://{address}/")})).unwrap();
        let result = do_http_request(&request);
        server.join().unwrap();
        result
    }

    #[test]
    fn complete_body_is_preserved() {
        let result = response(200, 5, b"hello");
        assert!(result.ok);
        assert_eq!(result.status, Some(200));
        assert_eq!(result.body_text.as_deref(), Some("hello"));
        assert!(result.error.is_none());
    }

    #[test]
    fn truncated_body_is_a_network_failure_with_status_preserved() {
        for status in [200, 403, 429] {
            let result = response(status, 64, b"short");
            assert!(!result.ok);
            assert_eq!(result.status, Some(status));
            assert!(result.network_performed);
            assert!(result.body_text.is_none());
            let error = result.error.unwrap();
            assert_eq!(error.code, "network_error");
            assert_eq!(error.phase, "response_body");
        }
    }
}
