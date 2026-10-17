#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bilibili.magireco.zh.koma",
    name: "魔法纪录官方漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "Public Chinese Magia Record first-season comics from the official bilibili game website.",
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
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"")
        && append_json_unescaped_then_escaped(p, c, v)
        && write_bytes(p, c, b"\"")
    {
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

const BASE: &[u8] = b"https://game.bilibili.com/magireco/comic1/";
const MID: &[u8] = b"magireco:zh:comic1";
const TITLE: &str = "魔法纪录官方漫画·第一季";
static mut CSS: [u8; 128 * 1024] = [0; 128 * 1024];
static mut IDS: [usize; 1000] = [0; 1000];
fn image_url(u: &[u8]) -> bool {
    let Some(tail) = u.strip_prefix(b"https://i0.hdslb.com/bfs/game-static/") else {
        return false;
    };
    let key = tail
        .strip_suffix(b".jpg")
        .or_else(|| tail.strip_suffix(b".png"));
    key.map(|key| key.len() == 40 && key.iter().all(u8::is_ascii_hexdigit))
        .unwrap_or(false)
}
fn url(s: &[u8], out: &mut [u8]) -> R<usize> {
    let mut b = [0; 2048];
    let len = raw_string(s, &mut b)?;
    let mut c = 0;
    if b[..len].starts_with(b"//") {
        w(out, &mut c, b"https:")?
    };
    w(out, &mut c, &b[..len])?;
    Ok(c)
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    };
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn nuxt(size: usize) -> R<&'static [u8]> {
    let h = &body_buf()[..size];
    let at = find_subslice(h, b"window.__NUXT__=").ok_or("parse_error")? + 16;
    let end = find_subslice(&h[at..], b";</script>").ok_or("parse_error")? + at;
    let v = &h[at..end];
    if v.first() != Some(&b'{') || v.last() != Some(&b'}') {
        return Err("parse_error");
    };
    let mut it = JsonArrayIter::from_raw_array(member(v, b"data")?).ok_or("parse_error")?;
    let d = it.next_object().ok_or("parse_error")?;
    if it.next_object().is_some() {
        return Err("parse_error");
    };
    let meta = member(d, b"metaInfo")?;
    let attrs = member(meta, b"htmlAttrs")?;
    if string(attrs, b"site-id")? != b"magireco" || member(attrs, b"game-id")? != b"101644" {
        return Err("wrong_owner");
    };
    let mut route = [0; 2048];
    let route_len = raw_string(string(attrs, b"h5-url")?, &mut route)?;
    if &route[..route_len] != b"https://game.bilibili.com/magireco/h5/comic1" {
        return Err("wrong_owner");
    }
    Ok(d)
}
// The official event bus consumes output.data as an ordered property path,
// not as a serialized JSON template. Unknown paths remain unsupported.
fn selector_path(v: &[u8]) -> bool {
    fn space(v: &[u8], pos: &mut usize) {
        while matches!(v.get(*pos), Some(b' ' | b'\t' | b'\r' | b'\n')) { *pos += 1; }
    }
    let mut pos = 0;
    space(v, &mut pos);
    if v.get(pos) != Some(&b'[') { return false; }
    pos += 1;
    for (index, expected) in [b"option".as_slice(), b"slideIndex".as_slice()].iter().enumerate() {
        space(v, &mut pos);
        if v.get(pos) != Some(&b'"') { return false; }
        pos += 1;
        let start = pos;
        while let Some(&b) = v.get(pos) {
            if b == b'"' { break; }
            if b == b'\\' { pos += 1; }
            pos += 1;
        }
        if v.get(pos) != Some(&b'"') { return false; }
        // This buffer bounds equality with the two known property names,
        // rather than imposing a general field-length limit.
        let mut decoded = [0u8; 10];
        let Ok(n) = raw_string(&v[start..pos], &mut decoded) else { return false; };
        if &decoded[..n] != *expected { return false; }
        pos += 1;
        space(v, &mut pos);
        let delimiter = if index == 0 { b',' } else { b']' };
        if v.get(pos) != Some(&delimiter) { return false; }
        pos += 1;
    }
    space(v, &mut pos);
    pos == v.len()
}
const BANNER: &[u8] = b"autoid-cmv5LAK9";
fn validate_catalogue(d: &[u8]) -> R<usize> {
    let mut it = JsonArrayIter::from_raw_array(member(d, b"comList")?).ok_or("parse_error")?;
    let mut banner = None;
    let mut count = 0;
    while let Some(x) = it.next_object() {
        let cat = string(x, b"category")?;
        if cat == b"BiliBanner" {
            if banner.is_some() || string(x, b"id")? != BANNER {
                return Err("unsupported_selector");
            }
            banner = Some(x);
        } else if cat == b"BiliExtra" {
            let opt = member(x, b"option")?;
            let Some(raw) = json_object_member(opt, b"slideIndex") else {
                continue;
            };
            let ep = number(raw)?;
            if ep == 0 || count >= 1000 || unsafe { IDS[..count].contains(&ep) } {
                return Err("parse_error");
            }
            let mut links =
                JsonArrayIter::from_raw_array(member(x, b"output")?).ok_or("parse_error")?;
            let link = links.next_object().ok_or("unsupported_selector")?;
            if links.next_object().is_some()
                || string(link, b"target")? != BANNER
                || string(link, b"emitter")? != b"click"
                || string(link, b"listener")? != b"slideTo"
                || !selector_path(member(link, b"data")?)
            {
                return Err("unsupported_selector");
            }
            unsafe {
                IDS[count] = ep;
            }
            count += 1;
        }
    }
    let banner = banner.ok_or("parse_error")?;
    let rows = member(banner, b"subComponentsList")?;
    let mut pos = 1;
    let mut rowcount = 0;
    loop {
        while pos < rows.len() && (rows[pos].is_ascii_whitespace() || rows[pos] == b',') {
            pos += 1;
        }
        if rows.get(pos) == Some(&b']') {
            break;
        }
        if rows.get(pos) != Some(&b'[') {
            return Err("parse_error");
        }
        let stop = end(rows, pos)?;
        let mut nested = JsonArrayIter::from_raw_array(&rows[pos..stop]).ok_or("parse_error")?;
        let x = nested.next_object().ok_or("parse_error")?;
        if nested.next_object().is_some()
            || string(x, b"category")? != b"BiliImage"
            || string(x, b"id")?.is_empty()
        {
            return Err("parse_error");
        }
        rowcount += 1;
        pos = stop;
    }
    if count == 0 || count != rowcount {
        return Err("incomplete_catalogue");
    }
    for n in 1..=rowcount {
        if !unsafe { IDS[..count].contains(&n) } {
            return Err("incomplete_catalogue");
        }
    }
    Ok(count)
}
fn catalogue() -> R<usize> {
    let size = fetch(BASE)?;
    validate_catalogue(nuxt(size)?)
}
fn cover(d: &[u8], out: &mut [u8]) -> R<usize> {
    let meta = member(d, b"metaInfo")?;
    let mut it = JsonArrayIter::from_raw_array(member(meta, b"meta")?).ok_or("parse_error")?;
    while let Some(x) = it.next_object() {
        if json_object_member(x, b"property") == Some(b"\"og:image\"") {
            let n = url(string(x, b"content")?, out)?;
            if !image_url(&out[..n]) {
                return Err("parse_error");
            };
            return Ok(n);
        }
    }
    Err("parse_error")
}
fn book(p: &mut [u8], c: &mut usize, cov: &[u8], detail: bool) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, TITLE.as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, cov)?;
    w(p,c,b"},\"language\":\"zh\",\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(
            p,
            c,
            "《魔法纪录 魔法少女小圆外传》官网公开第一季四格漫画。".as_bytes(),
        )?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, BASE)?;
        w(p, c, b"}]")?
    };
    w(p, c, b"}")
}
fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false}}";
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
fn listing() -> R<usize> {
    let size = fetch(BASE)?;
    let d = nuxt(size)?;
    validate_catalogue(d)?;
    let mut ub = [0; 2048];
    let len = cover(d, &mut ub)?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c, &ub[..len], false)?;
    w(p, &mut c, b"],\"page\":{\"hasMore\":false}}")?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let raw = extract_json_string(req, b"query").unwrap_or(b"");
    let mut b = [0; 1024];
    let len = match raw_string(raw, &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    if !b[..len].is_empty()
        && !contains_bytes(
            "魔法纪录 魔法记录 魔法少女小圆外传 Magia Record magireco".as_bytes(),
            &b[..len],
        )
    {
        return empty("search");
    };
    match page(req) {
        Ok(1) => result("search", listing()),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    };
    match page(req) {
        Ok(1) => {}
        Ok(_) => return empty("get_manga_list"),
        Err(e) => return write_error("get_manga_list", e, ""),
    };
    result("get_manga_list", listing())
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let size = fetch(BASE)?;
        let d = nuxt(size)?;
        validate_catalogue(d)?;
        let mut ub = [0; 2048];
        let len = cover(d, &mut ub)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, &ub[..len], true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let count = catalogue()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for i in 0..count {
            let ep = unsafe { IDS[i] };
            if i > 0 {
                w(p, &mut c, b",")?
            };
            w(p, &mut c, b"{\"id\":\"magirecoch:zh:comic1:")?;
            n(p, &mut c, ep)?;
            w(p, &mut c, b"\",\"mangaId\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"title\":\"")?;
            w(p, &mut c, b"\",\"chapterNumber\":")?;
            n(p, &mut c, ep)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            q(p, &mut c, MID)?;
            w(
                p,
                &mut c,
                b",\"continuationEligible\":true,\"language\":\"zh\",\"pageCount\":1}",
            )?
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
// Data-only balanced value scanner; it never evaluates website script.
fn end(v: &[u8], start: usize) -> R<usize> {
    let mut i = start;
    let mut depth = 0;
    let mut quoted = false;
    while i < v.len() {
        let b = v[i];
        if quoted {
            if b == b'\\' {
                i += 1
            } else if b == b'"' {
                quoted = false
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'[' | b'{' => depth += 1,
                b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(i + 1);
                    }
                }
                _ => {}
            }
        };
        i += 1;
    }
    Err("parse_error")
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let id = extract_json_string(req, b"chapterId").ok_or("invalid_request")?;
        let ep = number(
            id.strip_prefix(b"magirecoch:zh:comic1:")
                .ok_or("invalid_request")?,
        )?;
        if let Some(mid) = extract_json_string(req, b"mangaId") {
            if mid != MID {
                return Err("not_found");
            }
        }
        let size = fetch(BASE)?;
        let d = nuxt(size)?;
        let count = validate_catalogue(d)?;
        if !unsafe { IDS[..count].contains(&ep) } {
            return Err("not_found");
        }
        let mut it = JsonArrayIter::from_raw_array(member(d, b"comList")?).ok_or("parse_error")?;
        let mut banner = None;
        while let Some(x) = it.next_object() {
            if string(x, b"category")? == b"BiliBanner" {
                banner = Some(x);
            }
        }
        let banner = banner.ok_or("parse_error")?;
        let rows = member(banner, b"subComponentsList")?;
        if rows.first() != Some(&b'[') {
            return Err("parse_error");
        };
        let mut i = 1;
        let mut rowcount = 0;
        let mut selected = [0u8; 128];
        let mut selected_len = 0;
        loop {
            while i < rows.len() && (rows[i].is_ascii_whitespace() || rows[i] == b',') {
                i += 1
            }
            if rows.get(i) == Some(&b']') {
                break;
            };
            if rows.get(i) != Some(&b'[') {
                return Err("parse_error");
            };
            let stop = end(rows, i)?;
            let mut nested = JsonArrayIter::from_raw_array(&rows[i..stop]).ok_or("parse_error")?;
            let x = nested.next_object().ok_or("parse_error")?;
            if nested.next_object().is_some() || string(x, b"category")? != b"BiliImage" {
                return Err("parse_error");
            };
            let id = string(x, b"id")?;
            if rowcount + 1 == ep {
                if id.len() > selected.len() {
                    return Err("capacity_exceeded");
                };
                selected[..id.len()].copy_from_slice(id);
                selected_len = id.len()
            };
            rowcount += 1;
            i = stop
        }
        if rowcount != count || selected_len == 0 {
            return Err("incomplete_pages");
        };
        let meta = member(d, b"metaInfo")?;
        let mut it = JsonArrayIter::from_raw_array(member(meta, b"style")?).ok_or("parse_error")?;
        let css = unsafe { &mut *core::ptr::addr_of_mut!(CSS) };
        let mut c = 0;
        while let Some(x) = it.next_object() {
            let raw = string(x, b"cssText")?;
            let len = raw_string(raw, &mut css[c..])?;
            c += len;
        }
        let mut pattern = [0; 132];
        let mut n0 = 0;
        w(&mut pattern, &mut n0, b"#")?;
        w(&mut pattern, &mut n0, &selected[..selected_len])?;
        w(&mut pattern, &mut n0, b"{")?;
        let at = find_subslice(&css[..c], &pattern[..n0]).ok_or("parse_error")? + n0;
        let stop = css[at..c]
            .iter()
            .position(|b| *b == b'}')
            .ok_or("parse_error")?
            + at;
        let rule = &css[at..stop];
        let marker = b"background-image:url(";
        let at = find_subslice(rule, marker).ok_or("parse_error")? + marker.len();
        let stop = rule[at..]
            .iter()
            .position(|b| *b == b')')
            .ok_or("parse_error")?
            + at;
        let mut ub = [0; 2048];
        let len = url(&rule[at..stop], &mut ub)?;
        if !image_url(&ub[..len]) {
            return Err("parse_error");
        };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"magirecopage:zh:comic1:")?;
        n(p, &mut c, ep)?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, &ub[..len])?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if let Some(raw) = extract_json_string(req, b"mangaUrl") {
            let mut route = [0; 2048];
            let len = raw_string(raw, &mut route)?;
            if &route[..len] != BASE {
                return Err("invalid_request");
            }
        }
        let raw = extract_json_string(req, b"url").ok_or("invalid_request")?;
        let mut decoded = [0; 2048];
        let len = raw_string(raw, &mut decoded)?;
        let u = &decoded[..len];
        if !image_url(u) {
            return Err("invalid_request");
        };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, BASE)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方漫画\"}]}".as_bytes();
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
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
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
