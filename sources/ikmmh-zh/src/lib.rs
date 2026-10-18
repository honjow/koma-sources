#![no_std]
// Independent published anonymous Chinese metadata source. No auth/retry/dummy.
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://ymcdnyfqdapp.ikmmh.com";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.ikmmh.zh.koma",name:"Ikmmh",version:"0.1.0",api_version:"0.2",language:"zh",author:"Koma",description:"Ikmmh anonymous category catalogue and finite query. Per-work rating, picture availability and usage unknown.",content_rating:"nsfw"};
#[allow(dead_code)]
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    image_request: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut PAGES: [[u8; 1024]; 1024] = [[0; 1024]; 1024];
static mut PAGE_LEN: [usize; 1024] = [0; 1024];
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
    let n = host::html_text(d, b).map_err(|_| "parse_error")?;
    if n >= b.len() {
        return Err("capacity_exceeded");
    }
    Ok(trim_ascii(&b[..n]))
}
fn sa<'a>(d: HtmlDescriptor, s: &[u8], k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    attr(x.0, k, b)
}
fn st<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    txt(x.0, b)
}
const UA:&[u8]=b"Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
const ACCEPT:&[u8]=b"text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8";
static mut DATA: [u8; MAX] = [0; MAX];
fn ident(v: &[u8]) -> R<&[u8]> {
    if v.is_empty() || v.len() > 15 || v[0] == b'0' || number(v).is_none() {
        Err("numeric identity")
    } else {
        Ok(v)
    }
}
fn make<'a>(parts: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for x in parts {
        bytes(b, &mut n, x)?
    }
    Ok(&b[..n])
}
fn headers(b: &mut [u8], n: &mut usize) -> R<()> {
    bytes(b, n, b"{\"User-Agent\":")?;
    q(b, n, UA)?;
    bytes(b, n, b",\"Accept-Language\":\"zh-CN,zh;q=0.9\",\"Accept\":")?;
    q(b, n, ACCEPT)
}
fn fetch_body(url: &[u8], form: Option<&[u8]>) -> R<usize> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    if !url.starts_with(b"https://ymcdnyfqdapp.ikmmh.com/") {
        return Err("metadata host");
    };
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":")?;
    q(b, &mut n, if form.is_some() { b"POST" } else { b"GET" })?;
    bytes(b, &mut n, b",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b, &mut n, b",\"headers\":")?;
    headers(b, &mut n)?;
    if let Some(f) = form {
        bytes(
            b,
            &mut n,
            b",\"Content-Type\":\"application/x-www-form-urlencoded\"},\"bodyBase64\":",
        )?;
        q(b, &mut n, f)?
    } else {
        bytes(b, &mut n, b"}")?
    };
    bytes(
        b,
        &mut n,
        b",\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&b[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP capacity");
    };
    let env = &http_out()[..z];
    valid_json(env)?;
    let status = num(member(env, b"status")?
        .or(member(env, b"statusCode")?)
        .ok_or("status missing")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status");
    };
    let n = koma_source_sdk::decode_json_body_into(env, body_buf()).map_err(|_| "body decode")?;
    if n >= MAX {
        return Err("body capacity");
    };
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "body UTF8")?;
    Ok(n)
}
fn fetch(url: &[u8]) -> R<Owned> {
    let n = fetch_body(url, None)?;
    Ok(Owned(
        host::html_parse(&body_buf()[..n]).map_err(|_| "HTML parse")?,
    ))
}
fn work(r: &[u8]) -> R<&[u8]> {
    ident(
        text(args(r)?, b"mangaId")?
            .strip_prefix(b"ik:zh:")
            .ok_or("work namespace")?,
    )
}
fn cid(r: &[u8]) -> R<(&[u8], &[u8])> {
    let x = text(args(r)?, b"chapterId")?
        .strip_prefix(b"ikch:zh:")
        .ok_or("chapter namespace")?;
    let mut it = x.split(|c| *c == b':');
    let w = ident(it.next().ok_or("chapter identity")?)?;
    let c = ident(it.next().ok_or("chapter identity")?)?;
    if it.next().is_some() {
        return Err("chapter identity");
    };
    if let Some(v) = member(args(r)?, b"mangaId")? {
        let mut b = [0; 64];
        if uq(v, &mut b)?.strip_prefix(b"ik:zh:") != Some(w) {
            return Err("chapter owner");
        }
    };
    Ok((w, c))
}
fn image_url(v: &[u8]) -> R<()> {
    let path = v
        .strip_prefix(b"https://tuer.justpic01pt.com:666/picbed/")
        .ok_or("image host")?;
    if path.is_empty()
        || v.len() >= 1024
        || path.windows(2).any(|x| x == b"..")
        || !path
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || b"/-_.%".contains(c))
    {
        return Err("image path");
    };
    if path.contains(&b'%') {
        return Err("image escaped path");
    };
    Ok(())
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let a = args(r)?;
    let mut page = 1;
    if let Some(p) = member(a, b"page")? {
        if p != b"null" {
            if let Some(c) = member(p, b"cursor")? {
                if c != b"null" && quoted(c)? != b"" {
                    if search {
                        return Err("finite query cursor");
                    };
                    let mut tmp = [0; 64];
                    let cv = uq(c, &mut tmp)?;
                    page = number(cv.strip_prefix(b"ikcat:").ok_or("category cursor")?)
                        .ok_or("category page")?;
                    if page < 2 || page > 1024 {
                        return Err("category capacity");
                    }
                }
            }
        }
    };
    let mut u = [0; 8192];
    let mut n = 0;
    bytes(&mut u, &mut n, WEB)?;
    if search {
        let mut qb = [0; 2048];
        let query = uq(required(a, b"query")?, &mut qb)?;
        if trim_ascii(query).is_empty() {
            return Err("empty query");
        };
        bytes(&mut u, &mut n, b"/search?searchkey=")?;
        if !write_url_encoded(&mut u, &mut n, query) {
            return Err("query capacity");
        }
    } else {
        if text(a, b"listingId")? != b"listing:category" {
            return Err("listing identity");
        };
        bytes(&mut u, &mut n, b"/booklists/9/%E5%85%A8%E9%83%A8/3/")?;
        wn(&mut u, &mut n, page)?;
        bytes(&mut u, &mut n, b".html")?
    };
    let doc = fetch(&u[..n])?;
    let container =
        Owned(host::html_select(doc.0, b"#js_comicSortList").map_err(|_| "listing container")?);
    let cards = All::new(container.0, b"li.item.comic-item")?;
    if !search && cards.len > 48 {
        return Err("category count");
    };
    let out = payload_buf();
    let mut z = 0;
    bytes(out, &mut z, b"{\"items\":[")?;
    let mut seen = [0usize; 1000];
    for i in 0..cards.len {
        let d = cards.at(i);
        let mut hb = [0; 128];
        let href = sa(d, b"a[href]", b"href", &mut hb)?;
        let id = ident(
            href.strip_prefix(b"/book/")
                .and_then(|x| x.strip_suffix(b"/"))
                .ok_or("card href")?,
        )?;
        let ni = number(id).ok_or("card identity")?;
        if seen[..i].contains(&ni) {
            return Err("card duplicate");
        };
        seen[i] = ni;
        let mut tb = [0; 8192];
        let title = st(d, b"p.title", &mut tb)?;
        if title.is_empty() {
            return Err("card title");
        };
        let mut cb = [0; 1024];
        let cover = sa(d, b"img.img", b"src", &mut cb)?;
        image_url(cover)?;
        if i > 0 {
            bytes(out, &mut z, b",")?
        };
        bytes(out, &mut z, b"{\"id\":")?;
        let mut ib = [0; 64];
        q(out, &mut z, make(&[b"ik:zh:", id], &mut ib)?)?;
        bytes(out, &mut z, b",\"title\":")?;
        q(out, &mut z, title)?;
        bytes(out, &mut z, b",\"coverUrl\":")?;
        q(out, &mut z, cover)?;
        bytes(out, &mut z, b",\"contentRating\":\"unknown\"}")?
    }
    let more = !search && cards.len == 48;
    bytes(out, &mut z, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut z, if more { b"true" } else { b"false" })?;
    bytes(out, &mut z, b",\"nextCursor\":")?;
    if more {
        let next = page.checked_add(1).ok_or("page overflow")?;
        if next > 1024 {
            return Err("category capacity");
        };
        let mut c = [0; 64];
        let mut cn = 0;
        bytes(&mut c, &mut cn, b"ikcat:")?;
        wn(&mut c, &mut cn, next)?;
        q(out, &mut z, &c[..cn])?
    } else {
        bytes(out, &mut z, b"null")?
    };
    bytes(out, &mut z, b"}}")?;
    Ok(z)
}
fn doc(id: &[u8]) -> R<Owned> {
    let mut u = [0; 128];
    let d = fetch(make(&[WEB, b"/book/", id, b"/"], &mut u)?)?;
    let mut b = [0; 256];
    let actual = meta(d.0, b"og:url", &mut b)?;
    let mut expected = [0; 256];
    let secure = make(&[WEB, b"/book/", id, b"/"], &mut expected)?;
    let mut eb = [0; 256];
    let insecure = make(&[b"http://ymcdnyfqdapp.ikmmh.com/book/", id, b"/"], &mut eb)?;
    if actual != secure && actual != insecure {
        return Err("detail owner");
    };
    Ok(d)
}
fn meta<'a>(d: HtmlDescriptor, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut s = [0; 128];
    let selector = make(&[b"meta[property=\"", k, b"\"]"], &mut s)?;
    sa(d, selector, b"content", b)
}
fn array<'a>(s: &'a [u8]) -> R<Array<'a>> {
    if s.first() != Some(&b'[') || s.last() != Some(&b']') {
        return Err("expected array");
    };
    Ok(Array { s, p: 1 })
}
struct Array<'a> {
    s: &'a [u8],
    p: usize,
}
impl<'a> Array<'a> {
    fn next(&mut self) -> R<Option<&'a [u8]>> {
        self.p = ws(self.s, self.p);
        if self.s.get(self.p) == Some(&b']') {
            return Ok(None);
        };
        let end = json_value_end(self.s, self.p, 0).ok_or("array value")?;
        let v = &self.s[self.p..end];
        self.p = ws(self.s, end);
        if self.s.get(self.p) == Some(&b',') {
            self.p += 1
        } else if self.s.get(self.p) != Some(&b']') {
            return Err("array separator");
        };
        Ok(Some(v))
    }
}
fn catalogue(w: &[u8]) -> R<usize> {
    let mut u = [0; 512];
    let url = make(
        &[
            WEB,
            b"/api/comic/zyz/chapters?ph=1&tempid=3&zpid=",
            w,
            b"&page=0&line=48&orderby=desc",
        ],
        &mut u,
    )?;
    let n = fetch_body(url, None)?;
    valid_json(&body_buf()[..n])?;
    unsafe { DATA[..n].copy_from_slice(&body_buf()[..n]) };
    let s = unsafe { &DATA[..n] };
    if text(s, b"totalPage")? != b"1" {
        return Err("incomplete catalogue");
    };
    let mut it = array(required(s, b"length")?)?;
    let mut count = 0;
    let mut seen = [0usize; 4096];
    while let Some(v) = it.next()? {
        if count == 4096 {
            return Err("catalogue capacity");
        };
        let mut ub = [0; 128];
        let url = uq(required(v, b"url")?, &mut ub)?;
        let mut prefix = [0; 64];
        let p = make(&[b"/chapter/", w, b"/"], &mut prefix)?;
        let c = ident(
            url.strip_prefix(p)
                .and_then(|x| x.strip_suffix(b".html"))
                .ok_or("chapter owner")?,
        )?;
        let ci = number(c).ok_or("chapter identity")?;
        if seen[..count].contains(&ci) {
            return Err("chapter duplicate");
        };
        seen[count] = ci;
        let mut tb = [0; 8192];
        if uq(required(v, b"name")?, &mut tb)?.is_empty() {
            return Err("chapter title");
        };
        count += 1
    }
    if count == 0 {
        return Err("empty catalogue");
    };
    Ok(n)
}
fn member_chapter(w: &[u8], c: &[u8]) -> R<()> {
    let n = catalogue(w)?;
    let s = unsafe { &DATA[..n] };
    let mut it = array(required(s, b"length")?)?;
    let mut wanted = [0; 128];
    let expected = make(&[b"/chapter/", w, b"/", c, b".html"], &mut wanted)?;
    let mut found = false;
    while let Some(v) = it.next()? {
        let mut b = [0; 128];
        if uq(required(v, b"url")?, &mut b)? == expected {
            found = true
        }
    }
    if found {
        Ok(())
    } else {
        Err("chapter membership")
    }
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let w = work(r)?;
    if chapters {
        let n = catalogue(w)?;
        let s = unsafe { &DATA[..n] };
        let mut it = array(required(s, b"length")?)?;
        let out = payload_buf();
        let mut z = 0;
        bytes(out, &mut z, b"{\"chapters\":[")?;
        let mut i = 0;
        while let Some(v) = it.next()? {
            let mut ub = [0; 128];
            let href = uq(required(v, b"url")?, &mut ub)?;
            let c = ident(
                href.strip_suffix(b".html")
                    .ok_or("chapter URL")?
                    .rsplit(|x| *x == b'/')
                    .next()
                    .ok_or("chapter URL")?,
            )?;
            let mut tb = [0; 8192];
            let title = uq(required(v, b"name")?, &mut tb)?;
            if i > 0 {
                bytes(out, &mut z, b",")?
            };
            bytes(out, &mut z, b"{\"id\":")?;
            let mut ib = [0; 128];
            q(out, &mut z, make(&[b"ikch:zh:", w, b":", c], &mut ib)?)?;
            bytes(out, &mut z, b",\"title\":")?;
            q(out, &mut z, title)?;
            bytes(out, &mut z, b",\"chapterNumber\":null}")?;
            i += 1
        }
        bytes(out, &mut z, b"]}")?;
        Ok(z)
    } else {
        let d = doc(w)?;
        let mut tb = [0; 8192];
        let t = meta(d.0, b"og:title", &mut tb)?;
        if t.is_empty() {
            return Err("detail title");
        };
        let mut cb = [0; 1024];
        let cv = meta(d.0, b"og:image", &mut cb)?;
        image_url(cv)?;
        let out = payload_buf();
        let mut n = 0;
        bytes(out, &mut n, b"{\"manga\":{\"id\":")?;
        let mut ib = [0; 64];
        q(out, &mut n, make(&[b"ik:zh:", w], &mut ib)?)?;
        bytes(out, &mut n, b",\"title\":")?;
        q(out, &mut n, t)?;
        bytes(out, &mut n, b",\"coverUrl\":")?;
        q(out, &mut n, cv)?;
        let mut b = [0; 32768];
        bytes(out, &mut n, b",\"description\":")?;
        q(out, &mut n, meta(d.0, b"og:description", &mut b)?)?;
        let mut ab = [0; 4096];
        bytes(out, &mut n, b",\"author\":")?;
        q(out, &mut n, meta(d.0, b"og:cartoon:author", &mut ab)?)?;
        let mut sb = [0; 64];
        let status = meta(d.0, b"og:cartoon:status", &mut sb)?;
        bytes(out, &mut n, b",\"status\":")?;
        q(
            out,
            &mut n,
            if status == "连载".as_bytes() {
                b"ongoing"
            } else if status == "完结".as_bytes() {
                b"completed"
            } else {
                b"unknown"
            },
        )?;
        bytes(out, &mut n, b",\"contentRating\":\"unknown\"}}")?;
        Ok(n)
    }
}
fn batch(w: &[u8], c: &[u8], offset: usize) -> R<usize> {
    let mut form = [0; 256];
    let mut n = 0;
    bytes(&mut form, &mut n, b"id=")?;
    bytes(&mut form, &mut n, c)?;
    bytes(&mut form, &mut n, b"&aid=")?;
    bytes(&mut form, &mut n, w)?;
    bytes(&mut form, &mut n, b"&offset=")?;
    wn(&mut form, &mut n, offset)?;
    bytes(&mut form, &mut n, b"&limit=10")?;
    let z = fetch_body(
        b"https://ymcdnyfqdapp.ikmmh.com/api/comic/read/pics",
        Some(&form[..n]),
    )?;
    valid_json(&body_buf()[..z])?;
    let s = &body_buf()[..z];
    if num(required(s, b"code")?)? != 1 {
        return Err("page permission");
    };
    let d = required(s, b"data")?;
    let total = num(required(d, b"total")?)?;
    if total == 0
        || total > 1000
        || num(required(d, b"offset")?)? != offset
        || num(required(d, b"limit")?)? != 10
        || offset >= total
    {
        return Err("page metadata");
    };
    Ok(z)
}
fn pages(r: &[u8]) -> R<usize> {
    let (w, c) = cid(r)?;
    member_chapter(w, c)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    let mut offset = 0;
    let mut total = 0;
    let mut seen = [0usize; 1000];
    while offset == 0 || offset < total {
        let z = batch(w, c, offset)?;
        let s = &body_buf()[..z];
        let d = required(s, b"data")?;
        let t = num(required(d, b"total")?)?;
        if offset == 0 {
            total = t
        } else if total != t {
            return Err("page total drift");
        };
        let mut it = array(required(d, b"pic")?)?;
        let start = offset;
        while let Some(v) = it.next()? {
            if offset >= total || offset >= 1000 {
                return Err("page count");
            };
            let pid = num(required(v, b"id")?)?;
            if pid == 0 || seen[..offset].contains(&pid) {
                return Err("page identity duplicate");
            };
            seen[offset] = pid;
            let mut ub = [0; 1024];
            let url = uq(required(v, b"pic")?, &mut ub)?;
            image_url(url)?;
            unsafe {
                if PAGES[..offset]
                    .iter()
                    .zip(PAGE_LEN[..offset].iter())
                    .any(|(p, l)| &p[..*l] == url)
                {
                    return Err("page URL duplicate");
                };
                PAGES[offset][..url.len()].copy_from_slice(url);
                PAGE_LEN[offset] = url.len()
            };
            if offset > 0 {
                bytes(out, &mut n, b",")?
            };
            bytes(out, &mut n, b"{\"id\":")?;
            let mut b = [0; 128];
            let mut bn = 0;
            bytes(&mut b, &mut bn, b"ikpage:zh:")?;
            bytes(&mut b, &mut bn, w)?;
            bytes(&mut b, &mut bn, b":")?;
            bytes(&mut b, &mut bn, c)?;
            bytes(&mut b, &mut bn, b":")?;
            wn(&mut b, &mut bn, pid)?;
            bytes(&mut b, &mut bn, b":")?;
            wn(&mut b, &mut bn, offset)?;
            q(out, &mut n, &b[..bn])?;
            bytes(out, &mut n, b",\"index\":")?;
            wn(out, &mut n, offset)?;
            bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(out, &mut n, url)?;
            bytes(out, &mut n, b"}}")?;
            offset += 1
        }
        if offset - start != core::cmp::min(10, total - start) {
            return Err("incomplete page batch");
        }
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let id = text(a, b"pageId")?;
    let mut ub = [0; 1024];
    let url = uq(required(a, b"url")?, &mut ub)?;
    image_url(url)?;
    if let Some(w) = id.strip_prefix(b"ik:zh:") {
        let d = doc(ident(w)?)?;
        let mut b = [0; 1024];
        if meta(d.0, b"og:image", &mut b)? != url {
            return Err("cover identity");
        }
    } else {
        let x = id.strip_prefix(b"ikpage:zh:").ok_or("page namespace")?;
        let mut it = x.split(|c| *c == b':');
        let w = ident(it.next().ok_or("page identity")?)?;
        let c = ident(it.next().ok_or("page identity")?)?;
        let pid = number(it.next().ok_or("page identity")?).ok_or("page identity")?;
        let index = number(it.next().ok_or("page identity")?).ok_or("page index")?;
        if it.next().is_some() || index >= 1000 {
            return Err("page capacity");
        };
        member_chapter(w, c)?;
        let offset = index / 10 * 10;
        let z = batch(w, c, offset)?;
        let s = &body_buf()[..z];
        let d = required(s, b"data")?;
        let total = num(required(d, b"total")?)?;
        if index >= total {
            return Err("page index");
        };
        let mut a = array(required(d, b"pic")?)?;
        let mut i = offset;
        let mut matched = false;
        let mut seen = [0usize; 10];
        let mut urls = [[0u8; 1024]; 10];
        let mut lengths = [0usize; 10];
        while let Some(v) = a.next()? {
            if i - offset >= 10 {
                return Err("batch capacity");
            };
            let p = num(required(v, b"id")?)?;
            if p == 0 || seen[..i - offset].contains(&p) {
                return Err("batch duplicate");
            };
            seen[i - offset] = p;
            let mut b = [0; 1024];
            let u = uq(required(v, b"pic")?, &mut b)?;
            image_url(u)?;
            if urls[..i - offset]
                .iter()
                .zip(lengths[..i - offset].iter())
                .any(|(b, n)| &b[..*n] == u)
            {
                return Err("batch URL duplicate");
            };
            urls[i - offset][..u.len()].copy_from_slice(u);
            lengths[i - offset] = u.len();
            if i == index {
                if p != pid || u != url {
                    return Err("image ownership");
                };
                matched = true
            };
            i += 1
        }
        if !matched || i - offset != core::cmp::min(10, total - offset) {
            return Err("incomplete batch");
        }
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out, &mut n, b",\"headers\":")?;
    headers(out, &mut n)?;
    bytes(out, &mut n, b"}}")?;
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
    execute("get_manga", detail(r, false))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", detail(r, true))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let s="{\"listings\":[{\"id\":\"listing:category\",\"name\":\"分类\",\"kind\":\"popular\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
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
    response_buffer().write_success_parts("source_info", &[
 b"{\"sourceInfo\":{\"id\":\"", SOURCE_INFO.id.as_bytes(), b"\",\"name\":\"", SOURCE_INFO.name.as_bytes(), b"\",\"version\":\"", SOURCE_INFO.version.as_bytes(), b"\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"",SOURCE_INFO.description.as_bytes(),
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"爱看漫\",\"zh-Hant\":\"愛看漫\",\"en\":\"Ikmmh (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
 ])
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => $run(r),
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
