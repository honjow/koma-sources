#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.fjumonkey.pixnet.zh-hant.koma", name:"輔大猴・四格漫畫", version:"0.1.0", api_version:"0.2", language:"zh-Hant", author:"Koma",
 description:"收錄輔大猴作者部落格「[專欄]四格漫畫」的漫畫，依原欄目分頁瀏覽。每篇獨立閱讀，作品評級尚未確認。", content_rating:"unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:2*1024*1024,body:1024*1024,http_req:4096,scratch:65536,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn whitespace(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn end_value(s: &[u8], start: usize) -> R<usize> {
    parse_value(s, start, 0)
}
fn parse_value(s: &[u8], start: usize, depth: usize) -> R<usize> {
    if depth > 64 {
        return Err("capacity_exceeded");
    }
    let start = whitespace(s, start);
    let first = *s.get(start).ok_or("parse_error")?;
    if first == b'"' {
        let mut i = start + 1;
        while i < s.len() {
            match s[i] {
                b'"' => return Ok(i + 1),
                b'\\' => {
                    i += 1;
                    let e = *s.get(i).ok_or("parse_error")?;
                    if e == b'u' {
                        if i + 5 > s.len() || !s[i + 1..i + 5].iter().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("parse_error");
                        }
                        i += 4;
                    } else if !matches!(e, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') {
                        return Err("parse_error");
                    }
                }
                b if b < 32 => return Err("parse_error"),
                _ => {}
            }
            i += 1;
        }
        return Err("parse_error");
    }
    if matches!(first, b'{' | b'[') {
        let end = if first == b'{' { b'}' } else { b']' };
        let mut i = whitespace(s, start + 1);
        if s.get(i) == Some(&end) {
            return Ok(i + 1);
        }
        loop {
            if first == b'{' {
                if s.get(i) != Some(&b'"') {
                    return Err("parse_error");
                }
                i = whitespace(s, parse_value(s, i, depth + 1)?);
                if s.get(i) != Some(&b':') {
                    return Err("parse_error");
                }
                i = whitespace(s, i + 1);
            }
            i = whitespace(s, parse_value(s, i, depth + 1)?);
            match s.get(i) {
                Some(v) if *v == end => return Ok(i + 1),
                Some(b',') => {
                    i = whitespace(s, i + 1);
                    if s.get(i) == Some(&end) {
                        return Err("parse_error");
                    }
                }
                _ => return Err("parse_error"),
            }
        }
    }
    for v in [b"true".as_slice(), b"false".as_slice(), b"null".as_slice()] {
        if s[start..].starts_with(v) {
            return Ok(start + v.len());
        }
    }
    let mut i = start;
    if s.get(i) == Some(&b'-') {
        i += 1;
    }
    match s.get(i) {
        Some(b'0') => {
            i += 1;
            if s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                return Err("parse_error");
            }
        }
        Some(b'1'..=b'9') => {
            i += 1;
            while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                i += 1;
            }
        }
        _ => return Err("parse_error"),
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    if matches!(s.get(i), Some(b'e') | Some(b'E')) {
        i += 1;
        if matches!(s.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    Ok(i)
}
fn direct<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'{') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    let mut found = None;
    if s.get(i) == Some(&b'}') {
        if whitespace(s, i + 1) != s.len() {
            return Err("parse_error");
        }
        return Ok(None);
    }
    loop {
        if s.get(i) != Some(&b'"') {
            return Err("parse_error");
        }
        let e = end_value(s, i)?;
        let k = &s[i + 1..e - 1];
        if k.contains(&b'\\') {
            return Err("parse_error");
        }
        i = whitespace(s, e);
        if s.get(i) != Some(&b':') {
            return Err("parse_error");
        }
        i = whitespace(s, i + 1);
        let e = end_value(s, i)?;
        if k == key {
            if found.is_some() {
                return Err("duplicate_identity");
            }
            found = Some(&s[i..e]);
        }
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => i = whitespace(s, i + 1),
            Some(b'}') => {
                if whitespace(s, i + 1) != s.len() {
                    return Err("parse_error");
                }
                return Ok(found);
            }
            _ => return Err("parse_error"),
        }
    }
}
fn arg_string<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    match direct(s, key).map_err(|_| "invalid_request")? {
        None => Ok(None),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
            Ok(Some(&v[1..v.len() - 1]))
        }
        _ => Err("invalid_request"),
    }
}

fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    direct(v, k)?.ok_or("parse_error")
}
fn string<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let x = member(v, k)?;
    if x.len() < 2 || x[0] != b'"' || x[x.len() - 1] != b'"' {
        return Err("parse_error");
    };
    Ok(&x[1..x.len() - 1])
}
fn number(v: &[u8]) -> R<usize> {
    if v.is_empty() {
        return Err("parse_error");
    };
    let mut n = 0usize;
    for b in v {
        if !b.is_ascii_digit() {
            return Err("parse_error");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("parse_error")?;
    }
    Ok(n)
}
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"") && append_json_escaped(p, c, v) && write_bytes(p, c, b"\"") {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn w(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn n(p: &mut [u8], c: &mut usize, v: usize) -> R<()> {
    if write_usize(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}

fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn close_raw(raw: &[u8], count: i32) {
    for i in 0..core::cmp::min(core::cmp::max(count, 0) as usize, raw.len() / 4) {
        let a = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[a],
            raw[a + 1],
            raw[a + 2],
            raw[a + 3],
        ]));
        let _ = host::html_close(node);
    }
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    let mut buf = [0; 128];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n < 0 {
        return Err("html_error");
    }
    for i in 0..core::cmp::min(n as usize, 32) {
        let at = i * 4;
        let d = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        host::html_close(host::HtmlDescriptor::from_raw(d)).map_err(|_| "html_error")?;
    }
    Ok(n as usize)
}
fn text(doc: host::HtmlDescriptor, sel: &[u8], dst: &mut [u8]) -> R<usize> {
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_text(node, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}
fn attr(doc: host::HtmlDescriptor, sel: &[u8], key: &[u8], dst: &mut [u8]) -> R<usize> {
    // Caller selector requires presence of the same key; Attr never probes absent attributes.
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_attr(node, key, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}

fn raw_string(v: &[u8], out: &mut [u8]) -> R<usize> {
    let mut i = 0;
    let mut c = 0;
    while i < v.len() {
        let b = v[i];
        i += 1;
        if b != b'\\' {
            if b < 32 || !write_bytes(out, &mut c, &[b]) {
                return Err("parse_error");
            };
            continue;
        }
        if i == v.len() {
            return Err("parse_error");
        };
        let e = v[i];
        i += 1;
        match e {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut c, &[e]) {
                    return Err("capacity_exceeded");
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let b = match e {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut c, &[b]) {
                    return Err("capacity_exceeded");
                }
            }
            b'u' => {
                if i + 4 > v.len() || !v[i..i + 4].iter().all(|b| b.is_ascii_hexdigit()) {
                    return Err("parse_error");
                };
                let mut cp = hex_to_u16(&v[i..i + 4]) as u32;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if i + 6 > v.len()
                        || &v[i..i + 2] != b"\\u"
                        || !v[i + 2..i + 6].iter().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err("parse_error");
                    };
                    let low = hex_to_u16(&v[i + 2..i + 6]) as u32;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("parse_error");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    i += 6;
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("parse_error");
                };
                let mut b = [0; 4];
                let n = encode_utf8(cp, &mut b);
                if !write_bytes(out, &mut c, &b[..n]) {
                    return Err("capacity_exceeded");
                }
            }
            _ => return Err("parse_error"),
        }
    }
    Ok(c)
}
const BLOG: &[u8] = b"https://fjumonkey.pixnet.net/blog";
const CATEGORY: &[u8] = b"https://fjumonkey.pixnet.net/blog/categories/1708975";
const POSTS: &[u8] = b"https://fjumonkey.pixnet.net/blog/posts/";
const MEDIA: &[u8] = b"https://pimg.1px.tw/fjumonkey/";
const PERSON: &[u8] = b"https://fjumonkey.pixnet.net#person";
const MID: &[u8] = b"fjumonkey:zh-hant:post:";
const CID: &[u8] = b"fjumonkeyread:zh-hant:post:";
const PID: &[u8] = b"fjumonkeypage:zh-hant:post:";
const COVER_MEDIA: &[u8] = b"http://pic.pimg.tw/fjumonkey/";
const MAX_IMAGES: usize = 64;
static mut NOTE: [u8; 16384] = [0; 16384];
static mut NOTE_LEN: usize = 0;
static mut NOTE_LINKS: [u8; 4096] = [0; 4096];
static mut NOTE_LINKS_LEN: usize = 0;
static mut TITLE: [u8; 1024] = [0; 1024];
static mut TL: usize = 0;
static mut IMAGES: [[u8; 256]; MAX_IMAGES] = [[0; 256]; MAX_IMAGES];
static mut IL: [usize; MAX_IMAGES] = [0; MAX_IMAGES];
static mut IC: usize = 0;
static mut DATE: [u8; 64] = [0; 64];
static mut DL: usize = 0;
static mut NEXT: [u8; 256] = [0; 256];
static mut NL: usize = 0;
fn digits(v: &[u8]) -> bool {
    !v.is_empty() && v.len() <= 24 && v[0] != b'0' && v.iter().all(|b| b.is_ascii_digit())
}
// External post IDs remain opaque strings, including original short migrated IDs. Only local counters use usize.
fn id<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let x = v.strip_prefix(prefix).ok_or("invalid_request")?;
    if !digits(x) {
        return Err("invalid_request");
    }
    Ok(x)
}
fn ident(p: &mut [u8], c: &mut usize, prefix: &[u8], post: &[u8]) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, post)
}
fn route(post: &[u8], out: &mut [u8]) -> R<usize> {
    let mut n = 0;
    w(out, &mut n, POSTS)?;
    w(out, &mut n, post)?;
    Ok(n)
}
fn owner(req: &[u8], post: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaId")? {
        if id(v, MID)? != post {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        if id(v, POSTS)? != post {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn request_post<'a>(req: &'a [u8], key: &[u8], prefix: &[u8]) -> R<&'a [u8]> {
    let post = id(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, post)?;
    Ok(post)
}
fn positive(v: &[u8]) -> R<usize> {
    if v.is_empty() || (v.len() > 1 && v[0] == b'0') {
        return Err("invalid_request");
    }
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 || n > 10000 {
        return Err("invalid_request");
    }
    Ok(n)
}
fn cursor_page(url: &[u8]) -> R<usize> {
    let s = url
        .strip_prefix(CATEGORY)
        .and_then(|v| v.strip_prefix(b"?page="))
        .ok_or("invalid_request")?;
    positive(s)
}
fn list_route(req: &[u8], out: &mut [u8]) -> R<(usize, usize)> {
    let pg = direct(req, b"page").map_err(|_| "invalid_request")?;
    let cursor = match pg {
        None => None,
        Some(p) if p.first() == Some(&b'{') => {
            direct(p, b"cursor").map_err(|_| "invalid_request")?
        }
        _ => return Err("invalid_request"),
    };
    match cursor {
        None | Some(b"null") => {
            out[..CATEGORY.len()].copy_from_slice(CATEGORY);
            Ok((CATEGORY.len(), 1))
        }
        Some(v) => {
            if v.len() < 2 || v[0] != b'"' || v[v.len() - 1] != b'"' {
                return Err("invalid_request");
            }
            let u = &v[1..v.len() - 1];
            let pn = cursor_page(u)?;
            if pn < 2 || u.len() > out.len() {
                return Err("invalid_request");
            }
            out[..u.len()].copy_from_slice(u);
            Ok((u.len(), pn))
        }
    }
}
fn fetch(url: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), url, Some(url), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    let n = decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")?;
    if n == 0 || n >= 1024 * 1024 {
        return Err("capacity_exceeded");
    }
    Ok(n)
}
fn equal_attr(doc: host::HtmlDescriptor, sel: &[u8], key: &[u8], expected: &[u8]) -> R<()> {
    let mut b = [0; 1024];
    let n = attr(doc, sel, key, &mut b)?;
    if &b[..n] != expected {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn json_equal(obj: &[u8], key: &[u8], expected: &[u8]) -> R<()> {
    let mut b = [0; 1024];
    let n = raw_string(string(obj, key)?, &mut b)?;
    if &b[..n] != expected {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn validate_identity(doc: host::HtmlDescriptor, canonical: &[u8], post: &[u8]) -> R<()> {
    equal_attr(doc, b"head link[rel='canonical'][href]", b"href", canonical)?;
    equal_attr(
        doc,
        b"head meta[property='og:url'][content]",
        b"content",
        canonical,
    )?;
    equal_attr(
        doc,
        b"head meta[name='author'][content]",
        b"content",
        "輔大猴部落格".as_bytes(),
    )?;
    let n = text(doc, b"script#pixnet-vars", scratch_a())?;
    let s = &scratch_a()[..n];
    let a = find_subslice(s, b"window.PIXNET = ").ok_or("identity_mismatch")? + 16;
    let start = whitespace(s, a);
    let end = end_value(s, start)?;
    let config = &s[start..end];
    json_equal(config, b"name", b"fjumonkey")?;
    json_equal(config, b"blog_id", b"10737")?;
    json_equal(config, b"post_id", post)?;
    if member(member(config, b"options")?, b"AdultWarning")? != b"false" {
        return Err("permission_required");
    }
    Ok(())
}
fn validate_author_json(doc: host::HtmlDescriptor) -> R<()> {
    let mut raw = [0; 16];
    let count = unsafe {
        select_all_raw(
            doc.raw(),
            b"head script[type='application/ld+json']".as_ptr(),
            b"head script[type='application/ld+json']".len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if count < 1 || count > 4 {
        close_raw(&raw, count);
        return Err("parse_error");
    }
    let mut found = 0;
    let mut err = None;
    for i in 0..count as usize {
        let off = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[off],
            raw[off + 1],
            raw[off + 2],
            raw[off + 3],
        ]));
        let r = (|| {
            let n = host::html_text(node, scratch_a()).map_err(|_| "html_error")?;
            let s = trim_ascii(&scratch_a()[..n]);
            if let Some(graph) = direct(s, b"@graph")? {
                let mut at = whitespace(graph, 1);
                if graph.first() != Some(&b'[') {
                    return Err("parse_error");
                }
                while at < graph.len() && graph[at] != b']' {
                    let end = end_value(graph, at)?;
                    let entry = &graph[at..end];
                    if string(entry, b"@type").ok() == Some(b"Person") {
                        json_equal(entry, b"@id", PERSON)?;
                        json_equal(entry, b"name", "輔大猴部落格".as_bytes())?;
                        json_equal(entry, b"url", BLOG)?;
                        found += 1;
                    }
                    at = whitespace(graph, end);
                    if graph.get(at) == Some(&b',') {
                        at = whitespace(graph, at + 1);
                    } else if graph.get(at) != Some(&b']') {
                        return Err("parse_error");
                    }
                }
            }
            Ok(())
        })();
        let close = host::html_close(node).map_err(|_| "html_error");
        if let Err(e) = r.and(close) {
            if err.is_none() {
                err = Some(e);
            }
        }
    }
    if let Some(e) = err {
        return Err(e);
    }
    if found != 1 {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn media_file(u: &[u8], prefix: &[u8]) -> R<()> {
    let name = u
        .strip_prefix(prefix)
        .and_then(|v| v.strip_suffix(b".jpg"))
        .ok_or("invalid_request")?;
    let split = name
        .iter()
        .position(|b| *b == b'-')
        .ok_or("invalid_request")?;
    let timestamp = &name[..split];
    let file = &name[split + 1..];
    if timestamp.len() != 10 || !timestamp.iter().all(|b| b.is_ascii_digit()) || !digits(file) {
        return Err("invalid_request");
    }
    Ok(())
}
fn selected_images(doc: host::HtmlDescriptor, _post: &[u8], full: bool) -> R<()> {
    unsafe {
        IC = 0;
    }
    let scope = b"div.article-body > div.article-content > div.article-content-inner";
    if count(doc, scope)? != 1 {
        return Err("parse_error");
    }
    let all = b"div.article-body > div.article-content > div.article-content-inner img";
    // Both retained migrated catalogue and detail have direct img children, not p>img.
    let selected = b"div.article-body > div.article-content > div.article-content-inner > img[src]";
    let n = count(doc, all)?;
    let n2 = count(doc, selected)?;
    if n == 0 || n > MAX_IMAGES || n != n2 || (!full && n != 1) {
        return Err("parse_error");
    }
    // No embedded players/frames or active images outside the verified direct image tree.
    if count(doc,b"div.article-content-inner iframe, div.article-content-inner video, div.article-content-inner script, div.article-content-inner picture")?!=0{return Err("parse_error");}
    let mut raw = [0; MAX_IMAGES * 4];
    let got = unsafe {
        select_all_raw(
            doc.raw(),
            selected.as_ptr(),
            selected.len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if got != n as i32 {
        close_raw(&raw, got);
        return Err("html_error");
    }
    let mut error = None;
    for i in 0..n {
        let at = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[at],
            raw[at + 1],
            raw[at + 2],
            raw[at + 3],
        ]));
        let r = (|| {
            let len = host::html_attr(node, b"src", unsafe { &mut IMAGES[i] })
                .map_err(|_| "html_error")?;
            if len == 0 || len >= 256 {
                return Err("capacity_exceeded");
            }
            let u = unsafe { &IMAGES[i][..len] };
            media_file(u, if full { MEDIA } else { COVER_MEDIA }).map_err(|_| "parse_error")?;
            for j in 0..i {
                if unsafe { &IMAGES[j][..IL[j]] } == u {
                    return Err("parse_error");
                }
            }
            unsafe {
                IL[i] = len;
            }
            Ok(())
        })();
        let closed = host::html_close(node).map_err(|_| "html_error");
        if let Err(e) = r.and(closed) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    unsafe {
        IC = n;
    }
    Ok(())
}
fn category(doc: host::HtmlDescriptor) -> R<()> {
    let s = b"div.article-footer > ul.refer > li > a[href^='https://fjumonkey.pixnet.net/blog/categories/']";
    equal_attr(doc, s, b"href", CATEGORY)?;
    let mut buf = [0; 256];
    let n = text(doc, s, &mut buf)?;
    if trim_ascii(&buf[..n]) != "[專欄]四格漫畫".as_bytes() {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn article_node(doc: host::HtmlDescriptor, post: &[u8], full: bool) -> R<()> {
    let mut b = [0; 1024];
    let mut url = [0; 128];
    let len = route(post, &mut url)?;
    if !full {
        let l = attr(
            doc,
            b"ul.article-head > li.title[data-article-link]",
            b"data-article-link",
            &mut b,
        )?;
        if &b[..l] != &url[..len] {
            return Err("identity_mismatch");
        }
    }
    equal_attr(
        doc,
        b"ul.article-head > li.title > h2 > a[href]",
        b"href",
        &url[..len],
    )?;
    let n = text(doc, b"ul.article-head > li.title > h2 > a", unsafe {
        &mut TITLE
    })?;
    let v = trim_ascii(unsafe { &TITLE[..n] });
    if v.is_empty() || n >= 1024 || core::str::from_utf8(v).is_err() {
        return Err("parse_error");
    }
    let k = v.len();
    let start = v.as_ptr() as usize - unsafe { TITLE.as_ptr() } as usize;
    unsafe {
        TITLE.copy_within(start..start + k, 0);
        TL = k;
    }
    category(doc)?;
    selected_images(doc, post, full)
}
fn article_note(node: host::HtmlDescriptor) -> R<()> {
    let n = text(
        node,
        b"div.article-body > div.article-content > div.article-content-inner",
        unsafe { &mut NOTE },
    )?;
    if n >= 16384 || core::str::from_utf8(unsafe { &NOTE[..n] }).is_err() {
        return Err("capacity_exceeded");
    }
    unsafe {
        NOTE_LEN = n;
        NOTE_LINKS_LEN = 0;
    }
    let sel = b"div.article-body > div.article-content > div.article-content-inner a[href]";
    let mut raw = [0; 64];
    let total = unsafe {
        select_all_raw(
            node.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if total < 0 || total > 16 {
        close_raw(&raw, total);
        return Err("capacity_exceeded");
    }
    let mut error = None;
    for i in 0..total as usize {
        let at = i * 4;
        let d = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[at],
            raw[at + 1],
            raw[at + 2],
            raw[at + 3],
        ]));
        let r = (|| {
            let mut u = [0; 1024];
            let n = host::html_attr(d, b"href", &mut u).map_err(|_| "html_error")?;
            if n == 0
                || n >= 1024
                || !(u[..n].starts_with(b"https://") || u[..n].starts_with(b"http://"))
            {
                return Err("parse_error");
            }
            unsafe {
                let mut c = NOTE_LINKS_LEN;
                w(&mut NOTE_LINKS, &mut c, &u[..n])?;
                w(&mut NOTE_LINKS, &mut c, b"\n")?;
                NOTE_LINKS_LEN = c;
            }
            Ok(())
        })();
        let closed = host::html_close(d).map_err(|_| "html_error");
        if let Err(e) = r.and(closed) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    Ok(())
}
fn article(post: &[u8]) -> R<()> {
    let mut url = [0; 128];
    let n = route(post, &mut url)?;
    let len = fetch(&url[..n])?;
    let doc = host::html_parse(&body_buf()[..len]).map_err(|_| "html_error")?;
    let r = (|| {
        validate_identity(doc, &url[..n], post)?;
        // Detail declares BlogPosting.author, not the catalogue Person graph.
        // The original detail has one classed article without catalogue id/data-link attributes.
        let sel = b"#article-area > #article-box > div.article";
        if count(doc, sel)? != 1 {
            return Err("parse_error");
        }
        let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
        let r = article_node(node, post, true).and_then(|_| article_note(node));
        let closed = host::html_close(node).map_err(|_| "html_error");
        r.and(closed)?;
        let mut raw = [0; 16];
        let s = b"head script[type='application/ld+json']";
        let count = unsafe {
            select_all_raw(
                doc.raw(),
                s.as_ptr(),
                s.len() as u32,
                raw.as_mut_ptr(),
                raw.len() as u32,
            )
        };
        if count < 1 || count > 4 {
            close_raw(&raw, count);
            return Err("parse_error");
        }
        let mut found = 0;
        let mut error = None;
        for i in 0..count as usize {
            let at = i * 4;
            let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
                raw[at],
                raw[at + 1],
                raw[at + 2],
                raw[at + 3],
            ]));
            let r = (|| {
                let n = host::html_text(node, scratch_a()).map_err(|_| "html_error")?;
                let j = trim_ascii(&scratch_a()[..n]);
                if string(j, b"@type").ok() == Some(b"BlogPosting") {
                    found += 1;
                    json_equal(j, b"url", &url[..url_len(&url)])?;
                    let mut article_id = [0; 160];
                    let mut al = 0;
                    w(&mut article_id, &mut al, &url[..url_len(&url)])?;
                    w(&mut article_id, &mut al, b"#article")?;
                    json_equal(j, b"@id", &article_id[..al])?;
                    json_equal(
                        member(j, b"mainEntityOfPage")?,
                        b"@id",
                        &url[..url_len(&url)],
                    )?;
                    json_equal(member(j, b"author")?, b"@id", PERSON)?;
                    json_equal(j, b"headline", unsafe { &TITLE[..TL] })?;
                    json_equal(j, b"articleSection", "[專欄]四格漫畫".as_bytes())?;
                    json_equal(j, b"inLanguage", b"zh-TW")?;
                    let mut part = [0; 128];
                    let mut pc = 0;
                    w(&mut part, &mut pc, BLOG)?;
                    w(&mut part, &mut pc, b"#blog")?;
                    json_equal(member(j, b"isPartOf")?, b"@id", &part[..pc])?;
                    let d = string(j, b"datePublished")?;
                    let size = raw_string(d, unsafe { &mut DATE })?;
                    if size < 20 || size >= 64 {
                        return Err("parse_error");
                    }
                    unsafe {
                        DL = size;
                    }
                }
                Ok(())
            })();
            let closed = host::html_close(node).map_err(|_| "html_error");
            if let Err(e) = r.and(closed) {
                if error.is_none() {
                    error = Some(e);
                }
            }
        }
        if let Some(e) = error {
            return Err(e);
        }
        if found != 1 {
            return Err("identity_mismatch");
        }
        Ok(())
    })();
    let closed = host::html_close(doc).map_err(|_| "html_error");
    r.and(closed)
}
fn url_len(v: &[u8]) -> usize {
    v.iter().position(|b| *b == 0).unwrap_or(v.len())
}
fn book(p: &mut [u8], c: &mut usize, post: &[u8], full: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    ident(p, c, MID, post)?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLE[..TL] })?;
    let mut url = [0; 128];
    let len = route(post, &mut url)?;
    w(p, c, b",\"url\":")?;
    q(p, c, &url[..len])?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &IMAGES[0][..IL[0]] })?;
    w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":[")?;
    q(p, c, "輔大猴部落格".as_bytes())?;
    w(
        p,
        c,
        b"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"",
    )?;
    if full {
        w(p, c, b",\"description\":\"")?;
        for v in ["輔大猴作者部落格「[專欄]四格漫畫」的獨立作品，保留原圖、題字與版權。作品評級尚未確認。發表：".as_bytes(), unsafe{&DATE[..DL]}, "\n原文附記：\n".as_bytes(), unsafe{&NOTE[..NOTE_LEN]}, "\n原文連結（只保留文字）：\n".as_bytes(), unsafe{&NOTE_LINKS[..NOTE_LINKS_LEN]}] {
         if !append_json_escaped(p,c,v){return Err("capacity_exceeded");}
        }
        w(
            p,
            c,
            b"\",\"tags\":[],\"links\":[{\"kind\":\"source\",\"url\":",
        )?;
        q(p, c, &url[..len])?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn pagination(doc: host::HtmlDescriptor, pn: usize) -> R<()> {
    unsafe {
        NL = 0;
    }
    let mut buf = [0; 256];
    let n = text(doc, b"#article-area > div.page > span", &mut buf)?;
    if positive(trim_ascii(&buf[..n]))? != pn {
        return Err("identity_mismatch");
    }
    let sel = b"#article-area > div.page > a[href]";
    let mut raw = [0; 64];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if n < 0 || n > 16 {
        close_raw(&raw, n);
        return Err("parse_error");
    }
    let mut nexts = 0;
    let mut backs = 0;
    let mut error = None;
    for i in 0..n as usize {
        let at = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[at],
            raw[at + 1],
            raw[at + 2],
            raw[at + 3],
        ]));
        let r = (|| {
            let k = host::html_text(node, &mut buf).map_err(|_| "html_error")?;
            let label = trim_ascii(&buf[..k]);
            let forward = label == "»".as_bytes();
            let backward = label == "«".as_bytes();
            let mut u = [0; 256];
            let k = host::html_attr(node, b"href", &mut u).map_err(|_| "html_error")?;
            let target = cursor_page(&u[..k]).map_err(|_| "parse_error")?;
            if forward {
                if target != pn + 1 {
                    return Err("identity_mismatch");
                }
                nexts += 1;
                unsafe {
                    NEXT[..k].copy_from_slice(&u[..k]);
                    NL = k;
                }
            }
            if backward {
                if pn <= 1 || target != pn - 1 {
                    return Err("identity_mismatch");
                }
                backs += 1;
            }
            Ok(())
        })();
        let close = host::html_close(node).map_err(|_| "html_error");
        if let Err(e) = r.and(close) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    if nexts > 1 || backs != if pn == 1 { 0 } else { 1 } {
        return Err("parse_error");
    }
    Ok(())
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:weekly\",\"name\":\"[專欄]四格漫畫\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if arg_string(req, b"listingId")? != Some(b"listing:weekly") {
            return Err("invalid_request");
        }
        let mut url = [0; 256];
        let (len, pn) = list_route(req, &mut url)?;
        let n = fetch(&url[..len])?;
        let doc = host::html_parse(&body_buf()[..n]).map_err(|_| "html_error")?;
        let r = (|| {
            validate_identity(doc, CATEGORY, b"0")?;
            validate_author_json(doc)?;
            pagination(doc, pn)?;
            let sel = b"#article-area > #article-box > div.article[id]";
            let total = count(doc, sel)?;
            if total == 0
                || total > 24
                || count(doc, b"#article-area > #article-box > div.article")? != total
            {
                return Err("parse_error");
            }
            let mut raw = [0; 24 * 4];
            let got = unsafe {
                select_all_raw(
                    doc.raw(),
                    sel.as_ptr(),
                    sel.len() as u32,
                    raw.as_mut_ptr(),
                    raw.len() as u32,
                )
            };
            if got != total as i32 {
                close_raw(&raw, got);
                return Err("html_error");
            }
            let p = payload_buf();
            let mut c = 0;
            w(p, &mut c, b"{\"items\":[")?;
            let mut error = None;
            let mut seen = [[0; 24]; 24];
            let mut seen_len = [0usize; 24];
            for i in 0..total {
                let off = i * 4;
                let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
                    raw[off],
                    raw[off + 1],
                    raw[off + 2],
                    raw[off + 3],
                ]));
                let r = (|| {
                    let mut s = [0; 64];
                    let n = host::html_attr(node, b"id", &mut s).map_err(|_| "html_error")?;
                    let post = id(&s[..n], b"article-").map_err(|_| "parse_error")?;
                    for before in 0..i {
                        if seen_len[before] == post.len() && &seen[before][..post.len()] == post {
                            return Err("duplicate_identity");
                        }
                    }
                    seen[i][..post.len()].copy_from_slice(post);
                    seen_len[i] = post.len();
                    article_node(node, post, false)?;
                    if i > 0 {
                        w(p, &mut c, b",")?;
                    }
                    book(p, &mut c, post, false)
                })();
                let close = host::html_close(node).map_err(|_| "html_error");
                if let Err(e) = r.and(close) {
                    if error.is_none() {
                        error = Some(e);
                    }
                }
            }
            if let Some(e) = error {
                return Err(e);
            }
            w(p, &mut c, b"],\"hasNextPage\":")?;
            w(
                p,
                &mut c,
                if unsafe { NL } > 0 { b"true" } else { b"false" },
            )?;
            w(p, &mut c, b",\"page\":{\"nextCursor\":")?;
            if unsafe { NL } > 0 {
                q(p, &mut c, unsafe { &NEXT[..NL] })?;
            } else {
                w(p, &mut c, b"null")?;
            }
            w(p, &mut c, b",\"hasMore\":")?;
            w(
                p,
                &mut c,
                if unsafe { NL } > 0 { b"true" } else { b"false" },
            )?;
            w(p, &mut c, b"}}")?;
            Ok(c)
        })();
        let close = host::html_close(doc).map_err(|_| "html_error");
        match (r, close) {
            (Ok(n), Ok(())) => Ok(n),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }
    result("get_manga_list", inner(req))
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let post = request_post(req, b"mangaId", MID)?;
        article(post)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, post, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let post = request_post(req, b"mangaId", MID)?;
        if let Some(page) = direct(req, b"page").map_err(|_| "invalid_request")? {
            if page.first() != Some(&b'{') {
                return Err("invalid_request");
            }
            if !matches!(
                direct(page, b"cursor").map_err(|_| "invalid_request")?,
                None | Some(b"null")
            ) {
                return Err("invalid_request");
            }
        }
        article(post)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        ident(p, &mut c, CID, post)?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        ident(p, &mut c, MID, post)?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLE[..TL] })?;
        let mut u = [0; 128];
        let ul = route(post, &mut u)?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, &u[..ul])?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        ident(p, &mut c, MID, post)?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, unsafe { IC })?;
        w(
            p,
            &mut c,
            b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let post = request_post(req, b"chapterId", CID)?;
        article(post)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for i in 0..unsafe { IC } {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            ident(p, &mut c, PID, post)?;
            w(p, &mut c, b":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, unsafe { &IMAGES[i][..IL[i]] })?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let u = arg_string(req, b"url")?.ok_or("invalid_request")?;
        let identity = arg_string(req, b"pageId")?.ok_or("invalid_request")?;
        let post;
        if identity.starts_with(MID) {
            post = id(identity, MID)?;
            owner(req, post)?;
            if arg_string(req, b"mangaUrl")?.is_none() {
                return Err("invalid_request");
            }
            if u.starts_with(COVER_MEDIA) {
                media_file(u, COVER_MEDIA)?;
            } else {
                media_file(u, MEDIA)?;
            }
            // Cover role trusts the original catalogue pairing. Shared author covers are legal.
            // This does NOT independently revalidate post->cover mapping and makes no alias rewrite.
        } else {
            let rest = identity.strip_prefix(PID).ok_or("invalid_request")?;
            let sep = rest
                .iter()
                .position(|b| *b == b':')
                .ok_or("invalid_request")?;
            post = id(&rest[..sep], b"")?;
            owner(req, post)?;
            if arg_string(req, b"mangaUrl")?.is_none() {
                return Err("invalid_request");
            }
            let v = &rest[sep + 1..];
            let index = if v == b"0" { 0 } else { positive(v)? };
            if index >= MAX_IMAGES {
                return Err("invalid_request");
            }
            media_file(u, MEDIA)?;
            article(post)?;
            if index >= unsafe { IC } || u != unsafe { &IMAGES[index][..IL[index]] } {
                return Err("invalid_request");
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p,&mut c,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":")?;
        let mut r = [0; 128];
        let n = route(post, &mut r)?;
        q(p, &mut c, &r[..n])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_implemented", "此來源不提供搜尋")
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "")
}
const REQUEST_CAP: usize = 1024 * 1024;
static mut REQUEST: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= REQUEST_CAP || end > cap {
        return None;
    }
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= REQUEST_CAP {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_init(ptr: u32, len: u32) -> i32 {
    if checked_request(ptr, len).is_some() {
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
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            let req = match checked_request(ptr, len) {
                Some(r) => r,
                None => return write_error($op, "invalid_request", "invalid ABI range"),
            };
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match direct(req, b"args") {
                Ok(Some(a)) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"operation").ok() != Some($op.as_bytes()) {
                return write_error($op, "invalid_request", "operation mismatch");
            }
            $run(args)
        }
    };
}
export_op!(koma_source_search, "search", run_search);
export_op!(koma_source_get_manga, "get_manga", run_get_manga);
export_op!(koma_source_get_chapters, "get_chapters", run_get_chapters);
export_op!(koma_source_get_pages, "get_pages", run_get_pages);
export_op!(koma_source_get_listings, "get_listings", run_get_listings);
export_op!(
    koma_source_get_manga_list,
    "get_manga_list",
    run_get_manga_list
);
export_op!(koma_source_get_home, "get_home", run_get_home);
export_op!(koma_source_get_filters, "get_filters", run_get_filters);
export_op!(koma_source_get_settings, "get_settings", run_get_settings);
export_op!(
    koma_source_get_image_request,
    "get_image_request",
    run_get_image_request
);
