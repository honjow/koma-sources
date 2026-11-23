// SPDX-License-Identifier: Apache-2.0
//! Raw-document BodyRef delivery; canonical Web vs existing Native, not images.
use alloc::vec::Vec;
use koma_source_sdk::{
    body::{BodyRef, CHUNK_BYTES},
    http::{CallError, Client, Header, Request, RequestSpec, ResponseKind},
    runtime::Context,
    source_protocol,
};
use url::Url;
use super::{body_buf, http_out, http_req_buf, Node, Result, BASE, DOCUMENT_ACCEPT, LANGUAGE};

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
    let canonical_web = context.supports(b"source_web_http_v1")
        && url.path().starts_with("/manga/") && url.path().ends_with('/')
        && url.query_pairs().all(|(key, value)| key == "style" && value == "list");
    // Same canonical selection and public Web headers as R10. Browser fetch owns
    // its request metadata: never send Native navigation Sec/UIC/Priority here.
    let web_headers = [
        Header { name: b"Accept", value: b"*/*" },
        Header { name: b"Accept-Language", value: LANGUAGE.as_bytes() },
        Header { name: b"Referer", value: b"https://bakamh.com/" },
    ];
    let mut spec = RequestSpec::get(url.as_str().as_bytes());
    // BrowserGrant is the SDK default. Do not substitute a source session, image
    // purpose or final URL; the Host keeps admission and redirect guards.
    spec.headers = if canonical_web { &web_headers } else { &headers };
    spec.response = ResponseKind::BodyRef;
    spec.response_headers = context.supports(b"source_http_headers_v1");
    let request_capacity = http_req_buf().len();
    let encoded = spec.encode(http_req_buf())?;
    let mut web_request = Vec::new();
    let encoded = if canonical_web {
        let prefix = encoded.as_json().strip_suffix(b"}").ok_or("internal_error")?;
        let extension = br#","webFetch":{"version":1}}"#;
        let length = prefix.len().checked_add(extension.len())
            .filter(|length| *length <= request_capacity).ok_or("buffer_overflow")?;
        web_request.reserve(length);
        web_request.extend_from_slice(prefix);
        web_request.extend_from_slice(extension);
        Request::from_json(&web_request)
    } else { encoded };
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
    let input = response.body_ref();
    // The ID now owns its data. Body read/close RPCs may reuse HTTP_OUT only
    // after this response borrow ends.
    drop(response);
    let input = match input {
        Ok(input) => input,
        Err(_) => return Err(error.unwrap_or("invalid_response")),
    };
    let budget = body_buf().len().min(4 * 1024 * 1024)
        .min(context.limit(b"body", b"maxBytes").unwrap_or(4 * 1024 * 1024));
    let chunk = CHUNK_BYTES.min(context.limit(b"body", b"chunkBytes").unwrap_or(CHUNK_BYTES));
    let parsed = (|| -> Result<(Node, bool)> {
        if let Some(code) = error {
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
