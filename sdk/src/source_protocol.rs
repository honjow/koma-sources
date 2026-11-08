// Opt-in strict request/transport validation. The SDK's legacy
// extract_* helpers accept nested namesakes and duplicate keys; source identity
// and valid JSON search text must instead follow this strict direct-member path.
// HTTP denial status takes precedence, then Host transport completion, then body
// decoding. In particular status:200 + ok:false is never a site parse error.
use crate::{plain_text::decode_json_content, FetchError};
type R<T> = Result<T, &'static str>;
fn ws(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1
    }
    i
}
fn end(s: &[u8], start: usize, depth: usize) -> R<usize> {
    if depth > 64 {
        return Err("invalid_request");
    };
    let start = ws(s, start);
    let c = *s.get(start).ok_or("invalid_request")?;
    if c == b'"' {
        let mut i = start + 1;
        while i < s.len() {
            match s[i] {
                b'"' => return Ok(i + 1),
                b'\\' => {
                    i += 1;
                    let e = *s.get(i).ok_or("invalid_request")?;
                    if e == b'u' {
                        if i + 5 > s.len() || !s[i + 1..i + 5].iter().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("invalid_request");
                        };
                        i += 4
                    } else if !matches!(e, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') {
                        return Err("invalid_request");
                    }
                }
                c if c < 32 => return Err("invalid_request"),
                _ => {}
            }
            i += 1
        }
        return Err("invalid_request");
    }
    if c == b'{' || c == b'[' {
        let close = if c == b'{' { b'}' } else { b']' };
        let mut i = ws(s, start + 1);
        if s.get(i) == Some(&close) {
            return Ok(i + 1);
        }
        loop {
            if c == b'{' {
                if s.get(i) != Some(&b'"') {
                    return Err("invalid_request");
                };
                i = ws(s, end(s, i, depth + 1)?);
                if s.get(i) != Some(&b':') {
                    return Err("invalid_request");
                };
                i = ws(s, i + 1)
            }
            i = ws(s, end(s, i, depth + 1)?);
            match s.get(i) {
                Some(x) if *x == close => return Ok(i + 1),
                Some(b',') => {
                    i = ws(s, i + 1);
                    if s.get(i) == Some(&close) {
                        return Err("invalid_request");
                    }
                }
                _ => return Err("invalid_request"),
            }
        }
    }
    for word in [b"true".as_slice(), b"false", b"null"] {
        if s[start..].starts_with(word) {
            return Ok(start + word.len());
        }
    }
    let mut i = start;
    if s.get(i) == Some(&b'-') {
        i += 1
    }
    match s.get(i) {
        Some(b'0') => {
            i += 1;
            if s.get(i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("invalid_request");
            }
        }
        Some(b'1'..=b'9') => {
            i += 1;
            while s.get(i).is_some_and(|c| c.is_ascii_digit()) {
                i += 1
            }
        }
        _ => return Err("invalid_request"),
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        let at = i;
        while s.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1
        }
        if at == i {
            return Err("invalid_request");
        }
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(s.get(i), Some(b'+' | b'-')) {
            i += 1
        }
        let at = i;
        while s.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1
        }
        if at == i {
            return Err("invalid_request");
        }
    }
    Ok(i)
}
pub fn complete(s: &[u8]) -> R<()> {
    core::str::from_utf8(s).map_err(|_| "invalid_request")?;
    if ws(s, end(s, 0, 0)?) != s.len() {
        return Err("invalid_request");
    };
    Ok(())
}
pub fn direct<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    let mut i = ws(s, 0);
    if s.get(i) != Some(&b'{') {
        return Err("invalid_request");
    };
    i = ws(s, i + 1);
    let mut found = None;
    if s.get(i) == Some(&b'}') {
        return if ws(s, i + 1) == s.len() {
            Ok(None)
        } else {
            Err("invalid_request")
        };
    }
    loop {
        let e = end(s, i, 0)?;
        if s.get(i) != Some(&b'"') {
            return Err("invalid_request");
        };
        let mut kb = [0; 1024];
        let k = decode_json_content(&s[i + 1..e - 1], &mut kb).map_err(|_| "invalid_request")?;
        i = ws(s, e);
        if s.get(i) != Some(&b':') {
            return Err("invalid_request");
        };
        i = ws(s, i + 1);
        let e = end(s, i, 0)?;
        if k == key {
            if found.is_some() {
                return Err("invalid_request");
            };
            found = Some(&s[i..e])
        }
        i = ws(s, e);
        match s.get(i) {
            Some(b',') => i = ws(s, i + 1),
            Some(b'}') => {
                return if ws(s, i + 1) == s.len() {
                    Ok(found)
                } else {
                    Err("invalid_request")
                }
            }
            _ => return Err("invalid_request"),
        }
    }
}
pub fn raw_string<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    let Some(v) = direct(s, key)? else {
        return Ok(None);
    };
    if v.len() < 2 || v[0] != b'"' || v[v.len() - 1] != b'"' {
        return Err("invalid_request");
    };
    Ok(Some(&v[1..v.len() - 1]))
}
pub fn text<'a>(s: &[u8], key: &[u8], out: &'a mut [u8]) -> R<Option<&'a [u8]>> {
    match raw_string(s, key)? {
        Some(v) => decode_json_content(v, out)
            .map(Some)
            .map_err(|_| "invalid_request"),
        None => Ok(None),
    }
}
pub fn request(s: &[u8]) -> R<()> {
    complete(s)?;
    let a = match direct(s, b"args")? {
        Some(v) => {
            if v.first() != Some(&b'{') {
                return Err("invalid_request");
            };
            v
        }
        None => s,
    };
    let mut textbuf = [0; 16384];
    for key in [
        b"query".as_slice(),
        b"mangaId",
        b"chapterId",
        b"pageId",
        b"url",
        b"listingId",
    ] {
        let _ = text(a, key, &mut textbuf)?;
    }
    for key in [b"settings".as_slice(), b"hostHints"] {
        if let Some(v) = direct(s, key)? {
            if v.first() != Some(&b'{') {
                return Err("invalid_request");
            }
        }
    }
    if let Some(v) = direct(a, b"filters")? {
        if v.first() != Some(&b'{') {
            return Err("invalid_request");
        }
    }
    // Even optional identity keys may not be duplicated, null, or non-string.
    Ok(())
}
pub fn body(response: &[u8], out: &mut [u8]) -> R<usize> {
    let status = direct(response, b"status").map_err(|_| "network_error")?;
    let status = match status {
        Some(v) if v.iter().all(|c| c.is_ascii_digit()) && !v.is_empty() => {
            Some(crate::parse_status_code(v))
        }
        Some(b"null") | None => None,
        _ => return Err("network_error"),
    };
    if let Some(s) = status {
        match s {
            401 => return Err("auth_required"),
            403 => return Err("forbidden"),
            404 => return Err("not_found"),
            429 => return Err("rate_limit"),
            500..=599 => return Err("server_error"),
            200..=299 => {}
            _ => return Err("http_error"),
        }
    }
    if direct(response, b"ok").map_err(|_| "network_error")? != Some(b"true") {
        return Err("network_error");
    }
    if status.is_none() {
        return Err("network_error");
    }
    crate::decode_json_body_into(response, out).map_err(|e| match e {
        FetchError::Network => "network_error",
        FetchError::NotFound => "not_found",
        FetchError::RateLimit => "rate_limit",
        FetchError::ClientError => "client_error",
        FetchError::ServerError => "server_error",
    })
}
pub fn feature(r: &[u8], name: &[u8]) -> bool {
    let Ok(Some(v)) = direct(r, b"hostFeatures") else {
        return false;
    };
    let v = v.trim_ascii();
    if v.first() != Some(&b'[') || v.last() != Some(&b']') {
        return false;
    }
    let mut i = ws(v, 1);
    let mut found = false;
    while v.get(i) != Some(&b']') {
        let Ok(e) = end(v, i, 0) else { return false };
        if v.get(i) != Some(&b'"') {
            return false;
        };
        let mut b = [0; 128];
        let Ok(t) = decode_json_content(&v[i + 1..e - 1], &mut b) else {
            return false;
        };
        found |= t == name;
        i = ws(v, e);
        match v.get(i) {
            Some(b',') => i = ws(v, i + 1),
            Some(b']') => break,
            _ => return false,
        }
    }
    found
}
