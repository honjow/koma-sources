//! Limited HTML data extraction; scripts/comments are skipped, never executed.
#[derive(Clone, Copy)]
pub struct Tag<'a> {
    pub name: &'a [u8],
    pub raw: &'a [u8],
    pub rest: &'a [u8],
}
fn ws(b: u8) -> bool {
    b.is_ascii_whitespace()
}
pub fn find(s: &[u8], n: &[u8]) -> Option<usize> {
    s.windows(n.len()).position(|x| x == n)
}
pub struct Tags<'a> {
    s: &'a [u8],
    p: usize,
}
impl<'a> Tags<'a> {
    pub fn new(s: &'a [u8]) -> Self {
        Self { s, p: 0 }
    }
}
impl<'a> Iterator for Tags<'a> {
    type Item = Tag<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let start = self.p + find(&self.s[self.p..], b"<")?;
            let mut p = start + 1;
            if self.s.get(p..p + 3) == Some(b"!--") {
                self.p = p + 3 + find(&self.s[p + 3..], b"-->")? + 3;
                continue;
            }
            let n = p;
            while self
                .s
                .get(p)
                .map(|x| x.is_ascii_alphanumeric())
                .unwrap_or(false)
            {
                p += 1;
            }
            if p == n {
                self.p = p;
                continue;
            }
            let name = &self.s[n..p];
            let mut quote = 0;
            while p < self.s.len() {
                let b = self.s[p];
                if quote != 0 {
                    if b == quote {
                        quote = 0;
                    }
                } else if b == b'\'' || b == b'"' {
                    quote = b;
                } else if b == b'>' {
                    break;
                }
                p += 1;
            }
            if p == self.s.len() {
                self.p = p;
                return None;
            }
            self.p = p + 1;
            if matches!(name, b"script" | b"style") {
                let end = if name == b"script" {
                    b"</script>".as_slice()
                } else {
                    b"</style>".as_slice()
                };
                self.p += find(&self.s[self.p..], end)? + end.len();
                continue;
            }
            return Some(Tag {
                name,
                raw: &self.s[start + 1..p],
                rest: &self.s[p + 1..],
            });
        }
    }
}
impl<'a> Tag<'a> {
    pub fn attr(self, key: &[u8]) -> Option<&'a [u8]> {
        let s = self.raw;
        let mut p = self.name.len();
        while p < s.len() {
            while p < s.len() && (ws(s[p]) || s[p] == b'/') {
                p += 1;
            }
            let a = p;
            while p < s.len() && !ws(s[p]) && !matches!(s[p], b'=' | b'/') {
                p += 1;
            }
            if a == p {
                break;
            }
            let k = &s[a..p];
            while p < s.len() && ws(s[p]) {
                p += 1;
            }
            let mut v = b"".as_slice();
            if s.get(p) == Some(&b'=') {
                p += 1;
                while p < s.len() && ws(s[p]) {
                    p += 1;
                }
                if let Some(q @ (b'\'' | b'"')) = s.get(p) {
                    let q = *q;
                    p += 1;
                    let a = p;
                    while p < s.len() && s[p] != q {
                        p += 1;
                    }
                    v = &s[a..p];
                    p += 1;
                } else {
                    let a = p;
                    while p < s.len() && !ws(s[p]) {
                        p += 1;
                    }
                    v = &s[a..p];
                }
            }
            if k == key {
                return Some(v);
            }
        }
        None
    }
    pub fn class(self, c: &[u8]) -> bool {
        self.attr(b"class")
            .map(|s| s.split(|x| ws(*x)).any(|x| x == c))
            .unwrap_or(false)
    }
    pub fn inner(self) -> Option<&'a [u8]> {
        let s = self.rest;
        let mut p = 0;
        let mut depth = 1;
        while p < s.len() {
            let begin = p + find(&s[p..], b"<")?;
            let mut q = begin + 1;
            let closing = s.get(q) == Some(&b'/');
            if closing {
                q += 1;
            }
            let start = q;
            while s.get(q).map(|b| b.is_ascii_alphanumeric()).unwrap_or(false) {
                q += 1;
            }
            let name = &s[start..q];
            let mut quote = 0;
            while q < s.len() {
                let b = s[q];
                if quote != 0 {
                    if b == quote {
                        quote = 0;
                    }
                } else if b == b'\'' || b == b'"' {
                    quote = b;
                } else if b == b'>' {
                    break;
                }
                q += 1;
            }
            if q == s.len() {
                return None;
            }
            if name == self.name {
                if closing {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[..begin]);
                    }
                } else if s.get(q.wrapping_sub(1)) != Some(&b'/') {
                    depth += 1;
                }
            }
            p = q + 1;
        }
        None
    }
}
pub fn id<'a>(s: &'a [u8], id: &[u8]) -> Option<Tag<'a>> {
    Tags::new(s).find(|t| t.attr(b"id") == Some(id))
}
pub fn decoded(s: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut p = 0;
    let mut n = 0;
    while p < s.len() {
        if s[p] == b'<' {
            p += find(&s[p..], b">")? + 1;
            continue;
        }
        if s[p] != b'&' {
            if n == out.len() {
                return None;
            }
            out[n] = s[p];
            n += 1;
            p += 1;
            continue;
        }
        // Titles can contain literal ampersands, and HTML numeric references
        // do not need a trailing semicolon (WHATWG decimal/hex reference states).
        // Keep unknown named entities and invalid scalars as explicit failures.
        let (v, end) = if s.get(p + 1) == Some(&b'#') {
            let mut end = p + 2;
            let radix = if matches!(s.get(end), Some(b'x' | b'X')) {
                end += 1;
                16
            } else {
                10
            };
            let start = end;
            while end < s.len() && if radix == 16 { s[end].is_ascii_hexdigit() } else { s[end].is_ascii_digit() } {
                end += 1;
            }
            if end == start {
                return None;
            }
            let value = u32::from_str_radix(core::str::from_utf8(&s[start..end]).ok()?, radix).ok()?;
            let value = char::from_u32(value)?;
            if s.get(end) == Some(&b';') {
                end += 1;
            }
            (value, end)
        } else {
            let mut end = p + 1;
            while end < s.len() && s[end].is_ascii_alphanumeric() {
                end += 1;
            }
            if s.get(end) != Some(&b';') {
                if n == out.len() {
                    return None;
                }
                out[n] = b'&';
                n += 1;
                p += 1;
                continue;
            }
            let value = match &s[p + 1..end] {
                b"amp" => '&',
                b"lt" => '<',
                b"gt" => '>',
                b"quot" => '"',
                b"apos" => '\'',
                b"nbsp" => ' ',
                _ => return None,
            };
            (value, end + 1)
        };
        let mut buf = [0; 4];
        let b = v.encode_utf8(&mut buf).as_bytes();
        if n + b.len() > out.len() {
            return None;
        }
        out[n..n + b.len()].copy_from_slice(b);
        n += b.len();
        p = end;
    }
    Some(n)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_only() {
        let s=br#"<!-- <input name='fake'> --><script>evil();</script><a href='javascript:__doPostBack(&#39;x&#39;,&#39;&#39;)' class='a b'>title&amp;</a>"#;
        let ts: std::vec::Vec<_> = Tags::new(s).collect();
        assert_eq!(ts.len(), 1);
        assert!(ts[0].class(b"b"));
        let mut out = [0; 100];
        let n = decoded(ts[0].attr(b"href").unwrap(), &mut out).unwrap();
        assert_eq!(&out[..n], b"javascript:__doPostBack('x','')");
    }
    #[test]
    fn literal_ampersands_and_numeric_references() {
        let input = "秦&唐 / AT&T &#40a&#41 &#x28文&#X29; &amp;";
        let mut out = [0; 128];
        let n = decoded(input.as_bytes(), &mut out).unwrap();
        assert_eq!(core::str::from_utf8(&out[..n]).unwrap(), "秦&唐 / AT&T (a) (文) &");
        assert!(decoded(b"&#x;", &mut out).is_none());
        assert!(decoded(b"&#1114112", &mut out).is_none());
        assert!(decoded("秦&唐".as_bytes(), &mut [0; 4]).is_none());
    }
    #[test]
    fn reject_unknown_entities() {
        assert!(decoded(b"&unknown;", &mut [0; 20]).is_none());
    }
}
