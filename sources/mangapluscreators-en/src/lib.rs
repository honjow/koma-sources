#![no_std]
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{host, json_utils::*, source::SourceInfo};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://mangaplus-creators.jp";
const ASSET: &[u8] = b"https://assets.mangaplus-creators.jp";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangapluscreators.en.koma",
    name: "MANGA Plus Creators by SHUEISHA",
    version: "0.1.5",
    api_version: "0.2",
    language: "en",
    author: "Koma",
    description: "Official English creator catalogue. Per-work rating and source usage unknown.",
    content_rating: "safe",
};
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut CH_IDS: [[u8; 64]; 1000] = [[0; 64]; 1000];
static mut CH_LEN: [usize; 1000] = [0; 1000];
static mut CH_NAMES: [[u8; 128]; 1000] = [[0; 128]; 1000];
static mut CH_NLEN: [usize; 1000] = [0; 1000];
static mut PAGES: [[u8; 2048]; 1000] = [[0; 2048]; 1000];
static mut PAGE_LEN: [usize; 1000] = [0; 1000];
static mut JSON: [u8; MAX] = [0; MAX];
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
fn args(r: &[u8]) -> Result<&[u8], E> {
    valid_json(r)?;
    if let Some(s) = member(r, b"sourceId")? {
        let value = quoted(s)?;
        let imageHostSelected = value.is_empty()
            && member(r, b"operation")?.and_then(|v| quoted(v).ok())
                == Some(b"get_image_request".as_slice());
        if value != SOURCE_INFO.id.as_bytes() && !imageHostSelected {
            return Err("request source identity");
        }
    }
    if let Some(v) = member(r, b"version")? {
        if num(v)? != 1 {
            return Err("request version");
        }
    }
    let a = member(r, b"args")?.unwrap_or(r);
    if a.first() != Some(&b'{') {
        return Err("args object");
    }
    Ok(a)
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

fn wn(b: &mut [u8], n: &mut usize, x: usize) -> Result<(), E> {
    if write_usize(b, n, x) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}

fn uq<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    unescape(quoted(v)?, b)
}
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn all_raw(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
struct Owned(HtmlDescriptor);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
struct All {
    ids: [i32; 1000],
    len: usize,
}
impl All {
    fn new(d: HtmlDescriptor, s: &[u8]) -> R<Self> {
        let mut a = Self {
            ids: [0; 1000],
            len: 0,
        };
        let count = unsafe {
            all_raw(
                d.raw(),
                s.as_ptr(),
                s.len() as u32,
                a.ids.as_mut_ptr() as *mut u8,
                4000,
            )
        };
        if count < 0 {
            return Err("parse_error");
        }
        a.len = (count as usize).min(1000);
        if count as usize > 1000 {
            return Err("capacity_exceeded");
        }
        if a.ids[..a.len].iter().any(|x| *x <= 0) {
            return Err("parse_error");
        }
        Ok(a)
    }
    fn at(&self, i: usize) -> HtmlDescriptor {
        HtmlDescriptor::from_raw(self.ids[i])
    }
}
impl Drop for All {
    fn drop(&mut self) {
        for id in &self.ids[..self.len] {
            if *id > 0 {
                let _ = host::html_close(HtmlDescriptor::from_raw(*id));
            }
        }
    }
}
fn attr<'a>(d: HtmlDescriptor, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let n = host::html_attr(d, k, b).map_err(|_| "parse_error")?;
    if n >= b.len() {
        return Err("capacity_exceeded");
    }
    core::str::from_utf8(&b[..n]).map_err(|_| "attribute UTF8")?;
    Ok(&b[..n])
}
fn txt<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut raw = [0u8; 8192];
    let z = host::html_text(d, &mut raw).map_err(|_| "text capacity or parse")?;
    if z >= raw.len() {
        return Err("text capacity");
    };
    core::str::from_utf8(&raw[..z]).map_err(|_| "text UTF8")?;
    let mut n = 0;
    let mut pending = false;
    for c in &raw[..z] {
        if c.is_ascii_whitespace() {
            if n > 0 {
                pending = true
            }
        } else {
            if pending {
                bytes(b, &mut n, b" ")?;
                pending = false
            }
            bytes(b, &mut n, &[*c])?
        }
    }
    Ok(&b[..n])
}
fn sa<'a>(d: HtmlDescriptor, s: &[u8], k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    attr(x.0, k, b)
}
fn st<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    txt(x.0, b)
}
const UA:&[u8]=b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36";
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://mangaplus-creators.jp/") {
        return Err("metadata host");
    }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"Referer\":\"https://mangaplus-creators.jp/\",\"Origin\":\"https://mangaplus-creators.jp\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
    q(b, &mut n, UA)?;
    bytes(
        b,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&b[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP capacity");
    }
    let e = &http_out()[..z];
    valid_json(e)?;
    let status = num(member(e, b"status")?
        .or(member(e, b"statusCode")?)
        .ok_or("HTTP status")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status error");
    }
    let z = koma_source_sdk::decode_json_body_into(e, body_buf()).map_err(|_| "HTTP decode")?;
    if z >= MAX {
        return Err("body capacity");
    };
    core::str::from_utf8(&body_buf()[..z]).map_err(|_| "HTML UTF8")?;
    Ok(Owned(
        host::html_parse(&body_buf()[..z]).map_err(|_| "HTML parse")?,
    ))
}
fn make<'a>(ps: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for p in ps {
        bytes(b, &mut n, p)?
    }
    Ok(&b[..n])
}
fn ident(s: &[u8]) -> R<&[u8]> {
    if s.is_empty() || s.len() > 64 || !s.iter().all(u8::is_ascii_alphanumeric) {
        Err("invalid identity")
    } else {
        Ok(s)
    }
}
fn key<'a>(r: &'a [u8], f: &[u8], prefix: &[u8]) -> R<&'a [u8]> {
    ident(
        text(args(r)?, f)?
            .strip_prefix(prefix)
            .ok_or("identity namespace")?,
    )
}
fn absolute<'a>(s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    if s.starts_with(b"/") && !s.starts_with(b"//") {
        make(&[WEB, s], b)
    } else if s.starts_with(b"https://mangaplus-creators.jp/") {
        make(&[s], b)
    } else {
        Err("route host")
    }
}
fn episode_href(s: &[u8]) -> R<&[u8]> {
    let s = s
        .strip_prefix(b"https://mangaplus-creators.jp/episodes/")
        .or(s.strip_prefix(b"/episodes/"))
        .ok_or("episode route")?;
    ident(s.strip_suffix(b"/").unwrap_or(s))
}
fn asset(s: &[u8]) -> R<()> {
    let s = s
        .strip_prefix(b"https://assets.mangaplus-creators.jp/")
        .ok_or("asset host")?;
    if s.len() > 1900
        || s.iter()
            .any(|c| *c <= 32 || matches!(c, b'\\' | b'#' | b'@'))
    {
        return Err("asset URL");
    }
    let path = s.split(|c| *c == b'?').next().ok_or("asset path")?;
    if path
        .split(|c| *c == b'/')
        .any(|v| v.is_empty() || v == b"." || v == b".." || v.contains(&b'%'))
    {
        return Err("asset path");
    };
    Ok(())
}
fn cover_owner<'a>(url: &'a [u8]) -> R<&'a [u8]> {
    asset(url)?;
    let p = url
        .strip_prefix(b"https://assets.mangaplus-creators.jp/u/title/")
        .ok_or("cover path")?;
    ident(p.split(|c| *c == b'/').next().ok_or("cover identity")?)
}
fn work_url<'a>(id: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    make(&[WEB, b"/titles/", ident(id)?], b)
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut b = [0; 256];
    let d = fetch(work_url(id, &mut b)?)?;
    let mut lang = [0; 32];
    if st(d.0, b".book-box .book-locale", &mut lang)? != b"EN" {
        return Err("detail language");
    }
    let mut c = [0; 2048];
    if cover_owner(sa(d.0, b".book-box div.cover img", b"data-src", &mut c)?)? != id {
        return Err("detail ownership");
    }
    Ok(d)
}
fn no_cursor(r: &[u8]) -> R<()> {
    let a = args(r)?;
    if let Some(p) = member(a, b"page")? {
        if p.first() == Some(&b'{') {
            if let Some(c) = member(p, b"cursor")? {
                if c != b"null" && c != b"\"\"" {
                    return Err("listing cursor unsupported");
                }
            }
        } else if num(p)? != 1 {
            return Err("listing page unsupported");
        }
    }
    Ok(())
}
fn list(r: &[u8], search: bool) -> R<usize> {
    no_cursor(r)?;
    let a = args(r)?;
    let mut u = [0; 8192];
    let url = if search {
        let mut qb = [0; 4096];
        let v = uq(required(a, b"query")?, &mut qb)?;
        if trim_ascii(v).is_empty() {
            return Err("empty search");
        };
        let mut n = 0;
        bytes(&mut u, &mut n, b"https://mangaplus-creators.jp/keywords?q=")?;
        if !write_url_encoded(&mut u, &mut n, v) {
            return Err("query capacity");
        };
        bytes(&mut u, &mut n, b"&s=date&lang=en")?;
        &u[..n]
    } else {
        if let Some(v) = member(a, b"listingId")? {
            if quoted(v)? != b"listing:popular" {
                return Err("listing identity");
            }
        }
        let filters = member(a, b"filters")?.unwrap_or(b"{}");
        let genre = member(filters, b"genre")?.map(quoted).transpose()?.unwrap_or(b"");
        let sort = member(filters, b"sort")?.map(quoted).transpose()?.unwrap_or(b"");
        if !matches!(genre, b"" | b"fantasy" | b"action" | b"romance" | b"horror" | b"slice_of_life" | b"comedy" | b"sports" | b"sf" | b"mystery" | b"others") || !matches!(sort, b"" | b"popular" | b"latest_desc" | b"like_desc") { return Err("invalid filters"); }
        if genre.is_empty() && sort.is_empty() {
            b"https://mangaplus-creators.jp/titles/popular/?p=m&l=en".as_slice()
        } else {
            let mut n = 0;
            bytes(&mut u, &mut n, b"https://mangaplus-creators.jp/genres")?;
            if !genre.is_empty() { bytes(&mut u, &mut n, b"/")?; bytes(&mut u, &mut n, genre)?; }
            bytes(&mut u, &mut n, b"?l=en")?;
            if !sort.is_empty() && sort != b"popular" { bytes(&mut u, &mut n, b"&s=")?; bytes(&mut u, &mut n, sort)?; }
            &u[..n]
        }
    };
    let d = fetch(url)?;
    let cards = All::new(
        d.0,
        if search {
            b"div.item-search"
        } else {
            b"div.item-recent"
        },
    )?;
    if !search && cards.len == 0 {
        return Err("popular template empty");
    };
    // This published provider branch is finite, but unexpected future pagination fails closed.
    if All::new(d.0, b".mod-pagination .next, a[rel=next]")?.len != 0 {
        return Err("unreviewed listing pagination");
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let mut ids = [[0u8; 64]; 1000];
    let mut lens = [0usize; 1000];
    for i in 0..cards.len {
        let c = cards.at(i);
        let mut cv = [0; 2048];
        let cover = sa(c, b".image-area img", b"src", &mut cv)?;
        let id = cover_owner(cover)?;
        for j in 0..i {
            if id == &ids[j][..lens[j]] {
                return Err("duplicate work");
            }
        }
        ids[i][..id.len()].copy_from_slice(id);
        lens[i] = id.len();
        let mut t = [0; 4096];
        let title = st(c, b".title-area .title", &mut t)?;
        if title.is_empty() {
            return Err("empty title");
        };
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        emit_work(out, &mut n, id, title, cover)?;
    }
    bytes(
        out,
        &mut n,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    Ok(n)
}
fn emit_work(out: &mut [u8], n: &mut usize, id: &[u8], title: &[u8], cover: &[u8]) -> R<()> {
    let mut mid = [0; 128];
    let mid = make(&[b"mpc:en:", id], &mut mid)?;
    let mut u = [0; 256];
    bytes(out, n, b"{\"id\":")?;
    q(out, n, mid)?;
    bytes(out, n, b",\"title\":")?;
    q(out, n, title)?;
    bytes(out, n, b",\"url\":")?;
    q(out, n, work_url(id, &mut u)?)?;
    bytes(
        out,
        n,
        b",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":",
    )?;
    q(out, n, cover)?;
    bytes(out, n, b"}}")
}
fn detail(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"mpc:en:")?;
    let d = detail_doc(id)?;
    let mut t = [0; 4096];
    let title = st(d.0, b".book-box div.title", &mut t)?;
    let mut c = [0; 2048];
    let cover = sa(d.0, b".book-box div.cover img", b"data-src", &mut c)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":")?;
    emit_work(out, &mut n, id, title, cover)?;
    n -= 1;
    let mut a = [0; 4096];
    let author = st(d.0, b".book-box div.mod-btn-profile div.name", &mut a)?;
    bytes(out, &mut n, b",\"authors\":[")?;
    q(out, &mut n, author)?;
    bytes(out, &mut n, b"],\"description\":")?;
    let paragraphs = All::new(d.0, b".book-box div.summary p")?;
    let mut desc = [0; 32768];
    let mut z = 0;
    for i in 0..paragraphs.len {
        let mut b = [0; 8192];
        let p = txt(paragraphs.at(i), &mut b)?;
        if i > 0 {
            bytes(&mut desc, &mut z, b"\n\n")?
        };
        bytes(&mut desc, &mut z, p)?
    }
    q(out, &mut n, &desc[..z])?;
    let mut s = [0; 128];
    let status = match st(d.0, b".book-box div.book-submit-type", &mut s)? {
        b"Series" => b"ongoing".as_slice(),
        b"One-shot" => b"completed",
        _ => b"unknown",
    };
    bytes(out, &mut n, b",\"status\":")?;
    q(out, &mut n, status)?;
    bytes(out, &mut n, b",\"tags\":[")?;
    let tags = All::new(d.0, b".book-box div.genre-area div.tag-genre")?;
    for i in 0..tags.len {
        let mut b = [0; 512];
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        q(out, &mut n, txt(tags.at(i), &mut b)?)?
    }
    bytes(out, &mut n, b"]}}")?;
    Ok(n)
}
fn collect(id: &[u8]) -> R<usize> {
    let mut total = 0;
    let mut page = 1usize;
    loop {
        if page > 1000 {
            return Err("catalogue pagination capacity");
        };
        let mut u = [0; 256];
        let mut n = 0;
        bytes(&mut u, &mut n, WEB)?;
        bytes(&mut u, &mut n, b"/titles/")?;
        bytes(&mut u, &mut n, ident(id)?)?;
        bytes(&mut u, &mut n, b"/?page=")?;
        wn(&mut u, &mut n, page)?;
        let d = fetch(&u[..n])?;
        let mut lang = [0; 32];
        if st(d.0, b".book-box .book-locale", &mut lang)? != b"EN" {
            return Err("catalogue language");
        }
        let mut cover = [0; 2048];
        if cover_owner(sa(
            d.0,
            b".book-box div.cover img",
            b"data-src",
            &mut cover,
        )?)? != id
        {
            return Err("catalogue work ownership");
        }
        let rows = All::new(d.0, b".mod-item-series")?;
        if rows.len == 0 {
            return Err("catalogue empty or no progress");
        };
        for i in 0..rows.len {
            if total >= 1000 {
                return Err("chapter capacity");
            };
            let mut h = [0; 256];
            let ep = episode_href(attr(rows.at(i), b"href", &mut h)?)?;
            for j in 0..total {
                if unsafe { ep == &CH_IDS[j][..CH_LEN[j]] } {
                    return Err("duplicate chapter");
                }
            }
            let mut b = [0; 128];
            let title = st(rows.at(i), b".number", &mut b)?;
            if title.is_empty() {
                return Err("chapter title");
            };
            unsafe {
                CH_IDS[total][..ep.len()].copy_from_slice(ep);
                CH_LEN[total] = ep.len();
                CH_NAMES[total][..title.len()].copy_from_slice(title);
                CH_NLEN[total] = title.len()
            };
            total += 1
        }
        let next = All::new(d.0, b".mod-pagination .next")?;
        if next.len == 0 {
            break;
        }
        if next.len != 1 {
            return Err("ambiguous chapter next");
        };
        let x = next.at(0);
        let mut href = [0; 512];
        let z = attr(x, b"href", &mut href)?;
        let mut childhref = [0; 512];
        let raw = if z.is_empty() {
            sa(x, b"a", b"href", &mut childhref)?
        } else {
            z
        };
        let mut abs = [0; 512];
        let url = absolute(raw, &mut abs)?;
        let mut expected = [0; 256];
        let mut len = 0;
        bytes(&mut expected, &mut len, WEB)?;
        bytes(&mut expected, &mut len, b"/titles/")?;
        bytes(&mut expected, &mut len, id)?;
        bytes(&mut expected, &mut len, b"/?page=")?;
        let np = page.checked_add(1).ok_or("page overflow")?;
        wn(&mut expected, &mut len, np)?;
        if url != &expected[..len] {
            return Err("chapter next ownership");
        };
        page = np;
    }
    Ok(total)
}
fn chapters(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"mpc:en:")?;
    let count = collect(id)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"chapters\":[")?;
    for i in 0..count {
        let j = count - 1 - i;
        let ep = unsafe { &CH_IDS[j][..CH_LEN[j]] };
        let title = unsafe { &CH_NAMES[j][..CH_NLEN[j]] };
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"mpcch:en:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, ep)?;
        bytes(out, &mut n, b"\",\"mangaId\":\"mpc:en:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b"\",\"title\":")?;
        q(out, &mut n, title)?;
        bytes(out, &mut n, b",\"chapterNumber\":")?;
        if title == b"One-shot" {
            bytes(out, &mut n, b"0")?
        } else if let Some(s) = title.strip_prefix(b"#") {
            let mut parts = s.split(|c| *c == b'.');
            let integer = parts.next().ok_or("chapter number")?;
            let fraction = parts.next();
            if parts.next().is_some()
                || number(integer).is_none()
                || fraction
                    .map(|v| v.is_empty() || !v.iter().all(u8::is_ascii_digit))
                    .unwrap_or(false)
            {
                return Err("chapter number");
            }
            let at = integer
                .iter()
                .position(|c| *c != b'0')
                .unwrap_or(integer.len().saturating_sub(1));
            bytes(out, &mut n, &integer[at..])?;
            if let Some(f) = fraction {
                bytes(out, &mut n, b".")?;
                bytes(out, &mut n, f)?
            }
        } else {
            bytes(out, &mut n, b"null")?
        };
        bytes(out, &mut n, b",\"volumeNumber\":null,\"language\":\"en\"}")?
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn pair<'a>(s: &'a [u8]) -> R<(&'a [u8], &'a [u8])> {
    let p = s
        .iter()
        .position(|c| *c == b':')
        .ok_or("chapter identity")?;
    Ok((ident(&s[..p])?, ident(&s[p + 1..])?))
}
fn reader(id: &[u8], ep: &[u8]) -> R<usize> {
    let c = collect(id)?;
    if !(0..c).any(|i| unsafe { ep == &CH_IDS[i][..CH_LEN[i]] }) {
        return Err("chapter membership");
    };
    let mut u = [0; 256];
    let d = fetch(make(&[WEB, b"/episodes/", ep], &mut u)?)?;
    let v = All::new(d.0, b"div[react=viewer]")?;
    if v.len != 1 {
        return Err("reader viewer");
    };
    let v = v.at(0);
    for key in [
        b"data-is_preview".as_slice(),
        b"data-is_blocked",
        b"data-is_blocking",
    ] {
        let mut b = [0; 16];
        let flag = attr(v, key, &mut b)?;
        if !matches!(flag, b"" | b"false" | b"0") {
            return Err("reader preview or access gate");
        }
    }
    let mut title = [0; 32768];
    let title = attr(v, b"data-title", &mut title)?;
    valid_json(title)?;
    let mut b = [0; 128];
    if uq(required(title, b"contents_id")?, &mut b)? != id {
        return Err("reader title owner");
    };
    let tid = num(required(title, b"id")?)?;
    if text(title, b"status")? != b"Public" || text(title, b"language")? != b"EN" {
        return Err("reader public language");
    };
    let mut chapter = [0; 32768];
    let chapter = attr(v, b"data-episode", &mut chapter)?;
    valid_json(chapter)?;
    if uq(required(chapter, b"contents_id")?, &mut b)? != ep
        || num(required(chapter, b"title_id")?)? != tid
    {
        return Err("reader episode owner");
    };
    if text(chapter, b"publish_status")? != b"01" {
        return Err("chapter unavailable");
    }
    let j = unsafe { &mut JSON };
    let j = attr(v, b"data-pages", j)?;
    valid_json(j)?;
    let arr = required(j, b"pc")?;
    if arr.first() != Some(&b'[') {
        return Err("pages array");
    };
    let mut p = ws(arr, 1);
    let mut count = 0;
    while arr.get(p) != Some(&b']') {
        if count >= 1000 {
            return Err("page capacity");
        };
        let end = json_value_end(arr, p, 0).ok_or("page JSON")?;
        let x = &arr[p..end];
        if num(required(x, b"page_no")?)? != count + 1 {
            return Err("page sequence");
        };
        let w = num(required(x, b"image_width")?)?;
        let h = num(required(x, b"image_height")?)?;
        if w == 0 || h == 0 || w > 100000 || h > 100000 {
            return Err("page dimensions");
        };
        let mut ub = [0; 2048];
        let url = uq(required(x, b"image_url")?, &mut ub)?;
        asset(url)?;
        let mut prefix = [0; 256];
        let pr = make(&[ASSET, b"/u/episode/", ep, b"/"], &mut prefix)?;
        if !url.starts_with(pr) {
            return Err("page asset owner");
        };
        for i in 0..count {
            if unsafe { url == &PAGES[i][..PAGE_LEN[i]] } {
                return Err("duplicate page");
            }
        }
        unsafe {
            PAGES[count][..url.len()].copy_from_slice(url);
            PAGE_LEN[count] = url.len()
        };
        count += 1;
        p = ws(arr, end);
        if arr.get(p) == Some(&b',') {
            p = ws(arr, p + 1)
        } else if arr.get(p) != Some(&b']') {
            return Err("page separator");
        }
    }
    if count == 0 {
        return Err("zero pages");
    };
    Ok(count)
}
fn pages(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let (id, ep) = pair(
        text(a, b"chapterId")?
            .strip_prefix(b"mpcch:en:")
            .ok_or("chapter namespace")?,
    )?;
    if let Some(m) = member(a, b"mangaId")? {
        let mut b = [0; 128];
        if uq(m, &mut b)? != make(&[b"mpc:en:", id], &mut [0; 128])? {
            return Err("page work context");
        }
    }
    let count = reader(id, ep)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"mpcpage:en:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, ep)?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b"\",\"index\":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        unsafe { q(out, &mut n, &PAGES[i][..PAGE_LEN[i]])? };
        bytes(out, &mut n, b"}}")?
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
// Consumer boundary: input is the actual descriptor previously emitted by this source.
// Full provider membership/sequence remains in get_pages; no per-image metadata I/O.
fn image_asset_tail(url: &[u8], prefix: &[u8], page: bool) -> R<()> {
    asset(url)?;
    let tail = url.strip_prefix(prefix).ok_or("image URL owner")?;
    let mut parts = tail.split(|c| *c == b'?');
    let path = parts.next().ok_or("image path")?;
    let query = parts.next().ok_or("image signature missing")?;
    if parts.next().is_some() {
        return Err("image query ambiguity");
    }
    let signature = query.strip_prefix(b"h=").ok_or("image signature field")?;
    if signature.is_empty()
        || signature.len() > 256
        || !signature
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("image signature syntax");
    }
    let filename = if page {
        if let Some(at) = path.iter().position(|c| *c == b'/') {
            if number(&path[..at]).filter(|n| *n > 0).is_none() {
                return Err("page asset directory");
            }
            &path[at + 1..]
        } else {
            path
        }
    } else {
        path
    };
    if filename.is_empty()
        || filename.len() > 128
        || !filename
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
        || !filename.ends_with(b".jpg")
    {
        return Err("image filename syntax");
    }
    Ok(())
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut ub = [0; 2048];
    let url = uq(required(a, b"url")?, &mut ub)?;
    asset(url)?;
    let pid = text(a, b"pageId")?;
    let mut routebuf = [0; 256];
    let route = uq(required(a, b"mangaUrl")?, &mut routebuf)?;
    let mut referer = [0; 256];
    let (work, refurl) = if let Some(work) = pid.strip_prefix(b"mpc:en:") {
        let work = ident(work)?;
        let mut prefix = [0; 256];
        image_asset_tail(
            url,
            make(&[ASSET, b"/u/title/", work, b"/"], &mut prefix)?,
            false,
        )?;
        (work, work_url(work, &mut referer)?)
    } else {
        let v = pid.strip_prefix(b"mpcpage:en:").ok_or("image namespace")?;
        let at = v.iter().rposition(|c| *c == b':').ok_or("page identity")?;
        let index = number(&v[at + 1..]).ok_or("page index")?;
        if index >= 1000 {
            return Err("page index capacity");
        }
        let (work, ep) = pair(&v[..at])?;
        let mut prefix = [0; 256];
        image_asset_tail(
            url,
            make(&[ASSET, b"/u/episode/", ep, b"/"], &mut prefix)?,
            true,
        )?;
        (work, make(&[WEB, b"/episodes/", ep], &mut referer)?)
    };
    let mut expected = [0; 256];
    if route != work_url(work, &mut expected)? {
        return Err("image independent work route");
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"imageRequest\":{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out, &mut n, b",\"headers\":{\"Referer\":")?;
    q(out, &mut n, refurl)?;
    bytes(out, &mut n, b",\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(out, &mut n, b"}}}")?;
    Ok(n)
}
fn execute(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r, false))
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
fn run_get_listings(_: &[u8]) -> u32 {
    let s=b"{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"Popular (English)\",\"kind\":\"popular\",\"supportsPaging\":false}]}";
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let supported = json_object_member(r, b"hostFeatures").map(|v| {
        let v = trim_ascii(v);
        v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c == b',')
            .any(|item| trim_ascii(item) == b"\"source_filter_scope_header_v1\"")
    }).unwrap_or(false);
    // Old hosts get no controls; the scope and static note require the new contract.
    let schema = if supported { br#"{"filters":[{"id":"note","type":"header","name":"NOTE: Ignored if using text search!","scope":"browse"},{"id":"sort","name":"Sort","type":"select","scope":"browse","options":[{"value":"popular","label":"Popularity"},{"value":"latest_desc","label":"Date"},{"value":"like_desc","label":"Likes"}]},{"id":"genre","name":"Genres","type":"select","scope":"browse","options":[{"value":"fantasy","label":"Fantasy"},{"value":"action","label":"Action"},{"value":"romance","label":"Romance"},{"value":"horror","label":"Horror"},{"value":"slice_of_life","label":"Slice of Life"},{"value":"comedy","label":"Comedy"},{"value":"sports","label":"Sports"},{"value":"sf","label":"Sci-Fi"},{"value":"mystery","label":"Mystery"},{"value":"others","label":"Others"}]}]}"#.as_slice() } else { br#"{"filters":[]}"#.as_slice() };
    payload_buf()[..schema.len()].copy_from_slice(schema);
    write_success_payload("get_filters", schema.len())
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
}
// Source-local ABI gate; SDK remains byte-identical. Check before helper forms a slice.
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= MAX || end > cap {
        return None;
    };
    read_request(ptr, len)
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
    response_buffer().write_success_parts("source_info", &[b"{\"sourceInfo\":{\"id\":\"com.mangapluscreators.en.koma\",\"name\":\"MANGA Plus Creators by SHUEISHA\",\"version\":\"0.1.5\",\"apiVersion\":\"0.2\",\"language\":\"en\",\"author\":\"Koma\",\"description\":\"Official English creator catalogue. Per-work rating and source usage unknown.\",\"contentRating\":\"safe\",\"localizedNames\":", "{\"zh-Hans\":\"MANGA Plus Creators（英语）\",\"zh-Hant\":\"MANGA Plus Creators（英語）\",\"en\":\"MANGA Plus Creators by SHUEISHA\"}".as_bytes(), b"},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":true,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}"])
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => match args(r) {
                    Ok(_) => match member(r, b"operation") {
                        Ok(Some(v)) if quoted(v).ok() != Some($op.as_bytes()) => {
                            write_error($op, "invalid_request", "operation mismatch")
                        }
                        Err(_) => write_error($op, "invalid_request", "malformed request"),
                        _ => $run(r),
                    },
                    Err(e) => write_error($op, "invalid_request", e),
                },
                None => write_error($op, "invalid_request", "invalid ABI request range"),
            }
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
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= MAX {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
