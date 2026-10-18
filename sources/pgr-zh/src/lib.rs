#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.kurogame.pgr.zh.koma",
    name: "战双帕弥什官方漫画",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "战双帕弥什官网公开中文漫画，保留接口实际作者字段。",
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

const MP: &[u8] = b"pgr:";
const CP: &[u8] = b"pgrchapter:";
const PP: &[u8] = b"pgrpage:";
const BASE: &[u8] = b"https://pns.kurogames.com/v2/Comics/";
const SITE: &[u8] = b"https://pns.kurogames.com/comics/comicsOfficial/";
const DOC_CAP: usize = 256 * 1024;
const CH_CAP: usize = 1024 * 1024;
static mut LIST: [u8; DOC_CAP] = [0; DOC_CAP];
static mut DETAIL: [u8; DOC_CAP] = [0; DOC_CAP];
static mut CHAPTERS: [u8; CH_CAP] = [0; CH_CAP];
fn list_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(LIST) }
}
fn detail_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL) }
}
fn chapters_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(CHAPTERS) }
}
fn ident(v: &[u8], prefix: &[u8], fields: usize) -> R<(usize, usize, usize)> {
    let rest = v.strip_prefix(prefix).ok_or("invalid_request")?;
    let mut parts = rest.split(|b| *b == b':');
    let mut out = [0; 3];
    for slot in out.iter_mut().take(fields) {
        let x = parts.next().ok_or("invalid_request")?;
        *slot = number(x).map_err(|_| "invalid_request")?;
        if x.len() > 1 && x[0] == b'0' {
            return Err("invalid_request");
        }
    }
    if parts.next().is_some()
        || out[0] == 0
        || out[0] > u32::MAX as usize
        || (fields > 1 && (out[1] == 0 || out[1] > u32::MAX as usize))
    {
        return Err("invalid_request");
    }
    Ok((out[0], out[1], out[2]))
}
fn idout(
    p: &mut [u8],
    c: &mut usize,
    prefix: &[u8],
    mid: usize,
    cid: Option<usize>,
    ordinal: Option<usize>,
) -> R<()> {
    w(p, c, b"\"")?;
    w(p, c, prefix)?;
    n(p, c, mid)?;
    if let Some(x) = cid {
        w(p, c, b":")?;
        n(p, c, x)?
    }
    if let Some(x) = ordinal {
        w(p, c, b":")?;
        n(p, c, x)?
    }
    w(p, c, b"\"")
}
fn view(out: &mut [u8], mid: usize) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, SITE)?;
    n(out, &mut c, mid)?;
    Ok(c)
}
fn owner(req: &[u8], mid: usize) -> R<()> {
    if let Some(v) = json_object_member(req, b"mangaId") {
        if v != b"null" {
            if ident(string(req, b"mangaId")?, MP, 1)?.0 != mid {
                return Err("invalid_request");
            }
        }
    }
    if let Some(v) = json_object_member(req, b"mangaUrl") {
        if v != b"null" {
            let mut expected = [0; 256];
            let e = view(&mut expected, mid)?;
            let mut raw = [0; 512];
            let a = raw_string(string(req, b"mangaUrl")?, &mut raw)?;
            if a == 0 || raw[..a] != expected[..e] {
                return Err("invalid_request");
            }
        }
    }
    Ok(())
}
fn eq_string(a: &[u8], b: &[u8]) -> R<bool> {
    let mut x = [0; 4096];
    let mut y = [0; 4096];
    let xn = raw_string(a, &mut x)?;
    let yn = raw_string(b, &mut y)?;
    Ok(x[..xn] == y[..yn])
}
fn uri(raw: &[u8], out: &mut [u8]) -> R<usize> {
    let mut b = [0; 2048];
    let n = raw_string(raw, &mut b)?;
    let u = &b[..n];
    let rel = if let Some(x) = u.strip_prefix(b"https:") {
        x
    } else {
        u
    };
    if !rel.starts_with(b"//media-cdn-zspms.kurogame.com/comics/")
        || rel.len() <= b"//media-cdn-zspms.kurogame.com/comics/".len()
        || rel.iter().any(|b| *b < 33 || *b == b'\\' || *b == b'#')
    {
        return Err("unsupported_content");
    }
    let mut c = 0;
    w(out, &mut c, b"https:")?;
    w(out, &mut c, rel)?;
    Ok(c)
}
// Source-local POST envelope preserves the existing raw-string bodyBase64 ABI.
// This public Site-Id is declared by the official client; it is not a credential.
fn fetch(route: &[u8], body: Option<&[u8]>) -> R<usize> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let r = http_req_buf();
    let mut c = 0;
    w(r, &mut c, b"{\"version\":1,\"method\":\"POST\",\"url\":\"")?;
    w(r, &mut c, BASE)?;
    w(r, &mut c, route)?;
    w(
        r,
        &mut c,
        b"\",\"headers\":{\"Site-Id\":\"1\",\"Accept\":\"application/json, text/plain, */*\"",
    )?;
    if let Some(b) = body {
        w(
            r,
            &mut c,
            b",\"Content-Type\":\"application/json;charset=utf-8\"},\"bodyBase64\":",
        )?;
        q_text(r, &mut c, b)?
    } else {
        w(r, &mut c, b"}")?
    }
    w(
        r,
        &mut c,
        b",\"timeoutMs\":5000,\"responseKind\":\"bodyText\"}",
    )?;
    let out = host::http_request(&r[..c], http_out()).map_err(|_| "network")?;
    let envelope = &http_out()[..out];
    valid_json(envelope)?;
    if member(envelope, b"ok")? != b"true" {
        return Err("network");
    }
    let status = number(member(envelope, b"status").or_else(|_| member(envelope, b"statusCode"))?)?;
    if status != 200 {
        return Err(match status {
            403 => "client_error",
            404 => "not_found",
            429 => "rate_limit",
            500..=599 => "server_error",
            _ => "network",
        });
    }
    let len = decode_json_body_into(envelope, body_buf()).map_err(|_| "network")?;
    valid_json(&body_buf()[..len])?;
    if member(&body_buf()[..len], b"_code")? != b"0" {
        return Err("upstream_error");
    }
    member(&body_buf()[..len], b"_data")?;
    Ok(len)
}
fn fetch_list() -> R<usize> {
    let len = fetch(b"list", None)?;
    if len > DOC_CAP {
        return Err("capacity_exceeded");
    }
    list_buf()[..len].copy_from_slice(&body_buf()[..len]);
    let d = member(&list_buf()[..len], b"_data")?;
    object_array(d)?;
    let mut it = JsonArrayIter::from_raw_array(d).ok_or("parse_error")?;
    let mut seen = [0; 256];
    let mut count = 0;
    while let Some(v) = it.next_object() {
        let mid = number(member(v, b"comicsId")?)?;
        if mid == 0 || mid > u32::MAX as usize || count >= 256 || seen[..count].contains(&mid) {
            return Err("parse_error");
        }
        string(v, b"name")?;
        let mut u = [0; 2048];
        uri(string(v, b"verticalCover")?, &mut u)?;
        number(member(v, b"status")?)?;
        seen[count] = mid;
        count += 1;
    }
    Ok(len)
}
fn book(len: usize, mid: usize) -> R<&'static [u8]> {
    let d = member(&list_buf()[..len], b"_data")?;
    let mut it = JsonArrayIter::from_raw_array(d).ok_or("parse_error")?;
    while let Some(v) = it.next_object() {
        if number(member(v, b"comicsId")?)? == mid {
            return Ok(v);
        }
    }
    Err("not_found")
}
fn body_mid(out: &mut [u8], mid: usize) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, b"{\"comicsId\":")?;
    n(out, &mut c, mid)?;
    w(out, &mut c, b"}")?;
    Ok(c)
}
fn fetch_detail(mid: usize, listed: &[u8]) -> R<&'static [u8]> {
    let mut b = [0; 128];
    let c = body_mid(&mut b, mid)?;
    let len = fetch(b"detail", Some(&b[..c]))?;
    if len > DOC_CAP {
        return Err("capacity_exceeded");
    }
    detail_buf()[..len].copy_from_slice(&body_buf()[..len]);
    let d = member(&detail_buf()[..len], b"_data")?;
    if d.first() != Some(&b'{')
        || number(member(d, b"comicsId")?)? != mid
        || !eq_string(string(d, b"name")?, string(listed, b"name")?)?
        || !eq_string(
            string(d, b"verticalCover")?,
            string(listed, b"verticalCover")?,
        )?
    {
        return Err("owner_mismatch");
    }
    string(d, b"author")?;
    string(d, b"desc")?;
    Ok(d)
}
// Complete each anonymous asc page in original API order. Never numeric-sort CID.
fn fetch_chapters(mid: usize) -> R<usize> {
    let mut c = 0;
    w(chapters_buf(), &mut c, b"[")?;
    let mut seen = [0usize; 1000];
    let mut count = 0;
    let mut total = 0;
    let mut pages = 0;
    let mut per = 0;
    for pg in 1..=50 {
        let mut b = [0; 128];
        let mut x = 0;
        w(&mut b, &mut x, b"{\"comicsId\":")?;
        n(&mut b, &mut x, mid)?;
        w(&mut b, &mut x, b",\"sort\":\"asc\",\"page\":")?;
        n(&mut b, &mut x, pg)?;
        w(&mut b, &mut x, b"}")?;
        let len = fetch(b"chapterList", Some(&b[..x]))?;
        let d = member(&body_buf()[..len], b"_data")?;
        let t = number(member(d, b"totalCount")?)?;
        let p = number(member(d, b"totalPage")?)?;
        let pn = number(member(d, b"perPage")?)?;
        if pg == 1 {
            total = t;
            pages = p;
            per = pn;
            if total == 0
                || total > 1000
                || pages == 0
                || pages > 50
                || per == 0
                || per > 1000
                || pages != (total + per - 1) / per
            {
                return Err("parse_error");
            }
        }
        if t != total || p != pages || pn != per || number(member(d, b"current")?)? != pg {
            return Err("parse_error");
        }
        let a = member(d, b"item")?;
        object_array(a)?;
        let mut it = JsonArrayIter::from_raw_array(a).ok_or("parse_error")?;
        let mut got = 0;
        while let Some(v) = it.next_object() {
            let cid = number(member(v, b"comicsChapterId")?)?;
            if cid == 0 || cid > u32::MAX as usize || count >= 1000 || seen[..count].contains(&cid)
            {
                return Err("parse_error");
            }
            string(v, b"chapter")?;
            string(v, b"title")?;
            string(v, b"createdAt")?;
            let mut u = [0; 2048];
            uri(string(v, b"cover")?, &mut u)?;
            if count > 0 {
                w(chapters_buf(), &mut c, b",")?
            }
            w(chapters_buf(), &mut c, v)?;
            seen[count] = cid;
            count += 1;
            got += 1;
        }
        let expected = if pg < pages {
            per
        } else {
            total - (pg - 1) * per
        };
        if got != expected {
            return Err("parse_error");
        }
        if pg == pages {
            if count != total {
                return Err("parse_error");
            }
            w(chapters_buf(), &mut c, b"]")?;
            return Ok(c);
        }
    }
    Err("capacity_exceeded")
}
fn chapter_at(len: usize, cid: usize) -> R<(&'static [u8], usize, usize)> {
    let mut it = JsonArrayIter::from_raw_array(&chapters_buf()[..len]).ok_or("parse_error")?;
    let mut prev = 0;
    while let Some(v) = it.next_object() {
        let id = number(member(v, b"comicsChapterId")?)?;
        if id == cid {
            let next = match it.next_object() {
                Some(n) => number(member(n, b"comicsChapterId")?)?,
                None => 0,
            };
            return Ok((v, prev, next));
        }
        prev = id;
    }
    Err("not_found")
}
fn fetch_content(
    mid: usize,
    cid: usize,
    listed: &[u8],
    ch: &[u8],
    prev: usize,
    next: usize,
) -> R<&'static [u8]> {
    let mut b = [0; 128];
    let mut c = 0;
    w(&mut b, &mut c, b"{\"comicsChapterId\":")?;
    n(&mut b, &mut c, cid)?;
    w(&mut b, &mut c, b"}")?;
    let len = fetch(b"content", Some(&b[..c]))?;
    let d = member(&body_buf()[..len], b"_data")?;
    if d.first() != Some(&b'{')
        || number(member(d, b"comicsId")?)? != mid
        || !eq_string(string(d, b"name")?, string(listed, b"name")?)?
        || !eq_string(string(d, b"chapter")?, string(ch, b"chapter")?)?
        || !eq_string(string(d, b"title")?, string(ch, b"title")?)?
        || number(member(d, b"preComicsChapterId")?)? != prev
        || number(member(d, b"nextComicsChapterId")?)? != next
    {
        return Err("owner_mismatch");
    }
    let a = member(d, b"imageList")?;
    string_array(a)?;
    let mut i = skip(a, 1);
    let mut count = 0;
    let mut u = [0; 2048];
    while a.get(i) != Some(&b']') {
        let e = scan(a, i, 0)?;
        uri(&a[i + 1..e - 1], &mut u)?;
        count += 1;
        if count > 10000 {
            return Err("capacity_exceeded");
        }
        i = skip(a, e);
        if a.get(i) == Some(&b',') {
            i = skip(a, i + 1)
        }
    }
    if count == 0 {
        return Err("parse_error");
    }
    Ok(d)
}
fn pic_at(d: &[u8], ordinal: usize) -> R<&[u8]> {
    let a = member(d, b"imageList")?;
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
fn manga_write(p: &mut [u8], c: &mut usize, v: &[u8], detail: bool, listed: &[u8]) -> R<()> {
    let mid = number(member(v, b"comicsId")?)?;
    w(p, c, b"{\"id\":")?;
    idout(p, c, MP, mid, None, None)?;
    w(p, c, b",\"title\":")?;
    q(p, c, string(v, b"name")?)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    let mut u = [0; 2048];
    let len = uri(string(v, b"verticalCover")?, &mut u)?;
    q_text(p, c, &u[..len])?;
    w(p, c, b"},\"authors\":[")?;
    if detail {
        q(p, c, string(v, b"author")?)?
    }
    w(p, c, b"],\"description\":")?;
    if detail {
        q(p, c, string(v, b"desc")?)?
    } else {
        w(p, c, b"null")?
    }
    w(p, c, b",\"status\":")?;
    let state = match number(member(listed, b"status")?)? {
        1 => b"ongoing".as_slice(),
        2 => b"completed",
        _ => b"unknown",
    };
    q_text(p, c, state)?;
    w(
        p,
        c,
        b",\"contentRating\":\"unknown\",\"language\":\"zh\",\"tags\":[],\"url\":",
    )?;
    let len = view(&mut u, mid)?;
    q_text(p, c, &u[..len])?;
    w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q_text(p, c, &u[..len])?;
    w(p, c, b"}]}")
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mid = ident(string(req, b"mangaId")?, MP, 1)?.0;
        owner(req, mid)?;
        let len = fetch_list()?;
        let b = book(len, mid)?;
        let d = fetch_detail(mid, b)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        manga_write(p, &mut c, d, true, b)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
// Keep the API's date-only precision. Do not invent time, offset or epoch.
fn date_only(v: &[u8]) -> bool {
    if v.len() != 10
        || v[4] != b'-'
        || v[7] != b'-'
        || !v
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return false;
    }
    let Ok(y) = number(&v[..4]) else { return false };
    let Ok(m) = number(&v[5..7]) else {
        return false;
    };
    let Ok(d) = number(&v[8..]) else { return false };
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    y > 0 && d > 0 && d <= max
}
// Only the upstream chapter label can identify a regular episode. Titles and
// catalogue positions do not establish chapter numbers or continuation order.
fn explicit_chapter(label: &[u8]) -> Option<usize> {
    let label = core::str::from_utf8(label).ok()?.trim();
    let label = label.strip_prefix('第')?.strip_suffix('话')
        .or_else(|| label.strip_prefix('第')?.strip_suffix('話'))?;
    if !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit()) {
        return label.bytes().try_fold(0usize, |v, b| {
            v.checked_mul(10)?.checked_add((b - b'0') as usize)
        });
    }
    if label == "零" || label == "〇" { return Some(0); }
    let mut total = 0usize;
    let mut section = 0usize;
    let mut digit = None;
    let mut last_unit = usize::MAX;
    let mut last_section = usize::MAX;
    let mut zero = false;
    let mut digit_after_zero = false;
    for c in label.chars() {
        let value = match c {
            '一' => Some(1usize), '二' => Some(2), '三' => Some(3),
            '四' => Some(4), '五' => Some(5), '六' => Some(6),
            '七' => Some(7), '八' => Some(8), '九' => Some(9), _ => None,
        };
        if let Some(v) = value {
            if digit.is_some() { return None; }
            digit = Some(v); digit_after_zero = zero; zero = false; continue;
        }
        if c == '零' || c == '〇' {
            if zero || digit.is_some() || (section == 0 && total == 0) { return None; }
            zero = true; continue;
        }
        let unit = match c { '十' => 10, '百' => 100, '千' => 1000, _ => 0 };
        if unit > 0 {
            if zero || unit >= last_unit { return None; }
            if last_unit != usize::MAX && (last_unit / unit > 10) != digit_after_zero {
                return None;
            }
            let v = match digit.take() {
                Some(v) => v,
                None if unit == 10 && section == 0 => 1,
                _ => return None,
            };
            section = section.checked_add(v.checked_mul(unit)?)?;
            last_unit = unit;
            continue;
        }
        let scale = match c { '万' => 10000, '亿' => 100000000, _ => return None };
        if zero || scale >= last_section { return None; }
        section = section.checked_add(digit.take().unwrap_or(0))?;
        if section == 0 { return None; }
        total = total.checked_add(section.checked_mul(scale)?)?;
        section = 0; last_unit = usize::MAX; last_section = scale;
    }
    if zero { return None; }
    if digit.is_some() && ((last_unit != usize::MAX && last_unit > 10) ||
        (last_unit == usize::MAX && total > 0)) && !digit_after_zero { return None; }
    total.checked_add(section)?.checked_add(digit.unwrap_or(0))
        .filter(|v| *v > 0)
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mid = ident(string(req, b"mangaId")?, MP, 1)?.0;
        owner(req, mid)?;
        let len = fetch_list()?;
        book(len, mid)?;
        let len = fetch_chapters(mid)?;
        let mut it = JsonArrayIter::from_raw_array(&chapters_buf()[..len]).ok_or("parse_error")?;
        let p = payload_buf();
        let mut c = 0;
        let mut count = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        while let Some(v) = it.next_object() {
            if count > 0 {
                w(p, &mut c, b",")?
            }
            let cid = number(member(v, b"comicsChapterId")?)?;
            w(p, &mut c, b"{\"id\":")?;
            idout(p, &mut c, CP, mid, Some(cid), None)?;
            w(p, &mut c, b",\"mangaId\":")?;
            idout(p, &mut c, MP, mid, None, None)?;
            w(p, &mut c, b",\"title\":\"")?;
            if !append_json_unescaped_then_escaped(p, &mut c, string(v, b"chapter")?) {
                return Err("capacity_exceeded");
            }
            w(p, &mut c, b" ")?;
            if !append_json_unescaped_then_escaped(p, &mut c, string(v, b"title")?) {
                return Err("capacity_exceeded");
            }
            w(p, &mut c, b"\",\"chapterNumber\":")?;
            let mut label = [0; 4096];
            let label_len = raw_string(string(v, b"chapter")?, &mut label)?;
            if let Some(number) = explicit_chapter(&label[..label_len]) {
                n(p, &mut c, number)?;
                w(p, &mut c, b",\"readingGroup\":\"chapter\",\"continuationEligible\":true")?;
            } else {
                w(p, &mut c, b"null,\"readingGroup\":\"special\",\"continuationEligible\":false")?;
            }
            w(p, &mut c, b",\"volumeNumber\":null,\"publishedAt\":")?;
            let raw = string(v, b"createdAt")?;
            let mut date = [0; 64];
            let date_len = raw_string(raw, &mut date)?;
            if date_only(&date[..date_len]) {
                q_text(p, &mut c, &date[..date_len])?
            } else {
                w(p, &mut c, b"null")?
            }
            w(
                p,
                &mut c,
                b",\"updatedAt\":null,\"language\":\"zh\",\"pageCount\":null}",
            )?;
            count += 1;
        }
        w(
            p,
            &mut c,
            b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn pages(req: &[u8]) -> R<usize> {
    let (mid, cid, _) = ident(string(req, b"chapterId")?, CP, 2)?;
    owner(req, mid)?;
    let len = fetch_list()?;
    let b = book(len, mid)?;
    let len = fetch_chapters(mid)?;
    let (ch, prev, next) = chapter_at(len, cid)?;
    let d = fetch_content(mid, cid, b, ch, prev, next)?;
    let a = member(d, b"imageList")?;
    let mut i = skip(a, 1);
    let mut ordinal = 0;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"pages\":[")?;
    while a.get(i) != Some(&b']') {
        let e = scan(a, i, 0)?;
        if ordinal > 0 {
            w(p, &mut c, b",")?
        }
        w(p, &mut c, b"{\"id\":")?;
        idout(p, &mut c, PP, mid, Some(cid), Some(ordinal))?;
        w(p, &mut c, b",\"index\":")?;
        n(p, &mut c, ordinal)?;
        w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        let mut u = [0; 2048];
        let len = uri(&a[i + 1..e - 1], &mut u)?;
        q_text(p, &mut c, &u[..len])?;
        w(p, &mut c, b"}}")?;
        ordinal += 1;
        i = skip(a, e);
        if a.get(i) == Some(&b',') {
            i = skip(a, i + 1)
        }
    }
    w(p, &mut c, b"]}")?;
    Ok(c)
}
fn run_get_pages(req: &[u8]) -> u32 {
    result("get_pages", pages(req))
}
fn image(req: &[u8]) -> R<usize> {
    let id = string(req, b"pageId")?;
    let cover = id.starts_with(MP);
    let (mid, cid, ordinal) = ident(id, if cover { MP } else { PP }, if cover { 1 } else { 3 })?;
    owner(req, mid)?;
    if let Some(v) = json_object_member(req, b"chapterId") {
        if v != b"null" {
            if cover {
                return Err("invalid_request");
            }
            let (m, ch, _) = ident(string(req, b"chapterId")?, CP, 2)?;
            if m != mid || ch != cid {
                return Err("invalid_request");
            }
        }
    }
    let mut asked = [0; 2048];
    let alen = raw_string(string(req, b"url")?, &mut asked)?;
    let len = fetch_list()?;
    let b = book(len, mid)?;
    let original = if cover {
        let d = fetch_detail(mid, b)?;
        string(d, b"verticalCover")?
    } else {
        let len = fetch_chapters(mid)?;
        let (ch, prev, next) = chapter_at(len, cid)?;
        let d = fetch_content(mid, cid, b, ch, prev, next)?;
        pic_at(d, ordinal)?
    };
    let mut expected = [0; 2048];
    let elen = uri(original, &mut expected)?;
    if asked[..alen] != expected[..elen] {
        return Err("owner_mismatch");
    }
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
    q_text(p, &mut c, &expected[..elen])?;
    w(p, &mut c, b",\"headers\":{}}}")?;
    Ok(c)
}
fn run_get_image_request(req: &[u8]) -> u32 {
    result("get_image_request", image(req))
}
fn list(req: &[u8], search: bool) -> R<usize> {
    let pg = page(req)?;
    if pg > 10000 {
        return Err("invalid_request");
    }
    if !search && string(req, b"listingId")? != b"listing:all" {
        return Err("invalid_request");
    }
    let mut query = [0; 512];
    let len = if search {
        raw_string(string(req, b"query")?, &mut query)?
    } else {
        0
    };
    let query = &mut query[..len];
    query.make_ascii_lowercase();
    let query = core::str::from_utf8(query)
        .map_err(|_| "invalid_request")?
        .trim()
        .as_bytes();
    let len = fetch_list()?;
    let d = member(&list_buf()[..len], b"_data")?;
    let mut it = JsonArrayIter::from_raw_array(d).ok_or("parse_error")?;
    let mut matches = 0;
    let mut written = 0;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    while let Some(v) = it.next_object() {
        let mut title = [0; 4096];
        let len = raw_string(string(v, b"name")?, &mut title)?;
        let title = &mut title[..len];
        title.make_ascii_lowercase();
        if !query.is_empty() && !title.windows(query.len()).any(|x| x == query) {
            continue;
        }
        if matches >= (pg - 1) * 20 && matches < pg * 20 {
            if written > 0 {
                w(p, &mut c, b",")?
            }
            manga_write(p, &mut c, v, false, v)?;
            written += 1
        }
        matches += 1;
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
    let v = "{\"listings\":[{\"id\":\"listing:all\",\"name\":\"官方漫画\"}]}".as_bytes();
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
