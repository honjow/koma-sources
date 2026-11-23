// SPDX-License-Identifier: Apache-2.0
//! Provider documents consume their actual effective URL, like upstream document.location.
use koma_source_sdk::{
    http::{CallError, Client, Header, RequestSpec},
    runtime::Context,
    source_protocol,
};
use url::Url;
use super::{body_buf, http_out, http_req_buf, manga_path, numeric_id, path_segment, post_id, shortlink_id, Node, Result,
    BASE, DOCUMENT_ACCEPT, LANGUAGE};

pub(super) struct WorkDocument {
    pub(super) doc: Node,
    pub(super) complete: bool,
    pub(super) final_url: Option<Url>,
}

pub(super) fn target(url: &Url) -> bool {
    url.origin().ascii_serialization() == BASE
        && (manga_path(url.as_str()).is_some()
            || (url.path() == "/" && url.query_pairs().count() == 1 && shortlink_id(url).is_some()))
}

fn read(url: &Url, request: &[u8]) -> Result<WorkDocument> {
    let context = Context::from_request(request);
    if context.cancelled() { return Err("cancelled"); }
    if url.origin().ascii_serialization() != BASE { return Err("invalid_request"); }
    // Native DOCUMENT public headers only. The Host retains the authorized
    // browser UA/Cookie and owns redirects, admission and profile selection.
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
    let mut spec = RequestSpec::get(url.as_str().as_bytes());
    // Text + BrowserGrant are the existing defaults. This metadata has a real
    // consumer below; it is not an invented switch or a fabricated final URL.
    spec.headers = &headers;
    spec.final_url = true;
    spec.response_headers = context.supports(b"source_http_headers_v1");
    let response = Client::<CallError>::new(&[]).send(spec.encode(http_req_buf())?, http_out())
        .map_err(|_| if context.cancelled() { "cancelled" } else { "network_error" })?;
    if context.cancelled() { return Err("cancelled"); }
    // Classify the actual HTTP/accessSignal failure first, before requiring URL
    // metadata. A missing URL cannot replace a403/challenge with a parse error.
    let length = source_protocol::body(response.as_json(), body_buf())?;
    let mut final_url = [0u8; 8192];
    let final_url = response.final_url(&mut final_url)
        .map_err(|_| "invalid_response")?.ok_or("invalid_response")?;
    let final_url = core::str::from_utf8(final_url).map_err(|_| "invalid_response")?;
    let final_url = Url::parse(final_url).map_err(|_| "invalid_response")?;
    if final_url.origin().ascii_serialization() != BASE
        || !final_url.username().is_empty() || final_url.password().is_some()
        || final_url.fragment().is_some()
    { return Err("invalid_response"); }
    let html = core::str::from_utf8(&body_buf()[..length]).map_err(|_| "parse_error")?;
    let body = html.as_bytes().windows(7).rposition(|part| part.eq_ignore_ascii_case(b"</body>"));
    let end = html.as_bytes().windows(7).rposition(|part| part.eq_ignore_ascii_case(b"</html>"));
    let complete = matches!((body, end), (Some(body), Some(end)) if body < end);
    let doc = Node::parse(html)?;
    Ok(WorkDocument { doc, complete, final_url: Some(final_url) })
}

/// Upstream fetchFilterData is a DOCUMENT navigation, not an in-page fetch.
/// Keep archive validation separate from work/reader identity: this response can
/// supply public genre options, but can never become a work or chapter memo.
pub(super) fn catalogue(url: &Url, request: &[u8]) -> Result<WorkDocument> {
    let context = Context::from_request(request);
    if context.cancelled() { return Err("cancelled"); }
    if url.origin().ascii_serialization() != BASE || url.path() != "/manga/"
        || url.query().is_some() || url.fragment().is_some()
    { return Err("invalid_request"); }
    if !context.supports(b"source_http_response_metadata_v1") { return Err("unsupported"); }
    let document = read(url, request)?;
    let final_url = document.final_url.as_ref().ok_or("invalid_response")?;
    if final_url.path().trim_end_matches('/') != "/manga" || final_url.query() != url.query() {
        return Err("invalid_response");
    }
    if context.cancelled() { return Err("cancelled"); }
    Ok(document)
}

fn reader_path(url: &Url) -> bool {
    url.path().strip_prefix("/manga/").is_some_and(|path| {
        let mut segments = path.trim_end_matches('/').split('/');
        segments.next().is_some_and(path_segment)
            && segments.next().is_some_and(path_segment) && segments.next().is_none()
    })
}

pub(super) fn run(url: &Url, request: &[u8], reader_url_search: bool) -> Result<WorkDocument> {
    if Context::from_request(request).cancelled() { return Err("cancelled"); }
    if !target(url) { return Err("invalid_request"); }
    let document = read(url, request)?;
    let final_url = document.final_url.as_ref().ok_or("invalid_response")?;
    let same_numeric = url.path() == "/" && final_url.path() == "/"
        && final_url.query_pairs().count() == 1
        && shortlink_id(url).is_some() && shortlink_id(final_url) == shortlink_id(url);
    // URL search retains its existing owner lookup. A reader cannot become the
    // canonical memo of a work merely because it has the same origin.
    let reader = reader_url_search && reader_path(final_url)
        && document.doc.one("body.reading-manga").is_some() && post_id(&document.doc).is_some();
    if !same_numeric && manga_path(final_url.as_str()).is_none() && !reader {
        return Err("invalid_response");
    }
    Ok(document)
}

/// The caller built this route from the numeric owner and its work/chapter path.
/// A redirect must not substitute another work, chapter, login or reader mode.
/// Keep stable page IDs separate; the confirmed location drives resources and
/// document Referer, just as upstream Response.asJsoup()/document.location().
pub(super) fn reader(url: &Url, request: &[u8], owner: &str) -> Result<WorkDocument> {
    if Context::from_request(request).cancelled() { return Err("cancelled"); }
    if url.origin().ascii_serialization() != BASE || !reader_path(url) || !numeric_id(owner) {
        return Err("invalid_request");
    }
    let document = read(url, request)?;
    let final_url = document.final_url.as_ref().ok_or("invalid_response")?;
    if !reader_path(final_url) || final_url.path() != url.path() || final_url.query() != url.query()
        || post_id(&document.doc).is_some_and(|id| id != owner)
    {
        return Err("invalid_response");
    }
    Ok(document)
}
