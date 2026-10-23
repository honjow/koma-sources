// Structural validator follows pgr-zh's bounded source-local JSON contract.
// Refuse ambiguous/malformed bodies; HTTP refusal alone is never a challenge.
type R<T> = Result<T, &'static str>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AccessSignal { Challenge, Authentication }

pub fn from_host_response(status: u16, response: &[u8], body: &mut [u8]) -> Option<AccessSignal> {
    if !matches!(status, 401 | 403) { return None; }
    valid_json(response).ok()?;
    if member(response, b"ok")? != b"true" { return None; }
    let raw = member(response, b"bodyText")?;
    let len = decode_string(raw, body)?;
    from_body(status, &body[..len])
}

fn from_body(status: u16, body: &[u8]) -> Option<AccessSignal> {
    if !matches!(status, 401 | 403) { return None; }
    valid_json(body).ok()?;
    let mut error = [0u8; 32];
    let len = decode_string(member(body, b"error")?, &mut error)?;
    match &error[..len] {
        b"challenge_required" => Some(AccessSignal::Challenge),
        b"auth_required" => Some(AccessSignal::Authentication),
        _ => None,
    }
}

// Call only after complete validation, so every scanned member is unambiguous.
fn member<'a>(v: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = skip(v, 0);
    if v.get(i) != Some(&b'{') { return None; }
    i = skip(v, i + 1);
    while v.get(i) == Some(&b'"') {
        let end = scan(v, i, 0).ok()?;
        let matches = &v[i + 1..end - 1] == key;
        i = skip(v, end);
        i = skip(v, i + 1); // validated colon
        let end = scan(v, i, 0).ok()?;
        if matches { return Some(&v[i..end]); }
        i = skip(v, end);
        if v.get(i) != Some(&b',') { return None; }
        i = skip(v, i + 1);
    }
    None
}

fn decode_string(raw: &[u8], out: &mut [u8]) -> Option<usize> {
    if raw.first() != Some(&b'"') || raw.last() != Some(&b'"') { return None; }
    let mut i = 1;
    let mut n = 0;
    while i < raw.len() - 1 {
        let mut bytes = [0u8; 4];
        let value: &[u8];
        if raw[i] != b'\\' { bytes[0] = raw[i]; value = &bytes[..1]; i += 1; }
        else {
            i += 1;
            bytes[0] = match raw[i] {
                b'"' => b'"', b'\\' => b'\\', b'/' => b'/', b'n' => b'\n',
                b'r' => b'\r', b't' => b'\t', b'b' => 8, b'f' => 12,
                b'u' => {
                    let hex = core::str::from_utf8(raw.get(i + 1..i + 5)?).ok()?;
                    let code = u32::from_str_radix(hex, 16).ok()?;
                    // Unpaired/surrogate escapes cannot create a trusted signal.
                    let c = char::from_u32(code)?;
                    value = c.encode_utf8(&mut bytes).as_bytes();
                    i += 5;
                    let dst = out.get_mut(n..n + value.len())?;
                    dst.copy_from_slice(value); n += value.len();
                    continue;
                }
                _ => return None,
            };
            i += 1; value = &bytes[..1];
        }
        let dst = out.get_mut(n..n + value.len())?;
        dst.copy_from_slice(value); n += value.len();
    }
    Some(n)
}

fn skip(v: &[u8], mut i: usize) -> usize {
    while i < v.len() && matches!(v[i], b' ' | b'\t' | b'\r' | b'\n') {
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

#[cfg(test)]
mod tests {
    use super::*;
    fn classify(status: u16, body: &str) -> Option<AccessSignal> {
        let mut host = String::from("{\"ok\":true,\"bodyText\":\"");
        for c in body.chars() {
            match c { '\\' => host.push_str("\\\\"), '"' => host.push_str("\\\""), '\n' => host.push_str("\\n"), _ => host.push(c) }
        }
        host.push_str("\"}");
        from_host_response(status, host.as_bytes(), &mut [0; 4096])
    }
    #[test]
    fn explicit_top_level_refusals_preserve_signal() {
        assert_eq!(classify(403, r#"{"error":"challenge_\u0072equired"}"#), Some(AccessSignal::Challenge));
        for status in [401, 403] {
            assert_eq!(classify(status, r#"{"error":"challenge_required"}"#), Some(AccessSignal::Challenge));
            assert_eq!(classify(status, r#" { "error" : "auth_required", "message": "login" } "#), Some(AccessSignal::Authentication));
        }
    }
    #[test]
    fn ordinary_and_pseudo_signals_stay_http() {
        for body in [r#"{"error":"permission_denied"}"#, r#"{"data":{"error":"challenge_required"}}"#,
            r#"{"message":"error challenge_required"}"#, r#"{"error":{"code":"challenge_required"}}"#,
            r#"{"error":"unknown"}"#, r#"{"error":null}"#, "<html>challenge_required</html>",
            r#"[ {"error":"challenge_required"} ]"#, r#"{"error":"Challenge_required"}"#,
            r#"{"error":"challenge_required suffix"}"#] {
            assert_eq!(classify(403, body), None, "{body}");
        }
    }
    #[test]
    fn malformed_and_ambiguous_bodies_stay_http() {
        for body in [r#"{"error":"challenge_required""#, r#"{"error":"challenge_required",}"#,
            r#"{"error":"challenge_required"} garbage"#, r#"{"error":"challenge_required","error":"auth_required"}"#,
            r#"{"error":"challenge_required","nested":{"a":1,"a":2}}"#,
            r#"{"\u0065rror":"challenge_required"}"#, r#"{"error":"challenge_required","n":01}"#,
            r#"{"error":"challenge_required","s":"\q"}"#,
            "{\u{000b}\"error\":\"challenge_required\"}"] {
            assert_eq!(classify(403, body), None, "{body}");
        }
    }
    #[test]
    fn unrelated_statuses_never_become_access_signal() {
        for status in [0, 200, 302, 400, 404, 429, 500, 503] {
            assert_eq!(classify(status, r#"{"error":"challenge_required"}"#), None);
            assert_eq!(classify(status, r#"{"error":"auth_required"}"#), None);
        }
    }
    #[test]
    fn invalid_host_envelope_or_truncation_cannot_supply_signal() {
        for host in [br#"{"ok":false,"bodyText":"{\"error\":\"challenge_required\"}"}"#.as_slice(),
            br#"{"ok":true,"bodyText":"{\"error\":\"challenge_required\"}","ok":false}"#,
            br#"{"ok":true,"nested":{"bodyText":"{\"error\":\"challenge_required\"}"}}"#,
            br#"{"ok":true,"bodyText":"{\"error\":\"challenge_required\"}"}x"#] {
            assert_eq!(from_host_response(403, host, &mut [0; 1024]), None);
        }
        assert_eq!(from_host_response(403, br#"{"ok":true,"bodyText":"{\"error\":\"challenge_required\"}"}"#, &mut [0; 8]), None);
    }
    #[test]
    fn bounded_nesting_and_unicode_outer_escape() {
        assert_eq!(from_host_response(403, br#"{"ok":true,"bodyText":"{\u0022error\u0022:\u0022challenge_required\u0022}"}"#, &mut [0; 512]), Some(AccessSignal::Challenge));
        let deep = format!("{{\"error\":\"challenge_required\",\"x\":{}0{}}}", "[".repeat(25), "]".repeat(25));
        assert_eq!(classify(403, &deep), None);
        let mut many = String::from("{\"error\":\"challenge_required\"");
        for n in 0..65 { many.push_str(&format!(",\"k{n}\":0")); }
        many.push('}');
        assert_eq!(classify(403, &many), None);
    }
}
