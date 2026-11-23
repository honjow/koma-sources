// SPDX-License-Identifier: Apache-2.0
//! TEMPORARY parent-confirmed local evidence for the designated anonymous403 run.
//! Static HTML text only, not CSS-computed visibility. Remove after capture.
use alloc::{collections::BTreeSet, format, string::String};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{host, source_protocol};
use url::Url;

const MAX_HTML: usize = 8192;
const MAX_TEXT: usize = 1024;
const MAX_NODES: usize = 8;

// Existing HTML ABI, already used by this source. Only the returned descriptors
// fitting this bounded output array are allocated; all are explicitly retired.
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(parent: i32, selector: *const u8, selector_len: u32, out: *mut u8, cap: u32) -> i32;
}
struct Html(host::HtmlDescriptor);
impl Drop for Html {
    fn drop(&mut self) { let _ = host::html_close(self.0); }
}

pub(super) fn allowed(url: &Url, request: &[u8]) -> bool {
    // series() has one canonical get_manga leg; its numeric fallback is excluded.
    // No session/account material is inspected to implement this local diagnostic.
    url.origin().ascii_serialization() == super::BASE
        && url.path().starts_with("/manga/") && url.path().ends_with('/')
        && url.query().is_none()
        && source_protocol::raw_string(request, b"operation").ok().flatten() == Some(b"get_manga")
}

fn omit_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if text.contains(['/', '\\', '@']) || lower.contains("www.") || lower.contains("://") {
        return true;
    }
    if ["cookie", "authorization", "bearer", "password", "passwd", "secret", "credential",
        "csrf", "nonce", "token", "sessionid", "session_id", "api_key", "api-key", "apikey"]
        .iter().any(|marker| lower.contains(marker)) { return true; }
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c: char| c.is_ascii_punctuation() && !matches!(c, '.' | '-' | '_' | '+' | '='));
        let mut previous = false;
        for label in word.split('.') {
            let letters = label.bytes().any(|byte| byte.is_ascii_alphabetic());
            if previous && letters { return true; }
            previous = letters;
        }
        if word.len() >= 16 && word.bytes().all(|byte| byte.is_ascii_hexdigit()) { return true; }
        if word.len() >= 20 && word.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_+=".contains(&byte))
            && word.bytes().any(|byte| byte.is_ascii_alphabetic())
            && word.bytes().any(|byte| byte.is_ascii_digit()) { return true; }
        if word.len() >= 24 && word.bytes().any(|byte| byte.is_ascii_uppercase())
            && word.bytes().any(|byte| byte.is_ascii_lowercase()) { return true; }
    }
    false
}

fn eligible_node(node: &Html) -> bool {
    // Descendant scripts, style and form controls can occur inside a paragraph.
    // Omit the whole candidate rather than reading or logging those values.
    if let Ok(child) = host::html_select(node.0, b"script,style,input,textarea,select,button,form") {
        drop(Html(child));
        return false;
    }
    let mut style = [0; 256];
    if let Ok(length) = host::html_attr(node.0, b"style", &mut style) {
        if length == style.len() { return false; }
        let mut compact = String::new();
        for byte in &style[..length] {
            if !byte.is_ascii_whitespace() { compact.push(byte.to_ascii_lowercase() as char); }
        }
        if compact.contains("display:none") || compact.contains("visibility:hidden") { return false; }
    }
    true
}

pub(super) fn capture(html: &[u8]) {
    if html.is_empty() || html.len() > MAX_HTML || core::str::from_utf8(html).is_err() { return; }
    let Ok(document) = host::html_parse(html) else { return; };
    let document = Html(document);
    let mut output = String::with_capacity(MAX_TEXT);
    let mut seen = BTreeSet::new();
    let mut visited = 0;
    let mut text = [0; MAX_HTML];
    for tag in ["title", "h1", "h2", "p"] {
        let selector = format!("{tag}:not(form *):not(script *):not(style *):not([hidden]):not([hidden] *):not([aria-hidden='true']):not([aria-hidden='true'] *)");
        let mut selected = [0u8; MAX_NODES * 4];
        let count = unsafe { select_all(document.0.raw(), selector.as_ptr(), selector.len() as u32,
            selected.as_mut_ptr(), selected.len() as u32) };
        if count < 0 { continue; }
        for index in 0..(count as usize).min(MAX_NODES) {
            let offset = index * 4;
            let id = i32::from_le_bytes(selected[offset..offset + 4].try_into().unwrap_or([0; 4]));
            if id <= 0 { continue; }
            let node = Html(host::HtmlDescriptor::from_raw(id));
            // Even after the extraction budget is exhausted, close every handle
            // written by SelectAll before retiring the document.
            if visited >= MAX_NODES { continue; }
            visited += 1;
            if !eligible_node(&node) { continue; }
            let Ok(length) = host::html_text(node.0, &mut text) else { continue; };
            if length == text.len() { continue; }
            let Ok(value) = core::str::from_utf8(&text[..length]) else { continue; };
            let mut value = value.split_whitespace().fold(String::new(), |mut out, word| {
                if !out.is_empty() { out.push(' '); }
                out.push_str(word); out
            });
            if value.is_empty() || omit_text(&value) || seen.contains(&value) { continue; }
            let remaining = MAX_TEXT.saturating_sub(output.len() + tag.len() + 3);
            if remaining == 0 { continue; }
            let mut end = value.len().min(remaining);
            while !value.is_char_boundary(end) { end -= 1; }
            value.truncate(end);
            if value.trim().is_empty() { continue; }
            seen.insert(value.clone());
            output.push_str(tag); output.push_str(": "); output.push_str(&value); output.push('\n');
        }
        if visited >= MAX_NODES { break; }
    }
    if output.is_empty() { return; }
    // Aggregate UTF-8 is<=1KiB. Chunks may split a code point: concatenate decoded
    // bytes in sequence before UTF-8 decoding locally. Base64 is not redaction.
    let parts = output.len().div_ceil(256);
    for (index, bytes) in output.as_bytes().chunks(256).enumerate() {
        let message = format!("BKERR403 v=1 part={}/{} total_bytes={} b64={}",
            index + 1, parts, output.len(), STANDARD.encode(bytes));
        host::log_info(message.as_bytes());
    }
}
