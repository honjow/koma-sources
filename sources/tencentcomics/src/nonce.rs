//! Bounded constant arithmetic from public nonce text; never executes JavaScript.
//! Unsupported symbols/calls, noninteger final values and excessive work fail closed.
fn bounded(n: f64) -> Option<f64> {
    if n.is_finite() && (-1_000_000.0..=1_000_000.0).contains(&n) {
        Some(n)
    } else {
        None
    }
}
struct Constant<'a> {
    b: &'a [u8],
    p: usize,
    steps: usize,
}
impl<'a> Constant<'a> {
    fn ws(&mut self) {
        while self.b.get(self.p).map_or(false, u8::is_ascii_whitespace) {
            self.p += 1
        }
    }
    fn take(&mut self, s: &[u8]) -> bool {
        self.ws();
        if self.b[self.p..].starts_with(s) {
            self.p += s.len();
            true
        } else {
            false
        }
    }
    fn tick(&mut self, d: usize) -> Option<()> {
        self.steps += 1;
        if d > 32 || self.steps > 256 {
            None
        } else {
            Some(())
        }
    }
    fn conditional(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        let c = self.compare(d + 1)?;
        if self.take(b"?") {
            let t = self.conditional(d + 1)?;
            if !self.take(b":") {
                return None;
            }
            let f = self.conditional(d + 1)?;
            Some(if c != 0.0 { t } else { f })
        } else {
            Some(c)
        }
    }
    fn compare(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        let mut v = self.add(d + 1)?;
        loop {
            let op = if self.take(b"<=") {
                0
            } else if self.take(b">=") {
                1
            } else if self.take(b"<") {
                2
            } else if self.take(b">") {
                3
            } else if self.take(b"===") {
                4
            } else if self.take(b"==") {
                4
            } else if self.take(b"!==") {
                5
            } else if self.take(b"!=") {
                5
            } else {
                break;
            };
            let r = self.add(d + 1)?;
            v = match op {
                0 => (v <= r) as u8 as f64,
                1 => (v >= r) as u8 as f64,
                2 => (v < r) as u8 as f64,
                3 => (v > r) as u8 as f64,
                4 => (v == r) as u8 as f64,
                _ => (v != r) as u8 as f64,
            };
        }
        Some(v)
    }
    fn add(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        let mut v = self.mul(d + 1)?;
        loop {
            if self.take(b"+") {
                v = bounded(v + self.mul(d + 1)?)?
            } else if self.take(b"-") {
                v = bounded(v - self.mul(d + 1)?)?
            } else {
                break;
            }
        }
        Some(v)
    }
    fn mul(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        let mut v = self.unary(d + 1)?;
        loop {
            if self.take(b"*") {
                v = bounded(v * self.unary(d + 1)?)?
            } else if self.take(b"/") {
                let r = self.unary(d + 1)?;
                if r == 0.0 {
                    return None;
                }
                v = bounded(v / r)?
            } else if self.take(b"%") {
                let r = self.unary(d + 1)?;
                if r == 0.0 {
                    return None;
                }
                let q = bounded(v / r)? as i64;
                v = bounded(v - q as f64 * r)?
            } else {
                break;
            }
        }
        Some(v)
    }
    fn unary(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        if self.take(b"!") {
            Some((self.unary(d + 1)? == 0.0) as u8 as f64)
        } else if self.take(b"~") {
            Some(!(self.unary(d + 1)? as i32) as f64)
        } else if self.take(b"+") {
            self.unary(d + 1)
        } else if self.take(b"-") {
            bounded(-self.unary(d + 1)?)
        } else {
            self.atom(d + 1)
        }
    }
    fn atom(&mut self, d: usize) -> Option<f64> {
        self.tick(d)?;
        if self.take(b"(") {
            let v = self.conditional(d + 1)?;
            return if self.take(b")") { Some(v) } else { None };
        }
        if self.take(b"Math.pow(") {
            let b = self.conditional(d + 1)?;
            if !self.take(b",") {
                return None;
            }
            let e = self.conditional(d + 1)?;
            if !(0.0..=8.0).contains(&e) || e != (e as i32) as f64 || !self.take(b")") {
                return None;
            }
            let mut v = 1.0;
            for _ in 0..e as u32 {
                v = bounded(v * b)?
            }
            return Some(v);
        }
        if self.take(b"Math.round(") {
            let v = self.conditional(d + 1)?;
            if !self.take(b")") {
                return None;
            }
            let t = v as i64;
            let f = v - t as f64;
            return bounded(
                (if f >= 0.5 {
                    t + 1
                } else if f < -0.5 {
                    t - 1
                } else {
                    t
                }) as f64,
            );
        }
        self.ws();
        if matches!(self.b.get(self.p), Some(b'\'' | b'"')) {
            let quote = self.b[self.p];
            self.p += 1;
            let first = *self.b.get(self.p)?;
            let start = self.p;
            while self.b.get(self.p).map_or(false, |&b| {
                b != quote && b != b'\\' && (32..127).contains(&b)
            }) {
                self.p += 1;
            }
            if self.p == start || self.p - start > 64 || self.b.get(self.p) != Some(&quote) {
                return None;
            }
            self.p += 1;
            return if self.take(b".charCodeAt()") {
                Some(first as f64)
            } else {
                None
            };
        }
        let a = self.p;
        let mut v = 0u32;
        while self.b.get(self.p).map_or(false, u8::is_ascii_digit) {
            v = v
                .checked_mul(10)?
                .checked_add((self.b[self.p] - b'0') as u32)?;
            self.p += 1
        }
        let before = self.p - a;
        if before > 7 || (before > 1 && self.b[a] == b'0') {
            return None;
        }
        let mut n = v as f64;
        let mut after = 0;
        if self.b.get(self.p) == Some(&b'.') {
            self.p += 1;
            let mut f = 0u32;
            while self.b.get(self.p).map_or(false, u8::is_ascii_digit) {
                if after == 3 {
                    return None;
                }
                f = f * 10 + (self.b[self.p] - b'0') as u32;
                self.p += 1;
                after += 1
            }
            let scale = match after {
                0 => 1.0,
                1 => 10.0,
                2 => 100.0,
                _ => 1000.0,
            };
            n += f as f64 / scale;
        }
        if before + after == 0 {
            return None;
        }
        bounded(n)
    }
}
// eval's entire result may be a decimal string; unary + outside eval converts it.
// Keep this whole-expression rule separate from numeric operators (JS string + differs).
fn literal_substring_number(b: &[u8]) -> Option<i64> {
    let mut p = Nonce { b, p: 0 };
    let literal = p.quoted()?;
    if literal.len() > 64 || !literal.iter().all(u8::is_ascii_digit) || !p.take(b".substring(") {
        return None;
    }
    p.ws();
    let start = p.p;
    while p.b.get(p.p).map_or(false, u8::is_ascii_digit) {
        p.p += 1;
    }
    let index = &p.b[start..p.p];
    if index.is_empty() || index.len() > 2 || !p.take(b")") {
        return None;
    }
    p.ws();
    if p.p != b.len() {
        return None;
    }
    let index = index
        .iter()
        .fold(0usize, |n, d| n * 10 + (d - b'0') as usize)
        .min(literal.len());
    let result = &literal[index..];
    if result.len() > 7 {
        return None;
    }
    let n = result.iter().fold(0i64, |n, d| n * 10 + (d - b'0') as i64);
    if n <= 1_000_000 {
        Some(n)
    } else {
        None
    }
}
pub fn constant(b: &[u8]) -> Option<i64> {
    if b.is_empty() || b.len() > 256 || b.windows(2).any(|v| v == b"++" || v == b"--") {
        return None;
    }
    if let Some(n) = literal_substring_number(b) {
        return Some(n);
    }
    let mut p = Constant { b, p: 0, steps: 0 };
    let v = p.conditional(0)?;
    p.ws();
    let n = v as i64;
    if p.p == b.len() && n as f64 == v {
        Some(n)
    } else {
        None
    }
}

struct Nonce<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Nonce<'a> {
    fn ws(&mut self) {
        while self.b.get(self.p).map_or(false, u8::is_ascii_whitespace) {
            self.p += 1
        }
    }
    fn take(&mut self, s: &[u8]) -> bool {
        self.ws();
        if self.b[self.p..].starts_with(s) {
            self.p += s.len();
            true
        } else {
            false
        }
    }
    fn quoted(&mut self) -> Option<&'a [u8]> {
        self.ws();
        let q = *self.b.get(self.p)?;
        if q != b'"' && q != b'\'' {
            return None;
        }
        self.p += 1;
        let a = self.p;
        while self
            .b
            .get(self.p)
            .map_or(false, |&b| b != q && b != b'\\' && b >= 32 && b < 127)
        {
            self.p += 1
        }
        if self.b.get(self.p) != Some(&q) {
            return None;
        }
        let v = &self.b[a..self.p];
        self.p += 1;
        Some(v)
    }
}
fn append(out: &mut [u8], n: &mut usize, b: &[u8]) -> Option<()> {
    let e = n.checked_add(b.len())?;
    out.get_mut(*n..e)?.copy_from_slice(b);
    *n = e;
    Some(())
}
fn number(out: &mut [u8], n: &mut usize, v: i64) -> Option<()> {
    if v < 0 {
        return None;
    }
    let mut digits = [0u8; 7];
    let mut p = digits.len();
    let mut v = v as u64;
    loop {
        p -= 1;
        digits[p] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    append(out, n, &digits[p..])
}
pub fn nonce(html: &[u8], out: &mut [u8]) -> Option<usize> {
    let a = html.windows(7).rposition(|x| x == b"window[")? + 7;
    let b = html[a..].windows(9).position(|x| x == b"</script>")? + a;
    let expr = html.get(a..b)?;
    if expr.len() > 2048 {
        return None;
    }
    let mut p = Nonce { b: expr, p: 0 };
    let mut key = [0u8; 16];
    let mut n = 0;
    loop {
        append(&mut key, &mut n, p.quoted()?)?;
        if !p.take(b"+") {
            break;
        }
    }
    if &key[..n] != b"nonce" || !p.take(b"]") || !p.take(b"=") {
        return None;
    }
    let mut n = 0;
    let mut atoms = 0;
    loop {
        atoms += 1;
        if atoms > 64 {
            return None;
        }
        p.ws();
        if matches!(p.b.get(p.p), Some(b'"' | b'\'')) {
            let v = p.quoted()?;
            if !v.iter().all(u8::is_ascii_alphanumeric) {
                return None;
            }
            append(out, &mut n, v)?
        } else {
            if !p.take(b"(+eval(") {
                return None;
            }
            let v = constant(p.quoted()?)?;
            if !p.take(b")).toString()") {
                return None;
            }
            number(out, &mut n, v)?
        }
        if !p.take(b"+") {
            break;
        }
    }
    let _ = p.take(b";");
    p.ws();
    if p.p != p.b.len() || n == 0 || n > 256 {
        return None;
    }
    Some(n)
}
fn b64(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
pub fn chapter_data(html: &[u8], encoded: &mut [u8], decoded: &mut [u8]) -> Option<usize> {
    let mut salt = [0u8; 256];
    let n = nonce(html, &mut salt)?;
    let salt = &salt[..n];
    let a = html.windows(10).rposition(|x| x == b"var DATA =")? + 10;
    let mut p = a;
    while html.get(p).map_or(false, u8::is_ascii_whitespace) {
        p += 1
    }
    if html.get(p) != Some(&b'\'') {
        return None;
    }
    p += 1;
    let end = p + html[p..].iter().position(|&b| b == b'\'')?;
    let raw = html.get(p..end)?;
    if raw.len() > encoded.len() || raw.is_empty() {
        return None;
    }
    encoded[..raw.len()].copy_from_slice(raw);
    let mut len = raw.len();
    let mut tokens = [(0usize, 0usize); 64];
    let (mut t, mut p) = (0, 0);
    while p < salt.len() {
        if !salt[p].is_ascii_digit() {
            p += 1;
            continue;
        }
        let mut offset = 0usize;
        while p < salt.len() && salt[p].is_ascii_digit() {
            offset = (offset * 10 + (salt[p] - b'0') as usize) & 255;
            p += 1
        }
        let start = p;
        while p < salt.len() && salt[p].is_ascii_alphabetic() {
            p += 1
        }
        if p > start {
            if t == 64 {
                return None;
            }
            tokens[t] = (offset, p - start);
            t += 1
        }
    }
    if t == 0 {
        return None;
    }
    for &(offset, noise) in tokens[..t].iter().rev() {
        if offset.checked_add(noise)? > len {
            return None;
        }
        encoded.copy_within(offset + noise..len, offset);
        len -= noise
    }
    if len % 4 != 0 {
        return None;
    }
    let mut n = 0;
    for (i, c) in encoded[..len].chunks_exact(4).enumerate() {
        let a = b64(c[0])?;
        let b = b64(c[1])?;
        let last = i + 1 == len / 4;
        if c[2] == b'=' {
            if !last || c[3] != b'=' || b & 15 != 0 {
                return None;
            }
            append(decoded, &mut n, &[(a << 2) | (b >> 4)])?
        } else {
            let d = b64(c[2])?;
            if c[3] == b'=' {
                if !last || d & 3 != 0 {
                    return None;
                }
                append(decoded, &mut n, &[(a << 2) | (b >> 4), (b << 4) | (d >> 2)])?
            } else {
                let e = b64(c[3])?;
                append(
                    decoded,
                    &mut n,
                    &[(a << 2) | (b >> 4), (b << 4) | (d >> 2), (d << 6) | e],
                )?
            }
        }
    }
    Some(n)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_constants() {
        for (s, n) in [
            ("!!1*5", 5),
            ("Math.pow(1,3)+1+1", 3),
            ("3", 3),
            ("!!1", 1),
            ("Math.round(.5) + ~~1.9", 2),
            ("1/2 + 5/2", 3),
            ("4 * 1 / 4", 1),
            ("\'123\'.substring(2)", 3),
            ("\'1\'.charCodeAt() - 45", 4),
            ("0<=3?2:1", 2),
        ] {
            assert_eq!(constant(s.as_bytes()), Some(n));
        }
    }
    #[test]
    fn fail_closed() {
        for s in [
            "document.children",
            "\'123\'.substring(2)+1",
            "!!document.children",
            "fetch(1)",
            "Math.random()",
            "Math.pow(2,99)",
            "1/2",
            "1/0",
            "1000000*1000000",
            "1; alert(1)",
            "(1",
            "012",
        ] {
            assert_eq!(constant(s.as_bytes()), None, "{}", s);
        }
        assert_eq!(constant(&[b'!'; 257]), None);
    }
    #[test]
    fn known_page_nonce() {
        let h=br#"window["n"+"once"] = "c8" + (+eval("Math.pow(1,3)+1+1")).toString() + "3c23c1466b7ecb368" + (+eval("3")).toString() + (+eval("!!1")).toString() + "c9b766f9b" + (+eval("0<=3?2:1")).toString() + "";</script>"#;
        let mut b = [0u8; 256];
        let n = nonce(h, &mut b).unwrap();
        assert_eq!(&b[..n], b"c833c23c1466b7ecb36831c9b766f9b2");
    }
}
