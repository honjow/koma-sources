#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.protectors.zh.koma",
    name: "那些需要守护的中文・官方漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "《那些需要守护的》官方漫画。收录当前栏目中5篇有明确官方发布标记的漫画。创作者和作品评级尚未确认。",
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
    b"https://www.taptap.cn/app/167261/topic?group_label_id=476119&type=group_label";
const MEDIA: &[u8] = b"https://img2-tc.tapimg.com/bbcode/images/";
const LIMIT: usize = 5;
const FEED_LIMIT: usize = 6;
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
    image: [u8; 256],
    ul: usize,
    width: usize,
    height: usize,
    author_user: usize,
}
const ZERO: Work = Work {
    id: [0; 20],
    il: 0,
    title: [0; 384],
    tl: 0,
    image: [0; 256],
    ul: 0,
    width: 0,
    height: 0,
    author_user: 0,
};
static mut WORKS: [Work; LIMIT] = [ZERO; LIMIT];
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
    if !matches!(id.len(), 17 | 18) || id[0] == b'0' || !id.iter().all(|b| b.is_ascii_digit()) {
        return Err("invalid_request");
    }
    Ok(())
}
fn media(url: &[u8]) -> R<()> {
    let tail = url.strip_prefix(MEDIA).ok_or("wrong_owner")?;
    let stem = tail
        .strip_suffix(b".jpg")
        .or_else(|| tail.strip_suffix(b".JPG"))
        .ok_or("wrong_owner")?;
    if stem.len() != 32
        || !stem
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
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
    let width = number(link(s, b"width")?)?;
    let height = number(link(s, b"height")?)?;
    if width == 0 || height == 0 || width > 32768 || height > 100000 {
        return Err("parse_error");
    }
    eqstr(link(s, b"original_format")?, b"jpeg")?;
    Ok(n)
}
fn parse_work(s: &[u8]) -> R<Work> {
    if link(s, b"is_official")? != b"true" {
        return Err("wrong_owner");
    }
    let group = link(s, b"group")?;
    eqnum(link(group, b"id")?, 200225)?;
    eqnum(link(group, b"app_id")?, 167261)?;
    let label = link(s, b"group_label")?;
    eqnum(link(label, b"id")?, 476119)?;
    eqstr(link(label, b"name")?, "四格漫画".as_bytes())?;
    let author = link(s, b"author")?;
    let app = link(author, b"app")?;
    eqnum(link(app, b"id")?, 167261)?;
    eqstr(link(app, b"identifier")?, b"com.hegshy.nxxyshd.tt")?;
    let user = link(author, b"user")?;
    let user_id = number(link(user, b"id")?)?;
    if user_id == 0 {
        return Err("missing_identity");
    }
    let mut bindings = [0; 16];
    let n = array_entries(link(s, b"forum_bindings")?, &mut bindings)?;
    let mut primary = 0;
    for idx in bindings[..n].iter() {
        let binding = node(*idx)?;
        if link(binding, b"is_primary")? == b"true" {
            primary += 1;
            if link(binding, b"is_official")? != b"true" {
                return Err("wrong_owner");
            }
            let g = link(binding, b"group")?;
            eqnum(link(g, b"id")?, 200225)?;
            eqnum(link(g, b"app_id")?, 167261)?;
            let l = link(binding, b"group_label")?;
            eqnum(link(l, b"id")?, 476119)?;
        }
    }
    if primary != 1 {
        return Err("wrong_owner");
    }
    let mut work = ZERO;
    work.author_user = user_id;
    work.il = decoded(link(s, b"id_str")?, &mut work.id)?;
    valid_id(&work.id[..work.il])?;
    let topic = link(s, b"topic")?;
    work.tl = decoded(link(topic, b"title")?, &mut work.title)?;
    if work.tl == 0 || core::str::from_utf8(&work.title[..work.tl]).is_err() {
        return Err("wrong_owner");
    }
    let mut images = [0; 2];
    if array_entries(link(topic, b"images")?, &mut images)? != 1 {
        return Err("incomplete_pages");
    }
    work.ul = parse_image(node(images[0])?, &mut work.image)?;
    work.width = number(link(node(images[0])?, b"width")?)?;
    work.height = number(link(node(images[0])?, b"height")?)?;
    Ok(work)
}
fn catalogue() -> R<()> {
    let size = fetch(BASE)?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        index_json(doc)?;
        let mut ids = [0; FEED_LIMIT];
        let container = container(false)?;
        eqnum(link(container, b"total")?, FEED_LIMIT)?;
        if direct(container, b"next_page")?.is_some() || count(doc, b"link[rel=next]")? != 0 {
            return Err("catalogue_changed");
        }
        if array_entries(link(container, b"list")?, &mut ids)? != FEED_LIMIT {
            return Err("catalogue_changed");
        }
        let mut accepted = 0;
        for item_index in ids.iter() {
            let item = node(*item_index)?;
            let moment = link(item, b"moment")?;
            // Missing official flag is retained externally, never inferred from a verified user.
            let Some(flag) = direct(moment, b"is_official")? else {
                continue;
            };
            if node(number(flag)?)? != b"true" {
                continue;
            }
            if accepted >= LIMIT {
                return Err("catalogue_changed");
            }
            let i = accepted;
            let work = parse_work(moment)?;
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
            let mut sel = [0; 256];
            let mut c = 0;
            w(
                &mut sel,
                &mut c,
                b"a.moment-article__image-list[href='/moment/",
            )?;
            w(&mut sel, &mut c, &work.id[..work.il])?;
            w(&mut sel, &mut c, b"?group_id=200225']")?;
            if count(doc, &sel[..c])? != 1 {
                return Err("wrong_owner");
            }
            unsafe {
                WORKS[i] = work;
            }
            accepted += 1;
        }
        if accepted != LIMIT {
            return Err("catalogue_changed");
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn find(id: &[u8]) -> R<usize> {
    for i in 0..LIMIT {
        let w = unsafe { WORKS[i] };
        if &w.id[..w.il] == id {
            return Ok(i);
        }
    }
    Err("not_found")
}
fn route(dst: &mut [u8], c: &mut usize, id: &[u8], group: bool) -> R<()> {
    w(dst, c, b"https://www.taptap.cn/moment/")?;
    w(dst, c, id)?;
    if group {
        w(dst, c, b"?group_id=200225")?;
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
        if sid(v, b"protectors:zh:comic:")? != id {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn current_image(i: usize) -> R<()> {
    let work = unsafe { WORKS[i] };
    let mut uri = [0; 256];
    let mut c = 0;
    route(&mut uri, &mut c, &work.id[..work.il], true)?;
    let size = fetch(&uri[..c])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        index_json(doc)?;
        let entry = container(true)?;
        let actual = parse_work(link(entry, b"moment")?)?;
        if actual.id[..actual.il] != work.id[..work.il]
            || actual.title[..actual.tl] != work.title[..work.tl]
            || actual.image[..actual.ul] != work.image[..work.ul]
            || actual.width != work.width
            || actual.height != work.height
            || actual.author_user != work.author_user
        {
            return Err("wrong_owner");
        }
        let post = link(entry, b"first_post")?;
        if link(post, b"is_topic")? != b"true" {
            return Err("wrong_owner");
        }
        eqnum(link(post, b"position")?, 1)?;
        let moment_topic = link(link(entry, b"moment")?, b"topic")?;
        let mut id1 = [0; 32];
        let mut id2 = [0; 32];
        let a = decoded(link(moment_topic, b"id_str")?, &mut id1)?;
        let b = decoded(link(post, b"id_str")?, &mut id2)?;
        // The first-post ID is a distinct post identity; it is NOT the topic ID.
        if a == 0
            || b == 0
            || id1[..a].iter().any(|b| !b.is_ascii_digit())
            || id2[..b].iter().any(|b| !b.is_ascii_digit())
        {
            return Err("missing_identity");
        }
        let mut images = [0; 2];
        if array_entries(link(post, b"images")?, &mut images)? != 1 {
            return Err("incomplete_pages");
        }
        let mut url = [0; 256];
        let n = parse_image(node(images[0])?, &mut url)?;
        eqnum(link(node(images[0])?, b"width")?, work.width)?;
        eqnum(link(node(images[0])?, b"height")?, work.height)?;
        if url[..n] != work.image[..work.ul] {
            return Err("wrong_owner");
        }
        let mut content_rows = [0; 64];
        let content_count =
            array_entries(link(link(post, b"contents")?, b"json")?, &mut content_rows)?;
        let mut body_image_count = 0;
        for row in content_rows[..content_count].iter() {
            let row = node(*row)?;
            let mut kind = [0; 32];
            let length = decoded(link(row, b"type")?, &mut kind)?;
            if &kind[..length] != b"image" {
                continue;
            }
            body_image_count += 1;
            let info = link(row, b"info")?;
            let mut original = [0; 256];
            let length = decoded(link(info, b"img_url")?, &mut original)?;
            if original[..length] != work.image[..work.ul] {
                return Err("wrong_owner");
            }
            let image = link(info, b"image")?;
            let length = parse_image(image, &mut original)?;
            if original[..length] != work.image[..work.ul] {
                return Err("wrong_owner");
            }
            eqnum(link(image, b"width")?, work.width)?;
            eqnum(link(image, b"height")?, work.height)?;
        }
        if body_image_count != 1 {
            return Err("incomplete_pages");
        }
        let mut canonical = [0; 256];
        let size = attr(
            doc,
            b"meta[property='og:url'][content]",
            b"content",
            &mut canonical,
        )?;
        let mut expected = [0; 256];
        let mut len = 0;
        route(&mut expected, &mut len, &work.id[..work.il], false)?;
        if canonical[..size] != expected[..len] {
            return Err("wrong_owner");
        }
        if count(doc,b".tap-rich-content__wrapper.moment-detail__module-space")?!=1||count(doc,b".tap-rich-content__wrapper.moment-detail__module-space .tap-rich-content__image--topic img")?!=1{return Err("incomplete_pages");}
        let len=attr(doc,b".tap-rich-content__wrapper.moment-detail__module-space .tap-rich-content__image--topic img[src]",b"src",&mut url)?;
        let mut expected = [0; 280];
        let mut c = 0;
        w(&mut expected, &mut c, &work.image[..work.ul])?;
        w(&mut expected, &mut c, b"/_tap_ugc.jpg")?;
        if url[..len] != expected[..c] {
            return Err("wrong_owner");
        }
        let mut user_selector = [0; 128];
        let mut uc = 0;
        w(&mut user_selector, &mut uc, b".from-user-name[href='/user/")?;
        if !write_usize(&mut user_selector, &mut uc, work.author_user) {
            return Err("capacity_exceeded");
        }
        w(&mut user_selector, &mut uc, b"']")?;
        if count(doc, &user_selector[..uc])? != 1 {
            return Err("wrong_owner");
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn book(dst: &mut [u8], c: &mut usize, i: usize, detail: bool) -> R<()> {
    let work = unsafe { WORKS[i] };
    w(dst, c, b"{\"id\":\"")?;
    pref(dst, c, b"protectors:zh:comic:", &work.id[..work.il])?;
    w(dst, c, b"\",\"title\":")?;
    q(dst, c, &work.title[..work.tl])?;
    w(dst, c, b",\"url\":\"")?;
    route(dst, c, &work.id[..work.il], true)?;
    w(dst, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(dst, c, &work.image[..work.ul])?;
    w(dst,c,b"},\"language\":\"zh\",\"authors\":[],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(dst, c, b",\"description\":")?;
        q(dst,c,"收录当前栏目中5篇有明确官方发布标记的漫画。创作者和作品评级尚未确认。".as_bytes())?;
        w(dst, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(dst, c, &work.id[..work.il], true)?;
        w(dst, c, b"\"}]")?;
    }
    w(dst, c, b"}")
}
fn listing(query: &[u8]) -> R<usize> {
    catalogue()?;
    let dst = payload_buf();
    let mut c = 0;
    let mut written = false;
    w(dst, &mut c, b"{\"items\":[")?;
    for i in 0..LIMIT {
        let work = unsafe { WORKS[i] };
        if !query.is_empty() && !contains_bytes(&work.title[..work.tl], query) {
            continue;
        }
        if written {
            w(dst, &mut c, b",")?;
        }
        book(dst, &mut c, i, false)?;
        written = true;
    }
    w(
        dst,
        &mut c,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false},\"supportsPaging\":false}",
    )?;
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
    if page(req) != Ok(1) {
        return write_error(
            "search",
            "invalid_request",
            "Only the verified five-entry official comic subset is supported",
        );
    }
    result("search", listing(&query[..size]))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId") != Ok(Some(b"listing:official-comics".as_slice()))
        || page(req) != Ok(1)
    {
        return write_error(
            "get_manga_list",
            "invalid_request",
            "Only the verified five-entry official comic subset is supported",
        );
    }
    result("get_manga_list", listing(b""))
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let id = sid(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, id)?;
    catalogue()?;
    find(id)
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"protectors:zh:comic:")?;
        current_image(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, i, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"protectors:zh:comic:")?;
        current_image(i)?;
        let work = unsafe { WORKS[i] };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        pref(p, &mut c, b"protectorsread:zh:comic:", &work.id[..work.il])?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        pref(p, &mut c, b"protectors:zh:comic:", &work.id[..work.il])?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, &work.title[..work.tl])?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, &work.id[..work.il], true)?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        pref(p, &mut c, b"protectors:zh:comic:", &work.id[..work.il])?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh\",\"pageCount\":1}]}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", b"protectorsread:zh:comic:")?;
        current_image(i)?;
        let work = unsafe { WORKS[i] };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"")?;
        pref(p, &mut c, b"protectorspage:zh:comic:", &work.id[..work.il])?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, &work.image[..work.ul])?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 512];
        let size = raw_string(arg_string(req, b"url")?.ok_or("invalid_request")?, &mut url)?;
        let url = &url[..size];
        media(url).map_err(|_| "invalid_request")?;
        let pid = arg_string(req, b"pageId")?;
        let (id, body) = match pid {
            Some(v) => {
                if let Some(v) = v.strip_prefix(b"protectorspage:zh:comic:") {
                    (
                        Some(sid(v.strip_suffix(b":0").ok_or("invalid_request")?, b"")?),
                        true,
                    )
                } else {
                    (Some(sid(v, b"protectors:zh:comic:")?), false)
                }
            }
            None => (None, false),
        };
        if let Some(id) = id {
            owner(req, id)?;
        }
        catalogue()?;
        let i = if let Some(id) = id {
            find(id)?
        } else {
            let mut found = None;
            for i in 0..LIMIT {
                let work = unsafe { WORKS[i] };
                if url == &work.image[..work.ul] {
                    if found.is_some() {
                        return Err("invalid_request");
                    }
                    found = Some(i);
                }
            }
            found.ok_or("invalid_request")?
        };
        let work = unsafe { WORKS[i] };
        owner(req, &work.id[..work.il])?;
        if url != &work.image[..work.ul] {
            return Err("invalid_request");
        }
        if body {
            current_image(i)?;
        }
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
    let b="{\"listings\":[{\"id\":\"listing:official-comics\",\"name\":\"官方漫画・当前5篇\",\"supportsPaging\":false}]}".as_bytes();
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
