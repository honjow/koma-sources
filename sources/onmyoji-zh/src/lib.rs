#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.netease.onmyoji.zh.koma",
    name: "阴阳师漫画屋",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "阴阳师漫画屋公开中文漫画，包含官方栏目与同人作者作品。",
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

// Source-local structural validation precedes SDK field projections. Refuse
// malformed/duplicate objects and mixed body arrays rather than return a prefix.
fn skip(v: &[u8], mut i: usize) -> usize {
    while i < v.len() && v[i].is_ascii_whitespace() {
        i += 1
    }
    i
}
fn scan(v: &[u8], start: usize, depth: usize) -> R<usize> {
    if depth > 20 {
        return Err("parse_error");
    }
    let mut i = skip(v, start);
    let b = *v.get(i).ok_or("parse_error")?;
    if b == b'"' {
        i += 1;
        while let Some(b) = v.get(i) {
            match *b {
                b'"' => return Ok(i + 1),
                b'\\' => {
                    i += 1;
                    let e = *v.get(i).ok_or("parse_error")?;
                    if e == b'u' {
                        if i + 5 > v.len() || !v[i + 1..i + 5].iter().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("parse_error");
                        }
                        i += 4;
                    } else if !b"\"\\/nrtbf".contains(&e) {
                        return Err("parse_error");
                    }
                }
                0..=31 => return Err("parse_error"),
                _ => {}
            }
            i += 1;
        }
        return Err("parse_error");
    }
    if b == b'{' || b == b'[' {
        let end = if b == b'{' { b'}' } else { b']' };
        i = skip(v, i + 1);
        if v.get(i) == Some(&end) {
            return Ok(i + 1);
        }
        let mut keys = [(0usize, 0usize); 64];
        let mut count = 0;
        loop {
            if b == b'{' {
                if v.get(i) != Some(&b'"') {
                    return Err("parse_error");
                }
                let k = i;
                let e = scan(v, i, depth + 1)?;
                // Field projections use literal schema keys. Reject every escaped
                // key before comparison so equivalent JSON spellings cannot
                // bypass duplicate-key detection. Escaped string VALUES remain valid.
                if v[k + 1..e - 1].contains(&b'\\') {
                    return Err("parse_error");
                }
                if count >= 64 || keys[..count].iter().any(|(a, z)| v[*a..*z] == v[k..e]) {
                    return Err("parse_error");
                }
                keys[count] = (k, e);
                count += 1;
                i = skip(v, e);
                if v.get(i) != Some(&b':') {
                    return Err("parse_error");
                }
                i = skip(v, i + 1);
            }
            i = skip(v, scan(v, i, depth + 1)?);
            if v.get(i) == Some(&end) {
                return Ok(i + 1);
            }
            if v.get(i) != Some(&b',') {
                return Err("parse_error");
            }
            i = skip(v, i + 1);
        }
    }
    for x in [b"true".as_slice(), b"false".as_slice(), b"null".as_slice()] {
        if v[i..].starts_with(x) {
            return Ok(i + x.len());
        }
    }
    if b == b'-' {
        i += 1
    }
    let b = *v.get(i).ok_or("parse_error")?;
    if b == b'0' {
        i += 1
    } else if (b'1'..=b'9').contains(&b) {
        i += 1;
        while v.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1
        }
    } else {
        return Err("parse_error");
    }
    if v.get(i) == Some(&b'.') {
        i += 1;
        let s = i;
        while v.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1
        }
        if i == s {
            return Err("parse_error");
        }
    }
    if v.get(i).is_some_and(|b| *b == b'e' || *b == b'E') {
        i += 1;
        if v.get(i).is_some_and(|b| *b == b'+' || *b == b'-') {
            i += 1
        }
        let s = i;
        while v.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1
        }
        if i == s {
            return Err("parse_error");
        }
    }
    Ok(i)
}
fn valid_json(v: &[u8]) -> R<()> {
    core::str::from_utf8(v).map_err(|_| "parse_error")?;
    if skip(v, scan(v, 0, 0)?) != v.len() {
        return Err("parse_error");
    }
    Ok(())
}
fn string_array(v: &[u8]) -> R<()> {
    let mut i = skip(v, 0);
    if v.get(i) != Some(&b'[') {
        return Err("parse_error");
    }
    i = skip(v, i + 1);
    if v.get(i) == Some(&b']') {
        return Ok(());
    }
    loop {
        if v.get(i) != Some(&b'"') {
            return Err("parse_error");
        }
        i = skip(v, scan(v, i, 0)?);
        if v.get(i) == Some(&b']') {
            return Ok(());
        }
        if v.get(i) != Some(&b',') {
            return Err("parse_error");
        }
        i = skip(v, i + 1);
    }
}
fn object_array(v: &[u8]) -> R<()> {
    let mut i = skip(v, 0);
    if v.get(i) != Some(&b'[') {
        return Err("parse_error");
    }
    i = skip(v, i + 1);
    if v.get(i) == Some(&b']') {
        return Ok(());
    }
    loop {
        if v.get(i) != Some(&b'{') {
            return Err("parse_error");
        }
        i = skip(v, scan(v, i, 0)?);
        if v.get(i) == Some(&b']') {
            return Ok(());
        }
        if v.get(i) != Some(&b',') {
            return Err("parse_error");
        }
        i = skip(v, i + 1);
    }
}

const REFERER: &[u8] = b"https://comic.163.com/";
const API: &[u8] = b"https://huajia.tongren.163.com/article/apps/web/";
const MP: &[u8] = b"onmyoji:";
const CP: &[u8] = b"onmyojichapter:";
const PP: &[u8] = b"onmyojipage:";
fn id(v: &[u8]) -> R<&[u8]> {
    if v.len() == 24
        && v.iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        Ok(v)
    } else {
        Err("invalid_request")
    }
}
fn unprefix<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    id(v.strip_prefix(prefix).ok_or("invalid_request")?)
}
fn positive(v: &[u8]) -> R<usize> {
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 || n > 1000000 || v.first() == Some(&b'0') {
        Err("invalid_request")
    } else {
        Ok(n)
    }
}
fn owner(req: &[u8], raw: &[u8]) -> R<()> {
    let mut mid = [0; 40];
    let mut c = 0;
    w(&mut mid, &mut c, MP)?;
    w(&mut mid, &mut c, raw)?;
    if let Some(v) = json_object_member(req, b"mangaId") {
        if v != b"null" && string(req, b"mangaId")? != &mid[..c] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = json_object_member(req, b"mangaUrl") {
        if v != b"null" {
            let mut route = [0; 128];
            let n = route_url(&mut route, raw)?;
            let mut given = [0; 256];
            let k = raw_string(string(req, b"mangaUrl")?, &mut given)?;
            if given[..k] != route[..n] {
                return Err("invalid_request");
            }
        }
    }
    Ok(())
}
fn route_url(dst: &mut [u8], raw: &[u8]) -> R<usize> {
    let mut c = 0;
    w(dst, &mut c, b"https://comic.163.com/comic-detail/")?;
    w(dst, &mut c, raw)?;
    Ok(c)
}
fn api_url(dst: &mut [u8], raw: &[u8], idx: Option<usize>, list: bool) -> R<usize> {
    let mut c = 0;
    w(dst, &mut c, API)?;
    w(dst, &mut c, raw)?;
    if list {
        w(dst, &mut c, b"/chapters/?cartoon_id=")?;
        w(dst, &mut c, raw)?;
    } else if let Some(nr) = idx {
        w(dst, &mut c, b"/chapters/")?;
        n(dst, &mut c, nr)?;
    }
    Ok(c)
}
fn fetch(u: &[u8]) -> R<usize> {
    let l = build_get_request(
        http_req_buf(),
        u,
        Some(REFERER),
        &[(b"Origin", b"https://comic.163.com")],
    )
    .ok_or("capacity_exceeded")?;
    let l = host::http_request(&http_req_buf()[..l], http_out()).map_err(|_| "network_error")?;
    if member(&http_out()[..l], b"status")? != b"200" {
        return Err("http_error");
    }
    let l = decode_json_body_into(&http_out()[..l], body_buf()).map_err(|_| "parse_error")?;
    valid_json(&body_buf()[..l])?;
    if string(&body_buf()[..l], b"status")? != b"ok" {
        return Err("upstream_error");
    }
    Ok(l)
}
fn style(v: &[u8]) -> R<&[u8]> {
    let s = string(v, b"style")?;
    if s == b"cartoon" || s == b"official_cartoon" {
        Ok(s)
    } else {
        Err("unsupported_content")
    }
}
fn book_check(v: &[u8], raw: &[u8]) -> R<()> {
    if string(v, b"id")? != raw
        || string(v, b"status")? != b"accepted"
        || string(v, b"game")? != b"web"
    {
        return Err("owner_mismatch");
    }
    style(v)?;
    Ok(())
}
fn chapter_check(v: &[u8], m: &[u8], c: &[u8], idx: usize) -> R<()> {
    if string(v, b"article")? != m
        || string(v, b"id")? != c
        || number(member(v, b"index")?)? != idx
        || string(v, b"status")? != b"accepted"
        || string(v, b"game")? != b"web"
    {
        return Err("owner_mismatch");
    }
    style(v)?;
    Ok(())
}
fn image(v: &[u8]) -> R<&[u8]> {
    if string(v, b"file_type")? != b"image" {
        return Err("unsupported_content");
    }
    let u = string(v, b"url")?;
    let mut tmp = [0; 2048];
    let n = raw_string(u, &mut tmp)?;
    if !tmp[..n].starts_with(b"https://g37-manga-resource.fp.ps.netease.com/file/")
        || tmp[..n].iter().any(|x| *x < 33 || *x == b'\\')
    {
        return Err("parse_error");
    }
    Ok(u)
}
fn body_array(v: &[u8]) -> R<&[u8]> {
    member(v, b"body")
}
fn ary(v: &[u8]) -> R<JsonArrayIter<'_>> {
    object_array(v)?;
    JsonArrayIter::from_raw_array(v).ok_or("parse_error")
}
fn body_page(v: &[u8], ordinal: usize) -> R<&[u8]> {
    let a = body_array(v)?;
    let mut it = ary(a)?;
    let mut i = 0;
    let mut found = None;
    while let Some(x) = it.next_object() {
        if i == ordinal {
            found = Some(x)
        }
        i += 1;
    }
    if i == 0 || i > 10000 {
        return Err("parse_error");
    }
    let x = found.ok_or("invalid_request")?;
    if !string(x, b"text_content")?.is_empty() {
        return Err("unsupported_content");
    }
    image(member(x, b"fp_data")?)
}
fn body_count(v: &[u8]) -> R<usize> {
    let mut it = ary(body_array(v)?)?;
    let mut k = 0;
    while let Some(x) = it.next_object() {
        if !string(x, b"text_content")?.is_empty() {
            return Err("unsupported_content");
        }
        image(member(x, b"fp_data")?)?;
        k += 1;
        if k > 10000 {
            return Err("capacity_exceeded");
        }
    }
    if k == 0 {
        Err("parse_error")
    } else {
        Ok(k)
    }
}
fn midout(p: &mut [u8], c: &mut usize, raw: &[u8]) -> R<()> {
    w(p, c, b"\"onmyoji:")?;
    w(p, c, raw)?;
    w(p, c, b"\"")
}
fn chapter_out_id(
    p: &mut [u8],
    c: &mut usize,
    raw: &[u8],
    cid: &[u8],
    idx: usize,
    prefix: &[u8],
) -> R<()> {
    w(p, c, b"\"")?;
    w(p, c, prefix)?;
    w(p, c, raw)?;
    w(p, c, b":")?;
    w(p, c, cid)?;
    w(p, c, b":")?;
    n(p, c, idx)?;
    w(p, c, b"\"")
}
fn parse_ch<'a>(v: &'a [u8], prefix: &[u8]) -> R<(&'a [u8], &'a [u8], usize, Option<usize>)> {
    let v = v.strip_prefix(prefix).ok_or("invalid_request")?;
    let mut it = v.split(|b| *b == b':');
    let m = id(it.next().ok_or("invalid_request")?)?;
    let ch = id(it.next().ok_or("invalid_request")?)?;
    let idx = positive(it.next().ok_or("invalid_request")?)?;
    let pg = it
        .next()
        .map(|v| {
            let n = number(v).map_err(|_| "invalid_request")?;
            if n >= 10000 || (v.len() > 1 && v[0] == b'0') {
                Err("invalid_request")
            } else {
                Ok(n)
            }
        })
        .transpose()?;
    if it.next().is_some() {
        return Err("invalid_request");
    }
    Ok((m, ch, idx, pg))
}
fn metadata(raw: &[u8]) -> R<usize> {
    let mut url = [0; 512];
    let n = api_url(&mut url, raw, None, false)?;
    fetch(&url[..n])
}
fn chapter(raw: &[u8], idx: usize) -> R<usize> {
    let mut url = [0; 512];
    let n = api_url(&mut url, raw, Some(idx), false)?;
    fetch(&url[..n])
}
fn manga_write(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    let raw = id(string(v, b"id")?).map_err(|_| "parse_error")?;
    book_check(v, raw)?;
    w(p, c, b"{\"id\":")?;
    midout(p, c, raw)?;
    w(p, c, b",\"title\":")?;
    q(p, c, string(v, b"title")?)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, image(member(v, b"cover")?)?)?;
    w(p, c, b"},\"authors\":[")?;
    q(p, c, string(v, b"author_nickname")?)?;
    w(p, c, b"],\"status\":")?;
    let status = match json_object_member(v, b"extra")
        .and_then(|e| json_object_member(e, b"serialization_status"))
    {
        Some(b"0") => b"ongoing".as_slice(),
        Some(b"1") => b"completed".as_slice(),
        _ => b"unknown".as_slice(),
    };
    q(p, c, status)?;
    w(
        p,
        c,
        b",\"contentRating\":\"unknown\",\"language\":\"zh\",\"description\":",
    )?;
    let mut a = ary(body_array(v)?)?;
    let x = a.next_object().ok_or("parse_error")?;
    if a.next_object().is_some() {
        return Err("unsupported_content");
    }
    q(p, c, string(x, b"text_content")?)?;
    w(p, c, b",\"tags\":")?;
    let tags = member(v, b"tags")?;
    string_array(tags)?;
    w(p, c, tags)?;
    w(p, c, b",\"sourceTags\":[")?;
    q_text(
        p,
        c,
        if style(v)? == b"official_cartoon" {
            "官方栏目漫画".as_bytes()
        } else {
            "同人漫画".as_bytes()
        },
    )?;
    w(p, c, b"],\"links\":[{\"kind\":\"source\",\"url\":")?;
    let mut u = [0; 128];
    let n = route_url(&mut u, raw)?;
    q_text(p, c, &u[..n])?;
    w(p, c, b"}],\"url\":")?;
    q_text(p, c, &u[..n])?;
    w(p, c, b"}")
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let raw = unprefix(string(req, b"mangaId")?, MP)?;
        owner(req, raw)?;
        let l = metadata(raw)?;
        let v = member(&body_buf()[..l], b"data")?;
        book_check(v, raw)?;
        let mut c = 0;
        let p = payload_buf();
        w(p, &mut c, b"{\"manga\":")?;
        manga_write(p, &mut c, v)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let raw = unprefix(string(req, b"mangaId")?, MP)?;
        owner(req, raw)?;
        let mut url = [0; 512];
        let url_len = api_url(&mut url, raw, None, true)?;
        let l = fetch(&url[..url_len])?;
        let data = member(&body_buf()[..l], b"data")?;
        let mut it = ary(member(data, b"chapters")?)?;
        let count = number(member(data, b"count")?)?;
        if count > 10000 {
            return Err("capacity_exceeded");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        let mut k = 0;
        let mut prev = 0;
        let mut seen = [[0u8; 24]; 1000];
        while let Some(v) = it.next_object() {
            if k >= seen.len() {
                return Err("capacity_exceeded");
            }
            let ch = id(string(v, b"id")?).map_err(|_| "parse_error")?;
            let idx = positive(member(v, b"index")?)?;
            chapter_check(v, raw, ch, idx)?;
            if idx <= prev || seen[..k].iter().any(|s| s == ch) {
                return Err("parse_error");
            }
            prev = idx;
            seen[k].copy_from_slice(ch);
            let pages = body_count(v)?;
            if k > 0 {
                w(p, &mut c, b",")?
            }
            w(p, &mut c, b"{\"id\":")?;
            chapter_out_id(p, &mut c, raw, ch, idx, CP)?;
            w(p, &mut c, b",\"mangaId\":")?;
            midout(p, &mut c, raw)?;
            w(p, &mut c, b",\"title\":")?;
            q(p, &mut c, string(v, b"title")?)?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            n(p, &mut c, idx)?;
            w(
                p,
                &mut c,
                b",\"volumeNumber\":null,\"language\":\"zh\",\"pageCount\":",
            )?;
            n(p, &mut c, pages)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            midout(p, &mut c, raw)?;
            w(
                p,
                &mut c,
                b",\"continuationEligible\":true,\"publishedAt\":null,\"updatedAt\":null}",
            )?;
            k += 1;
        }
        if k != count {
            return Err("incomplete_chapters");
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let (m, ch, idx, pg) = parse_ch(string(req, b"chapterId")?, CP)?;
        if pg.is_some() {
            return Err("invalid_request");
        }
        owner(req, m)?;
        let l = chapter(m, idx)?;
        let v = member(&body_buf()[..l], b"data")?;
        chapter_check(v, m, ch, idx)?;
        let count = body_count(v)?;
        let mut it = ary(body_array(v)?)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        let mut ordinal = 0;
        while let Some(x) = it.next_object() {
            if ordinal > 0 {
                w(p, &mut c, b",")?
            }
            w(p, &mut c, b"{\"id\":")?;
            let mut tmp = [0; 128];
            let mut k = 0;
            chapter_out_id(&mut tmp, &mut k, m, ch, idx, PP)?;
            w(p, &mut c, &tmp[..k - 1])?;
            w(p, &mut c, b":")?;
            n(p, &mut c, ordinal)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, ordinal)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, image(member(x, b"fp_data")?)?)?;
            w(p, &mut c, b"}}")?;
            ordinal += 1;
        }
        if ordinal != count {
            return Err("parse_error");
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let given = string(req, b"pageId")?;
        let mut u = [0; 2048];
        let k = raw_string(string(req, b"url")?, &mut u)?;
        let actual = if given.starts_with(MP) {
            let raw = unprefix(given, MP)?;
            owner(req, raw)?;
            let l = metadata(raw)?;
            let v = member(&body_buf()[..l], b"data")?;
            book_check(v, raw)?;
            image(member(v, b"cover")?)?
        } else {
            let (m, ch, idx, pg) = parse_ch(given, PP)?;
            let ordinal = pg.ok_or("invalid_request")?;
            owner(req, m)?;
            let l = chapter(m, idx)?;
            let v = member(&body_buf()[..l], b"data")?;
            chapter_check(v, m, ch, idx)?;
            body_count(v)?;
            body_page(v, ordinal)?
        };
        let mut original = [0; 2048];
        let n = raw_string(actual, &mut original)?;
        if u[..k] != original[..n] {
            return Err("owner_mismatch");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q_text(p, &mut c, &u[..k])?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q_text(p, &mut c, REFERER)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn list_url(dst: &mut [u8], kind: &[u8], start: usize, search: Option<&[u8]>) -> R<usize> {
    let mut c = 0;
    w(dst, &mut c, API)?;
    if let Some(query) = search {
        w(dst, &mut c, b"search?type=")?;
        w(dst, &mut c, kind)?;
        w(dst, &mut c, b"&search_string=")?;
        if !write_url_encoded(dst, &mut c, query) {
            return Err("capacity_exceeded");
        }
        w(dst, &mut c, b"&span=30&start=")?;
    } else {
        w(dst, &mut c, b"?sort=update&style=")?;
        w(dst, &mut c, kind)?;
        w(dst, &mut c, b"&random_hot_value=0&span=10&start=")?;
    }
    n(dst, &mut c, start)?;
    w(dst, &mut c, b"&sub_site=1")?;
    Ok(c)
}
// Request buffers are per invocation. This copy only keeps first response across the
// second GET in this operation, never across fresh Module tasks.
static mut FIRST: [u8; 1024 * 1024] = [0; 1024 * 1024];
fn first_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(FIRST) }
}
fn list(req: &[u8], search: bool) -> R<usize> {
    let pg = page(req)?;
    if pg > 100000 {
        return Err("invalid_request");
    }
    let start = (pg - 1) * if search { 30 } else { 10 };
    let mut query = [0; 512];
    let qlen = if search {
        let n = raw_string(string(req, b"query")?, &mut query)?;
        if n == 0
            || core::str::from_utf8(&query[..n])
                .map_err(|_| "invalid_request")?
                .chars()
                .count()
                > 16
        {
            return Err("invalid_request");
        }
        n
    } else {
        0
    };
    if !search {
        if string(req, b"listingId")? != b"listing:classification" {
            return Err("invalid_request");
        }
    }
    let mut url = [0; 4096];
    let u = list_url(
        &mut url,
        if search { b"official" } else { b"cartoon" },
        start,
        if search { Some(&query[..qlen]) } else { None },
    )?;
    let l = fetch(&url[..u])?;
    first_buf()[..l].copy_from_slice(&body_buf()[..l]);
    let first = member(&first_buf()[..l], b"data")?;
    let second = if search || pg == 1 {
        let u = list_url(
            &mut url,
            if search { b"all" } else { b"official_cartoon" },
            start,
            if search { Some(&query[..qlen]) } else { None },
        )?;
        let l = fetch(&url[..u])?;
        Some(member(&body_buf()[..l], b"data")?)
    } else {
        None
    };
    let (first_array, second_array) = if search {
        (first, second)
    } else {
        (
            member(first, b"articles")?,
            second.map(|x| member(x, b"articles")).transpose()?,
        )
    };
    let order = if search {
        [Some(first_array), second_array]
    } else {
        [second_array, Some(first_array)]
    };
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    let mut ids = [[0u8; 24]; 60];
    let mut k = 0;
    let mut primary_count = 0;
    for ar in order.into_iter().flatten() {
        let mut it = ary(ar)?;
        let mut raw_count = 0;
        while let Some(v) = it.next_object() {
            raw_count += 1;
            let raw = id(string(v, b"id")?).map_err(|_| "parse_error")?;
            if ids[..k].iter().any(|x| x == raw) {
                continue;
            }
            if k >= 60 {
                return Err("capacity_exceeded");
            }
            ids[k].copy_from_slice(raw);
            if k > 0 {
                w(p, &mut c, b",")?
            }
            manga_write(p, &mut c, v)?;
            k += 1;
        }
        if (search && Some(ar) == second_array) || (!search && ar == first_array) {
            primary_count = raw_count
        }
    }
    let more = primary_count >= if search { 30 } else { 10 };
    w(p, &mut c, b"],\"page\":{\"hasMore\":")?;
    w(p, &mut c, if more { b"true" } else { b"false" })?;
    w(p, &mut c, b",\"nextCursor\":")?;
    if more {
        w(p, &mut c, b"\"")?;
        n(p, &mut c, pg + 1)?;
        w(p, &mut c, b"\"")?
    } else {
        w(p, &mut c, b"null")?
    }
    w(p, &mut c, b"}}")?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    result("search", list(req, true))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    result("get_manga_list", list(req, false))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let v = "{\"listings\":[{\"id\":\"listing:classification\",\"name\":\"漫画分类\"}]}".as_bytes();
    payload_buf()[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_: &[u8]) -> u32 {
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
            if valid_json(req).is_err() {
                return write_error($op, "invalid_request", "invalid JSON");
            }
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if member(req, b"version").ok() != Some(b"1".as_slice())
                || string(req, b"type").ok() != Some(b"request".as_slice())
                || string(req, b"sourceId").ok() != Some(SOURCE_INFO.id.as_bytes())
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
