#![no_std]
// Based on Keiyoushi Xkcd.kt, Apache-2.0, revision 1319949ca9bc456908196e3ba2d104e94182c5d6.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xkcd.zh.koma",
    name: "xkcd",
    version: "0.1.6",
    api_version: "0.2",
    language: "zh",
    author: "Koma source contributors",
    description: "Chinese xkcd translations. Actual image pages only; no synthesized text pages.",
    content_rating: "safe",
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
    settings: true,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:2*1024*1024,http_req:16384,scratch:16384,}
koma_source_sdk::koma_source_helpers!();
fn bytes(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    if write_bytes(b, n, s) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn q(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, s) {
        return Err("payload overflow");
    };
    bytes(b, n, b"\"")
}
fn number(s: &[u8]) -> Option<usize> {
    if s.is_empty() {
        return None;
    };
    let mut n = 0usize;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        };
        n = n.checked_mul(10)?.checked_add((c - b'0') as usize)?;
    }
    Some(n)
}
fn ws(s: &[u8], mut p: usize) -> usize {
    while p < s.len() && s[p].is_ascii_whitespace() {
        p += 1
    }
    p
}
fn json_string_end(s: &[u8], p: usize) -> Option<usize> {
    if s.get(p) != Some(&b'"') {
        return None;
    };
    let mut i = p + 1;
    while i < s.len() {
        match s[i] {
            b'"' => return Some(i + 1),
            0..=31 => return None,
            b'\\' => {
                i += 1;
                match *s.get(i)? {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                    b'u' => {
                        hex4(s.get(i + 1..i + 5)?)?;
                        i += 4;
                    }
                    _ => return None,
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
fn json_value_end(s: &[u8], p: usize, depth: usize) -> Option<usize> {
    if depth > 32 {
        return None;
    };
    let p = ws(s, p);
    match *s.get(p)? {
        b'"' => json_string_end(s, p),
        b'{' | b'[' => {
            let object = s[p] == b'{';
            let close = if object { b'}' } else { b']' };
            let mut i = ws(s, p + 1);
            if s.get(i) == Some(&close) {
                return Some(i + 1);
            };
            loop {
                if object {
                    i = json_string_end(s, i)?;
                    i = ws(s, i);
                    if s.get(i) != Some(&b':') {
                        return None;
                    };
                    i = ws(s, i + 1)
                }
                i = json_value_end(s, i, depth + 1)?;
                i = ws(s, i);
                if s.get(i) == Some(&close) {
                    return Some(i + 1);
                };
                if s.get(i) != Some(&b',') {
                    return None;
                };
                i = ws(s, i + 1);
            }
        }
        b't' => {
            if s.get(p..p + 4) == Some(b"true") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'f' => {
            if s.get(p..p + 5) == Some(b"false") {
                Some(p + 5)
            } else {
                None
            }
        }
        b'n' => {
            if s.get(p..p + 4) == Some(b"null") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'-' | b'0'..=b'9' => {
            let mut i = p;
            if s[i] == b'-' {
                i += 1
            };
            if s.get(i) == Some(&b'0') {
                i += 1
            } else {
                if !matches!(s.get(i), Some(b'1'..=b'9')) {
                    return None;
                };
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
            }
            if s.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            if matches!(s.get(i), Some(b'e' | b'E')) {
                i += 1;
                if matches!(s.get(i), Some(b'+' | b'-')) {
                    i += 1
                };
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            Some(i)
        }
        _ => None,
    }
}
fn args(r: &[u8]) -> &[u8] {
    json_object_member(r, b"args").unwrap_or(r)
}
fn page(r: &[u8]) -> Result<usize, &'static str> {
    let a = args(r);
    let raw = match json_object_member(a, b"page") {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if raw.first() == Some(&b'{') {
        json_object_member(raw, b"cursor").unwrap_or(b"null")
    } else {
        raw
    };
    if v == b"null" {
        return Ok(1);
    };
    let v = if v.first() == Some(&b'"') && v.last() == Some(&b'"') {
        &v[1..v.len() - 1]
    } else {
        v
    };
    number(v).filter(|n| *n > 0).ok_or("invalid page cursor")
}
fn string<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let v = json_object_member(obj, key)?;
    if v.len() < 2 || v[0] != b'"' || *v.last()? != b'"' {
        None
    } else {
        Some(&v[1..v.len() - 1])
    }
}
fn hex4(s: &[u8]) -> Option<u32> {
    if s.len() != 4 {
        return None;
    };
    let mut n = 0;
    for &c in s {
        n = n * 16
            + match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => return None,
            };
    }
    Some(n)
}
fn unescape<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != b'\\' {
            if c < 32 || c == b'"' {
                return Err("invalid JSON string");
            };
            bytes(out, &mut n, &[c])?;
            continue;
        }
        let c = *s.get(i).ok_or("invalid JSON escape")?;
        i += 1;
        let c = match c {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'b' => 8,
            b'f' => 12,
            b'u' => {
                let mut cp = hex4(s.get(i..i + 4).ok_or("invalid unicode escape")?)
                    .ok_or("invalid unicode escape")?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if s.get(i..i + 2) != Some(b"\\u") {
                        return Err("invalid unicode pair");
                    };
                    i += 2;
                    let low = hex4(s.get(i..i + 4).ok_or("invalid unicode pair")?)
                        .ok_or("invalid unicode pair")?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("invalid unicode pair");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("invalid unicode pair");
                };
                let ch = char::from_u32(cp).ok_or("invalid unicode scalar")?;
                let mut tmp = [0; 4];
                bytes(out, &mut n, ch.encode_utf8(&mut tmp).as_bytes())?;
                continue;
            }
            _ => return Err("invalid JSON escape"),
        };
        bytes(out, &mut n, &[c])?;
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}

fn diagnostic(stage: &[u8], code: &[u8]) {
    let mut out = [0; 160];
    let mut n = 0;
    if write_bytes(&mut out, &mut n, b"xkcd_zh stage=")
        && write_bytes(&mut out, &mut n, stage)
        && write_bytes(&mut out, &mut n, b" code=")
        && write_bytes(&mut out, &mut n, code)
    {
        host::log_info(&out[..n]);
    }
}
fn code(e: &str) -> &'static str {
    match e {
        "interactive comic unsupported" => "unsupported",
        "permission denied" => "permission_denied",
        "not found" => "not_found",
        "rate limited" => "rate_limit",
        "network failure" => "network_error",
        "timeout" => "timeout",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "invalid request"
        | "invalid page cursor"
        | "missing query"
        | "empty query"
        | "unknown listing"
        | "missing image URL"
        | "invalid image URL" => "invalid_request",
        "payload overflow" => "internal_error",
        _ => "parse_error",
    }
}
fn execute(op: &str, r: Result<usize, &'static str>) -> u32 {
    match r {
        Ok(n) => {
            diagnostic(b"operation_ok", op.as_bytes());
            write_success_payload(op, n)
        }
        Err(e) => {
            let c = code(e);
            diagnostic(b"operation_error", c.as_bytes());
            write_error(op, c, e)
        }
    }
}
fn valid_status(s: &[u8]) -> Option<usize> {
    number(s).filter(|n| (100..=599).contains(n))
}
fn input(r: &[u8]) -> Result<&[u8], &'static str> {
    core::str::from_utf8(r).map_err(|_| "invalid UTF8")?;
    if r.len() > 65536
        || r.first() != Some(&b'{')
        || json_value_end(r, 0, 0).map(|p| ws(r, p)) != Some(r.len())
    {
        return Err("invalid request");
    }
    let a = args(r);
    if a.first() != Some(&b'{') {
        return Err("invalid request");
    };
    Ok(a)
}
fn http(url: &[u8]) -> Result<&'static [u8], &'static str> {
    diagnostic(b"fetch_begin", b"request");
    let extra: &[(&[u8], &[u8])] = &[(b"Origin", b"https://xkcd.tw")];
    let n =
        koma_source_sdk::build_get_request(http_req_buf(), url, Some(b"https://xkcd.tw/"), extra)
            .ok_or("invalid request")?;
    let len = host::http_request(&http_req_buf()[..n], http_out()).map_err(|e| {
        diagnostic(
            b"http_return",
            match e {
                -1 => b"-1",
                -2 => b"-2",
                -3 => b"-3",
                _ => b"other",
            },
        );
        "network failure"
    })?;
    if len >= http_out().len() {
        return Err("HTTP envelope overflow");
    }
    let r = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT).cast::<u8>(), len) };
    if json_value_end(r, 0, 0).map(|p| ws(r, p)) != Some(r.len()) {
        return Err("invalid HTTP envelope");
    }
    let status = json_object_member(r, b"status")
        .and_then(valid_status)
        .or_else(|| json_object_member(r, b"statusCode").and_then(valid_status));
    let mut log = [0; 80];
    let mut l = 0;
    bytes(&mut log, &mut l, b"xkcd_zh stage=http_status value=")?;
    if !write_usize(&mut log, &mut l, status.unwrap_or(0)) {
        return Err("payload overflow");
    };
    host::log_info(&log[..l]);
    match status {
        Some(200..=299) => {}
        Some(401 | 403) => return Err("permission denied"),
        Some(404) => return Err("not found"),
        Some(429) => return Err("rate limited"),
        Some(400..=499) => return Err("client failure"),
        Some(500..=599) => return Err("server failure"),
        Some(_) => return Err("unexpected HTTP status"),
        None => {
            if json_object_member(r, b"ok") == Some(b"false".as_slice()) {
                return Err(
                    match string(json_object_member(r, b"error").unwrap_or(r), b"code") {
                        Some(b"timeout") => "timeout",
                        Some(
                            b"network_error"
                            | b"transport_unavailable"
                            | b"transport_invalid_response",
                        ) => "network failure",
                        _ => "invalid HTTP envelope",
                    },
                );
            };
            return Err("invalid HTTP envelope");
        }
    }
    let n = koma_source_sdk::decode_json_body_into(r, body_buf())
        .map_err(|_| "HTTP body invalid or too large")?;
    if n == 0 || n >= body_buf().len() {
        return Err("HTTP body empty or overflow");
    }
    let body =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), n) };
    core::str::from_utf8(body).map_err(|_| "invalid UTF8")?;
    diagnostic(b"body_ready", b"ok");
    Ok(body)
}

const ARCHIVE: &[u8] = b"https://xkcd.tw/api/strips.json";
fn valid_image(s: &[u8]) -> Result<(), &'static str> {
    let path = s
        .strip_prefix(b"https://xkcd.tw/strip/")
        .ok_or("invalid image URL")?;
    let digits = path.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0
        || number(&path[..digits]).filter(|n| *n > 0).is_none()
        || !matches!(&path[digits..], b".png" | b".jpg" | b".jpeg" | b".gif")
    {
        return Err("invalid image URL");
    };
    Ok(())
}
fn image<'a>(obj: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut path = [0; 1024];
    let v = unescape(
        string(obj, b"img_url").ok_or("missing image URL")?,
        &mut path,
    )?;
    let mut n = 0;
    bytes(out, &mut n, b"https://xkcd.tw")?;
    bytes(out, &mut n, v)?;
    if v != b"/s/1190/00/0001.png" {
        valid_image(&out[..n])?;
    }
    Ok(&out[..n])
}
// The real archive is an insertion-ordered object. Validate every item before rendering any output.
fn walk<'a, F: FnMut(&'a [u8], &'a [u8], usize) -> Result<(), &'static str>>(
    body: &'a [u8],
    mut f: F,
) -> Result<usize, &'static str> {
    if body.first() != Some(&b'{')
        || json_value_end(body, 0, 0).map(|p| ws(body, p)) != Some(body.len())
    {
        return Err("invalid archive JSON");
    };
    let mut p = ws(body, 1);
    let mut count = 0;
    let mut seen = [0usize; 4096];
    while body.get(p) != Some(&b'}') {
        if count == 4096 {
            return Err("too many chapters");
        };
        let ke = json_string_end(body, p).ok_or("invalid archive key")?;
        let key = &body[p + 1..ke - 1];
        let id = number(key).filter(|n| *n > 0).ok_or("invalid archive ID")?;
        if seen[..count].contains(&id) {
            return Err("duplicate archive ID");
        };
        seen[count] = id;
        p = ws(body, ke);
        if body.get(p) != Some(&b':') {
            return Err("invalid archive JSON");
        };
        p = ws(body, p + 1);
        let e = json_value_end(body, p, 0).ok_or("invalid archive JSON")?;
        let obj = &body[p..e];
        if obj.first() != Some(&b'{') || json_object_member(obj, b"id").and_then(number) != Some(id)
        {
            return Err("archive identity mismatch");
        };
        let mut title = [0; 4096];
        let title = unescape(string(obj, b"title").ok_or("missing title")?, &mut title)?;
        if title.is_empty() {
            return Err("missing title");
        };
        let mut url = [0; 1200];
        let url = image(obj, &mut url)?;
        let mut expected = [0; 1200];
        let mut l = 0;
        bytes(&mut expected, &mut l, b"https://xkcd.tw/strip/")?;
        bytes(&mut expected, &mut l, key)?;
        if !(key == b"1190" && url == b"https://xkcd.tw/s/1190/00/0001.png")
            && (!url.starts_with(&expected[..l]) || url.get(l) != Some(&b'.'))
        {
            return Err("archive image identity mismatch");
        };
        f(key, obj, count)?;
        count += 1;
        p = ws(body, e);
        match body.get(p) {
            Some(b',') => p = ws(body, p + 1),
            Some(b'}') => {}
            _ => return Err("invalid archive JSON"),
        }
    }
    if count == 0 {
        return Err("empty archive");
    };
    Ok(count)
}
fn archive() -> Result<&'static [u8], &'static str> {
    let body = http(ARCHIVE)?;
    walk(body, |_, _, _| Ok(()))?;
    Ok(body)
}
// The single-book IDs are an established consumer contract. Group IDs are
// independent and only selected explicitly; old books never follow a setting.
const GROUP_ARCHIVE_CAP: usize = 2 * 1024 * 1024;
static mut GROUP_ARCHIVE: [u8; GROUP_ARCHIVE_CAP] = [0; GROUP_ARCHIVE_CAP];
static mut EN_DATES: [u32; 8192] = [0; 8192]; // comic number -> YYYYMM

fn organization(r: &[u8]) -> u8 {
    match json_object_member(r, b"settings").and_then(|v| string(v, b"organization_method")) {
        Some(b"BY_YEAR") => 1,
        Some(b"BY_YEAR_MONTH") => 2,
        _ => 0,
    }
}
fn group_id(r: &[u8]) -> Result<Option<u32>, &'static str> {
    let id = string(input(r)?, b"mangaId").ok_or("invalid request")?;
    if id == b"xkcd:zh" {
        return Ok(None);
    }
    let raw = id.strip_prefix(b"xkcd:zh:g:").ok_or("invalid request")?;
    let key = number(raw)
        .filter(|v| *v >= 100000 && *v <= 999912)
        .ok_or("invalid request")? as u32;
    if key % 100 > 12 {
        return Err("invalid request");
    }
    Ok(Some(key))
}
fn grouped_archive() -> Result<&'static [u8], &'static str> {
    let zh = archive()?;
    let len = zh.len();
    if len > GROUP_ARCHIVE_CAP {
        return Err("payload overflow");
    }
    let saved = unsafe { &mut *core::ptr::addr_of_mut!(GROUP_ARCHIVE) };
    saved[..len].copy_from_slice(zh);
    let en = http(b"https://xkcd.com/archive/")?;
    let doc = host::html_parse(en).map_err(|_| "invalid English archive")?;
    let selector = b"#middleContainer > a";
    let selected = unsafe {
        koma_host_html_select_all(
            doc.raw(),
            selector.as_ptr(),
            selector.len() as u32,
            core::ptr::addr_of_mut!(BODY_NODES).cast::<u8>(),
            BODY_NODES_CAP as u32,
        )
    };
    let count = if selected > 0 { selected as usize } else { 0 };
    let dates = unsafe { &mut *core::ptr::addr_of_mut!(EN_DATES) };
    dates.fill(0);
    let mut valid = true;
    if count == 0 || count > BODY_NODES_CAP / 4 {
        valid = false;
    }
    for i in 0..core::cmp::min(count, BODY_NODES_CAP / 4) {
        let node = body_node(i);
        let mut href = [0u8; 64];
        let mut title = [0u8; 64];
        let parsed = (|| {
            let hn = host::html_attr(node, b"href", &mut href).ok()?;
            let tn = host::html_attr(node, b"title", &mut title).ok()?;
            let h = href.get(..hn)?.strip_prefix(b"/")?.strip_suffix(b"/")?;
            let id = number(h)?;
            if id == 0 || id >= dates.len() {
                return None;
            }
            let date = title.get(..tn)?;
            let dash = find_subslice(date, b"-")?;
            let year = number(&date[..dash])?;
            let rest = &date[dash + 1..];
            let dash2 = find_subslice(rest, b"-")?;
            let month = number(&rest[..dash2])?;
            let day = number(&rest[dash2 + 1..])?;
            if !(1..=12).contains(&month)
                || !(1..=31).contains(&day)
                || !(1900..=9999).contains(&year)
                || dates[id] != 0
            {
                return None;
            }
            dates[id] = (year * 100 + month) as u32;
            Some(())
        })();
        if parsed.is_none() {
            valid = false;
        }
        let _ = host::html_close(node);
    }
    let _ = host::html_close(doc);
    if !valid {
        return Err("invalid English archive");
    }
    let body = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(GROUP_ARCHIVE).cast::<u8>(), len)
    };
    walk(body, |id, _, _| {
        let n = number(id).ok_or("invalid archive ID")?;
        if n >= dates.len() || dates[n] == 0 {
            return Err("missing English date");
        }
        Ok(())
    })?;
    Ok(body)
}
fn comic_group(id: &[u8], mode: u8) -> Result<u32, &'static str> {
    let id = number(id).ok_or("invalid archive ID")?;
    let dates = unsafe { &*core::ptr::addr_of!(EN_DATES) };
    let key = *dates.get(id).ok_or("missing English date")?;
    if key == 0 {
        return Err("missing English date");
    }
    Ok(if mode == 1 { key / 100 * 100 } else { key })
}
fn group_mode(key: u32) -> u8 {
    if key % 100 == 0 {
        1
    } else {
        2
    }
}
fn group_label(buf: &mut [u8], n: &mut usize, key: u32) -> Result<(), &'static str> {
    bytes(buf, n, b"xkcd ")?;
    if !write_usize(buf, n, (key / 100) as usize) {
        return Err("payload overflow");
    }
    if key % 100 != 0 {
        bytes(buf, n, b"-")?;
        if key % 100 < 10 {
            bytes(buf, n, b"0")?;
        }
        if !write_usize(buf, n, (key % 100) as usize) {
            return Err("payload overflow");
        }
    }
    Ok(())
}
fn group_manga(
    b: &mut [u8],
    n: &mut usize,
    body: &[u8],
    key: u32,
    detail: bool,
) -> Result<(), &'static str> {
    let mut cover = [0u8; 1200];
    let mut cover_len = 0;
    walk(body, |id, obj, _| {
        if cover_len == 0 && comic_group(id, group_mode(key))? == key {
            cover_len = image(obj, &mut cover)?.len();
        }
        Ok(())
    })?;
    if cover_len == 0 {
        return Err("not found");
    }
    bytes(b, n, b"{\"id\":\"xkcd:zh:g:")?;
    if !write_usize(b, n, key as usize) {
        return Err("payload overflow");
    }
    bytes(b, n, b"\",\"title\":\"")?;
    group_label(b, n, key)?;
    bytes(
        b,
        n,
        b"\",\"authors\":[\"Randall Munroe\"],\"description\":",
    )?;
    q(
        b,
        n,
        "這裡翻譯某個關於浪漫、諷刺、數學、以及語言的漫畫".as_bytes(),
    )?;
    bytes(b,n,b",\"language\":\"zh\",\"status\":\"ongoing\",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, n, &cover[..cover_len])?;
    bytes(b, n, b"}")?;
    if detail {
        bytes(
            b,
            n,
            br#","links":[{"kind":"source","url":"https://xkcd.tw/"}]"#,
        )?;
    }
    bytes(b, n, b"}")
}
fn grouped_list(r: &[u8], mode: u8) -> Result<usize, &'static str> {
    let p = page(r)?;
    let a = input(r)?;
    if let Some(id) = string(a, b"listingId") {
        if id != b"popular" {
            return Err("unknown listing");
        }
    }
    let body = grouped_archive()?;
    let mut keys = [0u32; 4096];
    let mut count = 0usize;
    walk(body, |id, _, _| {
        let k = comic_group(id, mode)?;
        if !keys[..count].contains(&k) {
            if count >= keys.len() {
                return Err("payload overflow");
            }
            keys[count] = k;
            count += 1;
        }
        Ok(())
    })?;
    keys[..count].sort_unstable_by(|a, b| b.cmp(a));
    let start = p.saturating_sub(1).saturating_mul(10);
    let end = core::cmp::min(start.saturating_add(10), count);
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    if start < count {
        for i in start..end {
            if i > start {
                bytes(b, &mut n, b",")?;
            }
            group_manga(b, &mut n, body, keys[i], false)?;
        }
    }
    bytes(b, &mut n, b"],\"page\":{\"hasMore\":")?;
    if end < count {
        bytes(b, &mut n, b"true,\"nextCursor\":\"")?;
        if !write_usize(b, &mut n, p + 1) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\"}}")?;
    } else {
        bytes(b, &mut n, b"false,\"nextCursor\":null}}")?;
    }
    Ok(n)
}
fn grouped_detail(key: u32) -> Result<usize, &'static str> {
    let body = grouped_archive()?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    group_manga(b, &mut n, body, key, true)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn grouped_chapters(key: u32) -> Result<usize, &'static str> {
    let body = grouped_archive()?;
    let b = payload_buf();
    let mut n = 0;
    let mut written = 0usize;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    walk(body, |id, obj, _| {
        if comic_group(id, group_mode(key))? != key {
            return Ok(());
        }
        if written > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"xkcdch:zh:g:")?;
        if !write_usize(b, &mut n, key as usize) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"xkcd:zh:g:")?;
        if !write_usize(b, &mut n, key as usize) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\",\"title\":")?;
        let mut title = [0u8; 4096];
        let title = unescape(string(obj, b"title").ok_or("missing title")?, &mut title)?;
        let mut full = [0u8; 4200];
        let mut l = 0;
        bytes(&mut full, &mut l, id)?;
        bytes(&mut full, &mut l, b": ")?;
        bytes(&mut full, &mut l, title)?;
        q(b, &mut n, &full[..l])?;
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        bytes(b, &mut n, id)?;
        bytes(
            b,
            &mut n,
            b",\"volumeNumber\":null,\"uploadDate\":null,\"index\":",
        )?;
        if !write_usize(b, &mut n, written) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"}")?;
        written += 1;
        Ok(())
    })?;
    if written == 0 {
        return Err("not found");
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    Ok(n)
}

fn book_request(r: &[u8]) -> Result<(), &'static str> {
    if string(input(r)?, b"mangaId") != Some(b"xkcd:zh".as_slice()) {
        return Err("invalid request");
    };
    Ok(())
}
fn manga(b: &mut [u8], n: &mut usize, body: &[u8], detail: bool) -> Result<(), &'static str> {
    let mut cover = [0; 1200];
    let mut len = 0;
    walk(body, |_, obj, i| {
        if i == 0 {
            let u = image(obj, &mut cover)?;
            len = u.len()
        };
        Ok(())
    })?;
    bytes(
        b,
        n,
        b"{\"id\":\"xkcd:zh\",\"title\":\"xkcd\",\"authors\":[\"Randall Munroe\"],\"description\":",
    )?;
    q(
        b,
        n,
        "這裡翻譯某個關於浪漫、諷刺、數學、以及語言的漫畫".as_bytes(),
    )?;
    bytes(b,n,b",\"language\":\"zh\",\"status\":\"ongoing\",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, n, &cover[..len])?;
    bytes(b, n, b"}")?;
    if detail {
        bytes(
            b,
            n,
            br#","links":[{"kind":"source","url":"https://xkcd.tw/"}]"#,
        )?;
    }
    bytes(b, n, b"}")
}
fn list(r: &[u8]) -> Result<usize, &'static str> {
    let mode = organization(r);
    if mode != 0 {
        return grouped_list(r, mode);
    }
    let a = input(r)?;
    if page(r)? != 1 {
        return Err("invalid page cursor");
    };
    if let Some(id) = string(a, b"listingId") {
        if id != b"popular" {
            return Err("unknown listing");
        }
    };
    let body = archive()?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    manga(b, &mut n, body, false)?;
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    if let Some(key) = group_id(r)? {
        return grouped_detail(key);
    }
    book_request(r)?;
    let body = archive()?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    manga(b, &mut n, body, true)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    if let Some(key) = group_id(r)? {
        return grouped_chapters(key);
    }
    book_request(r)?;
    let body = archive()?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    walk(body, |id, obj, i| {
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":\"xkcdch:zh:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"xkcd:zh\",\"title\":")?;
        let mut t = [0; 4096];
        let title = unescape(string(obj, b"title").ok_or("missing title")?, &mut t)?;
        let mut full = [0; 4200];
        let mut l = 0;
        bytes(&mut full, &mut l, id)?;
        bytes(&mut full, &mut l, b": ")?;
        bytes(&mut full, &mut l, title)?;
        q(b, &mut n, &full[..l])?;
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        bytes(b, &mut n, id)?;
        bytes(
            b,
            &mut n,
            b",\"volumeNumber\":null,\"uploadDate\":null,\"index\":",
        )?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"}")
    })?;
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let chapter_id = string(input(r)?, b"chapterId").ok_or("invalid request")?;
    let tail = chapter_id
        .strip_prefix(b"xkcdch:zh:")
        .ok_or("invalid request")?;
    let (group, id) = if let Some(rest) = tail.strip_prefix(b"g:") {
        let sep = find_subslice(rest, b":").ok_or("invalid request")?;
        let key = number(&rest[..sep])
            .filter(|v| *v >= 100000 && *v <= 999912 && *v % 100 <= 12)
            .ok_or("invalid request")? as u32;
        (Some(key), &rest[sep + 1..])
    } else {
        (None, tail)
    };
    number(id).filter(|n| *n > 0).ok_or("invalid request")?;
    // A generated chapter ID already contains the original comic number.
    // Reading it must not depend on the English grouping archive remaining online.
    let body = archive()?;
    let mut found = None;
    walk(body, |key, obj, _| {
        if key == id {
            found = Some(obj)
        };
        Ok(())
    })?;
    let obj = found.ok_or("not found")?;
    // Archive establishes chapter membership, not the role of its preview image.
    let _ = obj;
    let mut chapter_url = [0; 128];
    let mut l = 0;
    bytes(&mut chapter_url, &mut l, b"https://xkcd.tw/")?;
    bytes(&mut chapter_url, &mut l, id)?;
    let html = http(&chapter_url[..l])?;
    let document = host::html_parse(html).map_err(|_| "invalid chapter HTML")?;
    let selector = b"#content > img:not([id])";
    let selected = unsafe {
        koma_host_html_select_all(
            document.raw(),
            selector.as_ptr(),
            selector.len() as u32,
            core::ptr::addr_of_mut!(BODY_NODES).cast::<u8>(),
            BODY_NODES_CAP as u32,
        )
    };
    let count = if selected > 0 { selected as usize } else { 0 };
    let result = (|| {
        if selected < 0 {
            return Err("invalid chapter HTML");
        }
        if count > BODY_NODES_CAP / 4 {
            return Err("payload overflow");
        }
        if count == 0 {
            return Err("interactive comic unsupported");
        }
        let b = payload_buf();
        let mut n = 0;
        bytes(b, &mut n, b"{\"chapterId\":")?;
        q(b, &mut n, chapter_id)?;
        bytes(b, &mut n, b",\"pages\":[")?;
        for i in 0..count {
            let node = body_node(i);
            let mut src = [0; 1200];
            let len = host::html_attr(node, b"src", &mut src).map_err(|_| "missing image URL")?;
            // Attr may truncate to capacity: never accept a truncated URI.
            if len == src.len() {
                return Err("payload overflow");
            }
            let mut url = [0; 1200];
            let mut u = 0;
            if src[..len].starts_with(b"/") && !src[..len].starts_with(b"//") {
                bytes(&mut url, &mut u, b"https://xkcd.tw")?;
            }
            bytes(&mut url, &mut u, &src[..len])?;
            valid_image(&url[..u])?;
            if i > 0 {
                bytes(b, &mut n, b",")?;
            }
            bytes(b, &mut n, b"{\"id\":\"xkcdpage:zh:")?;
            if let Some(group) = group {
                bytes(b, &mut n, b"g:")?;
                if !write_usize(b, &mut n, group as usize) {
                    return Err("payload overflow");
                }
                bytes(b, &mut n, b":")?;
            }
            bytes(b, &mut n, id)?;
            bytes(b, &mut n, b":")?;
            if !write_usize(b, &mut n, i) {
                return Err("payload overflow");
            }
            bytes(b, &mut n, b"\",\"index\":")?;
            if !write_usize(b, &mut n, i) {
                return Err("payload overflow");
            }
            bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(b, &mut n, &url[..u])?;
            bytes(b, &mut n, b"}}")?;
        }
        bytes(b, &mut n, b"]}")?;
        Ok(n)
    })();
    // SelectAll returns total count even if the descriptor buffer is insufficient.
    // Release every descriptor actually written on all success/error paths.
    for i in 0..core::cmp::min(count, BODY_NODES_CAP / 4) {
        let _ = host::html_close(body_node(i));
    }
    let _ = host::html_close(document);
    result
}
const BODY_NODES_CAP: usize = 1024 * 1024;
static mut BODY_NODES: [u8; BODY_NODES_CAP] = [0; BODY_NODES_CAP];
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
fn body_node(i: usize) -> host::HtmlDescriptor {
    let p = unsafe { core::ptr::addr_of!(BODY_NODES).cast::<u8>().add(i * 4) };
    let raw = unsafe { core::ptr::read_unaligned(p.cast::<i32>()) };
    host::HtmlDescriptor::from_raw(i32::from_le(raw))
}

fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let mut tmp = [0; 1200];
    let u = unescape(
        string(input(r)?, b"url").ok_or("missing image URL")?,
        &mut tmp,
    )?;
    valid_image(u)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://xkcd.tw/\",\"Origin\":\"https://xkcd.tw\"}}")?;
    Ok(n)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(r: &[u8]) -> u32 {
    let data = if organization(r) == 0 {
        br#"{"listings":[{"id":"popular","name":"xkcd","supportsPaging":false}]}"#.as_slice()
    } else {
        br#"{"listings":[{"id":"popular","name":"xkcd","supportsPaging":true}]}"#.as_slice()
    };
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_search(_: &[u8]) -> u32 {
    write_error("search", "unsupported", "search unsupported")
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "unsupported", "filters unsupported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    let data = r#"{"settings":[{"id":"organization_method","name":"組織方式","kind":"select","options":[{"value":"SINGLE","label":"單本（全部漫畫）"},{"value":"BY_YEAR","label":"按年"},{"value":"BY_YEAR_MONTH","label":"按年月"}],"default":"SINGLE","description":"變更組織方式會使用獨立作品與閱讀進度；原單本資料保留。"}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_settings", data.len())
}
koma_source_sdk::koma_source_exports!("xkcd_zh");
