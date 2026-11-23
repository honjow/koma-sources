// SPDX-License-Identifier: Apache-2.0
//! Native raw-document delivery through existing generic HTTP BodyRef, not images.
use koma_source_sdk::{
    body::{BodyRef, CHUNK_BYTES},
    http::{CallError, Client, Header, RequestSpec, ResponseKind},
    runtime::Context,
    source_protocol,
};
use url::Url;
use super::{body_buf, http_out, http_req_buf, DocumentDiagnostic, Node, Result, BASE, DOCUMENT_ACCEPT, LANGUAGE};

fn read_prefix(input: &BodyRef, length: usize, chunk: usize, context: &Context<'_>) -> Result<()> {
    let mut offset = 0;
    while offset < length {
        if context.cancelled() { return Err("cancelled"); }
        let count = chunk.min(length - offset);
        let read = input.read(offset, &mut body_buf()[offset..offset + count], http_out())?;
        // The immutable reference advertises an exact length; an early short read
        // cannot establish a complete document and must not silently pass parsing.
        if read != count { return Err("invalid_response"); }
        offset += read;
    }
    Ok(())
}

fn log_failure(input: &BodyRef, facts: DocumentDiagnostic, budget: usize,
    chunk: usize, context: &Context<'_>, capture_allowed: bool) {
    if input.len() > budget || chunk == 0 || context.cancelled() {
        facts.log("unknown", -1, -1, true, &[], false);
        return;
    }
    let length = input.len().min(CHUNK_BYTES);
    if read_prefix(input, length, chunk, context).is_err() {
        facts.log("unknown", -1, -1, true, &[], false);
        return;
    }
    let head = &body_buf()[..length];
    // A truncated prefix may split UTF-8: its ASCII marker scan is still bounded
    // raw data, but only a complete validated document gets an exact UTF-8 length.
    let complete = input.len() == length;
    if complete && core::str::from_utf8(head).is_err() {
        facts.log("unknown", -1, length as i64, true, &[], false);
    } else {
        facts.log(if complete { "ok" } else { "prefix" },
            if complete { length as i64 } else { -1 }, length as i64, !complete, head, true);
        if capture_allowed && complete && length <= 8192 && matches!(facts.mime, "html" | "xhtml") {
            super::error_capture::capture(head);
        }
    }
}

pub(super) fn run(url: &Url, request: &[u8]) -> Result<(Node, bool)> {
    let context = Context::from_request(request);
    if context.cancelled() { return Err("cancelled"); }
    // Fixed BrowserHeaders DOCUMENT public values. UA/Cookie stay Host-owned.
    // These Sec-* headers are never inserted into Browser fetch or image lanes.
    let headers = [
        Header { name: b"Accept", value: DOCUMENT_ACCEPT.as_bytes() },
        Header { name: b"Accept-Language", value: LANGUAGE.as_bytes() },
        Header { name: b"Referer", value: b"https://bakamh.com/" },
        Header { name: b"Sec-Fetch-Mode", value: b"navigate" },
        Header { name: b"Sec-Fetch-Dest", value: b"document" },
        Header { name: b"Sec-Fetch-Site", value: b"same-origin" },
        Header { name: b"Sec-Fetch-User", value: b"?1" },
        Header { name: b"Upgrade-Insecure-Requests", value: b"1" },
        Header { name: b"Priority", value: b"u=0, i" },
    ];
    if url.origin().ascii_serialization() != BASE { return Err("invalid_request"); }
    let mut spec = RequestSpec::get(url.as_str().as_bytes());
    // BrowserGrant is the SDK default. Do not substitute a source session, image
    // purpose, final URL or Web fetch; the Host keeps admission and redirect guards.
    spec.headers = &headers;
    spec.response = ResponseKind::BodyRef;
    spec.response_headers = context.supports(b"source_http_headers_v1");
    let encoded = spec.encode(http_req_buf())?;
    let response = Client::<CallError>::new(&[]).send(encoded, http_out())
        .map_err(|_| "network_error")?;
    let status = response.status();
    // Reuse the existing strict status/accessSignal policy for failures. Never use
    // decode-body success on a BodyRef envelope, which intentionally has no text.
    let success = status.is_some_and(|status| (200..300).contains(&status))
        && source_protocol::direct(response.as_json(), b"ok").ok().flatten() == Some(b"true");
    let error = if context.cancelled() { Some("cancelled") } else if success { None } else {
        Some(source_protocol::body(response.as_json(), &mut []).err().unwrap_or("network_error"))
    };
    let facts = (status == Some(403)).then(|| DocumentDiagnostic::capture(response.as_json()));
    let capture_allowed = error == Some("forbidden") && super::error_capture::allowed(url, request)
        && source_protocol::direct(response.as_json(), b"ok").ok().flatten() == Some(b"true");
    let input = response.body_ref();
    // Both ID and public classifications now own their data. Body read/close RPCs
    // may reuse HTTP_OUT only after this response borrow ends.
    drop(response);
    let input = match input {
        Ok(input) => input,
        Err(_) => {
            if let Some(facts) = facts { facts.log("missing", -1, -1, true, &[], false); }
            return Err(error.unwrap_or("invalid_response"));
        }
    };
    let budget = body_buf().len().min(4 * 1024 * 1024)
        .min(context.limit(b"body", b"maxBytes").unwrap_or(4 * 1024 * 1024));
    let chunk = CHUNK_BYTES.min(context.limit(b"body", b"chunkBytes").unwrap_or(CHUNK_BYTES));
    let parsed = (|| -> Result<(Node, bool)> {
        if let Some(code) = error {
            if let Some(facts) = facts { log_failure(&input, facts, budget, chunk, &context, capture_allowed); }
            return Err(code);
        }
        if input.len() > budget || chunk == 0 { return Err("body_limit_exceeded"); }
        if input.is_empty() { return Err("parse_error"); }
        read_prefix(&input, input.len(), chunk, &context)?;
        if context.cancelled() { return Err("cancelled"); }
        let html = core::str::from_utf8(&body_buf()[..input.len()]).map_err(|_| "parse_error")?;
        // Match the existing complete-document rule without allocating a full
        // lowercased copy or a second source-side HTML String.
        let body = html.as_bytes().windows(7).rposition(|part| part.eq_ignore_ascii_case(b"</body>"));
        let end = html.as_bytes().windows(7).rposition(|part| part.eq_ignore_ascii_case(b"</html>"));
        let complete = matches!((body, end), (Some(body), Some(end)) if body < end);
        Ok((Node::parse(html)?, complete))
    })();
    // BodyRef has no Drop: close on every acquired-handle path. A cleanup failure
    // must not replace the original HTTP/read/parse/cancel error. Host call scope
    // remains the final fallback for malformed handles or failed cleanup RPCs.
    let closed = input.close();
    match (parsed, closed) {
        (Err(code), _) => Err(code),
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(code)) => Err(code),
    }
}
