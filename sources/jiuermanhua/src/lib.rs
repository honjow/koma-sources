#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped,
    extract_json_string, find_subslice, json_object_member, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::plain_text::decode_json_content;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{
    build_get_request, decode_json_body_into, fetch_error_code, parse_status_code, FetchError,
};

#[link(wasm_import_module = "koma_host")]
extern "C" {
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

static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000];

const BASE_URL: &[u8] = b"http://www.92mh.com";
const MOBILE_URL: &[u8] = b"https://h5.92mh.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

// 官网 www.92mh.com 标题和版权均用“92漫画网”；保留现有 92 字标、ID 和地址。
// 现有字标不是本次新核实的官网素材，站点身份或图标需新依据才调整。
// 固定上游为 MIXED，官网可见 R-18，源级保守标记 nsfw；未取得作品级分级时用 unknown。
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.jiuermanhua.koma",
    name: "92漫画网",
    version: "0.1.9",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "",
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
    filters: true,
    settings: false,
    image_request: false,
    credentials: false,
};


fn strip_prefix_ascii<'a>(bytes: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix {
        trim_ascii(&bytes[prefix.len()..])
    } else {
        bytes
    }
}

// Numeric cursors identify pages. Older releases emitted only "next" while
// repeatedly loading page one; an in-flight cursor resumes at page two.
fn request_page(req: &[u8]) -> Option<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let value = match json_object_member(args, b"page") { None | Some(b"null") => return Some(1), Some(v) => v };
    let raw = if value.first() == Some(&b'{') {
        match json_object_member(value, b"cursor") { None | Some(b"null") => return Some(1), Some(v) => v.strip_prefix(b"\"")?.strip_suffix(b"\"")? }
    } else { value };
    if raw == b"next" { return Some(2); }
    if raw.is_empty() { return None; }
    let mut page = 0usize;
    for &digit in raw { if !digit.is_ascii_digit() { return None; } page = page.checked_mul(10)?.checked_add((digit-b'0') as usize)?; }
    if page == 0 { None } else { Some(page) }
}

fn fetch_response(url: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
    http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let resp_len = fetch_response(url)?;
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

fn redirects_to_same_detail(response: &[u8], manga_id: &[u8]) -> bool {
    let Some(id) = manga_id.strip_prefix(b"/manhua/").and_then(|path| path.strip_suffix(b"/")) else {
        return false;
    };
    if id.is_empty() || !id.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"));
    if status.map(parse_status_code) != Some(403) {
        return false;
    }
    let mut decoded = [0u8; 512];
    let Some(raw) = extract_json_string(response, b"bodyText") else {
        return false;
    };
    let Ok(body) = decode_json_content(raw, &mut decoded) else {
        return false;
    };
    let marker = b"<script> window.location.href";
    let Some(start) = find_subslice(body, marker) else {
        return false;
    };
    let Some(value) = trim_ascii(&body[start + marker.len()..]).strip_prefix(b"=") else {
        return false;
    };
    let Some(path) = trim_ascii(value).strip_prefix(b"\"") else {
        return false;
    };
    path.iter().position(|&byte| byte == b'"').is_some_and(|end| &path[..end] == manga_id)
}

// The desktop site declares the official mobile detail page. Only its observed
// self-redirect response can use that alternate frontend. Keep ordinary HTTP
// failures, working desktop details, chapter IDs and reader requests unchanged.
fn fetch_detail_body(url: &[u8], manga_id: &[u8]) -> Result<(usize, bool), FetchError> {
    let resp_len = fetch_response(url)?;
    let response = &http_out()[..resp_len];
    match decode_json_body_into(response, body_buf()) {
        Ok(len) => Ok((len, false)),
        Err(FetchError::ClientError) if redirects_to_same_detail(response, manga_id) => {
            let mut mobile_url = [0u8; 128];
            let mut len = 0;
            if !(write_bytes(&mut mobile_url, &mut len, MOBILE_URL)
                && write_bytes(&mut mobile_url, &mut len, manga_id))
            {
                return Err(FetchError::ClientError);
            }
            Ok((fetch_body(&mobile_url[..len])?, true))
        }
        Err(error) => Err(error),
    }
}

struct OwnedDescriptor(host::HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn attr_into<'a>(
    desc: host::HtmlDescriptor,
    name: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let len = html_attr(desc, name, out).ok()?;
    Some(&out[..len])
}

fn text_into<'a>(desc: host::HtmlDescriptor, out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_text(desc, out).ok()?;
    Some(&out[..len])
}

fn path_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.is_empty() {
        return None;
    }
    if href[0] == b'/' {
        return Some(href);
    }
    let mut slash_count = 0u8;
    let mut i = 0usize;
    while i < href.len() {
        if href[i] == b'/' {
            slash_count += 1;
            if slash_count == 3 {
                return Some(&href[i..]);
            }
        }
        i += 1;
    }
    None
}

fn write_json_url(dst: &mut [u8], c: &mut usize, url: &[u8]) -> bool {
    if url.starts_with(b"http://") || url.starts_with(b"https://") {
        append_json_escaped(dst, c, url)
    } else if url.starts_with(b"//") {
        write_bytes(dst, c, b"http:") && append_json_escaped(dst, c, url)
    } else if url.starts_with(b"/") {
        write_bytes(dst, c, BASE_URL) && append_json_escaped(dst, c, url)
    } else {
        append_json_escaped(dst, c, url)
    }
}

fn write_manga_item(
    payload: &mut [u8],
    c: &mut usize,
    id: &[u8],
    title: &[u8],
    cover: &[u8],
    written: usize,
) -> bool {
    (written == 0 || write_bytes(payload, c, b","))
        && write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"unknown","description":"","sourceTags":["jiuermanhua"]}"#)
}

fn parse_list(html: &[u8], operation: &str, page: usize) -> u32 {
    // A bare self-redirect is not an empty search result and needs recovery.
    if html.len() < 2048 && find_subslice(html, b"window.location.href").is_some() {
        return write_error(operation, "parse_error", "site returned redirect script");
    }
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error(operation, "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"li.list-comic", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }

    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 500 { 500 } else { total };
    let mut written = 0usize;

    for i in 0..total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            continue;
        }
        let item_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };

        let mut title_buf = [0u8; 512];
        let mut href_buf = [0u8; 512];
        let mut cover_buf = [0u8; 1024];

        let mut title = None;
        if let Ok(desc) = html_select(item_desc, b"h3 > a") {
            let owned = OwnedDescriptor(desc);
            title = text_into(owned.0, &mut title_buf).map(trim_ascii);
        }
        if title.is_none() {
            if let Ok(desc) = html_select(item_desc, b"p > a") {
                let owned = OwnedDescriptor(desc);
                title = text_into(owned.0, &mut title_buf).map(trim_ascii);
                if title.is_none() {
                    title = attr_into(owned.0, b"title", &mut title_buf).map(trim_ascii);
                }
            }
        }

        let mut href = None;
        if let Ok(desc) = html_select(item_desc, b"a.comic_img") {
            let owned = OwnedDescriptor(desc);
            href = attr_into(owned.0, b"href", &mut href_buf);
        }
        if href.is_none() {
            if let Ok(desc) = html_select(item_desc, b"a.image-link") {
                let owned = OwnedDescriptor(desc);
                href = attr_into(owned.0, b"href", &mut href_buf);
            }
        }
        if href.is_none() {
            if let Ok(desc) = html_select(item_desc, b"p > a") {
                let owned = OwnedDescriptor(desc);
                href = attr_into(owned.0, b"href", &mut href_buf);
            }
        }
        let cover = if let Ok(desc) = html_select(item_desc, b"img") {
            let owned = OwnedDescriptor(desc);
            attr_into(owned.0, b"src", &mut cover_buf).unwrap_or(b"")
        } else {
            b""
        };

        if let (Some(t), Some(h)) = (title, href) {
            if let Some(path) = path_from_href(h) {
                if !write_manga_item(payload, &mut c, path, t, cover, written) {
                    let _ = html_close(item_desc);
                    break;
                }
                written += 1;
            }
        }

        let _ = html_close(item_desc);
    }

    // Only the provider's next-page link establishes another page; the
    // cursor must encode the page identity consumed on the next request.
    let has_more = match html_select(document.0, b"li.next a") {
        Ok(next) => { let _owned = OwnedDescriptor(next); true },
        Err(_) => false,
    };
    let next_page = if has_more { match page.checked_add(1) { Some(v) => v, None => return write_error(operation, "invalid_request", "invalid page cursor") } } else { 0 };
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more {
            write_bytes(payload, &mut c, b"\"") && write_usize(payload, &mut c, next_page) && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if has_more { b"true" as &[u8] } else { b"false" as &[u8] })
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error(operation, "internal_error", "payload overflow");
    }

    write_success_payload(operation, c)
}

// Keep decoded query separate from URL and HTTP buffers and off the WAMR stack.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];
fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(query, decoded) {
        Ok(v) => v,
        Err(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    let page = match request_page(req) { Some(v) => v, None => return write_error("search", "invalid_request", "invalid page cursor") };

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/search/?keywords=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&page=")
        && write_usize(url_buf, &mut c, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("search", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html, "search", page)
}

// Only current browse filters alter the prior popular route; search stays independent.
const CATEGORY_IDS: [&[u8];8] = [b"type",b"region",b"genre",b"audience",b"year",b"gender",b"letter",b"status"];
const CATEGORY_NAMES: [&str;8] = ["按类型","按地区","按剧情","按受众","按年份","按性别","按字母","按进度"];
fn feature(req: &[u8], name: &[u8]) -> bool {
    json_object_member(req,b"hostFeatures").map(trim_ascii).map_or(false, |v| v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|b|*b==b',').any(|v| { let v=trim_ascii(v); v.strip_prefix(b"\"").and_then(|v|v.strip_suffix(b"\""))==Some(name) }))
}
fn filtered_value<'a>(f: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {json_object_member(f,key).and_then(|v|v.strip_prefix(b"\"")?.strip_suffix(b"\""))}
fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = match request_page(req) { Some(v) => v, None => return write_error("get_manga_list", "invalid_request", "invalid page cursor") };

    let url_buf = scratch_a();
    let mut c = 0usize;
    // The reference has a separate unfiltered latest feed. Older hosts retain
    // the previously exposed two-tab behavior until they can hide its filters.
    let listing = json_object_member(req, b"args").and_then(|a| filtered_value(a, b"listingId"));
    let latest = feature(req, b"source_listing_filter_scope_v1") && listing == Some(b"listing:latest".as_slice());
    if latest {
        if !(write_bytes(url_buf, &mut c, BASE_URL) && write_bytes(url_buf, &mut c, b"/list/update/?page=") && write_usize(url_buf, &mut c, page)) {
            return write_error("get_manga_list", "internal_error", "url overflow");
        }
    } else {
        let filters=if feature(req,b"source_filter_scope_header_v1") && feature(req,b"source_filter_fetching_v1") {json_object_member(req,b"args").and_then(|a|json_object_member(a,b"filters")).unwrap_or(b"{}")} else {b"{}"};
        let any=CATEGORY_IDS.iter().any(|id|filtered_value(filters,id).is_some()) || filtered_value(filters,b"sort").is_some();
        if any {
            if !(write_bytes(url_buf,&mut c,BASE_URL) && write_bytes(url_buf,&mut c,b"/list/")) {return write_error("get_manga_list","internal_error","url overflow");}
            let mut count=0;
            for id in CATEGORY_IDS { if let Some(v)=filtered_value(filters,id) { if v!=b"all" { if v.is_empty() || !v.iter().all(|b|b.is_ascii_alphanumeric() || *b==b'_') {return write_error("get_manga_list","invalid_request","invalid category");} if count>0 && !write_bytes(url_buf,&mut c,b"-") {return write_error("get_manga_list","internal_error","url overflow");} if !write_bytes(url_buf,&mut c,v) {return write_error("get_manga_list","internal_error","url overflow");} count+=1; } } }
            if !(write_bytes(url_buf,&mut c,b"-/")) {return write_error("get_manga_list","internal_error","url overflow");}
        } else if !(write_bytes(url_buf,&mut c,BASE_URL) && write_bytes(url_buf,&mut c,b"/list/")) {return write_error("get_manga_list","internal_error","url overflow");}
        let sort=filtered_value(filters,b"sort").unwrap_or(b"click");
        if ![b"post".as_slice(),b"-post",b"update",b"-update",b"click",b"-click"].contains(&sort) {return write_error("get_manga_list","invalid_request","invalid sort");}
        if !(write_bytes(url_buf,&mut c,sort) && write_bytes(url_buf,&mut c,b"/?page=") && write_usize(url_buf,&mut c,page))
        {
            return write_error("get_manga_list", "internal_error", "url overflow");
        }
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_manga_list", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html, "get_manga_list", page)
}

fn select_text<'a>(
    root: host::HtmlDescriptor,
    selector: &[u8],
    buf: &'a mut [u8],
) -> Option<&'a [u8]> {
    let desc = html_select(root, selector).ok()?;
    let owned = OwnedDescriptor(desc);
    text_into(owned.0, buf).map(trim_ascii)
}

fn select_attr<'a>(
    root: host::HtmlDescriptor,
    selector: &[u8],
    attr: &[u8],
    buf: &'a mut [u8],
) -> Option<&'a [u8]> {
    let desc = html_select(root, selector).ok()?;
    let owned = OwnedDescriptor(desc);
    attr_into(owned.0, attr, buf)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, manga_id)) {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let (html_len, mobile) = match fetch_detail_body(url, manga_id) {
        Ok(result) => result,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_manga", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    let mut title_buf = [0u8; 512];
    let mut cover_buf = [0u8; 1024];
    let mut author_buf = [0u8; 512];
    let mut status_buf = [0u8; 256];
    let mut desc_buf = [0u8; 4096];

    let (title, cover, author, status_raw, desc) = if mobile {
        let Some(title) = select_text(document.0, b"#comicName", &mut title_buf).map(trim_ascii)
            .filter(|title| !title.is_empty())
        else {
            return write_error("get_manga", "parse_error", "mobile detail title missing");
        };
        let cover = select_attr(document.0, b"#Cover img", b"src", &mut cover_buf).unwrap_or(b"");
        let author = select_text(document.0, b".Introduct_Sub .sub_r > p.txtItme:first-child", &mut author_buf)
            .map(trim_ascii).unwrap_or(b"");
        let status = select_text(document.0, b".Introduct_Sub .sub_r", &mut status_buf).unwrap_or(b"");
        let desc = select_text(document.0, b"#full-des", &mut desc_buf).unwrap_or(b"");
        (title, cover, author, status, strip_prefix_ascii(desc, "介绍:".as_bytes()))
    } else {
        let title = select_text(document.0, b"div.comic_deCon > h1", &mut title_buf).unwrap_or(manga_id);
        let cover = select_attr(document.0, b"div.comic_i_img > img", b"src", &mut cover_buf).unwrap_or(b"");

        let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
        let info_count = html_select_all(document.0.raw(), b".comic_deCon_liO > li", select_buf);
        let mut author_len = 0usize;
        let mut status_len = 0usize;
        let info_total = if info_count > 0 { info_count as usize } else { 0 };
        let info_total = if info_total > 8 { 8 } else { info_total };
        for i in 0..info_total {
            let offset = i * 4;
            if offset + 4 > select_buf.len() {
                break;
            }
            let desc_raw = i32::from_le_bytes([
                select_buf[offset],
                select_buf[offset + 1],
                select_buf[offset + 2],
                select_buf[offset + 3],
            ]);
            if desc_raw < 0 {
                continue;
            }
            let li_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
            if i == 0 {
                if let Some(t) = text_into(li_desc, &mut author_buf).map(trim_ascii) {
                    author_len = t.len();
                }
            } else if i == 1 {
                let mut found_status = false;
                if let Ok(a_desc) = html_select(li_desc, b"a") {
                    let owned = OwnedDescriptor(a_desc);
                    if let Some(t) = text_into(owned.0, &mut status_buf).map(trim_ascii) {
                        status_len = t.len();
                        found_status = true;
                    }
                }
                if !found_status {
                    if let Some(t) = text_into(li_desc, &mut status_buf).map(trim_ascii) {
                        status_len = t.len();
                    }
                }
            }
            let _ = html_close(li_desc);
        }

        let author = strip_prefix_ascii(trim_ascii(&author_buf[..author_len]), "作者：".as_bytes());
        let status = trim_ascii(&status_buf[..status_len]);
        let desc = select_text(document.0, b"p.comic_deCon_d", &mut desc_buf).unwrap_or(b"");
        (title, cover, author, status, desc)
    };

    let status = if find_subslice(status_raw, "已完结".as_bytes()).is_some() {
        b"completed" as &[u8]
    } else if find_subslice(status_raw, "连载中".as_bytes()).is_some() {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && if !author.is_empty() {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            true
        }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"unknown","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);

    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn explicit_chapter_number(title: &[u8]) -> Option<usize> {
    // The chapter-list label carries the real ordinal independently of the link order.
    let title = core::str::from_utf8(title).ok()?.trim().strip_prefix('第')?;
    let (number, _) = title.split_once(['话', '話', '回'])?;
    let number = number.trim();
    if number.is_empty() || !number.as_bytes().iter().all(u8::is_ascii_digit) {
        return None;
    }
    number.parse().ok()
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, manga_id)) {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let (html_len, mobile) = match fetch_detail_body(url, manga_id) {
        Ok(result) => result,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_chapters", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    // Both frontends expose all named groups in document order. The stable
    // chapter paths and existing order within each group do not depend on its label.
    let selector: &[u8] = if mobile {
        br#".chapter-body ul[id^="chapter-list-"] > li > a"#
    } else {
        br#".zj_list_con > ul[id^="chapter-list-"] > li > a"#
    };
    let count = html_select_all(document.0.raw(), selector, select_buf);
    if count < 0 || count as usize > select_buf.len() / 4 {
        return write_error("get_chapters", "parse_error", "chapter list selection failed");
    }
    if mobile {
        let mut title = [0u8; 256];
        let title = select_text(document.0, b"#comicName", &mut title).map(trim_ascii);
        if title.is_none_or(|title| title.is_empty()) {
            return write_error("get_chapters", "parse_error", "mobile detail title missing");
        }
    }
    let total = count as usize;

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in 0..total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            continue;
        }
        let a_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut name_buf = [0u8; 256];
        let mut href_buf = [0u8; 512];

        let title_selector: &[u8] = if mobile { b"span" } else { b"span.list_con_zj" };
        let name = if let Ok(span_desc) = html_select(a_desc, title_selector) {
            let owned = OwnedDescriptor(span_desc);
            text_into(owned.0, &mut name_buf).map(trim_ascii)
        } else {
            text_into(a_desc, &mut name_buf).map(trim_ascii)
        };
        let href = attr_into(a_desc, b"href", &mut href_buf);
        let _ = html_close(a_desc);

        if let (Some(ch_name), Some(ch_href)) = (name, href) {
            if let Some(path) = path_from_href(ch_href) {
                if mobile && !path.starts_with(manga_id) {
                    continue;
                }
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    return write_error("get_chapters", "internal_error", "payload overflow");
                }
                let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                    && append_json_escaped(payload, &mut c, path)
                    && write_bytes(payload, &mut c, br#"","mangaId":""#)
                    && append_json_escaped(payload, &mut c, manga_id)
                    && write_bytes(payload, &mut c, br#"","title":""#)
                    && append_json_escaped(payload, &mut c, ch_name)
                    && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
                    && match explicit_chapter_number(ch_name) {
                        Some(number) => write_usize(payload, &mut c, number),
                        None => write_bytes(payload, &mut c, b"null"),
                    }
                    && write_bytes(payload, &mut c, br#","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
                if !ok {
                    return write_error("get_chapters", "internal_error", "payload overflow");
                }
                written += 1;
            }
        }
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn extract_config_host(config: &[u8], out: &mut [u8]) -> Option<usize> {
    let marker = br#"domain":[""#;
    let start = find_subslice(config, marker)? + marker.len();
    let mut end = start;
    while end < config.len() && config[end] != b'"' {
        end += 1;
    }
    if end == start || end >= config.len() || end - start > out.len() {
        return None;
    }
    out[..end - start].copy_from_slice(&config[start..end]);
    Some(end - start)
}

fn extract_js_string<'a>(data: &'a [u8], start: usize) -> Option<(&'a [u8], usize)> {
    if start >= data.len() || data[start] != b'"' {
        return None;
    }
    let mut i = start + 1;
    while i < data.len() {
        if data[i] == b'\\' {
            i += 2;
            continue;
        }
        if data[i] == b'"' {
            return Some((&data[start + 1..i], i + 1));
        }
        i += 1;
    }
    None
}

fn extract_chapter_path<'a>(html: &'a [u8]) -> Option<&'a [u8]> {
    let marker = b"chapterPath";
    let pos = find_subslice(html, marker)? + marker.len();
    let rest = &html[pos..];
    let quote = find_subslice(rest, b"\"")?;
    let (s, _) = extract_js_string(rest, quote)?;
    Some(s)
}

fn write_page_url(
    payload: &mut [u8],
    c: &mut usize,
    host: &[u8],
    chapter_path: &[u8],
    image: &[u8],
) -> bool {
    if image.starts_with(b"https://") || image.starts_with(b"http://") {
        append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(br#"https:\/\/"#) || image.starts_with(br#"http:\/\/"#) {
        append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(br#"\/"#) {
        append_json_escaped(payload, c, host) && append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(b"/") {
        append_json_escaped(payload, c, host) && append_json_unescaped_then_escaped(payload, c, image)
    } else {
        append_json_escaped(payload, c, host)
            && write_bytes(payload, c, b"/")
            && append_json_escaped(payload, c, chapter_path)
            && append_json_unescaped_then_escaped(payload, c, image)
    }
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };

    let config_url = b"http://www.92mh.com/js/config.js";
    let config_len = match fetch_body(config_url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let host_len = {
        let config = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), config_len) };
        match extract_config_host(config, scratch_b()) {
            Some(n) => n,
            None => return write_error("get_pages", "parse_error", "image host not found"),
        }
    };

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, chapter_id)) {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let host = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), host_len) };
    let chapter_path = extract_chapter_path(html).unwrap_or(b"");

    let images_marker = b"chapterImages";
    let marker_pos = match find_subslice(html, images_marker) {
        Some(p) => p + images_marker.len(),
        None => return write_error("get_pages", "parse_error", "chapterImages not found"),
    };
    let rest = &html[marker_pos..];
    let array_start = match find_subslice(rest, b"[") {
        Some(p) => marker_pos + p + 1,
        None => return write_error("get_pages", "parse_error", "chapterImages array not found"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let mut pos = array_start;
    let mut written = 0usize;
    while pos < html.len() && html[pos] != b']' && written < 2000 {
        if html[pos] != b'"' {
            pos += 1;
            continue;
        }
        let (image, next_pos) = match extract_js_string(html, pos) {
            Some(v) => v,
            None => break,
        };
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_page_url(payload, &mut c, host, chapter_path, image)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            break;
        }
        written += 1;
        pos = next_pos;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"jiuermanhua source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 { 0 } else { -1 }
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
    log_info(b"jiuermanhua search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    let payload = if feature(req, b"source_listing_filter_scope_v1") {
        r#"{"listings":[{"id":"listing:popular","name":"热门","kind":"popular"},{"id":"listing:latest","name":"最新","kind":"latest","supportsFilters":false}]}"#.as_bytes()
    } else {
        r#"{"listings":[{"id":"listing:popular","name":"热门","kind":"popular"},{"id":"listing:latest","name":"最新","kind":"latest"}]}"#.as_bytes()
    };
    response_buffer().write_success("get_listings", payload)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_home", br#"{"sections":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req=match read_request(req_ptr,req_len) {Some(v)=>v,None=>return write_error("get_filters","invalid_request","empty request")};
    if !feature(req,b"source_filter_scope_header_v1") || !feature(req,b"source_filter_fetching_v1") {return response_buffer().write_success("get_filters",br#"{"filters":[]}"#);}
    let dynamic=feature(req,b"source_filter_fetching_v1");
    let fetch=dynamic && json_object_member(req,b"args").and_then(|a|json_object_member(a,b"fetchFilterData"))==Some(b"true");
    let mut cats=[0i32;8];
    let doc=if fetch {
        let mut url=[0u8;256];let mut n=0;if !(write_bytes(&mut url,&mut n,BASE_URL)&&write_bytes(&mut url,&mut n,b"/list/")){return write_error("get_filters","internal_error","url overflow");}
        let size=match fetch_body(&url[..n]) {Ok(n)=>n,Err(e)=>{let (code,msg)=fetch_error_code(e);return write_error("get_filters",code,msg)}};
        let body=unsafe{core::slice::from_raw_parts(BODY_BUF.as_ptr(),size)};
        let doc=match html_parse(body){Ok(v)=>OwnedDescriptor(v),Err(_)=>return write_error("get_filters","parse_error","category HTML")};
        let buf=unsafe{&mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF)};let count=html_select_all(doc.0.raw(),b".filter-nav > .filter-item",buf);
        if count!=8 {return write_error("get_filters","parse_error","category groups")}
        for i in 0..8 {cats[i]=i32::from_le_bytes(buf[i*4..i*4+4].try_into().unwrap_or([0;4]));if cats[i]<=0{return write_error("get_filters","parse_error","category descriptor");}}
        Some(doc)
    } else {None};
    let out=payload_buf();let mut n=0;
    if !write_bytes(out,&mut n,r#"{"supportsFilterFetching":true,"filters":[{"type":"header","scope":"browse","name":"分类筛选（搜索文本时无效）"},"#.as_bytes()) {return write_error("get_filters","internal_error","schema overflow")}
    if dynamic {for i in 0..8 {
        if i>0 && !write_bytes(out,&mut n,b","){return write_error("get_filters","internal_error","schema overflow")}
        if !(write_bytes(out,&mut n,br#"{"id":""#)&&append_json_escaped(out,&mut n,CATEGORY_IDS[i])&&write_bytes(out,&mut n,br#"","name":""#)&&append_json_escaped(out,&mut n,CATEGORY_NAMES[i].as_bytes())&&write_bytes(out,&mut n,br#"","type":"select","scope":"browse""#)){return write_error("get_filters","internal_error","schema overflow")}
        if fetch {
            let group=host::HtmlDescriptor::from_raw(cats[i]);let mut name=[0u8;128];
            if select_text(group,b"label",&mut name)!=Some(CATEGORY_NAMES[i].as_bytes()){return write_error("get_filters","parse_error","category identity")}
            let buf=unsafe{&mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF)};let count=html_select_all(group.raw(),b"a[href]",buf);
            if count<=0 || count>500 {return write_error("get_filters","parse_error","category options")}
            if !write_bytes(out,&mut n,br#", "options":["#){return write_error("get_filters","internal_error","schema overflow")}
            for j in 0..count as usize {
                let raw=i32::from_le_bytes(buf[j*4..j*4+4].try_into().unwrap_or([0;4]));if raw<=0{return write_error("get_filters","parse_error","option descriptor")};let node=OwnedDescriptor(host::HtmlDescriptor::from_raw(raw));
                let mut hb=[0u8;512];let mut lb=[0u8;512];let href=match attr_into(node.0,b"href",&mut hb){Some(v)=>v,None=>return write_error("get_filters","parse_error","option href")};let label=match text_into(node.0,&mut lb).map(trim_ascii){Some(v) if !v.is_empty()=>v,_=>return write_error("get_filters","parse_error","option label")};
                let value=match href.strip_prefix(b"/list/").and_then(|v| if v.is_empty(){Some(v)}else{v.strip_suffix(b"/")}) {Some(b"")=>b"all".as_slice(),Some(v) if v.iter().all(|b|b.is_ascii_alphanumeric()||*b==b'_')=>v,_=>return write_error("get_filters","parse_error","option path")};
                if (j>0 && !write_bytes(out,&mut n,b",")) || !(write_bytes(out,&mut n,br#"{"value":""#)&&append_json_escaped(out,&mut n,value)&&write_bytes(out,&mut n,br#"","label":""#)&&append_json_escaped(out,&mut n,label)&&write_bytes(out,&mut n,br#""}"#)){return write_error("get_filters","internal_error","schema overflow")}
            }
            if !write_bytes(out,&mut n,b"]"){return write_error("get_filters","internal_error","schema overflow")}
            let _=html_close(group);
        }
        if !write_bytes(out,&mut n,b"}"){return write_error("get_filters","internal_error","schema overflow")}
    } if !write_bytes(out,&mut n,b","){return write_error("get_filters","internal_error","schema overflow")}}
    let tail=r#"{"id":"sort","name":"排序方式","type":"select","scope":"browse","options":[{"value":"post","label":"按发布排序"},{"value":"-post","label":"按发布排序(逆序)"},{"value":"update","label":"按更新排序"},{"value":"-update","label":"按更新排序(逆序)"},{"value":"click","label":"按点击排序"},{"value":"-click","label":"按点击排序(逆序)"}]}]}"#;
    if !write_bytes(out,&mut n,tail.as_bytes()){return write_error("get_filters","internal_error","schema overflow")}
    let _keep=doc;write_success_payload("get_filters",n)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_settings", br#"{"settings":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_error("get_image_request", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
