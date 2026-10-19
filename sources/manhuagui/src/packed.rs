// Static literal/dictionary parser, never executes JavaScript.
// Algorithm reference: Keiyoushi Unpacker/SubstringExtractor, Apache-2.0.
use crate::lz;
const CAP: usize = 512 * 1024;
static mut PAYLOAD: [u8; CAP] = [0; CAP];
static mut ENCODED: [u8; CAP] = [0; CAP];
static mut DICT: [u8; CAP] = [0; CAP];
static mut UTF16: [u16; CAP / 2] = [0; CAP / 2];
static mut ENTRIES: [(u32, u32); 32768] = [(0, 0); 32768];
fn ws(s: &[u8], mut p: usize) -> usize {
    while s.get(p).is_some_and(|v| v.is_ascii_whitespace()) {
        p += 1
    }
    p
}
fn need(s: &[u8], p: &mut usize, v: &[u8]) -> Result<(), &'static str> {
    *p = ws(s, *p);
    if !s[*p..].starts_with(v) {
        return Err("unsupported packed grammar");
    };
    *p += v.len();
    Ok(())
}
fn hex(v: &[u8]) -> Result<u32, &'static str> {
    let mut n = 0;
    for c in v {
        n = (n << 4)
            + match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => return Err("invalid packed escape"),
            }
    }
    Ok(n)
}
fn quoted(
    s: &[u8],
    p: &mut usize,
    out: &mut [u8],
    apostrophe_hyphen: bool,
) -> Result<usize, &'static str> {
    *p = ws(s, *p);
    let quote = *s.get(*p).ok_or("packed string missing")?;
    if quote != b'\'' && quote != b'"' {
        return Err("packed value not literal");
    };
    *p += 1;
    let mut n = 0;
    loop {
        let c = *s.get(*p).ok_or("unterminated packed string")?;
        *p += 1;
        if c == quote {
            core::str::from_utf8(&out[..n]).map_err(|_| "invalid packed UTF8")?;
            return Ok(n);
        }
        let mut tmp = [0; 4];
        let slice: &[u8];
        if c == b'\\' {
            let e = *s.get(*p).ok_or("truncated packed escape")?;
            *p += 1;
            match e {
                b'\'' | b'"' | b'\\' | b'/' => {
                    tmp[0] = if e == b'\'' && apostrophe_hyphen {
                        b'-'
                    } else {
                        e
                    };
                    slice = &tmp[..1]
                }
                b'n' | b'r' | b't' => {
                    tmp[0] = match e {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        _ => b'\t',
                    };
                    slice = &tmp[..1]
                }
                b'x' => {
                    tmp[0] = hex(s.get(*p..*p + 2).ok_or("truncated packed hex")?)? as u8;
                    *p += 2;
                    slice = &tmp[..1]
                }
                b'u' => {
                    let mut v = hex(s.get(*p..*p + 4).ok_or("truncated packed Unicode")?)?;
                    *p += 4;
                    if (0xd800..=0xdbff).contains(&v) {
                        if s.get(*p..*p + 2) != Some(b"\\u") {
                            return Err("invalid packed surrogate");
                        };
                        *p += 2;
                        let lo = hex(s.get(*p..*p + 4).ok_or("truncated packed surrogate")?)?;
                        *p += 4;
                        if !(0xdc00..=0xdfff).contains(&lo) {
                            return Err("invalid packed surrogate");
                        };
                        v = 0x10000 + ((v - 0xd800) << 10) + (lo - 0xdc00)
                    }
                    let ch = char::from_u32(v).ok_or("invalid packed Unicode")?;
                    slice = ch.encode_utf8(&mut tmp).as_bytes();
                }
                _ => return Err("unsupported packed escape"),
            }
        } else {
            if c < 32 {
                return Err("control byte in packed string");
            };
            tmp[0] = c;
            slice = &tmp[..1]
        }
        if n + slice.len() > out.len() {
            return Err("packed string overflow");
        };
        out[n..n + slice.len()].copy_from_slice(slice);
        n += slice.len();
    }
}
fn skip_quote(s: &[u8], p: &mut usize) -> Result<(), &'static str> {
    let q = s[*p];
    *p += 1;
    loop {
        let c = *s.get(*p).ok_or("unterminated function literal")?;
        *p += 1;
        if c == q {
            return Ok(());
        };
        if c == b'\\' {
            if s.get(*p).is_none() {
                return Err("truncated function literal");
            };
            *p += 1
        }
    }
}
fn digits(s: &[u8], p: &mut usize) -> Result<usize, &'static str> {
    *p = ws(s, *p);
    let start = *p;
    let mut n = 0usize;
    while s.get(*p).is_some_and(u8::is_ascii_digit) {
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((s[*p] - b'0') as usize))
            .ok_or("packed integer overflow")?;
        *p += 1
    }
    if *p == start {
        return Err("packed number missing");
    };
    Ok(n)
}
fn parse(s: &[u8], start: usize) -> Result<(usize, usize, usize), &'static str> {
    let mut p = start;
    need(s, &mut p, b"window[")?;
    let mut method = [0; 256];
    quoted(s, &mut p, &mut method, false)?;
    need(s, &mut p, b"](")?;
    need(s, &mut p, b"function")?;
    need(s, &mut p, b"(")?;
    // Parameter names are inert; only the fixed six-argument packer form is accepted.
    for i in 0..6 {
        p = ws(s, p);
        let st = p;
        while s.get(p).is_some_and(|v| v.is_ascii_alphabetic()) {
            p += 1
        }
        if p == st {
            return Err("unknown packer parameter");
        };
        if i < 5 {
            need(s, &mut p, b",")?
        }
    }
    need(s, &mut p, b")")?;
    need(s, &mut p, b"{")?;
    let mut depth = 1usize;
    while depth > 0 {
        let c = *s.get(p).ok_or("unterminated packer function")?;
        if c == b'\'' || c == b'"' {
            skip_quote(s, &mut p)?;
            continue;
        };
        p += 1;
        match c {
            b'{' => {
                depth += 1;
                if depth > 32 {
                    return Err("packer nesting overflow");
                }
            }
            b'}' => depth -= 1,
            _ => {}
        }
    }
    need(s, &mut p, b"(")?;
    let payload = unsafe { &mut *core::ptr::addr_of_mut!(PAYLOAD) };
    let plen = quoted(s, &mut p, payload, true)?;
    need(s, &mut p, b",")?;
    if digits(s, &mut p)? != 62 {
        return Err("unsupported packer radix");
    };
    need(s, &mut p, b",")?;
    let declared = digits(s, &mut p)?;
    if declared == 0 || declared > 32768 {
        return Err("packed dictionary overflow");
    };
    need(s, &mut p, b",")?;
    let encoded = unsafe { &mut *core::ptr::addr_of_mut!(ENCODED) };
    let en = quoted(s, &mut p, encoded, true)?;
    p = ws(s, p);
    let dictionary = unsafe { &mut *core::ptr::addr_of_mut!(DICT) };
    let dn = if s[p..].starts_with(b".split") {
        need(s, &mut p, b".split(")?;
        let mut arg = [0; 16];
        let n = quoted(s, &mut p, &mut arg, false)?;
        if &arg[..n] != b"|" {
            return Err("unsupported dictionary separator");
        };
        need(s, &mut p, b")")?;
        dictionary[..en].copy_from_slice(&encoded[..en]);
        en
    } else {
        need(s, &mut p, b"[")?;
        let mut key = [0; 128];
        let n = quoted(s, &mut p, &mut key, false)?;
        if n == 0 {
            return Err("missing compressed dictionary literal method");
        };
        need(s, &mut p, b"](")?;
        let mut arg = [0; 16];
        let n = quoted(s, &mut p, &mut arg, false)?;
        if &arg[..n] != b"|" {
            return Err("unsupported dictionary separator");
        };
        need(s, &mut p, b")")?;
        let utf16 = unsafe { &mut *core::ptr::addr_of_mut!(UTF16) };
        let count = lz::decompress(&encoded[..en], utf16)?;
        lz::utf8(&utf16[..count], dictionary)?.len()
    };
    need(s, &mut p, b",0,{}))")?;
    let entries = unsafe { &mut *core::ptr::addr_of_mut!(ENTRIES) };
    let mut num = 0;
    let mut st = 0;
    for i in 0..=dn {
        if i == dn || dictionary[i] == b'|' {
            if num >= entries.len() {
                return Err("packed dictionary overflow");
            };
            entries[num] = (st as u32, i as u32);
            num += 1;
            st = i + 1
        }
    }
    if num != declared {
        return Err("packed dictionary count mismatch");
    };
    Ok((plen, num, p))
}
fn index(word: &[u8]) -> Option<usize> {
    let mut n = 0usize;
    for &c in word {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'z' => c - b'a' + 10,
            b'A'..=b'Z' => c - b'A' + 36,
            _ => return None,
        };
        n = n.checked_mul(62)?.checked_add(d as usize)?
    }
    Some(n)
}
pub fn unpack(html: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    if html.len() > 2 * 1024 * 1024 {
        return Err("packed HTML overflow");
    };
    let mut offset = 0;
    let mut found = None;
    while let Some(j) = html[offset..].windows(7).position(|v| v == b"window[") {
        let st = offset + j;
        offset = st + 7;
        // Only function-wrapped calls are candidates. Other window properties remain inert.
        let mut p = offset;
        let mut method = [0; 256];
        if quoted(html, &mut p, &mut method, false).is_err() {
            continue;
        };
        if need(html, &mut p, b"](").is_err() {
            continue;
        };
        p = ws(html, p);
        if !html[p..].starts_with(b"function") {
            continue;
        }
        if found.is_some() {
            return Err("ambiguous packed wrapper");
        };
        found = Some(st);
    }
    let st = found.ok_or("packed wrapper missing")?;
    let (plen, count, _) = parse(html, st)?;
    let payload = unsafe { &*core::ptr::addr_of!(PAYLOAD) };
    let dictionary = unsafe { &*core::ptr::addr_of!(DICT) };
    let entries = unsafe { &*core::ptr::addr_of!(ENTRIES) };
    let mut p = 0;
    let mut n = 0;
    while p < plen {
        let st = p;
        let bytes = if payload[p].is_ascii_alphanumeric() || payload[p] == b'_' {
            while p < plen && (payload[p].is_ascii_alphanumeric() || payload[p] == b'_') {
                p += 1
            }
            let word = &payload[st..p];
            if let Some(i) = index(word).filter(|i| *i < count) {
                let (a, b) = entries[i];
                if a != b {
                    &dictionary[a as usize..b as usize]
                } else {
                    word
                }
            } else {
                word
            }
        } else {
            p += 1;
            &payload[st..p]
        };
        if n + bytes.len() > out.len() {
            return Err("unpacked output overflow");
        };
        out[n..n + bytes.len()].copy_from_slice(bytes);
        n += bytes.len();
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid unpacked UTF8")?;
    Ok(n)
}
