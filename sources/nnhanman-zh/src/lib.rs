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
const WEB: &[u8] = b"https://nnhanman.xyz";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.nnhanman.zh.koma",name:"NNHanman",version:"0.1.3",api_version:"0.2",language:"zh",author:"Koma",description:"NNHanman published Chinese ranking and search metadata. Per-work rating, artwork availability and usage unknown.",content_rating:"nsfw"};
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
    filters: true,
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
static mut SEEN: [[u8; 256]; 1000] = [[0; 256]; 1000];
const UA: &[u8] =
    b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://nnhanman.xyz/") {
        return Err("metadata host");
    }
    let d = http_req_buf();
    let mut n = 0;
    bytes(d, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(d, &mut n, url)?;
    bytes(d, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(d, &mut n, UA)?;
    bytes(d,&mut n,b",\"Referer\":\"https://nnhanman.xyz/\",\"Origin\":\"https://nnhanman.xyz\"},\"responseKind\":\"bodyText\",\"timeoutMs\":15000}")?;
    let z = host::http_request(&d[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP envelope capacity");
    }
    let env = &http_out()[..z];
    valid_json(env)?;
    let status = num(member(env, b"status")?
        .or(member(env, b"statusCode")?)
        .ok_or("HTTP status missing")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status error");
    }
    let n =
        koma_source_sdk::decode_json_body_into(env, body_buf()).map_err(|_| "HTTP body decode")?;
    if n >= MAX {
        return Err("HTML body capacity");
    }
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "HTML UTF8")?;
    Ok(Owned(
        host::html_parse(&body_buf()[..n]).map_err(|_| "HTML parse")?,
    ))
}
fn make<'a>(parts: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for x in parts {
        bytes(b, &mut n, x)?
    }
    Ok(&b[..n])
}
fn child(d: HtmlDescriptor, s: &[u8]) -> R<Owned> {
    Ok(Owned(
        host::html_select(d, s).map_err(|_| "missing HTML field")?,
    ))
}
fn slug(v: &[u8]) -> R<&[u8]> {
    if v.is_empty()
        || v.len() > 256
        || v.iter()
            .any(|c| !c.is_ascii_alphanumeric() && *c != b'-' && *c != b'_')
    {
        Err("work identity")
    } else {
        Ok(v)
    }
}
fn cid(v: &[u8]) -> R<&[u8]> {
    if v.is_empty() || v.len() > 15 || v[0] == b'0' || number(v).is_none() {
        Err("chapter identity")
    } else {
        Ok(v)
    }
}
fn work_href(v: &[u8]) -> R<&[u8]> {
    slug(
        v.strip_prefix(WEB)
            .unwrap_or(v)
            .strip_prefix(b"/comic/")
            .and_then(|x| x.strip_suffix(b".html"))
            .ok_or("work href")?,
    )
}
fn chapter_href<'a>(v: &'a [u8], w: &[u8]) -> R<&'a [u8]> {
    let mut b = [0; 512];
    let prefix = make(&[b"/comic/", w, b"/chapter-"], &mut b)?;
    cid(v
        .strip_prefix(WEB)
        .unwrap_or(v)
        .strip_prefix(prefix)
        .and_then(|x| x.strip_suffix(b".html"))
        .ok_or("chapter owner")?)
}
fn work(r: &[u8]) -> R<&[u8]> {
    slug(
        text(args(r)?, b"mangaId")?
            .strip_prefix(b"nn:zh:")
            .ok_or("work namespace")?,
    )
}
fn chapter(r: &[u8]) -> R<(&[u8], &[u8])> {
    let v = text(args(r)?, b"chapterId")?
        .strip_prefix(b"nnch:zh:")
        .ok_or("chapter namespace")?;
    let p = v.iter().position(|x| *x == b':').ok_or("chapter parts")?;
    Ok((slug(&v[..p])?, cid(&v[p + 1..])?))
}
fn safe_path(v: &[u8]) -> R<()> {
    if v.is_empty()
        || v.len() > 1000
        || v.windows(2).any(|w| w == b"..")
        || v.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'/' | b'.' | b'-' | b'_'))
    {
        Err("image path")
    } else {
        Ok(())
    }
}
fn image_url(v: &[u8], cover: bool) -> R<()> {
    let p = if cover {
        v.strip_prefix(b"https://thumb.niaopic.com/")
            .or(v.strip_prefix(b"https://new.niaopic.com/"))
    } else {
        v.strip_prefix(b"https://img.nnpic.xyz/")
    }
    .ok_or("image host")?;
    safe_path(p)
}
fn nonempty<'a>(v: &'a [u8]) -> R<&'a [u8]> {
    core::str::from_utf8(v).map_err(|_| "text UTF8")?;
    if trim_ascii(v).is_empty() {
        Err("empty text")
    } else {
        Ok(v)
    }
}
fn card(
    out: &mut [u8],
    n: &mut usize,
    d: HtmlDescriptor,
    rank: bool,
    idbuf: &mut [u8],
) -> R<usize> {
    let a = child(d, if rank { b"a.title" } else { b"a.ImgA" })?;
    let mut h = [0; 1024];
    let w = work_href(attr(a.0, b"href", &mut h)?)?;
    idbuf[..w.len()].copy_from_slice(w);
    let mut t = [0; 4096];
    let mut tb = [0; 4096];
    let av = attr(a.0, b"title", &mut t)?;
    let title = if av.is_empty() {
        nonempty(txt(a.0, &mut tb)?)?
    } else {
        nonempty(av)?
    };
    let mut cb = [0; 1024];
    let cv = if !rank {
        let ss = All::new(d, b"source[srcset]")?;
        if ss.len > 1 {
            return Err("ambiguous cover");
        };
        if ss.len == 1 {
            attr(ss.at(0), b"srcset", &mut cb)?
        } else {
            sa(d, b"img[src]", b"src", &mut cb)?
        }
    } else {
        sa(d, b"img[src]", b"src", &mut cb)?
    };
    image_url(cv, true)?;
    bytes(out, n, b"{\"id\":\"nn:zh:")?;
    bytes(out, n, w)?;
    bytes(out, n, b"\",\"title\":")?;
    q(out, n, title)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, cv)?;
    bytes(out, n, b"},\"contentRating\":\"unknown\"}")?;
    Ok(w.len())
}
fn cursor(r: &[u8], b: &mut [u8]) -> R<usize> {
    if let Some(p) = member(args(r)?, b"page")? {
        if p != b"null" {
            if let Some(c) = member(p, b"cursor")? {
                if c != b"null" {
                    return Ok(uq(c, b)?.len());
                }
            }
        }
    }
    Ok(0)
}
fn query_path<'a>(query: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    bytes(b, &mut n, b"/search/")?;
    if !write_url_encoded(b, &mut n, query) {
        return Err("query capacity");
    };
    Ok(&b[..n])
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let mut cb = [0; 8192];
    let cl = cursor(r, &mut cb)?;
    let mut query = [0; 2048];
    let mut path = [0; 8192];
    let mut url = [0; 16384];
    let mut pageno = 1;
    let mut qlen = 0;
    let path = if search {
        let q = trim_ascii(uq(required(args(r)?, b"query")?, &mut query)?);
        nonempty(q)?;
        if q.iter().any(|x| *x < 32) {
            return Err("query control");
        };
        qlen = q.len();
        let offset = q.as_ptr() as usize - query.as_ptr() as usize;
        query.copy_within(offset..offset + qlen, 0);
        let base = query_path(&query[..qlen], &mut path)?;
        if cl == 0 {
            base
        } else {
            let tail = cb[..cl]
                .strip_prefix(base)
                .and_then(|x| x.strip_prefix(b"/page/"))
                .ok_or("cursor query owner")?;
            pageno = number(tail).ok_or("cursor page")?;
            if pageno < 2 || pageno > 1024 {
                return Err("cursor page capacity");
            };
            if cb[..cl].iter().any(|x| *x == b'"' || *x == b'\\') {
                return Err("cursor path");
            };
            &cb[..cl]
        }
    } else {
        if cl != 0 {
            return Err("finite ranking cursor");
        };
        if text(args(r)?, b"listingId")? != b"listing:popular" {
            return Err("listing identity");
        };
        b"/ranking"
    };
    let doc = fetch(make(&[WEB, path], &mut url)?)?;
    let cards = All::new(
        doc.0,
        if search {
            b"ul.col_3_1 > li"
        } else {
            b"div.itemBox"
        },
    )?;
    if !search && cards.len == 0 {
        return Err("empty or gated ranking");
    }
    let mut nextbuf = [0; 8192];
    let mut nextlen = 0;
    if search {
        // The site's SEO description reports a page total even for a one-page
        // result. Like the reference extension, follow actual pagination links.
        // Keep the current query as cursor owner and require the list container,
        // so an unrelated/gated HTML page cannot become an empty success.
        let _search_list = child(doc.0, b"ul.col_3_1")?;
        let mut base = [0; 8192];
        let mut wanted = [0; 8192];
        let mut pn = [0; 32];
        let mut pl = 0;
        wn(&mut pn, &mut pl, pageno.checked_add(1).ok_or("page overflow")?)?;
        let want = make(
            &[query_path(&query[..qlen], &mut base)?, b"/page/", &pn[..pl]],
            &mut wanted,
        )?;
        // The HTML emits literal Chinese path text; requests/cursors keep URL encoding.
        let mut raw_wanted = [0; 8192];
        let raw_want = make(&[b"/search/", &query[..qlen], b"/page/", &pn[..pl]], &mut raw_wanted)?;
        let links = All::new(doc.0, b"div.pagination-wrap a[href]")?;
        for i in 0..links.len {
            let mut h = [0; 8192];
            let href = attr(links.at(i), b"href", &mut h)?;
            let rel = href.strip_prefix(WEB).unwrap_or(href);
            if rel == want || rel == raw_want {
                nextbuf[..want.len()].copy_from_slice(want);
                nextlen = want.len();
            }
        }
        if cards.len == 0 && nextlen > 0 {
            return Err("empty nonterminal search");
        }
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let seen = unsafe { &mut *core::ptr::addr_of_mut!(SEEN) };
    let mut lens = [0usize; 1000];
    for i in 0..cards.len {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        lens[i] = card(out, &mut n, cards.at(i), !search, &mut seen[i])?;
        for j in 0..i {
            if lens[j] == lens[i] && seen[j][..lens[j]] == seen[i][..lens[i]] {
                return Err("duplicate work");
            }
        }
    }
    bytes(out, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if nextlen > 0 {
        q(out, &mut n, &nextbuf[..nextlen])?
    } else {
        bytes(out, &mut n, b"null")?
    };
    bytes(
        out,
        &mut n,
        if nextlen > 0 {
            b",\"hasMore\":true}}"
        } else {
            b",\"hasMore\":false}}"
        },
    )?;
    Ok(n)
}
fn detail_doc(w: &[u8]) -> R<Owned> {
    let mut u = [0; 1024];
    fetch(make(&[WEB, b"/comic/", w, b".html"], &mut u)?)
}
fn collect(doc: HtmlDescriptor, w: &[u8]) -> R<All> {
    let a = All::new(doc, b"#mh-chapter-list-ol-0 li a")?;
    if a.len == 0 {
        return Err("empty or gated catalogue");
    };
    let mut ids = [0usize; 1000];
    for i in 0..a.len {
        let mut h = [0; 1024];
        let id = chapter_href(attr(a.at(i), b"href", &mut h)?, w)?;
        let x = number(id).ok_or("chapter identity")?;
        if ids[..i].contains(&x) {
            return Err("duplicate chapter");
        };
        ids[i] = x
    }
    Ok(a)
}
fn chapter_number(t: &[u8]) -> Option<&[u8]> {
    let v = t.strip_prefix("第".as_bytes())?;
    let end = v.iter().position(|c| !c.is_ascii_digit() && *c != b'.')?;
    let x = &v[..end];
    if x.is_empty()
        || x[0] == b'.'
        || x[x.len() - 1] == b'.'
        || x.iter().filter(|c| **c == b'.').count() > 1
    {
        return None;
    };
    if !v[end..].starts_with("話".as_bytes()) && !v[end..].starts_with("话".as_bytes()) {
        return None;
    };
    if x.len() > 1 && x[0] == b'0' && x[1].is_ascii_digit() {
        return None;
    };
    Some(x)
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let w = work(r)?;
    let doc = detail_doc(w)?;
    let a = collect(doc.0, w)?;
    let out = payload_buf();
    let mut n = 0;
    if chapters {
        bytes(out, &mut n, b"{\"chapters\":[")?;
        for k in 0..a.len {
            let i = a.len - 1 - k;
            let mut h = [0; 1024];
            let c = chapter_href(attr(a.at(i), b"href", &mut h)?, w)?;
            let mut tb = [0; 4096];
            let t = nonempty(txt(a.at(i), &mut tb)?)?;
            if k > 0 {
                bytes(out, &mut n, b",")?
            };
            bytes(out, &mut n, b"{\"id\":\"nnch:zh:")?;
            bytes(out, &mut n, w)?;
            bytes(out, &mut n, b":")?;
            bytes(out, &mut n, c)?;
            bytes(out, &mut n, b"\",\"mangaId\":\"nn:zh:")?;
            bytes(out, &mut n, w)?;
            bytes(out, &mut n, b"\",\"title\":")?;
            q(out, &mut n, t)?;
            bytes(out, &mut n, b",\"chapterNumber\":")?;
            bytes(out, &mut n, chapter_number(t).unwrap_or(b"null"))?;
            bytes(out, &mut n, b",\"volumeNumber\":null,\"language\":\"zh\"}")?
        }
        bytes(out, &mut n, b"]}")?;
        return Ok(n);
    }
    let mut tb = [0; 4096];
    let raw = st(doc.0, b"h1", &mut tb)?;
    let t = nonempty(
        raw.strip_prefix("《".as_bytes())
            .and_then(|x| x.strip_suffix("》".as_bytes()))
            .ok_or("detail title")?,
    )?;
    let mut cvb = [0; 1024];
    let cv = sa(doc.0, b"div.pic img", b"src", &mut cvb)?;
    image_url(cv, true)?;
    bytes(out, &mut n, b"{\"manga\":{\"id\":\"nn:zh:")?;
    bytes(out, &mut n, w)?;
    bytes(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, t)?;
    bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, &mut n, cv)?;
    bytes(
        out,
        &mut n,
        b"},\"contentRating\":\"unknown\",\"description\":",
    )?;
    let mut db = [0; 32768];
    let d = st(doc.0, b"p.txtDesc", &mut db)?;
    q(
        out,
        &mut n,
        trim_ascii(d.strip_prefix("介绍:".as_bytes()).unwrap_or(d)),
    )?;
    bytes(out, &mut n, b",\"authors\":[")?;
    let ps = All::new(doc.0, b"div.sub_r > p.txtItme")?;
    let mut ac = 0;
    for i in 0..ps.len {
        let gs = All::new(ps.at(i), b"a[href^='/comics/']")?;
        let ds = All::new(ps.at(i), b"span.date")?;
        if gs.len == 0 && ds.len == 0 {
            let mut b = [0; 4096];
            let v = txt(ps.at(i), &mut b)?;
            if !v.is_empty() {
                if ac > 0 {
                    return Err("ambiguous author");
                };
                q(out, &mut n, v)?;
                ac += 1
            }
        }
    }
    bytes(out, &mut n, b"],\"tags\":[")?;
    let gs = All::new(doc.0, b"p.txtItme a[href^='/comics/']")?;
    for i in 0..gs.len {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        let mut b = [0; 1024];
        q(out, &mut n, nonempty(txt(gs.at(i), &mut b)?)?)?
    }
    let mut sb = [0; 1024];
    let s = st(doc.0, b"span.date", &mut sb)?;
    let status = if contains_bytes(s, "连载中".as_bytes()) {
        b"ongoing".as_slice()
    } else if contains_bytes(s, "已完结".as_bytes()) {
        b"completed".as_slice()
    } else {
        b"unknown".as_slice()
    };
    bytes(out, &mut n, b"],\"status\":")?;
    q(out, &mut n, status)?;
    bytes(
        out,
        &mut n,
        b",\"links\":[{\"kind\":\"source\",\"url\":\"https://nnhanman.xyz/comic/",
    )?;
    bytes(out, &mut n, w)?;
    bytes(out, &mut n, b".html\"}]}}")?;
    Ok(n)
}
fn chapter_doc(w: &[u8], c: &[u8]) -> R<Owned> {
    let d = detail_doc(w)?;
    let a = collect(d.0, w)?;
    let mut found = false;
    for i in 0..a.len {
        let mut h = [0; 1024];
        if chapter_href(attr(a.at(i), b"href", &mut h)?, w)? == c {
            found = true
        }
    }
    if !found {
        return Err("chapter absent");
    };
    let mut u = [0; 1024];
    fetch(make(
        &[WEB, b"/comic/", w, b"/chapter-", c, b".html"],
        &mut u,
    )?)
}
fn page_collect(d: HtmlDescriptor) -> R<usize> {
    let a = All::new(d, b"#m_r_imgbox_0 img[data-src]")?;
    if a.len == 0 {
        return Err("empty or gated pages");
    };
    for i in 0..a.len {
        let mut b = [0; 32];
        let index = number(attr(a.at(i), b"data-index", &mut b)?).ok_or("page index")?;
        if index != i {
            return Err("page sequence");
        };
        let mut u = [0; 1024];
        let v = attr(a.at(i), b"data-src", &mut u)?;
        image_url(v, false)?;
        unsafe {
            for j in 0..i {
                if &PAGES[j][..PAGE_LEN[j]] == v {
                    return Err("duplicate image");
                }
            }
            PAGES[i][..v.len()].copy_from_slice(v);
            PAGE_LEN[i] = v.len()
        }
    }
    Ok(a.len)
}
fn pages(r: &[u8]) -> R<usize> {
    let (w, c) = chapter(r)?;
    let d = chapter_doc(w, c)?;
    let count = page_collect(d.0)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"nnpg:zh:")?;
        bytes(out, &mut n, w)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, c)?;
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
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut pb = [0; 1024];
    let p = uq(required(a, b"pageId")?, &mut pb)?;
    let mut ub = [0; 1024];
    let url = uq(required(a, b"url")?, &mut ub)?;
    if let Some(w) = p.strip_prefix(b"nn:zh:") {
        let w = slug(w)?;
        let d = detail_doc(w)?;
        image_url(url, true)?;
        let images = All::new(d.0, b"div.pic img[src]")?;
        let sources = All::new(d.0, b"div.pic source[srcset]")?;
        if images.len != 1 || sources.len > 1 {
            return Err("ambiguous detail cover");
        }
        let mut b = [0; 1024];
        let declared = attr(images.at(0), b"src", &mut b)?;
        image_url(declared, true)?;
        let mut matched = declared == url;
        if sources.len == 1 {
            let mut source_buf = [0; 1024];
            let declared = attr(sources.at(0), b"srcset", &mut source_buf)?;
            image_url(declared, true)?;
            matched |= declared == url;
        }
        if !matched {
            return Err("cover owner");
        }
    } else {
        let p = p.strip_prefix(b"nnpg:zh:").ok_or("page namespace")?;
        let z = p.iter().position(|x| *x == b':').ok_or("page parts")?;
        let w = slug(&p[..z])?;
        let p = &p[z + 1..];
        let z = p.iter().position(|x| *x == b':').ok_or("page parts")?;
        let c = cid(&p[..z])?;
        let i = number(&p[z + 1..]).ok_or("page index")?;
        let d = chapter_doc(w, c)?;
        let count = page_collect(d.0)?;
        if i >= count {
            return Err("page range");
        };
        unsafe {
            if &PAGES[i][..PAGE_LEN[i]] != url {
                return Err("page URL owner");
            }
        }
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(
        out,
        &mut n,
        b",\"Referer\":\"https://nnhanman.xyz/\",\"Origin\":\"https://nnhanman.xyz\"}}",
    )?;
    Ok(n)
}
fn execute(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_error", e),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", list(r, true))
}
// Ranking remains the default; only a real selection uses the site's category route.
fn filtered_list(r: &[u8]) -> R<Option<usize>> {
    let a=args(r)?;
    if text(a,b"listingId")? != b"listing:popular" {return Err("listing identity")}
    let Some(f)=member(a,b"filters")? else {return Ok(None)};
    if f==b"null" {return Ok(None)}
    let mut gb=[0;256];let mut ob=[0;32];let mut sb=[0;32];
    let genre=match member(f,b"genre")? {Some(v)=>uq(v,&mut gb)?,None=>b"all"};
    let order=match member(f,b"order")? {Some(v)=>uq(v,&mut ob)?,None=>b"time"};
    let status=match member(f,b"status")? {Some(v)=>uq(v,&mut sb)?,None=>b"all"};
    let allowed=["all","正妹","恋爱","出版漫画","肉慾","浪漫","大尺度","巨乳","有夫之婦","女大生","狗血劇","同居","好友","調教","动作","後宮","不倫","3D","校園","耽美","日漫"];
    if !allowed.iter().any(|v|v.as_bytes()==genre) || !matches!(order,b"time"|b"hits") || !matches!(status,b"all"|b"completed"|b"serialized") {return Err("invalid filters")}
    let mut cb=[0;8192];let clen=cursor(r,&mut cb)?;
    // No schema defaults: absence preserves the existing ranking, while an
    // explicit All/Time selection must open the provider category.
    if member(f,b"genre")?.is_none() && member(f,b"order")?.is_none() && member(f,b"status")?.is_none() && clen==0 {return Ok(None)}
    let mut base=[0;8192];let mut n=0;bytes(&mut base,&mut n,b"/comics/")?;
    if !write_url_encoded(&mut base,&mut n,genre){return Err("genre overflow")}
    bytes(&mut base,&mut n,b"/ob/")?;bytes(&mut base,&mut n,order)?;bytes(&mut base,&mut n,b"/st/")?;bytes(&mut base,&mut n,status)?;
    let base=&base[..n];let page=if clen==0 {1} else {let tail=cb[..clen].strip_prefix(base).and_then(|v|v.strip_prefix(b"/page/")).ok_or("cursor filter owner")?;let p=number(tail).ok_or("cursor page")?;if p<2||p>1024{return Err("cursor page capacity")}p};
    let mut path=[0;8192];let mut pn=0;bytes(&mut path,&mut pn,base)?;if page>1 {bytes(&mut path,&mut pn,b"/page/")?;wn(&mut path,&mut pn,page)?}
    let mut url=[0;16384];let doc=fetch(make(&[WEB,&path[..pn]],&mut url)?)?;
    let _category_list=child(doc.0,b"ul.col_3_1")?;
    let cards=All::new(doc.0,b"ul.col_3_1 > li")?;
    let mut want=[0;8192];let mut wnlen=0;bytes(&mut want,&mut wnlen,base)?;bytes(&mut want,&mut wnlen,b"/page/")?;wn(&mut want,&mut wnlen,page.checked_add(1).ok_or("page overflow")?)?;
    let links=All::new(doc.0,b"div.pagination-wrap a[href]")?;let mut next=false;
    for i in 0..links.len {let mut h=[0;8192];let href=attr(links.at(i),b"href",&mut h)?;let rel=href.strip_prefix(WEB).unwrap_or(href);if rel==&want[..wnlen]{next=true}}
    if cards.len==0 && (next || page>1) {return Err("empty paged category")}
    let out=payload_buf();let mut n=0;bytes(out,&mut n,b"{\"items\":[")?;let seen=unsafe{&mut *core::ptr::addr_of_mut!(SEEN)};let mut lens=[0usize;1000];
    for i in 0..cards.len {if i>0{bytes(out,&mut n,b",")?}lens[i]=card(out,&mut n,cards.at(i),false,&mut seen[i])?;for j in 0..i{if lens[j]==lens[i]&&seen[j][..lens[j]]==seen[i][..lens[i]]{return Err("duplicate work")}}}
    bytes(out,&mut n,b"],\"page\":{\"nextCursor\":")?;if next {q(out,&mut n,&want[..wnlen])?}else{bytes(out,&mut n,b"null")?};bytes(out,&mut n,if next{b",\"hasMore\":true}}"}else{b",\"hasMore\":false}}"})?;Ok(Some(n))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    match filtered_list(r) {Ok(Some(n))=>write_success_payload("get_manga_list",n),Ok(None)=>execute("get_manga_list",list(r,false)),Err(e)=>write_error("get_manga_list","source_error",e)}
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
    let p = payload_buf();
    let v =
        "{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"热门\",\"supportsPaging\":false}]}"
            .as_bytes();
    p[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let supported=member(r,b"hostFeatures").ok().flatten().map(|v| { let v=trim_ascii(v); v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c|*c==b',').any(|x|trim_ascii(x)==b"\"source_filter_scope_header_v1\"") }).unwrap_or(false);
    let v=if supported {r#"{"filters":[{"id":"note","type":"header","name":"分类浏览（搜索时无效）","scope":"browse"},{"id":"genre","name":"分类","type":"select","scope":"browse","options":[{"value":"all","label":"全部"},{"value":"正妹","label":"正妹"},{"value":"恋爱","label":"恋爱"},{"value":"出版漫画","label":"出版漫画"},{"value":"肉慾","label":"肉慾"},{"value":"浪漫","label":"浪漫"},{"value":"大尺度","label":"大尺度"},{"value":"巨乳","label":"巨乳"},{"value":"有夫之婦","label":"有夫之婦"},{"value":"女大生","label":"女大生"},{"value":"狗血劇","label":"狗血劇"},{"value":"同居","label":"同居"},{"value":"好友","label":"好友"},{"value":"調教","label":"調教"},{"value":"动作","label":"动作"},{"value":"後宮","label":"後宮"},{"value":"不倫","label":"不倫"},{"value":"3D","label":"3D"},{"value":"校園","label":"校園"},{"value":"耽美","label":"耽美"},{"value":"日漫","label":"日漫"}]},{"id":"order","name":"排序","type":"select","scope":"browse","options":[{"value":"time","label":"按时间"},{"value":"hits","label":"按热度"}]},{"id":"status","name":"状态","type":"select","scope":"browse","options":[{"value":"all","label":"全部"},{"value":"completed","label":"已完结"},{"value":"serialized","label":"连载中"}]}]}"#.as_bytes()} else {b"{\"filters\":[]}".as_slice()};
    write_success_payload("get_filters", {let out=payload_buf();out[..v.len()].copy_from_slice(v);v.len()})
}
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
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"鸟鸟韩漫\",\"zh-Hant\":\"鳥鳥韓漫\",\"en\":\"NNHanman (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":true,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
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
