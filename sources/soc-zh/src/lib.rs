#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.soc.zh.koma",
    name: "铃兰之剑中文・官方漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Official Sword of Convallaria Chinese comics published on TapTap. Individual comic creators are unspecified.",
    content_rating: "unknown",
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
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
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
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let v = match json_object_member(args, b"page") {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        json_object_member(v, b"cursor").unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    };
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
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

const BASE: &[u8] =
    b"https://www.taptap.cn/app/209866/topic?group_label_id=946587&type=group_label";
const MEDIA: &[u8] = b"https://img2-tc.tapimg.com/moment/etag/";
const PAGE_SIZE: usize = 10;
const MAX_IMAGES: usize = 16;
const CANONICAL_FEED: &[u8] = b"https://www.taptap.cn/app/209866/topic?type=group_label";
const PAGE_LINK: &[u8] = b"&page=";
const LISTING_ID: &[u8] = b"listing:current-first-page";
const INDEX_CAP: usize = 4096;
static mut JSON: [u8; 1024 * 1024] = [0; 1024 * 1024];
static mut START: [usize; INDEX_CAP] = [0; INDEX_CAP];
static mut END: [usize; INDEX_CAP] = [0; INDEX_CAP];
static mut NODE_COUNT: usize = 0;
#[derive(Clone, Copy)]
struct Work {
    id: [u8; 20],
    il: usize,
    title: [u8; 384],
    tl: usize,
    images: [Image; MAX_IMAGES],
    image_count: usize,
}
#[derive(Clone, Copy)]
struct Image {
    url: [u8; 256],
    len: usize,
}
impl Image {
    fn url(&self) -> &[u8] {
        &self.url[..self.len]
    }
}
const EMPTY_IMAGE: Image = Image { url: [0; 256], len: 0 };
const ZERO: Work = Work {
    id: [0; 20],
    il: 0,
    title: [0; 384],
    tl: 0,
    images: [EMPTY_IMAGE; MAX_IMAGES],
    image_count: 0,
};
static mut WORKS: [Work; PAGE_SIZE] = [ZERO; PAGE_SIZE];
fn fetch(url: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), url, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
// Data parsing only: no JavaScript evaluation, API requests, tokens or hydration.
// Flat Nuxt devalue entries are indexed by their original array positions.
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
fn array_entries(s: &[u8], out: &mut [usize]) -> R<usize> {
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'[') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    let mut n = 0;
    if s.get(i) == Some(&b']') {
        if whitespace(s, i + 1) != s.len() {
            return Err("parse_error");
        }
        return Ok(0);
    }
    loop {
        let e = end_value(s, i)?;
        if n >= out.len() {
            return Err("capacity_exceeded");
        }
        out[n] = number(trim_ascii(&s[i..e]))?;
        n += 1;
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => {
                i = whitespace(s, i + 1);
                if s.get(i) == Some(&b']') {
                    return Err("parse_error");
                }
            }
            Some(b']') => {
                if whitespace(s, i + 1) != s.len() {
                    return Err("parse_error");
                }
                return Ok(n);
            }
            _ => return Err("parse_error"),
        }
    }
}
fn node(i: usize) -> R<&'static [u8]> {
    unsafe {
        if i >= NODE_COUNT {
            return Err("invalid_index");
        }
        Ok(&JSON[START[i]..END[i]])
    }
}
// A requested direct member must appear exactly once. Duplicate ownership fields fail closed.
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
        None | Some(b"null") => Ok(None),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
            Ok(Some(&v[1..v.len() - 1]))
        }
        _ => Err("invalid_request"),
    }
}
fn link(s: &[u8], key: &[u8]) -> R<&'static [u8]> {
    node(number(direct(s, key)?.ok_or("missing_identity")?)?)
}
fn decoded(v: &[u8], out: &mut [u8]) -> R<usize> {
    if v.len() < 2 || v[0] != b'"' || v[v.len() - 1] != b'"' {
        return Err("parse_error");
    }
    raw_string(&v[1..v.len() - 1], out)
}
fn eqstr(v: &[u8], wanted: &[u8]) -> R<()> {
    let mut b = [0; 512];
    let n = decoded(v, &mut b)?;
    if &b[..n] != wanted {
        return Err("wrong_owner");
    }
    Ok(())
}
fn eqnum(v: &[u8], wanted: usize) -> R<()> {
    if number(v)? != wanted {
        return Err("wrong_owner");
    }
    Ok(())
}
fn index_json(doc: host::HtmlDescriptor) -> R<()> {
    let n = text(
        doc,
        b"script#__NUXT_DATA__[type='application/json'][data-ssr='true']",
        unsafe {
            core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(JSON).cast::<u8>(), 1024 * 1024)
        },
    )?;
    unsafe {
        NODE_COUNT = 0;
    }
    let s = unsafe { &JSON[..n] };
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'[') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    if s.get(i) == Some(&b']') {
        return Err("parse_error");
    }
    loop {
        let e = end_value(s, i)?;
        let count = unsafe { NODE_COUNT };
        if count >= INDEX_CAP {
            return Err("capacity_exceeded");
        }
        unsafe {
            START[count] = i;
            END[count] = e;
            NODE_COUNT = count + 1;
        }
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => {
                i = whitespace(s, i + 1);
                if s.get(i) == Some(&b']') {
                    return Err("parse_error");
                }
            }
            Some(b']') => {
                if whitespace(s, i + 1) != n {
                    return Err("parse_error");
                }
                break;
            }
            _ => return Err("parse_error"),
        }
    }
    if node(0)? != b"[\"ShallowReactive\",1]" {
        return Err("parse_error");
    }
    if link(node(1)?, b"serverRendered")? != b"true" {
        return Err("parse_error");
    }
    Ok(())
}
fn data_root() -> R<&'static [u8]> {
    let tagged = link(node(1)?, b"data")?;
    if !tagged.starts_with(b"[\"ShallowReactive\",") || tagged.last() != Some(&b']') {
        return Err("parse_error");
    }
    node(number(
        &tagged[b"[\"ShallowReactive\",".len()..tagged.len() - 1],
    )?)
}
fn container(detail: bool) -> R<&'static [u8]> {
    let s = data_root()?;
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'{') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    let mut result = None;
    let mut count = 0;
    loop {
        if s.get(i) != Some(&b'"') {
            return Err("parse_error");
        }
        let e = end_value(s, i)?;
        i = whitespace(s, e);
        if s.get(i) != Some(&b':') {
            return Err("parse_error");
        }
        i = whitespace(s, i + 1);
        let e = end_value(s, i)?;
        let v = node(number(&s[i..e])?)?;
        if v.first() == Some(&b'{')
            && if detail {
                direct(v, b"first_post")?.is_some() && direct(v, b"moment")?.is_some()
            } else {
                direct(v, b"list")?.is_some() && direct(v, b"total")?.is_some()
            }
        {
            if result.is_some() {
                return Err("duplicate_identity");
            }
            result = Some(v);
        }
        count += 1;
        if count > 16 {
            return Err("capacity_exceeded");
        }
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => i = whitespace(s, i + 1),
            Some(b'}') => {
                if whitespace(s, i + 1) != s.len() {
                    return Err("parse_error");
                }
                break;
            }
            _ => return Err("parse_error"),
        }
    }
    result.ok_or("missing_identity")
}
fn valid_id(id: &[u8]) -> R<()> {
    if id.len() != 18 || id[0] == b'0' || !id.iter().all(|b| b.is_ascii_digit()) {
        return Err("invalid_request");
    }
    Ok(())
}
fn media(url: &[u8]) -> R<()> {
    let tail = url.strip_prefix(MEDIA).ok_or("wrong_owner")?;
    let stem = tail
        .strip_suffix(b".png")
        .or_else(|| tail.strip_suffix(b".jpg"))
        .ok_or("wrong_owner")?;
    if stem.is_empty()
        || stem.len() > 120
        || !stem
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'-'))
    {
        return Err("wrong_owner");
    }
    Ok(())
}
fn parse_image(s: &[u8], out: &mut [u8]) -> R<usize> {
    let n = decoded(link(s, b"original_url")?, out)?;
    media(&out[..n])?;
    let mut actual = [0; 256];
    let len = decoded(link(s, b"url")?, &mut actual)?;
    let mut expected = [0; 280];
    let mut c = 0;
    w(&mut expected, &mut c, &out[..n])?;
    w(&mut expected, &mut c, b"/_tap_ugc.jpg")?;
    if actual[..len] != expected[..c] {
        return Err("wrong_owner");
    }
    if number(link(s, b"width")?)? == 0 || number(link(s, b"height")?)? == 0 {
        return Err("incomplete_pages");
    }
    eqstr(
        link(s, b"original_format")?,
        if out[..n].ends_with(b".png") {
            b"png"
        } else {
            b"jpeg"
        },
    )?;
    Ok(n)
}
fn parse_work(s: &[u8]) -> R<Work> {
    if link(s, b"is_official")? != b"true" {
        return Err("wrong_owner");
    }
    let group = link(s, b"group")?;
    eqnum(link(group, b"id")?, 291080)?;
    eqnum(link(group, b"app_id")?, 209866)?;
    let label = link(s, b"group_label")?;
    eqnum(link(label, b"id")?, 946587)?;
    eqstr(link(label, b"name")?, "趣味漫画".as_bytes())?;
    let author = link(s, b"author")?;
    let app = link(author, b"app")?;
    eqnum(link(app, b"id")?, 209866)?;
    eqstr(link(app, b"identifier")?, b"com.xd.ssrpg")?;
    let user = link(author, b"user")?;
    eqnum(link(user, b"id")?, 391283780)?;
    eqstr(
        link(link(user, b"verified")?, b"reason")?,
        "《铃兰之剑》官方".as_bytes(),
    )?;
    let mut work = ZERO;
    work.il = decoded(link(s, b"id_str")?, &mut work.id)?;
    valid_id(&work.id[..work.il])?;
    let topic = link(s, b"topic")?;
    work.tl = decoded(link(topic, b"title")?, &mut work.title)?;
    let title = core::str::from_utf8(&work.title[..work.tl]).map_err(|_| "parse_error")?;
    // The verified official account also published this three-image special under an unbracketed title.
    if !title.starts_with("【伊利亚没问题！】") && title != "伊利亚没问题！这次真的没问题！！" {
        return Err("wrong_owner");
    }
    work.image_count = parse_images(topic, &mut work.images)?;
    Ok(work)
}
fn parse_images(parent: &[u8], images: &mut [Image; MAX_IMAGES]) -> R<usize> {
    let mut ids = [0; MAX_IMAGES];
    let count = array_entries(link(parent, b"images")?, &mut ids)?;
    if count == 0 {
        return Err("incomplete_pages");
    }
    for (id, image) in ids[..count].iter().zip(images.iter_mut()) {
        image.len = parse_image(node(*id)?, &mut image.url)?;
    }
    Ok(count)
}
fn feed_url(dst: &mut [u8], base: &[u8], page: usize) -> R<usize> {
    let mut len = 0;
    w(dst, &mut len, base)?;
    if page > 1 {
        w(dst, &mut len, PAGE_LINK)?;
        if !write_usize(dst, &mut len, page) {
            return Err("capacity_exceeded");
        }
    }
    Ok(len)
}
fn check_feed_link(doc: host::HtmlDescriptor, selector: &[u8], key: &[u8], base: &[u8], page: usize) -> R<()> {
    let mut actual = [0; 128];
    let size = attr(doc, selector, key, &mut actual)?;
    let mut expected = [0; 128];
    let len = feed_url(&mut expected, base, page)?;
    if actual[..size] != expected[..len] {
        return Err("wrong_owner");
    }
    Ok(())
}
fn catalogue(page: usize) -> R<(usize, bool)> {
    let mut uri = [0; 128];
    let len = feed_url(&mut uri, BASE, page)?;
    let size = fetch(&uri[..len])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        index_json(doc)?;
        let feed = container(false)?;
        let total = number(link(feed, b"total")?)?;
        let offset = (page - 1).checked_mul(PAGE_SIZE).ok_or("invalid_request")?;
        let remaining = total.checked_sub(offset).ok_or("not_found")?;
        if remaining == 0 && page != 1 {
            return Err("not_found");
        }
        // These are the public SSR navigation links, not the signed API cursors.
        check_feed_link(doc, b"link[rel='canonical'][href]", b"href", CANONICAL_FEED, page)?;
        if page > 1 {
            check_feed_link(doc, b"link[rel='prev'][href]", b"href", BASE, page - 1)?;
        }
        let has_more = remaining > PAGE_SIZE;
        if has_more {
            check_feed_link(doc, b"link[rel='next'][href]", b"href", BASE, page + 1)?;
        } else if count(doc, b"link[rel='next']")? != 0 {
            return Err("catalogue_changed");
        }
        let mut ids = [0; PAGE_SIZE];
        let rows = array_entries(link(feed, b"list")?, &mut ids)?;
        if rows != core::cmp::min(remaining, PAGE_SIZE) {
            return Err("catalogue_changed");
        }
        for (i, id) in ids[..rows].iter().enumerate() {
            let item = node(*id)?;
            let work = parse_work(link(item, b"moment")?)?;
            let mut ident = [0; 32];
            let k = decoded(link(item, b"identification")?, &mut ident)?;
            if ident[..k].strip_prefix(b"moment:") != Some(&work.id[..work.il]) {
                return Err("wrong_owner");
            }
            for j in 0..i {
                let other = unsafe { WORKS[j] };
                if other.id[..other.il] == work.id[..work.il] {
                    return Err("duplicate_identity");
                }
            }
            let mut sel = [0; 128];
            let mut c = 0;
            w(&mut sel, &mut c, b"a.moment-article__image-list[href='/moment/")?;
            w(&mut sel, &mut c, &work.id[..work.il])?;
            w(&mut sel, &mut c, b"?group_id=291080']")?;
            if count(doc, &sel[..c])? != 1 {
                return Err("wrong_owner");
            }
            unsafe {
                WORKS[i] = work;
            }
        }
        Ok((rows, has_more))
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn route(dst: &mut [u8], c: &mut usize, id: &[u8], group: bool) -> R<()> {
    w(dst, c, b"https://www.taptap.cn/moment/")?;
    w(dst, c, id)?;
    if group {
        w(dst, c, b"?group_id=291080")?;
    }
    Ok(())
}
fn pref(dst: &mut [u8], c: &mut usize, prefix: &[u8], id: &[u8]) -> R<()> {
    w(dst, c, prefix)?;
    w(dst, c, id)
}
fn sid<'a>(id: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let v = id.strip_prefix(prefix).ok_or("invalid_request")?;
    valid_id(v)?;
    Ok(v)
}
fn owner(req: &[u8], id: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        let mut u = [0; 256];
        let n = raw_string(v, &mut u)?;
        let mut expected = [0; 256];
        let mut c = 0;
        route(&mut expected, &mut c, id, true)?;
        if u[..n] != expected[..c] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaId")? {
        if sid(v, b"soc:zh:comic:")? != id {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn body_images_match(doc: host::HtmlDescriptor, work: &Work) -> R<()> {
    const WRAPPER: &[u8] = b".tap-rich-content__wrapper.moment-detail__module-space";
    const IMAGES: &[u8] = b".tap-rich-content__wrapper.moment-detail__module-space .tap-rich-content__image--topic img";
    const SOURCES: &[u8] = b".tap-rich-content__wrapper.moment-detail__module-space .tap-rich-content__image--topic img[src]:not([src^='data:']),.tap-rich-content__wrapper.moment-detail__module-space .tap-rich-content__image--topic img.lazy-image[src^='data:'][data-src]";
    const LAZY_PLACEHOLDER: &[u8] = b"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGP6zwAAAgcBApocMXEAAAAASUVORK5CYII=";
    if count(doc, WRAPPER)? != 1 || count(doc, IMAGES)? != work.image_count {
        return Err("incomplete_pages");
    }
    let mut descriptors = [0; MAX_IMAGES * 4];
    let rows = unsafe {
        select_all_raw(doc.raw(), SOURCES.as_ptr(), SOURCES.len() as u32, descriptors.as_mut_ptr(), descriptors.len() as u32)
    };
    if rows < 0 {
        return Err("html_error");
    }
    let rows = rows as usize;
    let mut result = if rows == work.image_count { Ok(()) } else { Err("incomplete_pages") };
    for (i, raw) in descriptors.chunks_exact(4).take(rows).enumerate() {
        let image = host::HtmlDescriptor::from_raw(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]));
        if result.is_ok() {
            result = (|| {
                let mut src = [0; 280];
                let mut len = host::html_attr(image, b"src", &mut src).map_err(|_| "html_error")?;
                if &src[..len] == LAZY_PLACEHOLDER {
                    len = host::html_attr(image, b"data-src", &mut src).map_err(|_| "html_error")?;
                }
                let original = work.images[i].url();
                if src[..len].strip_suffix(b"/_tap_ugc.jpg") != Some(original) {
                    return Err("wrong_owner");
                }
                Ok(())
            })();
        }
        if host::html_close(image).is_err() && result.is_ok() {
            result = Err("html_error");
        }
    }
    result
}
fn fetch_work(id: &[u8]) -> R<Work> {
    // The detail verifies the same official publisher, group, topic and image identity
    // independently, so a saved item never has to rescan all listing pages.
    let mut uri = [0; 128];
    let mut c = 0;
    route(&mut uri, &mut c, id, true)?;
    let size = fetch(&uri[..c])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        index_json(doc)?;
        let entry = container(true)?;
        let moment = link(entry, b"moment")?;
        let work = parse_work(moment)?;
        if &work.id[..work.il] != id {
            return Err("wrong_owner");
        }
        let post = link(entry, b"first_post")?;
        if link(post, b"is_topic")? != b"true" {
            return Err("wrong_owner");
        }
        eqnum(link(post, b"position")?, 1)?;
        let moment_topic = link(moment, b"topic")?;
        let mut id1 = [0; 32];
        let mut id2 = [0; 32];
        let a = decoded(link(moment_topic, b"id_str")?, &mut id1)?;
        let b = decoded(link(post, b"id_str")?, &mut id2)?;
        // The first-post ID is a distinct post identity; it is NOT the topic ID.
        if a == 0 || b == 0
            || id1[..a].iter().any(|b| !b.is_ascii_digit())
            || id2[..b].iter().any(|b| !b.is_ascii_digit())
        {
            return Err("missing_identity");
        }
        let mut images = [EMPTY_IMAGE; MAX_IMAGES];
        if parse_images(post, &mut images)? != work.image_count {
            return Err("incomplete_pages");
        }
        for (topic, post) in work.images[..work.image_count].iter().zip(&images) {
            if topic.url() != post.url() {
                return Err("wrong_owner");
            }
        }
        let mut canonical = [0; 128];
        let size = attr(doc, b"meta[property='og:url'][content]", b"content", &mut canonical)?;
        let mut expected = [0; 128];
        let mut len = 0;
        route(&mut expected, &mut len, id, false)?;
        if canonical[..size] != expected[..len] {
            return Err("wrong_owner");
        }
        body_images_match(doc, &work)?;
        if count(doc, b".from-user-name[href='/user/391283780']")? != 1 {
            return Err("wrong_owner");
        }
        Ok(work)
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn book(dst: &mut [u8], c: &mut usize, work: &Work, detail: bool) -> R<()> {
    w(dst, c, b"{\"id\":\"")?;
    pref(dst, c, b"soc:zh:comic:", &work.id[..work.il])?;
    w(dst, c, b"\",\"title\":")?;
    q(dst, c, &work.title[..work.tl])?;
    w(dst, c, b",\"url\":\"")?;
    route(dst, c, &work.id[..work.il], true)?;
    w(dst, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(dst, c, work.images[0].url())?;
    // This original is an ordinary anonymous GET. Old clients still resolve
    // the same bytes through get_image_request.
    w(dst,c,b",\"headers\":{}},\"language\":\"zh\",\"authors\":[],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(dst, c, b",\"description\":")?;
        q(dst,c,"铃兰之剑官方认证发布者的《伊利亚没问题！》漫画。发布者不等于逐篇创作者，未确认作者与正式评级。".as_bytes())?;
        w(dst, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(dst, c, &work.id[..work.il], true)?;
        w(dst, c, b"\"}]")?;
    }
    w(dst, c, b"}")
}
fn listing(page: usize, query: &[u8]) -> R<usize> {
    let (rows, has_more) = catalogue(page)?;
    let dst = payload_buf();
    let mut c = 0;
    let mut written = false;
    w(dst, &mut c, b"{\"items\":[")?;
    for i in 0..rows {
        let work = unsafe { &WORKS[i] };
        if !query.is_empty() && !contains_bytes(&work.title[..work.tl], query) {
            continue;
        }
        if written {
            w(dst, &mut c, b",")?;
        }
        book(dst, &mut c, work, false)?;
        written = true;
    }
    if has_more {
        w(dst, &mut c, b"],\"page\":{\"nextCursor\":\"")?;
        if !write_usize(dst, &mut c, page + 1) {
            return Err("capacity_exceeded");
        }
        w(dst, &mut c, b"\",\"hasMore\":true}}")?;
    } else {
        w(dst, &mut c, b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
    }
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let mut query = [0; 512];
    let size = match raw_string(
        match arg_string(req, b"query") {
            Ok(v) => v.unwrap_or(b""),
            Err(e) => return write_error("search", e, ""),
        },
        &mut query,
    ) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    result("search", page(req).and_then(|page| listing(page, &query[..size])))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId") != Ok(Some(LISTING_ID)) {
        return write_error("get_manga_list", "invalid_request", "Unknown listing");
    }
    result("get_manga_list", page(req).and_then(|page| listing(page, b"")))
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<Work> {
    let id = sid(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, id)?;
    fetch_work(id)
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let work = request_work(req, b"mangaId", b"soc:zh:comic:")?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, &work, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let work = request_work(req, b"mangaId", b"soc:zh:comic:")?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        pref(p, &mut c, b"socread:zh:comic:", &work.id[..work.il])?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        pref(p, &mut c, b"soc:zh:comic:", &work.id[..work.il])?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, &work.title[..work.tl])?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, &work.id[..work.il], true)?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        pref(p, &mut c, b"soc:zh:comic:", &work.id[..work.il])?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh\",\"pageCount\":",
        )?;
        if !write_usize(p, &mut c, work.image_count) {
            return Err("capacity_exceeded");
        }
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let work = request_work(req, b"chapterId", b"socread:zh:comic:")?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for (index, image) in work.images[..work.image_count].iter().enumerate() {
            if index != 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            pref(p, &mut c, b"socpage:zh:comic:", &work.id[..work.il])?;
            w(p, &mut c, b":")?;
            if !write_usize(p, &mut c, index) {
                return Err("capacity_exceeded");
            }
            w(p, &mut c, b"\",\"index\":")?;
            if !write_usize(p, &mut c, index) {
                return Err("capacity_exceeded");
            }
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, image.url())?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn verify_image(req: &[u8], url: &[u8]) -> R<()> {
    let page_id = match arg_string(req, b"pageId")? {
        Some(page_id) => page_id,
        None => {
            // An old id-less cover request had access only to the first feed page.
            let (rows, _) = catalogue(1)?;
            let mut matching = None;
            for i in 0..rows {
                let work = unsafe { &WORKS[i] };
                if work.images[0].url() == url && matching.replace(&work.id[..work.il]).is_some() {
                    return Err("invalid_request");
                }
            }
            return owner(req, matching.ok_or("invalid_request")?);
        }
    };
    let (id, index) = if let Some(page) = page_id.strip_prefix(b"socpage:zh:comic:") {
        let split = page.iter().position(|b| *b == b':').ok_or("invalid_request")?;
        let number_text = &page[split + 1..];
        if number_text.len() > 1 && number_text[0] == b'0' {
            return Err("invalid_request");
        }
        let index = number(number_text).map_err(|_| "invalid_request")?;
        if index >= MAX_IMAGES {
            return Err("invalid_request");
        }
        (sid(&page[..split], b"")?, index)
    } else {
        (sid(page_id, b"soc:zh:comic:")?, 0)
    };
    owner(req, id)?;
    let work = fetch_work(id)?;
    if index >= work.image_count || work.images[index].url() != url {
        return Err("invalid_request");
    }
    Ok(())
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 512];
        let size = raw_string(arg_string(req, b"url")?.ok_or("invalid_request")?, &mut url)?;
        let url = &url[..size];
        media(url).map_err(|_| "invalid_request")?;
        verify_image(req, url)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b="{\"listings\":[{\"id\":\"listing:current-first-page\",\"name\":\"官方漫画\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
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
            if arg_string(req, b"operation") != Ok(Some($op.as_bytes())) {
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
