#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.laya.zh-hant.koma",
    name: "醫院也瘋狂",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "雷亞醫師與兩元老師創作的《醫院也瘋狂》精選漫畫藝廊。作品評級尚未確認。",
    content_rating: "unknown",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
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
fn page(req: &[u8]) -> R<usize> {
    let args = req;
    let v = match direct(args, b"page").map_err(|_| "invalid_request")? {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        direct(v, b"cursor")
            .map_err(|_| "invalid_request")?
            .unwrap_or(b"null")
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
    let n = positive(v)?;
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

const LIST: &[u8] = b"https://www.laya.url.tw/comic/";
const MID: &[u8] = b"laya:zh-hant:comic:";
const CID: &[u8] = b"layaread:zh-hant:comic:";
const PID: &[u8] = b"layapage:zh-hant:comic:";
const MAX: usize = 256;
static mut SCRIPT: [u8; 131072] = [0; 131072];
static mut FILES: [[u8; 11]; MAX] = [[0; 11]; MAX];
static mut TITLES: [[u8; 512]; MAX] = [[0; 512]; MAX];
static mut TL: [usize; MAX] = [0; MAX];
static mut USED: usize = 0;
fn positive(v: &[u8]) -> R<usize> {
    if v.first() == Some(&b'0') {
        return Err("invalid_request");
    }
    let x = number(v).map_err(|_| "invalid_request")?;
    if x == 0 {
        Err("invalid_request")
    } else {
        Ok(x)
    }
}
fn filename(v: &[u8]) -> R<()> {
    if v.len() != 11
        || !v.starts_with(b"ch-")
        || &v[7..] != b".jpg"
        || !v[3..7].iter().all(|b| b.is_ascii_digit())
    {
        return Err("invalid_request");
    }
    Ok(())
}
fn sid<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let v = v.strip_prefix(prefix).ok_or("invalid_request")?;
    filename(v)?;
    Ok(v)
}
fn pref(p: &mut [u8], c: &mut usize, prefix: &[u8], id: &[u8]) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, id)
}
fn image_uri(p: &mut [u8], c: &mut usize, id: &[u8]) -> R<()> {
    w(p, c, LIST)?;
    w(p, c, b"data/")?;
    w(p, c, id)
}
fn owner(req: &[u8], id: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaId")? {
        if sid(v, MID)? != id {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        if v != LIST {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn unique<'a>(s: &'a [u8], marker: &[u8]) -> R<&'a [u8]> {
    let at = find_subslice(s, marker).ok_or("parse_error")?;
    if find_subslice(&s[at + marker.len()..], marker).is_some() {
        return Err("duplicate_identity");
    }
    Ok(&s[at + marker.len()..])
}
// This accepts only the site's observed finite JS literal grammar. It never evaluates JS.
fn quoted<'a>(s: &'a [u8], i: &mut usize) -> R<&'a [u8]> {
    *i = whitespace(s, *i);
    if s.get(*i) != Some(&b'\'') {
        return Err("parse_error");
    }
    *i += 1;
    let start = *i;
    while let Some(&b) = s.get(*i) {
        if b == b'\'' {
            let v = &s[start..*i];
            *i += 1;
            if v.is_empty() || core::str::from_utf8(v).is_err() {
                return Err("parse_error");
            }
            return Ok(v);
        }
        if b == b'\\' || b < 32 {
            return Err("parse_error");
        }
        *i += 1;
    }
    Err("parse_error")
}
fn literal_list(s: &[u8]) -> R<usize> {
    let s = unique(s, b"const mockFiles = [")?;
    let mut i = 0;
    let mut used = 0;
    loop {
        i = whitespace(s, i);
        if s.get(i) == Some(&b']') {
            if s.get(whitespace(s, i + 1)) != Some(&b';') {
                return Err("parse_error");
            }
            break;
        }
        let v = quoted(s, &mut i)?;
        filename(v).map_err(|_| "parse_error")?;
        if used >= MAX {
            return Err("capacity_exceeded");
        }
        for j in 0..used {
            if v == unsafe { &FILES[j] } {
                return Err("duplicate_identity");
            }
        }
        unsafe {
            FILES[used].copy_from_slice(v);
            TL[used] = 0;
        }
        used += 1;
        i = whitespace(s, i);
        match s.get(i) {
            Some(b',') => i += 1,
            Some(b']') => {}
            _ => return Err("parse_error"),
        }
    }
    if used == 0 {
        return Err("parse_error");
    }
    Ok(used)
}
fn literal_titles(s: &[u8], used: usize) -> R<()> {
    let s = unique(s, b"const predefinedTitles = {")?;
    let mut i = 0;
    let mut total = 0;
    loop {
        i = whitespace(s, i);
        if s.get(i) == Some(&b'}') {
            if s.get(whitespace(s, i + 1)) != Some(&b';') {
                return Err("parse_error");
            }
            break;
        }
        let key = quoted(s, &mut i)?;
        filename(key).map_err(|_| "parse_error")?;
        i = whitespace(s, i);
        if s.get(i) != Some(&b':') {
            return Err("parse_error");
        }
        i += 1;
        let value = quoted(s, &mut i)?;
        if value.len() > 512 {
            return Err("capacity_exceeded");
        }
        let mut matched = None;
        for j in 0..used {
            if key == unsafe { &FILES[j] } {
                matched = Some(j);
                break;
            }
        }
        let at = matched.ok_or("identity_mismatch")?;
        if unsafe { TL[at] } > 0 {
            return Err("duplicate_identity");
        }
        unsafe {
            TITLES[at][..value.len()].copy_from_slice(value);
            TL[at] = value.len();
        }
        total += 1;
        i = whitespace(s, i);
        match s.get(i) {
            Some(b',') => i += 1,
            Some(b'}') => {}
            _ => return Err("parse_error"),
        }
    }
    if total != used {
        return Err("identity_mismatch");
    }
    // The original page sorts by the number extracted from filename for gallery display only.
    // Fixed-width digit filenames have identical lexicographic/numeric ordering. No chapter number is inferred.
    for a in 1..used {
        let mut b = a;
        while b > 0 && unsafe { FILES[b] < FILES[b - 1] } {
            unsafe {
                FILES.swap(b, b - 1);
                TITLES.swap(b, b - 1);
                TL.swap(b, b - 1);
            }
            b -= 1;
        }
    }
    Ok(())
}
fn catalogue() -> R<usize> {
    unsafe {
        USED = 0;
    }
    let req_len =
        build_get_request(http_req_buf(), LIST, Some(LIST), &[]).ok_or("capacity_exceeded")?;
    let out_len =
        host::http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..out_len], b"status") != Some(b"200") {
        return Err("http_error");
    }
    let len =
        decode_json_body_into(&http_out()[..out_len], body_buf()).map_err(|_| "network_error")?;
    if len > 262144 {
        return Err("capacity_exceeded");
    }
    let doc = host::html_parse(&body_buf()[..len]).map_err(|_| "html_error")?;
    let parsed = (|| {
        let mut b = [0; 256];
        let n = attr(doc, b"head > link[rel='canonical'][href]", b"href", &mut b)?;
        if &b[..n] != LIST {
            return Err("identity_mismatch");
        }
        let n = attr(
            doc,
            b"head > meta[name='author'][content]",
            b"content",
            &mut b,
        )?;
        if &b[..n] != "雷亞醫師、兩元老師".as_bytes() {
            return Err("identity_mismatch");
        }
        let n = text(doc, b"body > script:not([src])", unsafe { &mut SCRIPT })?;
        if n == 131072 {
            return Err("capacity_exceeded");
        }
        let s = unsafe { &SCRIPT[..n] };
        for marker in [
            b"path: `data/${filename}`".as_slice(),
            b"allComics.sort((a, b) => a.number - b.number);",
            b"openModal(allComics.indexOf(comic));",
            b"modalImage.src = comic.path;",
            b"const INITIAL_LOAD_COUNT = 12;",
            b"const LOAD_MORE_COUNT = 24;",
        ] {
            unique(s, marker)?;
        }
        let used = literal_list(s)?;
        literal_titles(s, used)?;
        Ok(used)
    })();
    let closed = host::html_close(doc).map_err(|_| "html_error");
    closed?;
    let used = parsed?;
    unsafe {
        USED = used;
    }
    Ok(used)
}
fn find(id: &[u8]) -> R<usize> {
    let used = catalogue()?;
    for i in 0..used {
        if id == unsafe { &FILES[i] } {
            return Ok(i);
        }
    }
    Err("not_found")
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let id = sid(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, id)?;
    find(id)
}
fn book(p: &mut [u8], c: &mut usize, i: usize, detail: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    pref(p, c, MID, unsafe { &FILES[i] })?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLES[i][..TL[i]] })?;
    w(p, c, b",\"url\":")?;
    q(p, c, LIST)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":\"")?;
    image_uri(p, c, unsafe { &FILES[i] })?;
    w(p,c,"\"},\"language\":\"zh-Hant\",\"authors\":[\"雷亞（林子堯）\"],\"artists\":[\"兩元（梁德垣）\"],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"description\":\"《醫院也瘋狂》精選漫畫。雷亞醫師編寫故事腳本，兩元老師繪製漫畫。作品評級尚未確認。\"".as_bytes())?;
    if detail {
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, LIST)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn listing(which: usize) -> R<usize> {
    let used = catalogue()?;
    let start = if which == 1 { 0 } else { 12 + (which - 2) * 24 };
    if start >= used {
        return Err("invalid_request");
    }
    let end = core::cmp::min(used, if which == 1 { 12 } else { start + 24 });
    let more = end < used;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    for i in start..end {
        if i > start {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, i, false)?;
    }
    w(p, &mut c, b"],\"page\":{\"nextCursor\":")?;
    if more {
        w(p, &mut c, b"\"")?;
        n(p, &mut c, which + 1)?;
        w(p, &mut c, b"\",\"hasMore\":true},\"hasNextPage\":true}")?;
    } else {
        w(p, &mut c, b"null,\"hasMore\":false},\"hasNextPage\":false}")?;
    }
    Ok(c)
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId").ok().flatten() != Some(b"listing:comics") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    // 12 initial + 24 per subsequent page within the bounded 256-item parser capacity.
    match page(req) {
        Ok(n) if n <= 12 => result("get_manga_list", listing(n)),
        _ => write_error("get_manga_list", "invalid_request", ""),
    }
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", MID)?;
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
        let i = request_work(req, b"mangaId", MID)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        pref(p, &mut c, CID, unsafe { &FILES[i] })?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        pref(p, &mut c, MID, unsafe { &FILES[i] })?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLES[i][..TL[i]] })?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, LIST)?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        pref(p, &mut c, MID, unsafe { &FILES[i] })?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":1}]}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", CID)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"")?;
        pref(p, &mut c, PID, unsafe { &FILES[i] })?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":\"",
        )?;
        image_uri(p, &mut c, unsafe { &FILES[i] })?;
        w(p, &mut c, b"\"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let url = arg_string(req, b"url")?.ok_or("invalid_request")?;
        let tail = url
            .strip_prefix(LIST)
            .and_then(|s| s.strip_prefix(b"data/"))
            .ok_or("invalid_request")?;
        filename(tail)?;
        let id = if let Some(v) = arg_string(req, b"pageId")? {
            if let Some(v) = v.strip_prefix(PID) {
                let v = v.strip_suffix(b":0").ok_or("invalid_request")?;
                filename(v)?;
                v
            } else {
                sid(v, MID)?
            }
        } else {
            tail
        };
        if id != tail {
            return Err("invalid_request");
        }
        owner(req, id)?;
        let i = find(id)?;
        let mut real = [0; 256];
        let mut r = 0;
        image_uri(&mut real, &mut r, unsafe { &FILES[i] })?;
        if url != &real[..r] {
            return Err("invalid_request");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":")?;
        q(p, &mut c, LIST)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let s = "{\"listings\":[{\"id\":\"listing:comics\",\"name\":\"精選漫畫\"}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
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
