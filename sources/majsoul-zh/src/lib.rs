#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.catfood.majsoul.zhhant.koma",
    name: "雀魂官方漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "雀魂官网公开漫画合集，每条官方漫画记录一张原图。",
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
fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    json_object_member(v, k).ok_or("parse_error")
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
// Retained title/description projections are decoded plain text; JSON escape once.
fn q_text(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
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

fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"nextCursor\":null,\"hasMore\":false}}";
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
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

const REFERER: &[u8] = b"https://www.maj-soul.com/";
const ORIGIN: &[u8] = b"https://www.maj-soul.com";
const GALLERY: &[u8] = b"https://www.maj-soul.com/#/gallery";
const CATALOG: &[u8] =
    b"https://www.maj-soul.com/homepage/scripts/GalleryList.json?v=1791948532441";
const MID: &[u8] = b"majsoul:zh-Hant:official";
const CP: &[u8] = b"majsoulread:zh-Hant:official:";
const PP: &[u8] = b"majsoulpage:zh-Hant:official:";
mod legacy_links;
use legacy_links::LINKS;
fn fetch() -> R<usize> {
    let len = build_get_request(http_req_buf(), CATALOG, Some(REFERER), &[])
        .ok_or("capacity_exceeded")?;
    let len =
        host::http_request(&http_req_buf()[..len], http_out()).map_err(|_| "network_error")?;
    if member(&http_out()[..len], b"status")? != b"200" {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..len], body_buf()).map_err(|_| "parse_error")
}
fn owner(req: &[u8]) -> R<()> {
    if json_object_member(req, b"mangaId").is_some() && string(req, b"mangaId")? != MID {
        return Err("invalid_request");
    }
    if json_object_member(req, b"mangaUrl").is_some() {
        let mut u = [0; 512];
        let n = raw_string(string(req, b"mangaUrl")?, &mut u)?;
        if &u[..n] != GALLERY {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn require_mid(req: &[u8]) -> R<()> {
    if string(req, b"mangaId")? != MID {
        return Err("not_found");
    }
    owner(req)
}
// Resolve only same-origin Gallery links. Image format is the host image layer's job.
fn canonical_link(raw: &[u8], out: &mut [u8]) -> R<usize> {
    if core::str::from_utf8(raw).is_err() || raw.iter().any(|b| *b < 32 || *b == 127 || *b == b'\\')
    {
        return Err("invalid_request");
    }
    let raw = if let Some(v) = raw.strip_prefix(ORIGIN) {
        if !v.starts_with(b"/") {
            return Err("invalid_request");
        }
        v
    } else if let Some(v) = raw.strip_prefix(b"//www.maj-soul.com") {
        if !v.starts_with(b"/") {
            return Err("invalid_request");
        }
        v
    } else if raw.starts_with(b"//") || contains_bytes(raw, b"://") {
        return Err("invalid_request");
    } else {
        raw
    };
    let end = raw
        .iter()
        .position(|b| *b == b'?' || *b == b'#')
        .unwrap_or(raw.len());
    let path = &raw[..end];
    let path = path.strip_prefix(b"/").unwrap_or(path);
    let mut n = 0;
    for segment in path.split(|b| *b == b'/') {
        // WHATWG dot segments include percent-encoded dots, but ordinary '..' in a name is unchanged.
        let mut dots = 0;
        let mut i = 0;
        while i < segment.len() {
            if segment[i] == b'.' {
                dots += 1;
                i += 1;
            } else if i + 2 < segment.len()
                && segment[i] == b'%'
                && segment[i + 1] == b'2'
                && (segment[i + 2] == b'e' || segment[i + 2] == b'E')
            {
                dots += 1;
                i += 3;
            } else {
                dots = 3;
                break;
            }
        }
        if dots == 1 {
            continue;
        }
        if dots == 2 {
            n = out[..n].iter().rposition(|b| *b == b'/').unwrap_or(0);
            continue;
        }
        w(out, &mut n, b"/")?;
        for b in segment {
            if *b == b' ' {
                w(out, &mut n, b"%20")?;
            } else {
                w(out, &mut n, &[*b])?;
            }
        }
    }
    if !out[..n].starts_with(b"/homepage/comic/") {
        return Err("invalid_request");
    }
    for b in &raw[end..] {
        if *b == b' ' {
            w(out, &mut n, b"%20")?;
        } else {
            w(out, &mut n, &[*b])?;
        }
    }
    Ok(n)
}
fn explicit(s: &[u8]) -> Option<usize> {
    let s = core::str::from_utf8(s)
        .ok()?
        .strip_prefix('第')?
        .strip_suffix('話')?;
    if s.is_empty() {
        return None;
    }
    if s.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return number(s.as_bytes()).ok();
    }
    let mut total = 0usize;
    let mut digit = 0usize;
    let mut used = false;
    let mut last_unit = usize::MAX;
    let mut pending = false;
    let mut zero = false;
    for ch in s.chars() {
        match ch {
            '零' | '〇' => {
                if pending || zero {
                    return None;
                }
                zero = true;
                digit = 0;
                used = true;
            }
            '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' => {
                if pending {
                    return None;
                }
                pending = true;
                zero = false;
                digit = match ch {
                    '一' => 1,
                    '二' => 2,
                    '三' => 3,
                    '四' => 4,
                    '五' => 5,
                    '六' => 6,
                    '七' => 7,
                    '八' => 8,
                    _ => 9,
                };
                used = true;
            }
            '十' | '百' | '千' => {
                let m = match ch {
                    '十' => 10,
                    '百' => 100,
                    _ => 1000,
                };
                if m >= last_unit || zero || (digit == 0 && used) {
                    return None;
                }
                last_unit = m;
                pending = false;
                total = total.checked_add(if digit == 0 { m } else { digit.checked_mul(m)? })?;
                digit = 0;
                used = true;
            }
            _ => return None,
        }
    }
    if used {
        total.checked_add(digit)
    } else {
        None
    }
}
struct Row {
    title: [u8; 8192],
    tn: usize,
    sub: [u8; 1024],
    sn: usize,
    link: [u8; 2048],
    ln: usize,
    timestamp: usize,
    chapter: Option<usize>,
    legacy: usize,
}
fn row(v: &[u8]) -> R<Row> {
    let mut r = Row {
        title: [0; 8192],
        tn: 0,
        sub: [0; 1024],
        sn: 0,
        link: [0; 2048],
        ln: 0,
        timestamp: 0,
        chapter: None,
        legacy: 0,
    };
    r.tn = raw_string(string(v, b"title")?, &mut r.title)?;
    r.sn = raw_string(string(v, b"subTitle")?, &mut r.sub)?;
    let mut raw_link = [0; 2048];
    let raw_len = raw_string(string(v, b"link")?, &mut raw_link)?;
    r.ln = canonical_link(&raw_link[..raw_len], &mut r.link)?;
    if core::str::from_utf8(&r.title[..r.tn]).is_err()
        || core::str::from_utf8(&r.sub[..r.sn]).is_err()
    {
        return Err("parse_error");
    }
    r.timestamp = number(member(v, b"timestamp")?)?;
    r.chapter = explicit(&r.sub[..r.sn]);
    // Preserve legacy identity by original link even if its subtitle is edited.
    r.legacy = LINKS
        .iter()
        .position(|link| *link == &r.link[..r.ln])
        .map(|i| i + 1)
        .unwrap_or(0);
    Ok(r)
}
fn skip(v: &[u8], mut i: usize) -> usize {
    while i < v.len() && v[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn object_end(v: &[u8], start: usize) -> R<usize> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut i = start;
    while i < v.len() {
        let b = v[i];
        if quoted {
            if b == b'\\' {
                i += 1;
                if i >= v.len() {
                    return Err("parse_error");
                }
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.checked_sub(1).ok_or("parse_error")?;
                    if depth == 0 {
                        return Ok(i + 1);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    Err("parse_error")
}
// HTTP_OUT is no longer borrowed after fetch decodes the envelope into BODY.
// Byte-coded 20-byte spans avoid typed-pointer alignment/aliasing assumptions.
fn field(i: usize, k: usize) -> usize {
    let s = i * 20 + k * 4;
    let a = &http_out()[s..s + 4];
    u32::from_le_bytes([a[0], a[1], a[2], a[3]]) as usize
}
fn set_field(i: usize, k: usize, v: usize) {
    let s = i * 20 + k * 4;
    http_out()[s..s + 4].copy_from_slice(&(v as u32).to_le_bytes());
}
fn swap(a: usize, b: usize) {
    for k in 0..5 {
        let x = field(a, k);
        let y = field(b, k);
        set_field(a, k, y);
        set_field(b, k, x);
    }
}
fn greater(a: usize, b: usize, hash: bool) -> bool {
    let k = if hash { 4 } else { 2 };
    (field(a, k), field(a, 0)) > (field(b, k), field(b, 0))
}
fn sift(mut root: usize, end: usize, hash: bool) {
    loop {
        let left = root * 2 + 1;
        if left >= end {
            return;
        }
        let mut child = left;
        if left + 1 < end && greater(left + 1, left, hash) {
            child = left + 1;
        }
        if !greater(child, root, hash) {
            return;
        }
        swap(root, child);
        root = child;
    }
}
fn sort(count: usize, hash: bool) {
    for j in (0..count / 2).rev() {
        sift(j, count, hash);
    }
    for end in (1..count).rev() {
        swap(0, end);
        sift(0, end, hash);
    }
}
fn card(i: usize) -> R<Row> {
    row(&body_buf()[field(i, 0)..field(i, 1)])
}
fn metadata() -> R<usize> {
    let len = fetch()?;
    let body = &body_buf()[..len];
    let array = member(body, b"comic")?;
    let offset = array.as_ptr() as usize - body.as_ptr() as usize;
    if array.first() != Some(&b'[') {
        return Err("parse_error");
    }
    let mut i = skip(array, 1);
    let mut count = 0;
    loop {
        if array.get(i) == Some(&b']') {
            if skip(array, i + 1) != array.len() {
                return Err("parse_error");
            }
            break;
        }
        if array.get(i) != Some(&b'{') {
            return Err("parse_error");
        }
        let end = object_end(array, i)?;
        let r = row(&array[i..end])?;
        if count >= http_out().len() / 20 {
            return Err("capacity_exceeded");
        }
        set_field(count, 0, offset + i);
        set_field(count, 1, offset + end);
        set_field(count, 2, r.timestamp);
        set_field(count, 3, r.legacy);
        let h = r.link[..r.ln]
            .iter()
            .fold(2166136261u32, |h, b| (h ^ *b as u32).wrapping_mul(16777619));
        set_field(count, 4, h as usize);
        count += 1;
        i = skip(array, end);
        match array.get(i) {
            Some(b',') => {
                i = skip(array, i + 1);
                if array.get(i) == Some(&b']') {
                    return Err("parse_error");
                }
            }
            Some(b']') => {}
            _ => return Err("parse_error"),
        }
    }
    // Only a missing old URI can fall back to the old explicit-number identity.
    // A new duplicate-number comic beside an existing legacy URI gets a link ID.
    let mut present = [false; 150];
    for j in 0..count {
        let n = field(j, 3);
        if n > 0 {
            if present[n - 1] {
                return Err("duplicate_identity");
            }
            present[n - 1] = true;
        }
    }
    for j in 0..count {
        if field(j, 3) == 0 {
            let r = card(j)?;
            if let Some(n) = r.chapter {
                if n > 0 && n <= LINKS.len() && !present[n - 1] {
                    set_field(j, 3, n);
                    present[n - 1] = true;
                }
            }
        }
    }
    // Hash sorting only accelerates exact duplicate checks; hash equality is never identity.
    sort(count, true);
    for a in 0..count {
        let mut b = a;
        while b > 0 && field(b - 1, 4) == field(a, 4) {
            b -= 1;
            let x = card(a)?;
            let y = card(b)?;
            if x.link[..x.ln] == y.link[..y.ln] {
                return Err("duplicate_identity");
            }
        }
    }
    // Stable official ascending timestamp order; equal timestamps retain raw order.
    sort(count, false);
    Ok(count)
}
fn key(p: &mut [u8], c: &mut usize, r: &Row, legacy: usize) -> R<()> {
    if legacy > 0 {
        w(p, c, b"ch-")?;
        n(p, c, legacy)
    } else {
        w(p, c, b"link-")?;
        for b in &r.link[..r.ln] {
            const HEX: &[u8] = b"0123456789abcdef";
            w(p, c, &[HEX[(b >> 4) as usize], HEX[(b & 15) as usize]])?;
        }
        Ok(())
    }
}
fn matches(raw: &[u8], r: &Row, legacy: usize) -> R<bool> {
    let mut id = [0; 4096];
    let mut n = 0;
    key(&mut id, &mut n, r, legacy)?;
    Ok(raw == &id[..n])
}
fn find(raw: &[u8], count: usize) -> R<usize> {
    for j in 0..count {
        if matches(raw, &card(j)?, field(j, 3))? {
            return Ok(j);
        }
    }
    Err("not_found")
}
fn uri(p: &mut [u8], c: &mut usize, link: &[u8]) -> R<()> {
    w(p, c, ORIGIN)?;
    w(p, c, link)
}
fn cover_index(count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some((0..count).find(|i| field(*i, 3) == 1).unwrap_or(0))
}
fn book(p: &mut [u8], c: &mut usize, detail: bool, count: usize) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q_text(p, c, MID)?;
    w(p, c, ",\"title\":\"雀魂官方漫画\",\"url\":".as_bytes())?;
    q_text(p, c, GALLERY)?;
    if let Some(i) = cover_index(count) {
        let r = card(i)?;
        let mut u = [0; 2304];
        let mut z = 0;
        uri(&mut u, &mut z, &r.link[..r.ln])?;
        w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q_text(p, c, &u[..z])?;
        w(p, c, b"}")?;
    }
    w(p,c,b",\"authors\":[],\"language\":\"zh-Hant\",\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p,c,",\"description\":\"雀魂官网“漫畫壁紙”栏目中的官方漫画合集。封面采用第一话原图作为合集代表。\",\"links\":[{\"kind\":\"source\",\"url\":".as_bytes())?;
        q_text(p, c, GALLERY)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn listing(query: &[u8]) -> R<usize> {
    let count = metadata()?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    if query.is_empty()
        || contains_bytes("雀魂官方漫画".as_bytes(), query)
        || contains_bytes(
            "雀魂官网“漫畫壁紙”栏目中的官方漫画合集。封面采用第一话原图作为合集代表。".as_bytes(),
            query,
        )
    {
        book(p, &mut c, false, count)?;
    }
    w(
        p,
        &mut c,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut q = [0; 1024];
        let n = raw_string(string(req, b"query")?, &mut q)?;
        if page(req)? == 1 {
            listing(&q[..n])
        } else {
            let b = b"{\"items\":[],\"page\":{\"nextCursor\":null,\"hasMore\":false}}";
            payload_buf()[..b.len()].copy_from_slice(b);
            Ok(b.len())
        }
    }
    result("search", inner(req))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if string(req, b"listingId").ok() != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(1) => result("get_manga_list", listing(b"")),
        Ok(_) => empty("get_manga_list"),
        Err(e) => write_error("get_manga_list", e, ""),
    }
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        require_mid(req)?;
        let count = metadata()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, true, count)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        require_mid(req)?;
        let count = metadata()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for j in 0..count {
            let r = card(j)?;
            if j > 0 {
                w(p, &mut c, b",")?;
            }
            let mut id = [0; 4096];
            let mut z = 0;
            w(&mut id, &mut z, CP)?;
            key(&mut id, &mut z, &r, field(j, 3))?;
            w(p, &mut c, b"{\"id\":")?;
            q_text(p, &mut c, &id[..z])?;
            w(p, &mut c, b",\"mangaId\":")?;
            q_text(p, &mut c, MID)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            q_text(p, &mut c, MID)?;
            let mut title = [0; 12288];
            let mut t = 0;
            if r.sn > 0 {
                w(&mut title, &mut t, &r.sub[..r.sn])?;
                w(&mut title, &mut t, " · ".as_bytes())?;
            }
            w(&mut title, &mut t, &r.title[..r.tn])?;
            if let Some(pos) = find_subslice(&title[..t], b"\\n") {
                title[pos] = b'\n';
                title.copy_within(pos + 2..t, pos + 1);
                t -= 1;
            }
            w(p, &mut c, b",\"title\":")?;
            q_text(p, &mut c, &title[..t])?;
            w(p, &mut c, b",\"url\":")?;
            q_text(p, &mut c, GALLERY)?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            if let Some(ch) = r.chapter {
                n(p, &mut c, ch)?;
            } else {
                w(p, &mut c, b"null")?;
            }
            w(p,&mut c,b",\"volumeNumber\":null,\"publishedAt\":null,\"updatedAt\":null,\"pageCount\":1,\"continuationEligible\":")?;
            w(
                p,
                &mut c,
                if r.chapter.is_some() {
                    b"true"
                } else {
                    b"false"
                },
            )?;
            w(p, &mut c, b",\"language\":\"zh-Hant\"}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        owner(req)?;
        let raw = string(req, b"chapterId")?
            .strip_prefix(CP)
            .ok_or("invalid_request")?;
        let count = metadata()?;
        let j = find(raw, count)?;
        let r = card(j)?;
        let mut id = [0; 4096];
        let mut z = 0;
        w(&mut id, &mut z, PP)?;
        key(&mut id, &mut z, &r, field(j, 3))?;
        w(&mut id, &mut z, b":0")?;
        let mut u = [0; 2304];
        let mut un = 0;
        uri(&mut u, &mut un, &r.link[..r.ln])?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":")?;
        q_text(p, &mut c, &id[..z])?;
        w(
            p,
            &mut c,
            b",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q_text(p, &mut c, &u[..un])?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        owner(req)?;
        let mut asked = [0; 2304];
        let alen = raw_string(string(req, b"url")?, &mut asked)?;
        // Descriptor requests still require an absolute original-origin URI.
        if !asked[..alen].starts_with(ORIGIN) {
            return Err("invalid_request");
        }
        let mut canonical = [0; 2048];
        let nlink = canonical_link(&asked[..alen], &mut canonical)?;
        let link = &canonical[..nlink];
        let mut u = [0; 2304];
        let mut un = 0;
        uri(&mut u, &mut un, link)?;
        let url = &u[..un];
        let given = string(req, b"pageId")?;
        if given == MID {
            if link != LINKS[0] {
                let count = metadata()?;
                let j = cover_index(count).ok_or("not_found")?;
                let r = card(j)?;
                if link != &r.link[..r.ln] {
                    return Err("owner_mismatch");
                }
            }
        } else {
            let raw = given
                .strip_prefix(PP)
                .and_then(|x| x.strip_suffix(b":0"))
                .ok_or("invalid_request")?;
            if let Some(digits) = raw.strip_prefix(b"ch-") {
                let old = number(digits).map_err(|_| "invalid_request")?;
                if old == 0 || old > LINKS.len() || digits.first() == Some(&b'0') {
                    return Err("invalid_request");
                }
                if link != LINKS[old - 1] {
                    let count = metadata()?;
                    let j = find(raw, count)?;
                    let r = card(j)?;
                    if link != &r.link[..r.ln] {
                        return Err("owner_mismatch");
                    }
                }
            } else {
                let mut r = Row {
                    title: [0; 8192],
                    tn: 0,
                    sub: [0; 1024],
                    sn: 0,
                    link: [0; 2048],
                    ln: 0,
                    timestamp: 0,
                    chapter: None,
                    legacy: 0,
                };
                if link.len() > r.link.len() {
                    return Err("capacity_exceeded");
                }
                r.link[..link.len()].copy_from_slice(link);
                r.ln = link.len();
                if !matches(raw, &r, 0)? {
                    return Err("invalid_request");
                }
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q_text(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q_text(p, &mut c, REFERER)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let v = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方漫画合集\"}]}".as_bytes();
    payload_buf()[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
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
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"sourceId").ok() != Some(SOURCE_INFO.id.as_bytes())
                || string(req, b"operation").ok() != Some($op.as_bytes())
            {
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
