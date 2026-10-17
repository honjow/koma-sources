//! Bounded data-only decoder for Nuxt's parameterized literal serialization.
//! This is not JavaScript execution: calls, operators, functions and assignments
//! inside the data are rejected. Unknown serialization forms fail closed.
#[derive(Debug, PartialEq)]
pub enum Error {
    Shape,
    ReturnShape,
    ArgumentsShape,
    ClosingShape,
    Limit,
    Syntax,
}
#[derive(Clone, Copy)]
struct Binding<'a> {
    name: &'a [u8],
    value: &'a [u8],
}
const EMPTY: Binding<'static> = Binding {
    name: b"",
    value: b"",
};
const MAX_BINDINGS: usize = 4096;
const MAX_DEPTH: usize = 32;
#[derive(Clone, Copy)]
struct Record<'a> {
    target: &'a [u8],
    key: &'a [u8],
    array: bool,
    value: &'a [u8],
}
const EMPTY_RECORD: Record<'static> = Record {
    target: b"",
    key: b"",
    array: false,
    value: b"",
};
const MAX_RECORDS: usize = 256;
// Candidate WASM linear stack is 256KiB: tables are module storage, never recursive stack arrays.
static mut BINDINGS: [Binding<'static>; MAX_BINDINGS] = [EMPTY; MAX_BINDINGS];
static mut RECORDS: [Record<'static>; MAX_RECORDS] = [EMPTY_RECORD; MAX_RECORDS];

fn json_number(value: &[u8]) -> bool {
    let mut p = 0;
    if value.get(p) == Some(&b'-') {
        p += 1;
    }
    match value.get(p) {
        Some(b'0') => p += 1,
        Some(b'1'..=b'9') => {
            p += 1;
            while value.get(p).map(u8::is_ascii_digit).unwrap_or(false) {
                p += 1;
            }
        }
        _ => return false,
    }
    if value.get(p) == Some(&b'.') {
        p += 1;
        let start = p;
        while value.get(p).map(u8::is_ascii_digit).unwrap_or(false) {
            p += 1;
        }
        if p == start {
            return false;
        }
    }
    if matches!(value.get(p), Some(b'e' | b'E')) {
        p += 1;
        if matches!(value.get(p), Some(b'+' | b'-')) {
            p += 1;
        }
        let start = p;
        while value.get(p).map(u8::is_ascii_digit).unwrap_or(false) {
            p += 1;
        }
        if p == start {
            return false;
        }
    }
    p == value.len()
}
fn ws(s: &[u8], p: &mut usize) {
    while *p < s.len() && s[*p].is_ascii_whitespace() {
        *p += 1;
    }
}
fn ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}
fn string_end(s: &[u8], p: &mut usize) -> Result<(), Error> {
    if s.get(*p) != Some(&b'"') {
        return Err(Error::Syntax);
    }
    *p += 1;
    while *p < s.len() {
        match s[*p] {
            b'"' => {
                *p += 1;
                return Ok(());
            }
            b'\\' => {
                *p += 1;
                match s.get(*p) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => *p += 1,
                    Some(b'u') => {
                        if *p + 4 >= s.len() || !s[*p + 1..*p + 5].iter().all(u8::is_ascii_hexdigit)
                        {
                            return Err(Error::Syntax);
                        }
                        *p += 5;
                    }
                    _ => return Err(Error::Syntax),
                }
            }
            0..=31 => return Err(Error::Syntax),
            _ => *p += 1,
        }
    }
    Err(Error::Syntax)
}
fn value_end(s: &[u8], p: &mut usize, depth: usize) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::Limit);
    }
    ws(s, p);
    if s.get(*p..*p + 6) == Some(b"void 0") {
        *p += 6;
        return Ok(());
    }
    if s.get(*p..*p + 6) == Some(b"Array(") {
        *p += 6;
        let start = *p;
        while *p < s.len() && s[*p].is_ascii_digit() {
            *p += 1;
        }
        let n = core::str::from_utf8(&s[start..*p])
            .ok()
            .and_then(|x| x.parse::<usize>().ok())
            .ok_or(Error::Syntax)?;
        if n > MAX_RECORDS || s.get(*p) != Some(&b')') {
            return Err(Error::Limit);
        }
        *p += 1;
        return Ok(());
    }
    match s.get(*p) {
        Some(b'"') => string_end(s, p),
        Some(b'{' | b'[') => {
            let close = if s[*p] == b'{' { b'}' } else { b']' };
            *p += 1;
            while *p < s.len() {
                ws(s, p);
                if s.get(*p) == Some(&close) {
                    *p += 1;
                    return Ok(());
                }
                if matches!(s.get(*p), Some(b',' | b':')) {
                    *p += 1;
                    continue;
                }
                value_end(s, p, depth + 1)?;
            }
            Err(Error::Syntax)
        }
        Some(_) => {
            let start = *p;
            while *p < s.len()
                && !matches!(s[*p], b',' | b':' | b']' | b'}' | b')' | b';')
                && !s[*p].is_ascii_whitespace()
            {
                *p += 1;
            }
            if *p == start {
                Err(Error::Syntax)
            } else {
                Ok(())
            }
        }
        None => Err(Error::Syntax),
    }
}
struct Decoder<'a, 'b> {
    bindings: &'a [Binding<'a>],
    records: &'a [Record<'a>],
    out: &'b mut [u8],
    count: usize,
}
impl Decoder<'_, '_> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let end = self.count.checked_add(bytes.len()).ok_or(Error::Limit)?;
        if end > self.out.len() {
            return Err(Error::Limit);
        }
        self.out[self.count..end].copy_from_slice(bytes);
        self.count = end;
        Ok(())
    }
    fn emit(&mut self, s: &[u8], p: &mut usize, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::Limit);
        }
        ws(s, p);
        match s.get(*p) {
            Some(b'"') => {
                let start = *p;
                string_end(s, p)?;
                self.put(&s[start..*p])
            }
            Some(b'{') => {
                *p += 1;
                self.put(b"{")?;
                ws(s, p);
                if s.get(*p) == Some(&b'}') {
                    *p += 1;
                    return self.put(b"}");
                }
                loop {
                    ws(s, p);
                    let start = *p;
                    if s.get(*p) == Some(&b'"') {
                        string_end(s, p)?;
                        self.put(&s[start..*p])?;
                    } else {
                        while *p < s.len() && ident(s[*p]) {
                            *p += 1;
                        }
                        if start == *p {
                            return Err(Error::Syntax);
                        }
                        self.put(b"\"")?;
                        self.put(&s[start..*p])?;
                        self.put(b"\"")?;
                    }
                    ws(s, p);
                    if s.get(*p) != Some(&b':') {
                        return Err(Error::Syntax);
                    }
                    *p += 1;
                    self.put(b":")?;
                    self.emit(s, p, depth + 1)?;
                    ws(s, p);
                    match s.get(*p) {
                        Some(b',') => {
                            *p += 1;
                            self.put(b",")?;
                        }
                        Some(b'}') => {
                            *p += 1;
                            return self.put(b"}");
                        }
                        _ => return Err(Error::Syntax),
                    }
                }
            }
            Some(b'[') => {
                *p += 1;
                self.put(b"[")?;
                ws(s, p);
                if s.get(*p) == Some(&b']') {
                    *p += 1;
                    return self.put(b"]");
                }
                loop {
                    self.emit(s, p, depth + 1)?;
                    ws(s, p);
                    match s.get(*p) {
                        Some(b',') => {
                            *p += 1;
                            self.put(b",")?;
                        }
                        Some(b']') => {
                            *p += 1;
                            return self.put(b"]");
                        }
                        _ => return Err(Error::Syntax),
                    }
                }
            }
            Some(_) => {
                let start = *p;
                value_end(s, p, depth)?;
                let value = &s[start..*p];
                if value == b"void 0" {
                    return self.put(b"null");
                }
                if matches!(value, b"true" | b"false" | b"null") {
                    return self.put(value);
                }
                if json_number(value) {
                    return self.put(value);
                }
                let binding = self
                    .bindings
                    .iter()
                    .find(|b| b.name == value)
                    .ok_or(Error::Syntax)?;
                if self.records.iter().any(|r| r.target == value) {
                    let declared =
                        if binding.value.starts_with(b"Array(") && binding.value.ends_with(b")") {
                            core::str::from_utf8(&binding.value[6..binding.value.len() - 1])
                                .ok()
                                .and_then(|x| x.parse::<usize>().ok())
                        } else {
                            None
                        };
                    if !matches!(binding.value, b"[]" | b"{}") && declared.is_none() {
                        return Err(Error::Syntax);
                    }
                    let array = binding.value == b"[]" || declared.is_some();
                    self.put(if array { b"[" } else { b"{" })?;
                    let mut count = 0usize;
                    for i in 0..self.records.len() {
                        let r = self.records[i];
                        if r.target != value {
                            continue;
                        }
                        if r.array != array {
                            return Err(Error::Syntax);
                        }
                        if array
                            && core::str::from_utf8(r.key)
                                .ok()
                                .and_then(|x| x.parse::<usize>().ok())
                                != Some(count)
                        {
                            return Err(Error::Syntax);
                        }
                        if self.records[..i]
                            .iter()
                            .any(|x| x.target == r.target && x.key == r.key)
                        {
                            return Err(Error::Syntax);
                        }
                        if count > 0 {
                            self.put(b",")?;
                        }
                        if !array {
                            self.put(b"\"")?;
                            self.put(r.key)?;
                            self.put(b"\":")?;
                        }
                        let mut offset = 0;
                        self.emit(r.value, &mut offset, depth + 1)?;
                        ws(r.value, &mut offset);
                        if offset != r.value.len() {
                            return Err(Error::Syntax);
                        }
                        count += 1;
                    }
                    if declared.is_some() && declared != Some(count) {
                        return Err(Error::Syntax);
                    }
                    return self.put(if array { b"]" } else { b"}" });
                }
                let mut offset = 0;
                self.emit(binding.value, &mut offset, depth + 1)?;
                ws(binding.value, &mut offset);
                if offset != binding.value.len() {
                    return Err(Error::Syntax);
                }
                Ok(())
            }
            None => Err(Error::Syntax),
        }
    }
}
fn find(s: &[u8], needle: &[u8]) -> Option<usize> {
    s.windows(needle.len()).position(|w| w == needle)
}
pub fn decode(html: &'static [u8], out: &mut [u8]) -> Result<usize, Error> {
    let start = find(html, b"window.__NUXT__=(function(").ok_or(Error::Shape)?
        + b"window.__NUXT__=(function(".len();
    let s = &html[start..];
    let mut p = 0;
    let bindings = unsafe { &mut *core::ptr::addr_of_mut!(BINDINGS) };
    let records = unsafe { &mut *core::ptr::addr_of_mut!(RECORDS) };
    let mut n = 0;
    let mut nr = 0;
    loop {
        ws(s, &mut p);
        if s.get(p) == Some(&b')') {
            p += 1;
            break;
        }
        if n == MAX_BINDINGS {
            return Err(Error::Limit);
        }
        let begin = p;
        while p < s.len() && ident(s[p]) {
            p += 1;
        }
        if begin == p {
            return Err(Error::Syntax);
        }
        if bindings[..n].iter().any(|b| b.name == &s[begin..p]) {
            return Err(Error::Syntax);
        }
        bindings[n].name = &s[begin..p];
        n += 1;
        ws(s, &mut p);
        match s.get(p) {
            Some(b',') => p += 1,
            Some(b')') => {}
            _ => return Err(Error::Syntax),
        }
    }
    ws(s, &mut p);
    if s.get(p) != Some(&b'{') {
        return Err(Error::ReturnShape);
    }
    p += 1;
    loop {
        ws(s, &mut p);
        if s.get(p..p + 7) == Some(b"return ") {
            p += 7;
            break;
        }
        if nr == MAX_RECORDS {
            return Err(Error::Limit);
        }
        let begin = p;
        while p < s.len() && ident(s[p]) {
            p += 1;
        }
        let target = &s[begin..p];
        if target.is_empty() || !bindings[..n].iter().any(|b| b.name == target) {
            return Err(Error::Syntax);
        }
        let array = s.get(p) == Some(&b'[');
        let key;
        if array {
            p += 1;
            let begin = p;
            while p < s.len() && s[p].is_ascii_digit() {
                p += 1;
            }
            key = &s[begin..p];
            if key.is_empty() || s.get(p) != Some(&b']') {
                return Err(Error::Syntax);
            }
            p += 1;
        } else {
            if s.get(p) != Some(&b'.') {
                return Err(Error::Syntax);
            }
            p += 1;
            let begin = p;
            while p < s.len() && ident(s[p]) {
                p += 1;
            }
            key = &s[begin..p];
            if key.is_empty() || matches!(key, b"__proto__" | b"constructor" | b"prototype") {
                return Err(Error::Syntax);
            }
        }
        if s.get(p) != Some(&b'=') {
            return Err(Error::Syntax);
        }
        p += 1;
        let begin = p;
        value_end(s, &mut p, 0)?;
        records[nr] = Record {
            target,
            key,
            array,
            value: &s[begin..p],
        };
        nr += 1;
        ws(s, &mut p);
        if s.get(p) != Some(&b';') {
            return Err(Error::Syntax);
        }
        p += 1;
    }
    let object_start = p;
    value_end(s, &mut p, 0)?;
    let object = &s[object_start..p];
    ws(s, &mut p);
    if s.get(p) == Some(&b';') {
        p += 1;
        ws(s, &mut p);
    }
    if s.get(p..p + 2) != Some(b"}(") {
        return Err(Error::ArgumentsShape);
    }
    p += 2;
    for b in &mut bindings[..n] {
        ws(s, &mut p);
        let begin = p;
        value_end(s, &mut p, 0)?;
        b.value = &s[begin..p];
        ws(s, &mut p);
        if s.get(p) == Some(&b',') {
            p += 1;
        } else if s.get(p) != Some(&b')') {
            return Err(Error::Syntax);
        }
    }
    if s.get(p..p + 2) != Some(b"))") {
        return Err(Error::ClosingShape);
    }
    let mut d = Decoder {
        bindings: &bindings[..n],
        records: &records[..nr],
        out,
        count: 0,
    };
    // Validate even unused bindings and initialization values; no unsupported
    // call/expression is silently accepted merely because the page omits it.
    for binding in &bindings[..n] {
        d.count = 0;
        let mut q = 0;
        d.emit(binding.name, &mut q, 0)?;
        if q != binding.name.len() {
            return Err(Error::Syntax);
        }
    }
    for record in &records[..nr] {
        d.count = 0;
        let mut q = 0;
        d.emit(record.value, &mut q, 0)?;
        ws(record.value, &mut q);
        if q != record.value.len() {
            return Err(Error::Syntax);
        }
    }
    d.count = 0;
    let mut q = 0;
    d.emit(object, &mut q, 0)?;
    ws(object, &mut q);
    if q != object.len() {
        return Err(Error::Syntax);
    }
    Ok(d.count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_json_number_spellings() {
        for value in [b"+1".as_slice(), b"01", b"1.", b".1", b"1e", b"-01"] {
            assert!(!json_number(value));
        }
        for value in [b"0".as_slice(), b"-0", b"1.25", b"2e+3", b"-12E-2"] {
            assert!(json_number(value));
        }
    }
    #[test]
    fn resolves_data_without_execution() {
        let mut out = [0; 300];
        let n = decode(br#"window.__NUXT__=(function(a,b){return {data:[{title:a,id:b,url:"https:\u002F\u002Fx"}],ok:true}}("Chinese",116));"#, &mut out).unwrap();
        assert_eq!(
            &out[..n],
            br#"{"data":[{"title":"Chinese","id":116,"url":"https:\u002F\u002Fx"}],"ok":true}"#
        );
    }
    #[test]
    fn reads_only_empty_container_data_initialization() {
        let mut out = [0; 300];
        let n=decode(br#"window.__NUXT__=(function(a,b){a[0]=1;a[1]=2;b.name="Author";return {tags:a,user:b}}(Array(2),{}));"#,&mut out).unwrap();
        assert_eq!(&out[..n], br#"{"tags":[1,2],"user":{"name":"Author"}}"#);
        for input in [
            b"window.__NUXT__=(function(a){a[1]=2;return {x:a}}([]));".as_slice(),
            b"window.__NUXT__=(function(a){a.x=1;a.x=2;return {x:a}}({}));",
            b"window.__NUXT__=(function(a){a.x=fetch();return {x:a}}({}));",
        ] {
            assert!(decode(input, &mut [0; 100]).is_err());
        }
    }
    #[test]
    fn refuses_code_and_unresolved_names() {
        for input in [
            b"window.__NUXT__=(function(a){return {x:fetch(a)}}(1));".as_slice(),
            b"window.__NUXT__=(function(a){return {x:b}}(1));",
            b"window.__NUXT__=(function(a){return {x:a+1}}(1));",
            b"window.__NUXT__=(function(a,b){b.x=fetch();return {x:a}}(1,{}));",
        ] {
            assert!(decode(input, &mut [0; 100]).is_err());
        }
    }
    #[test]
    fn refuses_output_overflow_and_wrong_shape() {
        assert_eq!(
            decode(
                b"window.__NUXT__=(function(a){return {x:a}}(123));",
                &mut [0; 2]
            ),
            Err(Error::Limit)
        );
        assert_eq!(
            decode(b"window.__NUXT__=eval('bad');", &mut [0; 100]),
            Err(Error::Shape)
        );
    }
}
