#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tencent.honorofkings.zh.koma",
    name: "王者荣耀世界观漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "王者荣耀官网世界观体验站公开中文漫画，保留原作品作者。",
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

const CATALOG: &[u8] = b"https://pvp.qq.com/zlkdatasys/yuzhouzhan/cartoonlist.json";
const REFERER: &[u8] = b"https://pvp.qq.com/ip/";
const MP: &[u8] = b"honorofkings:";
const CP: &[u8] = b"honorofkingschapter:";
const PP: &[u8] = b"honorofkingspage:";
fn slug(v: &[u8]) -> R<&[u8]> {
    if v.is_empty()
        || v.len() > 64
        || !v
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-' || *b == b'_')
    {
        Err("invalid_request")
    } else {
        Ok(v)
    }
}
fn kind(v: &[u8]) -> R<&[u8]> {
    if v == b"dp" || v == b"lz" {
        Ok(v)
    } else {
        Err("invalid_request")
    }
}
fn ordinal(v: &[u8]) -> R<usize> {
    let n = number(v).map_err(|_| "invalid_request")?;
    if n >= 10000 || (v.len() > 1 && v[0] == b'0') {
        Err("invalid_request")
    } else {
        Ok(n)
    }
}
fn identity<'a>(
    v: &'a [u8],
    prefix: &[u8],
    parts: usize,
) -> R<(&'a [u8], &'a [u8], Option<usize>, Option<usize>)> {
    let v = v.strip_prefix(prefix).ok_or("invalid_request")?;
    let mut it = v.split(|b| *b == b':');
    let k = kind(it.next().ok_or("invalid_request")?)?;
    let s = slug(it.next().ok_or("invalid_request")?)?;
    let c = if parts >= 3 {
        Some(ordinal(it.next().ok_or("invalid_request")?)?)
    } else {
        None
    };
    let p = if parts >= 4 {
        Some(ordinal(it.next().ok_or("invalid_request")?)?)
    } else {
        None
    };
    if it.next().is_some() {
        return Err("invalid_request");
    }
    Ok((k, s, c, p))
}
fn identout(
    p: &mut [u8],
    c: &mut usize,
    prefix: &[u8],
    k: &[u8],
    s: &[u8],
    ch: Option<usize>,
    pg: Option<usize>,
) -> R<()> {
    w(p, c, b"\"")?;
    w(p, c, prefix)?;
    w(p, c, k)?;
    w(p, c, b":")?;
    w(p, c, s)?;
    if let Some(x) = ch {
        w(p, c, b":")?;
        n(p, c, x)?
    }
    if let Some(x) = pg {
        w(p, c, b":")?;
        n(p, c, x)?
    }
    w(p, c, b"\"")
}
fn view(dst: &mut [u8], k: &[u8], s: &[u8]) -> R<usize> {
    let mut c = 0;
    w(
        dst,
        &mut c,
        b"https://pvp.qq.com/ip/cartoon-detail.html?type=",
    )?;
    w(dst, &mut c, k)?;
    w(dst, &mut c, b"&title=")?;
    w(dst, &mut c, s)?;
    Ok(c)
}
fn owner(req: &[u8], k: &[u8], s: &[u8]) -> R<()> {
    if let Some(v) = json_object_member(req, b"mangaId") {
        if v != b"null" {
            let (a, b, _, _) = identity(string(req, b"mangaId")?, MP, 2)?;
            if a != k || b != s {
                return Err("invalid_request");
            }
        }
    }
    if let Some(v) = json_object_member(req, b"mangaUrl") {
        if v != b"null" {
            let mut expected = [0; 256];
            let len = view(&mut expected, k, s)?;
            let mut given = [0; 512];
            let n = raw_string(string(req, b"mangaUrl")?, &mut given)?;
            if given[..n] != expected[..len] {
                return Err("invalid_request");
            }
        }
    }
    Ok(())
}
fn fetch() -> R<usize> {
    let n = build_get_request(http_req_buf(), CATALOG, Some(REFERER), &[])
        .ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if member(&http_out()[..n], b"status")? != b"200" {
        return Err("http_error");
    }
    let n = decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "parse_error")?;
    valid_json(&body_buf()[..n])?;
    Ok(n)
}
fn records<'a>(body: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let cat = match k {
        b"dp" => b"mhdp_9497".as_slice(),
        b"lz" => b"mhlz_9402".as_slice(),
        _ => return Err("invalid_request"),
    };
    let a = member(member(body, cat)?, b"lzxqyz_3050")?;
    object_array(a)?;
    Ok(a)
}
fn book<'a>(body: &'a [u8], k: &[u8], s: &[u8]) -> R<&'a [u8]> {
    let mut it = JsonArrayIter::from_raw_array(records(body, k)?).ok_or("parse_error")?;
    let mut found = None;
    let mut count = 0;
    while let Some(x) = it.next_object() {
        if string(x, b"btpy_6556")? == s {
            if found.is_some() {
                return Err("parse_error");
            }
            found = Some(x)
        }
        count += 1;
        if count > 1000 {
            return Err("capacity_exceeded");
        }
    }
    let found = found.ok_or("not_found")?;
    let expected = if k == b"dp" {
        "短篇".as_bytes()
    } else {
        "连载".as_bytes()
    };
    let mut typ = [0; 64];
    let n = raw_string(string(found, b"mhlx_7846")?, &mut typ)?;
    if typ[..n] != *expected {
        return Err("parse_error");
    }
    Ok(found)
}
// Original arrayData accepts truthy single object or array. Only these known
// chapter shapes are supported; unknown primitives are refused rather than dropped.
fn chapters(v: &[u8]) -> R<&[u8]> {
    let c = member(v, b"mhlzyz_6279")?;
    if c.first() == Some(&b'{') {
        Ok(c)
    } else {
        object_array(c)?;
        Ok(c)
    }
}
fn ch_count(v: &[u8]) -> R<usize> {
    let a = chapters(v)?;
    if a.first() == Some(&b'{') {
        return Ok(1);
    }
    let mut it = JsonArrayIter::from_raw_array(a).ok_or("parse_error")?;
    let mut n = 0;
    while it.next_object().is_some() {
        n += 1;
        if n > 1000 {
            return Err("capacity_exceeded");
        }
    }
    if n == 0 {
        Err("parse_error")
    } else {
        Ok(n)
    }
}
fn chapter(v: &[u8], ordinal: usize) -> R<&[u8]> {
    let a = chapters(v)?;
    if a.first() == Some(&b'{') {
        if ordinal == 0 {
            return Ok(a);
        }
        return Err("invalid_request");
    }
    let mut it = JsonArrayIter::from_raw_array(a).ok_or("parse_error")?;
    let mut n = 0;
    while let Some(x) = it.next_object() {
        if n == ordinal {
            return Ok(x);
        }
        n += 1;
    }
    Err("invalid_request")
}
fn pictures(v: &[u8]) -> R<&[u8]> {
    let a = member(v, b"mhtpjl_3035")?;
    if a.first() == Some(&b'"') {
        Ok(a)
    } else {
        string_array(a)?;
        Ok(a)
    }
}
// Return exact escaped URI bytes from a single string or a validated string array.
fn pic_at(v: &[u8], ordinal: usize) -> R<&[u8]> {
    let a = pictures(v)?;
    if a.first() == Some(&b'"') {
        if ordinal == 0 {
            return Ok(&a[1..a.len() - 1]);
        }
        return Err("invalid_request");
    }
    let mut i = skip(a, 1);
    let mut n = 0;
    while a.get(i) != Some(&b']') {
        let e = scan(a, i, 0)?;
        if n == ordinal {
            return Ok(&a[i + 1..e - 1]);
        }
        n += 1;
        i = skip(a, e);
        if a.get(i) == Some(&b',') {
            i = skip(a, i + 1)
        }
    }
    Err("invalid_request")
}
fn pic_count(v: &[u8]) -> R<usize> {
    let a = pictures(v)?;
    if a.first() == Some(&b'"') {
        return Ok(1);
    }
    let mut i = skip(a, 1);
    let mut n = 0;
    while a.get(i) != Some(&b']') {
        i = skip(a, scan(a, i, 0)?);
        n += 1;
        if n > 10000 {
            return Err("capacity_exceeded");
        }
        if a.get(i) == Some(&b',') {
            i = skip(a, i + 1)
        }
    }
    if n == 0 {
        Err("parse_error")
    } else {
        Ok(n)
    }
}
fn uri(raw: &[u8], out: &mut [u8]) -> R<usize> {
    let mut tmp = [0; 2048];
    let n = raw_string(raw, &mut tmp)?;
    let u = &tmp[..n];
    let u = if let Some(x) = u.strip_prefix(b"https:") {
        x
    } else if let Some(x) = u.strip_prefix(b"http:") {
        x
    } else {
        u
    };
    if !u.starts_with(b"//game.gtimg.cn/images/yxzj/zlkdatasys/images/image/")
        || u.iter().any(|x| *x < 33 || *x == b'\\')
    {
        return Err("unsupported_content");
    }
    let mut c = 0;
    w(out, &mut c, b"https:")?;
    w(out, &mut c, u)?;
    Ok(c)
}
fn manga_write(p: &mut [u8], c: &mut usize, k: &[u8], v: &[u8]) -> R<()> {
    let s = slug(string(v, b"btpy_6556")?).map_err(|_| "parse_error")?;
    let expected = if k == b"dp" {
        "短篇".as_bytes()
    } else {
        "连载".as_bytes()
    };
    let mut typ = [0; 64];
    let t = raw_string(string(v, b"mhlx_7846")?, &mut typ)?;
    if typ[..t] != *expected {
        return Err("parse_error");
    }
    ch_count(v)?;
    w(p, c, b"{\"id\":")?;
    identout(p, c, MP, k, s, None, None)?;
    w(p, c, b",\"title\":")?;
    q(p, c, string(v, b"btlz_1638")?)?;
    w(p, c, b",\"authors\":[")?;
    q(p, c, string(v, b"zz_69")?)?;
    w(p, c, b"],\"description\":")?;
    q(p, c, string(v, b"mslz_4127")?)?;
    w(p, c, b",\"status\":")?;
    match string(v, b"sflzwc_5805")? {
        b"1" => q_text(p, c, b"completed")?,
        b"0" => q_text(p, c, b"ongoing")?,
        _ => q_text(p, c, b"unknown")?,
    };
    w(
        p,
        c,
        b",\"contentRating\":\"unknown\",\"language\":\"zh\",\"cover\":{\"kind\":\"url\",\"url\":",
    )?;
    let mut u = [0; 2048];
    let n = uri(string(v, b"fmlz_2740")?, &mut u)?;
    q_text(p, c, &u[..n])?;
    w(p, c, b"},\"tags\":[")?;
    q(p, c, string(v, b"mhlx_7846")?)?;
    w(p, c, b"],\"url\":")?;
    let n = view(&mut u, k, s)?;
    q_text(p, c, &u[..n])?;
    w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q_text(p, c, &u[..n])?;
    w(p, c, b"}]}")
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let (k, s, _, _) = identity(string(req, b"mangaId")?, MP, 2)?;
        owner(req, k, s)?;
        let len = fetch()?;
        let v = book(&body_buf()[..len], k, s)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        manga_write(p, &mut c, k, v)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let (k, s, _, _) = identity(string(req, b"mangaId")?, MP, 2)?;
        owner(req, k, s)?;
        let len = fetch()?;
        let v = book(&body_buf()[..len], k, s)?;
        let count = ch_count(v)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for i in 0..count {
            let ch = chapter(v, i)?;
            let pages = pic_count(ch)?;
            if i > 0 {
                w(p, &mut c, b",")?
            }
            w(p, &mut c, b"{\"id\":")?;
            identout(p, &mut c, CP, k, s, Some(i), None)?;
            w(p, &mut c, b",\"mangaId\":")?;
            identout(p, &mut c, MP, k, s, None, None)?;
            w(p, &mut c, b",\"title\":")?;
            q(p, &mut c, string(ch, b"btlz_1638")?)?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            n(p, &mut c, i + 1)?;
            w(p,&mut c,b",\"volumeNumber\":null,\"publishedAt\":null,\"updatedAt\":null,\"language\":\"zh\",\"pageCount\":")?;
            n(p, &mut c, pages)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            identout(p, &mut c, MP, k, s, None, None)?;
            w(p, &mut c, b",\"continuationEligible\":true}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let (k, s, ch, _) = identity(string(req, b"chapterId")?, CP, 3)?;
        owner(req, k, s)?;
        let len = fetch()?;
        let v = book(&body_buf()[..len], k, s)?;
        let ch = ch.ok_or("invalid_request")?;
        let x = chapter(v, ch)?;
        let count = pic_count(x)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for i in 0..count {
            if i > 0 {
                w(p, &mut c, b",")?
            }
            w(p, &mut c, b"{\"id\":")?;
            identout(p, &mut c, PP, k, s, Some(ch), Some(i))?;
            w(p, &mut c, b",\"index\":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            let mut u = [0; 2048];
            let n = uri(pic_at(x, i)?, &mut u)?;
            q_text(p, &mut c, &u[..n])?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let given = string(req, b"pageId")?;
        let cover = given.starts_with(MP);
        let (k, s, ch, pg) = identity(
            given,
            if cover { MP } else { PP },
            if cover { 2 } else { 4 },
        )?;
        owner(req, k, s)?;
        let mut given_url = [0; 2048];
        let url_len = raw_string(string(req, b"url")?, &mut given_url)?;
        let len = fetch()?;
        let v = book(&body_buf()[..len], k, s)?;
        let original = if cover {
            string(v, b"fmlz_2740")?
        } else {
            let x = chapter(v, ch.ok_or("invalid_request")?)?;
            pic_at(x, pg.ok_or("invalid_request")?)?
        };
        let mut expected = [0; 2048];
        let n = uri(original, &mut expected)?;
        if given_url[..url_len] != expected[..n] {
            return Err("owner_mismatch");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q_text(p, &mut c, &expected[..n])?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q_text(p, &mut c, REFERER)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn contains_title(title: &[u8], query: &[u8]) -> R<bool> {
    let mut t = [0; 2048];
    let n = raw_string(title, &mut t)?;
    let t = &mut t[..n];
    t.make_ascii_lowercase();
    Ok(query.is_empty() || t.windows(query.len()).any(|w| w == query))
}
fn listing(req: &[u8]) -> R<&[u8]> {
    let x = string(req, b"listingId")?;
    match x {
        b"listing:all" => Ok(b"all"),
        b"listing:dp" => Ok(b"dp"),
        b"listing:lz" => Ok(b"lz"),
        _ => Err("invalid_request"),
    }
}
fn list(req: &[u8], search: bool) -> R<usize> {
    let pg = page(req)?;
    if pg > 10000 {
        return Err("invalid_request");
    }
    let filter = if search {
        b"all".as_slice()
    } else {
        listing(req)?
    };
    let mut query = [0; 512];
    let len = if search {
        raw_string(string(req, b"query")?, &mut query)?
    } else {
        0
    };
    let query = &mut query[..len];
    if core::str::from_utf8(query).is_err() {
        return Err("invalid_request");
    }
    query.make_ascii_lowercase();
    let query = core::str::from_utf8(query)
        .map_err(|_| "invalid_request")?
        .trim()
        .as_bytes();
    let len = fetch()?;
    let body = &body_buf()[..len];
    let mut seen = [[0; 64]; 256];
    let mut seen_len = [0usize; 256];
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    let mut matches = 0;
    let mut written = 0;
    for k in [b"dp".as_slice(), b"lz".as_slice()] {
        if filter != b"all" && filter != k {
            continue;
        }
        let mut seen_count = 0;
        let mut it = JsonArrayIter::from_raw_array(records(body, k)?).ok_or("parse_error")?;
        while let Some(v) = it.next_object() {
            let s = slug(string(v, b"btpy_6556")?).map_err(|_| "parse_error")?;
            if seen_count >= 256 {
                return Err("capacity_exceeded");
            }
            if (0..seen_count).any(|i| seen[i][..seen_len[i]] == *s) {
                return Err("parse_error");
            }
            seen[seen_count][..s.len()].copy_from_slice(s);
            seen_len[seen_count] = s.len();
            seen_count += 1;
            if search
                && !contains_title(string(v, b"btlz_1638")?, query)?
                && !contains_title(string(v, b"btpy_6556")?, query)?
                && !contains_title(string(v, b"mhyxmc_8401")?, query)?
            {
                continue;
            }
            if matches >= ((pg - 1) * 20) && matches < pg * 20 {
                if written > 0 {
                    w(p, &mut c, b",")?
                }
                manga_write(p, &mut c, k, v)?;
                written += 1;
            }
            matches += 1;
        }
    }
    let more = matches > pg * 20;
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
    let v="{\"listings\":[{\"id\":\"listing:all\",\"name\":\"全部漫画\"},{\"id\":\"listing:dp\",\"name\":\"短篇漫画\"},{\"id\":\"listing:lz\",\"name\":\"连载漫画\"}]}".as_bytes();
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
