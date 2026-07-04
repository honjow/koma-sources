#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
    HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, fetch_error_code, parse_status_code};

#[link(wasm_import_module = "koma_host")]
unsafe extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(
        descriptor: i32,
        selector_ptr: *const u8,
        selector_len: u32,
        out_ptr: *mut u8,
        out_cap: u32,
    ) -> i32;
}

fn html_select_all(descriptor: i32, selector: &[u8], out: &mut [u8]) -> i32 {
    unsafe {
        koma_host_html_select_all(
            descriptor,
            selector.as_ptr(),
            selector.len() as u32,
            out.as_mut_ptr(),
            out.len() as u32,
        )
    }
}

const DEFAULT_BASE_URL: &[u8] = b"http://www.zerobywgbo2.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
const SELECT_CAP: usize = 4096 * 4;

static mut SELECT_BUF: [u8; SELECT_CAP] = [0; SELECT_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.zerobyw.koma",
    name: "Zerobyw",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Zerobyw HTML scraping source.",
    content_rating: "nsfw",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: true,
    image_request: true,
    credentials: false,
};


struct OwnedDescriptor(HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn trim_trailing_slash(bytes: &[u8]) -> &[u8] {
    if bytes.len() > 1 && bytes[bytes.len() - 1] == b'/' {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    }
}

fn current_base_url<'a>(out: &'a mut [u8]) -> &'a [u8] {
    match host::get_setting(b"baseUrl", out) {
        Some(v) => {
            let v = trim_trailing_slash(trim_ascii(v));
            if v.starts_with(b"http://") || v.starts_with(b"https://") {
                v
            } else {
                DEFAULT_BASE_URL
            }
        }
        None => DEFAULT_BASE_URL,
    }
}

fn base_referer<'a>(base: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut c = 0usize;
    write_bytes(out, &mut c, base).then_some(())?;
    write_bytes(out, &mut c, b"/").then_some(())?;
    Some(&out[..c])
}

fn attr_into<'a>(desc: HtmlDescriptor, name: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_attr(desc, name, out).ok()?;
    Some(trim_ascii(&out[..len]))
}

fn text_into<'a>(desc: HtmlDescriptor, out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_text(desc, out).ok()?;
    Some(trim_ascii(&out[..len]))
}

fn select_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(SELECT_BUF) }
}

fn desc_from_i32(raw: i32) -> HtmlDescriptor {
    HtmlDescriptor::from_raw(raw)
}

fn fetch_html(url: &[u8], referer: Option<&[u8]>) -> Result<usize, FetchError> {
    let req_len =
        build_get_request(http_req_buf(), url, referer, &[]).ok_or(FetchError::Network)?;
    let req = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_REQ_BUF) as *const u8, req_len)
    };
    let len = http_request(req, http_out()).map_err(|_| FetchError::Network)?;
    let resp =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT) as *const u8, len) };
    decode_json_body(resp)
}

fn make_absolute_url<'a>(base: &[u8], url: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    if url.starts_with(b"http://") || url.starts_with(b"https://") {
        if url.len() > out.len() {
            return None;
        }
        out[..url.len()].copy_from_slice(url);
        return Some(&out[..url.len()]);
    }
    let mut c = 0usize;
    if url.starts_with(b"//") {
        write_bytes(out, &mut c, b"https:").then_some(())?;
        write_bytes(out, &mut c, url).then_some(())?;
    } else if url.starts_with(b"/") {
        write_bytes(out, &mut c, base).then_some(())?;
        write_bytes(out, &mut c, url).then_some(())?;
    } else {
        write_bytes(out, &mut c, base).then_some(())?;
        write_bytes(out, &mut c, b"/").then_some(())?;
        write_bytes(out, &mut c, url).then_some(())?;
    }
    Some(&out[..c])
}

fn id_from_comic_path<'a>(href: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let marker = b"/comic/";
    let start = find_subslice(href, marker)? + marker.len();
    let mut end = start;
    while end < href.len() {
        let b = href[end];
        if b == b'/' || b == b'?' || b == b'#' || b == b'"' || b == b'\'' || b.is_ascii_whitespace()
        {
            break;
        }
        end += 1;
    }
    if end == start || end - start > out.len() {
        return None;
    }
    out[..end - start].copy_from_slice(&href[start..end]);
    Some(&out[..end - start])
}

fn clean_title(title: &[u8]) -> &[u8] {
    let mut i = 0usize;
    while i + 2 < title.len() {
        if title[i] == 0xE3 && title[i + 1] == 0x80 && title[i + 2] == 0x90 {
            return trim_ascii(&title[..i]);
        }
        i += 1;
    }
    trim_ascii(title)
}

fn is_image_url(url: &[u8]) -> bool {
    let lower_ext = |ext: &[u8]| {
        url.len() >= ext.len()
            && url[url.len() - ext.len()..]
                .iter()
                .zip(ext.iter())
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
    };
    lower_ext(b".jpg")
        || lower_ext(b".jpeg")
        || lower_ext(b".png")
        || lower_ext(b".webp")
        || lower_ext(b".gif")
        || contains_bytes(url, b".jpg?")
        || contains_bytes(url, b".jpeg?")
        || contains_bytes(url, b".png?")
        || contains_bytes(url, b".webp?")
}

fn normalize_chapter_path<'a>(base: &[u8], href: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let start = if href.starts_with(base) {
        base.len()
    } else {
        0
    };
    let mut end = start;
    while end < href.len() {
        let b = href[end];
        if b == b'#' || b == b'"' || b == b'\'' || b.is_ascii_whitespace() {
            break;
        }
        end += 1;
    }
    if end == start || end - start > out.len() {
        return None;
    }
    out[..end - start].copy_from_slice(&href[start..end]);
    Some(&out[..end - start])
}

fn write_manga_item(
    payload: &mut [u8],
    c: &mut usize,
    id: &[u8],
    title: &[u8],
    cover: Option<&[u8]>,
) -> bool {
    let mut ok = write_bytes(payload, c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, clean_title(title))
        && write_bytes(payload, c, br#"","cover":"#);
    ok = ok
        && if let Some(url) = cover {
            write_bytes(payload, c, br#"{"kind":"url","url":""#)
                && append_json_escaped(payload, c, url)
                && write_bytes(payload, c, br#""}"#)
        } else {
            write_bytes(payload, c, br#"{"kind":"none"}"#)
        };
    ok && write_bytes(
        payload,
        c,
        br#","authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["zerobyw"]}"#,
    )
}

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => trim_ascii(q),
        None => return write_error("search", "invalid_request", "missing query"),
    };
    let page = extract_json_number(req, b"page").unwrap_or(b"1");
    let mut base_buf = [0u8; 512];
    let base = current_base_url(&mut base_buf);
    let mut referer_buf = [0u8; 512];
    let referer = match base_referer(base, &mut referer_buf) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "referer overflow"),
    };

    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url_buf, &mut uc, base)
        && write_bytes(url_buf, &mut uc, b"/comic/search?query=")
        && write_url_encoded(url_buf, &mut uc, query)
        && write_bytes(url_buf, &mut uc, b"&page=")
        && write_bytes(url_buf, &mut uc, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };
    let html_len = match fetch_html(url, Some(referer)) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, html_len)
    };
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("search", "parse_error", "html_parse failed"),
    };

    let selects = select_buf();
    let count = html_select_all(
        doc.0.raw(),
        b"a.uk-card, div.uk-card, a[href*=\"/comic/\"]",
        selects,
    );
    let max = if count > 0 { count as usize } else { 0 }.min(200);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in 0..max {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let hd = desc_from_i32(raw);
        let mut href_buf = [0u8; 512];
        let mut title_buf = [0u8; 512];
        let mut id_buf = [0u8; 256];
        let href = attr_into(hd, b"href", &mut href_buf);
        let id = href.and_then(|h| id_from_comic_path(h, &mut id_buf));
        let title_len = html_attr(hd, b"title", &mut title_buf)
            .ok()
            .filter(|n| *n > 0)
            .or_else(|| html_text(hd, &mut title_buf).ok());

        let mut cover_buf = [0u8; 768];
        let cover_len = html_select(hd, b"img").ok().and_then(|img| {
            html_attr(img, b"data-src", &mut cover_buf)
                .ok()
                .filter(|n| *n > 0)
                .or_else(|| html_attr(img, b"src", &mut cover_buf).ok())
        });

        if let (Some(id), Some(title_len)) = (id, title_len) {
            let title = trim_ascii(&title_buf[..title_len]);
            if !title.is_empty() {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    return write_error("search", "internal_error", "payload overflow");
                }
                let mut cover_abs = [0u8; 768];
                let cover_abs = cover_len.and_then(|n| {
                    make_absolute_url(base, trim_ascii(&cover_buf[..n]), &mut cover_abs)
                });
                if !write_manga_item(payload, &mut c, id, title, cover_abs) {
                    return write_error("search", "internal_error", "payload overflow");
                }
                written += 1;
            }
        }
        let _ = html_close(hd);
    }

    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("search", "internal_error", "payload overflow");
    }
    write_success_payload("search", c)
}

fn manga_id_from_request<'a>(req: &'a [u8], op: &str) -> Result<&'a [u8], u32> {
    let manga_id = extract_json_string(req, b"mangaId")
        .or_else(|| extract_json_string(req, b"id"))
        .ok_or_else(|| write_error(op, "invalid_request", "missing mangaId"))?;
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return Err(write_error(op, "invalid_request", "unexpected mangaId"));
    }
    Ok(&manga_id[prefix.len()..])
}

fn fetch_detail_doc<'a>(
    id: &[u8],
    op: &str,
    base_buf: &'a mut [u8],
    referer_buf: &'a mut [u8],
) -> Result<(usize, &'a [u8], OwnedDescriptor), u32> {
    let base = current_base_url(base_buf);
    let referer = base_referer(base, referer_buf)
        .ok_or_else(|| write_error(op, "internal_error", "referer overflow"))?;
    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url_buf, &mut uc, base)
        && write_bytes(url_buf, &mut uc, b"/comic/")
        && write_bytes(url_buf, &mut uc, id))
    {
        return Err(write_error(op, "internal_error", "url overflow"));
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };
    let html_len = fetch_html(url, Some(referer)).map_err(|e| {
        let (c, m) = fetch_error_code(e);
        write_error(op, c, m)
    })?;
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, html_len)
    };
    let doc = html_parse(html).map_err(|_| write_error(op, "parse_error", "html_parse failed"))?;
    Ok((html_len, base, OwnedDescriptor(doc)))
}

fn first_text(doc: HtmlDescriptor, selectors: &[&[u8]], out: &mut [u8]) -> Option<usize> {
    let mut i = 0usize;
    while i < selectors.len() {
        if let Ok(d) = html_select(doc, selectors[i]) {
            if let Some(t) = text_into(d, out) {
                if !t.is_empty() {
                    return Some(t.len());
                }
            }
        }
        i += 1;
    }
    None
}

fn first_attr(
    doc: HtmlDescriptor,
    selectors: &[&[u8]],
    attr: &[u8],
    out: &mut [u8],
) -> Option<usize> {
    let mut i = 0usize;
    while i < selectors.len() {
        if let Ok(d) = html_select(doc, selectors[i]) {
            if let Some(t) = attr_into(d, attr, out) {
                if !t.is_empty() {
                    return Some(t.len());
                }
            }
        }
        i += 1;
    }
    None
}

fn has_next_page(doc: HtmlDescriptor) -> bool {
    html_select(doc, b"div.pg > a.nxt, a.nxt, a[rel=\"next\"]")
        .map(|d| {
            let _ = html_close(d);
            true
        })
        .unwrap_or(false)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let id = match manga_id_from_request(req, "get_manga") {
        Ok(v) => v,
        Err(ptr) => return ptr,
    };
    let mut base_buf = [0u8; 512];
    let mut referer_buf = [0u8; 512];
    let (_html_len, base, doc) =
        match fetch_detail_doc(id, "get_manga", &mut base_buf, &mut referer_buf) {
            Ok(v) => v,
            Err(ptr) => return ptr,
        };

    let mut title_buf = [0u8; 512];
    let title_len = first_text(
        doc.0,
        &[
            b"h3.uk-heading-line",
            b"h1",
            b".comic-title",
            b".detail-title",
            b".title",
        ],
        &mut title_buf,
    )
    .unwrap_or(id.len().min(title_buf.len()));
    if title_len == id.len().min(title_buf.len()) {
        title_buf[..title_len].copy_from_slice(&id[..title_len]);
    }
    let title = trim_ascii(&title_buf[..title_len]);

    let mut cover_buf = [0u8; 768];
    let cover_len = first_attr(
        doc.0,
        &[
            b"div.uk-width-medium > img",
            b".cover img",
            b".comic-cover img",
            b".detail-cover img",
            b"img",
        ],
        b"data-src",
        &mut cover_buf,
    )
    .or_else(|| {
        first_attr(
            doc.0,
            &[
                b"div.uk-width-medium > img",
                b".cover img",
                b".comic-cover img",
                b".detail-cover img",
                b"img",
            ],
            b"src",
            &mut cover_buf,
        )
    });

    let mut author_buf = [0u8; 512];
    let author_len = first_text(
        doc.0,
        &[
            b"div.cl > a.uk-label",
            b".author",
            b".authors",
            b"[class*=\"author\"]",
        ],
        &mut author_buf,
    );
    let mut desc_buf = [0u8; 2048];
    let desc_len = first_text(
        doc.0,
        &[
            b"li > div.uk-alert",
            b".description",
            b".summary",
            b".intro",
            b"[class*=\"desc\"]",
            b"[class*=\"intro\"]",
        ],
        &mut desc_buf,
    );

    let payload = payload_buf();
    let mut c = 0usize;
    let mut cover_abs = [0u8; 768];
    let cover = cover_len.and_then(|n| make_absolute_url(base, &cover_buf[..n], &mut cover_abs));
    let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, clean_title(title))
        && write_bytes(payload, &mut c, br#"","cover":"#)
        && if let Some(url) = cover {
            write_bytes(payload, &mut c, br#"{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, url)
                && write_bytes(payload, &mut c, br#""}"#)
        } else {
            write_bytes(payload, &mut c, br#"{"kind":"none"}"#)
        }
        && write_bytes(payload, &mut c, br#","authors":["#)
        && if let Some(n) = author_len {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, trim_ascii(&author_buf[..n]))
                && write_bytes(payload, &mut c, b"\"")
        } else {
            true
        }
        && write_bytes(
            payload,
            &mut c,
            br#"],"status":"unknown","contentRating":"nsfw","description":""#,
        )
        && if let Some(n) = desc_len {
            append_json_escaped(payload, &mut c, trim_ascii(&desc_buf[..n]))
        } else {
            true
        }
        && write_bytes(payload, &mut c, br#"","sourceTags":["zerobyw"]}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let id = match manga_id_from_request(req, "get_chapters") {
        Ok(v) => v,
        Err(ptr) => return ptr,
    };
    let mut base_buf = [0u8; 512];
    let mut referer_buf = [0u8; 512];
    let (_html_len, base, doc) =
        match fetch_detail_doc(id, "get_chapters", &mut base_buf, &mut referer_buf) {
            Ok(v) => v,
            Err(ptr) => return ptr,
        };

    let selects = select_buf();
    let count = html_select_all(doc.0.raw(), b"div.uk-grid-collapse > div.muludiv a.uk-button-default, a[href*=\"chapter\"], a[href*=\"/read/\"]", selects);
    let max = if count > 0 { count as usize } else { 0 }.min(1000);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in 0..max {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let hd = desc_from_i32(raw);
        let mut href_buf = [0u8; 768];
        let mut path_buf = [0u8; 768];
        let href = attr_into(hd, b"href", &mut href_buf);
        let path = href.and_then(|h| normalize_chapter_path(base, h, &mut path_buf));
        let is_chapter = path
            .map(|p| contains_bytes(p, b"chapter") || contains_bytes(p, b"/read/"))
            .unwrap_or(false);
        if !is_chapter {
            let _ = html_close(hd);
            continue;
        }

        let mut title_buf = [0u8; 512];
        let title = text_into(hd, &mut title_buf).unwrap_or(b"Chapter");
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        let chapter_path = path.unwrap_or(b"");
        let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, chapter_path)
            && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_escaped(payload, &mut c, if title.is_empty() { b"Chapter" } else { title })
            && write_bytes(payload, &mut c, br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        written += 1;
        let _ = html_close(hd);
    }

    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn extract_chapter_parts<'a>(chapter_id: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    let prefix = b"chapter:";
    if chapter_id.len() <= prefix.len() || &chapter_id[..prefix.len()] != prefix {
        return None;
    }
    let rest = &chapter_id[prefix.len()..];
    let sep = find_subslice(rest, b":")?;
    Some((&rest[..sep], &rest[sep + 1..]))
}

fn write_page_item(
    payload: &mut [u8],
    c: &mut usize,
    chapter_id: &[u8],
    idx: usize,
    url: &[u8],
) -> bool {
    write_bytes(payload, c, br#"{"id":"page:"#)
        && append_json_escaped(payload, c, chapter_id)
        && write_bytes(payload, c, b":")
        && write_usize(payload, c, idx)
        && write_bytes(payload, c, br#"","index":"#)
        && write_usize(payload, c, idx)
        && write_bytes(payload, c, br#","image":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, url)
        && write_bytes(payload, c, br#""}}"#)
}

fn append_js_image_urls(
    html: &[u8],
    base: &[u8],
    payload: &mut [u8],
    c: &mut usize,
    chapter_id: &[u8],
    written: &mut usize,
) -> bool {
    let mut pos = 0usize;
    while pos < html.len() && *written < 1000 {
        let rel_http = find_subslice(&html[pos..], b"http")
            .or_else(|| find_subslice(&html[pos..], b"//"))
            .or_else(|| find_subslice(&html[pos..], b"/uploads/"));
        let start = match rel_http {
            Some(v) => pos + v,
            None => break,
        };
        let mut end = start;
        while end < html.len() {
            let b = html[end];
            if b == b'"' || b == b'\'' || b == b'\\' || b == b'<' || b.is_ascii_whitespace() {
                break;
            }
            end += 1;
        }
        let url = &html[start..end];
        if is_image_url(url) {
            let mut abs_buf = [0u8; 1024];
            let image = match make_absolute_url(base, url, &mut abs_buf) {
                Some(v) => v,
                None => return false,
            };
            if *written > 0 && !write_bytes(payload, c, b",") {
                return false;
            }
            if !write_page_item(payload, c, chapter_id, *written, image) {
                return false;
            }
            *written += 1;
        }
        pos = end.saturating_add(1);
    }
    true
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let (_manga_id, path) = match extract_chapter_parts(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "unexpected chapterId"),
    };
    let mut base_buf = [0u8; 512];
    let base = current_base_url(&mut base_buf);
    let mut referer_buf = [0u8; 512];
    let referer = match base_referer(base, &mut referer_buf) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "referer overflow"),
    };

    let url_buf = scratch_a();
    let url = match make_absolute_url(base, path, url_buf) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let html_len = match fetch_html(url, Some(referer)) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, html_len)
    };
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let selects = select_buf();
    let count = html_select_all(
        doc.0.raw(),
        b"div.uk-text-center > img, img[class*=\"chapter\"], img[class*=\"page\"], img",
        selects,
    );
    let max = if count > 0 { count as usize } else { 0 }.min(1000);
    let mut written = 0usize;
    for i in 0..max {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let hd = desc_from_i32(raw);
        let mut src_buf = [0u8; 768];
        let src_len = html_attr(hd, b"data-src", &mut src_buf)
            .ok()
            .filter(|n| *n > 0)
            .or_else(|| {
                html_attr(hd, b"data-original", &mut src_buf)
                    .ok()
                    .filter(|n| *n > 0)
            })
            .or_else(|| html_attr(hd, b"src", &mut src_buf).ok());
        if let Some(src_len) = src_len {
            let src = trim_ascii(&src_buf[..src_len]);
            if is_image_url(src) {
                let mut abs_buf = [0u8; 1024];
                if let Some(abs) = make_absolute_url(base, src, &mut abs_buf) {
                    if written > 0 && !write_bytes(payload, &mut c, b",") {
                        return write_error("get_pages", "internal_error", "payload overflow");
                    }
                    if !write_page_item(payload, &mut c, chapter_id, written, abs) {
                        return write_error("get_pages", "internal_error", "payload overflow");
                    }
                    written += 1;
                }
            }
        }
        let _ = html_close(hd);
    }
    if written == 0 && !append_js_image_urls(html, base, payload, &mut c, chapter_id, &mut written)
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    const LISTINGS: &[u8] = br#"{"listings":[{"id":"popular","title":"Popular"}]}"#;
    let payload = payload_buf();
    if LISTINGS.len() > payload.len() {
        return write_error("get_listings", "internal_error", "payload overflow");
    }
    payload[..LISTINGS.len()].copy_from_slice(LISTINGS);
    write_success_payload("get_listings", LISTINGS.len())
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page")
        .or_else(|| extract_json_string(req, b"cursor"))
        .unwrap_or(b"1");
    let mut base_buf = [0u8; 512];
    let base = current_base_url(&mut base_buf);
    let mut referer_buf = [0u8; 512];
    let referer = match base_referer(base, &mut referer_buf) {
        Some(v) => v,
        None => return write_error("get_manga_list", "internal_error", "referer overflow"),
    };
    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url_buf, &mut uc, base)
        && write_bytes(url_buf, &mut uc, b"/comic/category/order/hit/page/")
        && write_bytes(url_buf, &mut uc, page))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };
    let html_len = match fetch_html(url, Some(referer)) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga_list", c, m);
        }
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, html_len)
    };
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga_list", "parse_error", "html_parse failed"),
    };
    let selects = select_buf();
    let count = html_select_all(
        doc.0.raw(),
        b"a.uk-card, div.uk-card, a[href*=\"/comic/\"]",
        selects,
    );
    let max = if count > 0 { count as usize } else { 0 }.min(500);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    for i in 0..max {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let hd = desc_from_i32(raw);
        let mut href_buf = [0u8; 512];
        let mut title_buf = [0u8; 512];
        let mut id_buf = [0u8; 256];
        let href_len = html_attr(hd, b"href", &mut href_buf)
            .ok()
            .filter(|n| *n > 0)
            .or_else(|| {
                html_select(hd, b"a").ok().and_then(|a| {
                    let len = html_attr(a, b"href", &mut href_buf).ok().filter(|n| *n > 0);
                    let _ = html_close(a);
                    len
                })
            });
        let href = href_len.map(|n| trim_ascii(&href_buf[..n]));
        let id = href.and_then(|h| id_from_comic_path(h, &mut id_buf));
        let title_len = html_select(hd, b"p.mt5")
            .ok()
            .and_then(|p| {
                let len = html_text(p, &mut title_buf).ok();
                let _ = html_close(p);
                len
            })
            .or_else(|| {
                html_attr(hd, b"title", &mut title_buf)
                    .ok()
                    .filter(|n| *n > 0)
            })
            .or_else(|| html_text(hd, &mut title_buf).ok());
        let mut cover_buf = [0u8; 768];
        let cover_len = html_select(hd, b"img").ok().and_then(|img| {
            let len = html_attr(img, b"data-src", &mut cover_buf)
                .ok()
                .filter(|n| *n > 0)
                .or_else(|| {
                    html_attr(img, b"data-original", &mut cover_buf)
                        .ok()
                        .filter(|n| *n > 0)
                })
                .or_else(|| html_attr(img, b"src", &mut cover_buf).ok());
            let _ = html_close(img);
            len
        });
        if let (Some(id), Some(title_len)) = (id, title_len) {
            let title = clean_title(trim_ascii(&title_buf[..title_len]));
            if !title.is_empty() {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    return write_error("get_manga_list", "internal_error", "payload overflow");
                }
                let mut cover_abs = [0u8; 768];
                let cover_abs = cover_len.and_then(|n| {
                    make_absolute_url(base, trim_ascii(&cover_buf[..n]), &mut cover_abs)
                });
                if !write_manga_item(payload, &mut c, id, title, cover_abs) {
                    return write_error("get_manga_list", "internal_error", "payload overflow");
                }
                written += 1;
            }
        }
        let _ = html_close(hd);
    }
    let more = has_next_page(doc.0);
    if !(write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, parse_status_code(page) as usize + 1)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(payload, &mut c, if more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}}"))
    {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga_list", c)
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &[u8] = br#"{"settings":[{"id":"baseUrl","name":"Base URL","kind":"text","default":"http://www.zerobywgbo2.com","hint":"Override Zerobyw host without trailing slash"}]}"#;
    let payload = payload_buf();
    if SETTINGS.len() > payload.len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload[..SETTINGS.len()].copy_from_slice(SETTINGS);
    write_success_payload("get_settings", SETTINGS.len())
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://www.zerobyw.com/","User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"zerobyw source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("search", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"zerobyw get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr);
}
