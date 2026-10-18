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
    while p < s.len() && matches!(s[p], b' ' | b'\t' | b'\r' | b'\n') {
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
                        let cp = hex4(s.get(i + 1..i + 5)?)?;
                        i += 4;
                        if (0xd800..=0xdbff).contains(&cp) {
                            if s.get(i + 1..i + 3) != Some(b"\\u") {
                                return None;
                            }
                            let low = hex4(s.get(i + 3..i + 7)?)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return None;
                            }
                            i += 6;
                        } else if (0xdc00..=0xdfff).contains(&cp) {
                            return None;
                        }
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

fn valid_json(s: &[u8]) -> Result<(), E> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    if json_value_end(s, 0, 0).map(|i| ws(s, i)) != Some(s.len()) {
        return Err("invalid complete JSON");
    }
    Ok(())
}
fn member<'a>(obj: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, E> {
    if obj.first() != Some(&b'{') {
        return Err("expected object");
    }
    let mut i = ws(obj, 1);
    let mut found = None;
    while obj.get(i) != Some(&b'}') {
        let end = json_string_end(obj, i).ok_or("invalid object key")?;
        let matches = &obj[i + 1..end - 1] == key;
        i = ws(obj, end);
        if obj.get(i) != Some(&b':') {
            return Err("invalid object member");
        }
        i = ws(obj, i + 1);
        let end = json_value_end(obj, i, 0).ok_or("invalid object value")?;
        if matches {
            if found.is_some() {
                return Err("duplicate object member");
            }
            found = Some(&obj[i..end]);
        }
        i = ws(obj, end);
        if obj.get(i) == Some(&b',') {
            i = ws(obj, i + 1)
        } else if obj.get(i) != Some(&b'}') {
            return Err("invalid object separator");
        }
    }
    Ok(found)
}
fn required<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    member(obj, k)?.ok_or("required field missing")
}
fn quoted(v: &[u8]) -> Result<&[u8], E> {
    if v.first() != Some(&b'"') || json_string_end(v, 0) != Some(v.len()) {
        return Err("expected string");
    }
    Ok(&v[1..v.len() - 1])
}
fn text<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    quoted(required(obj, k)?)
}
#[allow(dead_code)]
fn raw_string<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    let v = required(obj, k)?;
    quoted(v)?;
    Ok(v)
}
struct Array<'a> {
    s: &'a [u8],
    p: usize,
}
impl<'a> Array<'a> {
    fn new(s: &'a [u8]) -> Result<Self, E> {
        if s.first() != Some(&b'[') || s.last() != Some(&b']') {
            return Err("expected array");
        }
        Ok(Self { s, p: ws(s, 1) })
    }
    fn next(&mut self) -> Result<Option<&'a [u8]>, E> {
        if self.s.get(self.p) == Some(&b']') {
            return Ok(None);
        }
        let end = json_value_end(self.s, self.p, 0).ok_or("invalid array entry")?;
        let v = &self.s[self.p..end];
        self.p = ws(self.s, end);
        if self.s.get(self.p) == Some(&b',') {
            self.p = ws(self.s, self.p + 1)
        } else if self.s.get(self.p) != Some(&b']') {
            return Err("invalid array separator");
        }
        Ok(Some(v))
    }
}
fn args(r: &[u8]) -> Result<&[u8], E> {
    valid_json(r)?;
    Ok(member(r, b"args")?.unwrap_or(r))
}
fn scalar(v: &[u8]) -> Result<&[u8], E> {
    if v.first() == Some(&b'"') {
        quoted(v)
    } else {
        Ok(v)
    }
}
fn num(v: &[u8]) -> Result<usize, E> {
    number(scalar(v)?).ok_or("invalid integer")
}
fn field_num(v: &[u8], k: &[u8]) -> Result<usize, E> {
    num(required(v, k)?)
}
fn wn(b: &mut [u8], n: &mut usize, x: usize) -> Result<(), E> {
    if write_usize(b, n, x) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
