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
const WEB: &[u8] = b"https://www.jjmhw2.top";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.mangaxiaosi.zh.koma",name:"Manga Xiao Si",version:"0.1.4",api_version:"0.2",language:"zh",author:"Koma",description:"Manga Xiao Si published finite rank and query catalogue. Per-work rating, picture availability and usage unknown.",content_rating:"nsfw"};
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
const UA: &[u8] = b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://www.jjmhw2.top/") {
        return Err("metadata host");
    }
    let dst = http_req_buf();
    let mut n = 0;
    bytes(dst, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(dst, &mut n, url)?;
    bytes(dst, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(dst, &mut n, UA)?;
    bytes(
        dst,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&dst[..n], http_out()).map_err(|_| "transport error")?;
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
fn child(d: HtmlDescriptor, s: &[u8]) -> R<Owned> {
    Ok(Owned(
        host::html_select(d, s).map_err(|_| "missing HTML field")?,
    ))
}
fn abs<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if v.starts_with(b"//") {
        bytes(b, &mut n, b"https:")?
    } else if v.starts_with(b"/") {
        bytes(b, &mut n, WEB)?
    } else if !v.starts_with(b"https://") {
        return Err("URL scheme");
    }
    bytes(b, &mut n, v)?;
    Ok(&b[..n])
}
fn href_id<'a>(v: &'a [u8], kind: &[u8]) -> R<&'a [u8]> {
    ident(
        v.strip_prefix(WEB)
            .unwrap_or(v)
            .strip_prefix(kind)
            .ok_or("href identity")?,
    )
}
fn work(r: &[u8]) -> R<&[u8]> {
    ident(
        text(args(r)?, b"mangaId")?
            .strip_prefix(b"mx:zh:")
            .ok_or("work namespace")?,
    )
}
fn image_path(v: &[u8]) -> R<&[u8]> {
    let p = v
        .strip_prefix(b"https://www.jjmhw9.top/static/upload/book/")
        .or_else(|| v.strip_prefix(b"https://www.jjmhw10.top/static/upload/book/"))
        .ok_or("image host")?;
    if p.is_empty()
        || p.len() > 900
        || p.windows(2).any(|w| w == b"..")
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'/' | b'.' | b'_' | b'-'))
    {
        return Err("image path");
    };
    Ok(p)
}
fn cover(v: &[u8], id: &[u8]) -> R<()> {
    let p = match v.strip_prefix(b"https://www.jjmhw11.top/static/upload/book/") { Some(p) => p, None => image_path(v)? };
    let mut b = [0; 128];
    if p != make(&[id, b"/cover.jpg"], &mut b)? {
        return Err("cover ownership");
    }
    Ok(())
}
fn title<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = st(d, s, b)?;
    if x.is_empty() {
        Err("empty title")
    } else {
        core::str::from_utf8(x).map_err(|_| "title UTF8")?;
        Ok(x)
    }
}
fn emit_card(out: &mut [u8], n: &mut usize, d: HtmlDescriptor, rank: bool) -> R<usize> {
    let mut hb = [0; 256];
    let id = href_id(
        sa(
            d,
            if rank { b"h2.title a" } else { b".title a" },
            b"href",
            &mut hb,
        )?,
        b"/book/",
    )?;
    let mut tb = [0; 4096];
    let t = title(d, if rank { b"h2.title a" } else { b".title a" }, &mut tb)?;
    let mut sb = [0; 2048];
    let style = sa(d, b".mh-cover", b"style", &mut sb)?;
    let pos = style
        .windows(4)
        .position(|x| x == b"url(")
        .ok_or("cover style")?
        + 4;
    let end = style[pos..]
        .iter()
        .position(|x| *x == b')')
        .ok_or("cover style")?
        + pos;
    let raw = trim_ascii(&style[pos..end]);
    let raw = if raw.len() >= 2
        && ((raw[0] == b'\'' && raw[raw.len() - 1] == b'\'')
            || (raw[0] == b'"' && raw[raw.len() - 1] == b'"'))
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    let mut cb = [0; 1024];
    let cv = abs(raw, &mut cb)?;
    cover(cv, id)?;
    bytes(out, n, b"{\"id\":\"mx:zh:")?;
    bytes(out, n, id)?;
    bytes(out, n, b"\",\"title\":")?;
    q(out, n, t)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, cv)?;
    bytes(out, n, b"},\"contentRating\":\"unknown\"}")?;
    number(id).ok_or("identity")
}
fn first_page(r: &[u8]) -> R<()> {
    if let Some(p) = member(args(r)?, b"page")? {
        if p != b"null" {
            if let Some(c) = member(p, b"cursor")? {
                if c != b"null" && quoted(c)? != b"" {
                    return Err("finite listing cursor");
                }
            }
        }
    }
    Ok(())
}
// Filters only apply to explicit browse choices; default rank and search remain finite.
fn filter_feature(r: &[u8]) -> bool {
    member(r,b"hostFeatures").ok().flatten().map(trim_ascii).map_or(false, |v| v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|b|*b==b',').any(|v| trim_ascii(v)==b"\"source_filter_scope_header_v1\""))
}
fn selected<'a>(f: &'a [u8], k: &[u8], out: &'a mut [u8], fallback: &'a [u8]) -> R<&'a [u8]> {
    match member(f,k)? { Some(v)=>uq(v,out), None=>Ok(fallback) }
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let a=args(r)?;
    let f = if !search && filter_feature(r) {member(a,b"filters")?.unwrap_or(b"{}")} else {b"{}"};
    let filtered = member(f,b"tag")?.is_some() || member(f,b"area")?.is_some() || member(f,b"status")?.is_some();
    let mut tb=[0u8;256];let mut ab=[0u8;64];let mut sb=[0u8;64];
    let tag=selected(f,b"tag",&mut tb,"全部".as_bytes())?;
    let area=selected(f,b"area",&mut ab,b"-1")?;
    let status=selected(f,b"status",&mut sb,b"-1")?;
    let mut page=1;
    if filtered {
        if let Some(p)=member(a,b"page")? { if p!=b"null" { if let Some(c)=member(p,b"cursor")? {if c!=b"null" && quoted(c)?!=b"" {let mut b=[0;32];page=number(uq(c,&mut b)?).ok_or("invalid filter cursor")?;if page<2 || page>10000 {return Err("invalid filter cursor");}}}}}
        if !["全部","青春","性感","长腿","多人","御姐","巨乳","新婚","媳妇","暧昧","清纯","调教","少妇","风骚","同居","淫乱","好友","女神","诱惑","偷情","出轨","正妹","家教"].iter().any(|v| v.as_bytes()==tag) || ![b"-1".as_slice(),b"1",b"2",b"3"].contains(&area) || ![b"-1".as_slice(),b"0",b"1"].contains(&status) {return Err("invalid filter");}
    } else {first_page(r)?;}
    let mut qb = [0; 2048];
    let mut ub = [0; 8192];
    let mut un = 0;
    if search {
        let query = uq(required(args(r)?, b"query")?, &mut qb)?;
        if trim_ascii(query).is_empty() {
            return Err("empty query");
        };
        bytes(&mut ub, &mut un, WEB)?;
        bytes(&mut ub, &mut un, b"/search?keyword=")?;
        if !write_url_encoded(&mut ub, &mut un, query) {
            return Err("query capacity");
        }
    } else {
        if text(args(r)?, b"listingId")? != b"listing:popular" {return Err("listing identity");}
        bytes(&mut ub,&mut un,WEB)?;
        if filtered {
            bytes(&mut ub,&mut un,b"/booklist?page=")?;wn(&mut ub,&mut un,page)?;
            bytes(&mut ub,&mut un,b"&tag=")?;if !write_url_encoded(&mut ub,&mut un,tag) {return Err("tag capacity");}
            bytes(&mut ub,&mut un,b"&area=")?;bytes(&mut ub,&mut un,area)?;
            bytes(&mut ub,&mut un,b"&end=")?;bytes(&mut ub,&mut un,status)?;
        } else {bytes(&mut ub,&mut un,b"/rank")?;}
    }
    let doc = fetch(&ub[..un])?;
    let mut section = None;
    let mut sections_owner: Option<All> = None;
    if !search && !filtered {
        let sections = All::new(doc.0, b".mh-list.col3.top-cat > li")?;
        for i in 0..sections.len {
            let mut b = [0; 256];
            let name = title(sections.at(i), b".title", &mut b)?;
            if name == "人气榜".as_bytes() || name == "总榜".as_bytes() {
                if section.is_some() {
                    return Err("ambiguous rank");
                };
                section = Some(sections.at(i))
            }
        }
        if section.is_none() {
            return Err("rank section missing");
        }
        sections_owner = Some(sections);
    }
    let d = section.unwrap_or(doc.0);
    let _keep_sections = &sections_owner;
    let cards = All::new(
        d,
        if search || filtered {
            b".mh-item"
        } else {
            b".mh-item, .mh-itme-top"
        },
    )?;
    if cards.len == 0 {
        return Err("empty or gated listing");
    };
    let next = All::new(doc.0, b"a#nextPage")?;
    let mut next_page=None;
    if next.len > 0 {
        if !filtered || next.len != 1 {return Err("unverified query pagination");}
        let mut hb=[0u8;2048];let href=attr(next.at(0),b"href",&mut hb)?;
        let query=href.strip_prefix(WEB).unwrap_or(href).strip_prefix(b"/booklist?").ok_or("next path")?;
        let mut seen=0u8;let mut encoded_tag=[0u8;512];let mut etn=0;
        if !write_url_encoded(&mut encoded_tag,&mut etn,tag) {return Err("tag capacity");}
        for part in query.split(|v|*v==b'&') {
            let pos=part.iter().position(|v|*v==b'=').ok_or("next parameter")?;let (k,v)=(&part[..pos],&part[pos+1..]);
            let bit=match k {b"page"=>{if number(v)!=page.checked_add(1) {return Err("next page");}1},b"tag"=>{if v!=tag && v!=&encoded_tag[..etn] {return Err("next tag");}2},b"area"=>{if v!=area{return Err("next area");}4},b"end"=>{if v!=status{return Err("next status");}8},_=>return Err("next parameter")};
            if seen & bit != 0 {return Err("duplicate next parameter");}seen|=bit;
        }
        if seen!=15 {return Err("incomplete next parameter");}next_page=page.checked_add(1);
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let mut seen = [0usize; 1000];
    for i in 0..cards.len {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        let id = emit_card(out, &mut n, cards.at(i), !search && !filtered)?;
        if seen[..i].contains(&id) {
            return Err("listing duplicate");
        };
        seen[i] = id
    }
    bytes(out,&mut n,b"],\"page\":{\"nextCursor\":")?;
    if let Some(v)=next_page {bytes(out,&mut n,b"\"")?;wn(out,&mut n,v)?;bytes(out,&mut n,b"\",\"hasMore\":true}}")?;}
    else {bytes(out,&mut n,b"null,\"hasMore\":false}}")?;}
    Ok(n)
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut b = [0; 128];
    let doc = fetch(make(&[WEB, b"/book/", id], &mut b)?)?;
    let mut cb = [0; 1024];
    let cv = sa(doc.0, b".banner_detail_form .cover img", b"src", &mut cb)?;
    let mut ab = [0; 1024];
    cover(abs(cv, &mut ab)?, id)?;
    let mut t = [0; 4096];
    title(doc.0, b".banner_detail_form .info h1", &mut t)?;
    Ok(doc)
}
fn chapter_id<'a>(r: &'a [u8]) -> R<(&'a [u8], &'a [u8])> {
    let v = text(args(r)?, b"chapterId")?
        .strip_prefix(b"mxch:zh:")
        .ok_or("chapter namespace")?;
    let pos = v
        .iter()
        .position(|c| *c == b':')
        .ok_or("chapter identity")?;
    Ok((ident(&v[..pos])?, ident(&v[pos + 1..])?))
}
fn collect<'a>(doc: &Owned) -> R<All> {
    let a = All::new(doc.0, b"#detail-list-select li a")?;
    if a.len == 0 {
        return Err("empty or gated catalogue");
    };
    let mut seen = [0usize; 1000];
    for i in 0..a.len {
        let mut h = [0; 256];
        let id =
            number(href_id(attr(a.at(i), b"href", &mut h)?, b"/chapter/")?).ok_or("chapter ID")?;
        if seen[..i].contains(&id) {
            return Err("chapter duplicate");
        };
        seen[i] = id;
        let mut t = [0; 4096];
        if txt(a.at(i), &mut t)?.is_empty() {
            return Err("empty chapter title");
        }
    }
    Ok(a)
}
fn chapter_number<'a>(t: &'a [u8]) -> Option<&'a [u8]> {
    let v = t.strip_prefix("第".as_bytes())?;
    let end = v.iter().position(|c| !c.is_ascii_digit() && *c != b'.')?;
    let raw = &v[..end];
    if raw.is_empty()
        || raw[0] == b'.'
        || raw[raw.len() - 1] == b'.'
        || raw.iter().filter(|x| **x == b'.').count() > 1
    {
        return None;
    };
    if raw.len() > 1 && raw[0] == b'0' && raw[1].is_ascii_digit() {
        return None;
    };
    if !v[end..].starts_with("話".as_bytes()) && !v[end..].starts_with("话".as_bytes()) {
        return None;
    };
    Some(raw)
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let id = work(r)?;
    let doc = detail_doc(id)?;
    let out = payload_buf();
    let mut n = 0;
    if chapters {
        let a = collect(&doc)?;
        bytes(out, &mut n, b"{\"chapters\":[")?;
        for k in 0..a.len {
            let i = a.len - 1 - k;
            if k > 0 {
                bytes(out, &mut n, b",")?
            };
            let mut h = [0; 256];
            let cid = href_id(attr(a.at(i), b"href", &mut h)?, b"/chapter/")?;
            let mut tb = [0; 4096];
            let t = txt(a.at(i), &mut tb)?;
            bytes(out, &mut n, b"{\"id\":\"mxch:zh:")?;
            bytes(out, &mut n, id)?;
            bytes(out, &mut n, b":")?;
            bytes(out, &mut n, cid)?;
            bytes(out, &mut n, b"\",\"mangaId\":\"mx:zh:")?;
            bytes(out, &mut n, id)?;
            bytes(out, &mut n, b"\",\"title\":")?;
            q(out, &mut n, t)?;
            bytes(out, &mut n, b",\"chapterNumber\":")?;
            bytes(out, &mut n, chapter_number(t).unwrap_or(b"null"))?;
            bytes(out, &mut n, b",\"volumeNumber\":null,\"language\":\"zh\"}")?;
        }
        bytes(out, &mut n, b"]}")?;
        return Ok(n);
    }
    let info = child(doc.0, b".banner_detail_form .info")?;
    let mut tb = [0; 4096];
    let t = title(info.0, b"h1", &mut tb)?;
    let mut cb = [0; 1024];
    let cv = sa(doc.0, b".banner_detail_form .cover img", b"src", &mut cb)?;
    let mut ab = [0; 1024];
    let cv = abs(cv, &mut ab)?;
    cover(cv, id)?;
    bytes(out, &mut n, b"{\"manga\":{\"id\":\"mx:zh:")?;
    bytes(out, &mut n, id)?;
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
    q(out, &mut n, st(info.0, b".content", &mut db)?)?;
    let mut author = [0; 4096];
    let subs = All::new(info.0, b".subtitle")?;
    let mut found = false;
    for i in 0..subs.len {
        let mut b = [0; 4096];
        let v = txt(subs.at(i), &mut b)?;
        if let Some(v) = v.strip_prefix("作者：".as_bytes()) {
            if found {
                return Err("author ambiguous");
            };
            author[..v.len()].copy_from_slice(v);
            author[v.len()] = 0;
            found = true
        }
    }
    bytes(out, &mut n, b",\"authors\":[")?;
    if found {
        let z = author
            .iter()
            .position(|c| *c == 0)
            .ok_or("author capacity")?;
        q(out, &mut n, trim_ascii(&author[..z]))?
    }
    bytes(out, &mut n, b"],\"tags\":[")?;
    let blocks = All::new(info.0, b".tip span.block")?;
    let mut status = b"unknown".as_slice();
    let mut tagcount = 0;
    for i in 0..blocks.len {
        let mut b = [0; 4096];
        let v = txt(blocks.at(i), &mut b)?;
        if v.starts_with("状态".as_bytes()) {
            if v.windows("完结".len()).any(|s| s == "完结".as_bytes()) {
                status = b"completed"
            } else if v.windows("连载".len()).any(|s| s == "连载".as_bytes()) {
                status = b"ongoing"
            }
        }
        if v.starts_with("标签".as_bytes()) {
            let ts = All::new(blocks.at(i), b"a")?;
            for j in 0..ts.len {
                if tagcount > 0 {
                    bytes(out, &mut n, b",")?
                };
                let mut b = [0; 1024];
                q(out, &mut n, txt(ts.at(j), &mut b)?)?;
                tagcount += 1
            }
        }
    }
    bytes(out, &mut n, b"],\"status\":")?;
    q(out, &mut n, status)?;
    bytes(
        out,
        &mut n,
        b",\"links\":[{\"kind\":\"source\",\"url\":\"https://www.jjmhw2.top/book/",
    )?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"\"}]}}")?;
    Ok(n)
}
fn pages(r: &[u8]) -> R<usize> {
    let (wid, cid) = chapter_id(r)?;
    let doc = detail_doc(wid)?;
    let ch = collect(&doc)?;
    let mut matched = false;
    for i in 0..ch.len {
        let mut b = [0; 256];
        if href_id(attr(ch.at(i), b"href", &mut b)?, b"/chapter/")? == cid {
            matched = true
        }
    }
    if !matched {
        return Err("chapter membership");
    };
    drop(ch);
    drop(doc);
    let mut u = [0; 256];
    let doc = fetch(make(&[WEB, b"/chapter/", cid], &mut u)?)?;
    let mut h = [0; 256];
    if href_id(sa(doc.0, b"a.comic-name", b"href", &mut h)?, b"/book/")? != wid {
        return Err("chapter work ownership");
    }
    let images = All::new(doc.0, b".comicpage img")?;
    if images.len == 0 {
        return Err("empty or gated pages");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..images.len {
        let mut rb = [0; 1024];
        let raw = attr(images.at(i), b"data-original", &mut rb)?;
        let raw = if raw.is_empty() {
            attr(images.at(i), b"src", &mut rb)?
        } else {
            raw
        };
        let mut ub = [0; 1024];
        let url = abs(raw, &mut ub)?;
        let p = image_path(url)?;
        let mut pre = [0; 128];
        let pre = make(&[wid, b"/", cid, b"/"], &mut pre)?;
        let file = p.strip_prefix(pre).ok_or("page ownership")?;
        let stem = file.strip_suffix(b".jpg").ok_or("page filename")?;
        ident(stem)?;
        for j in 0..i {
            if unsafe { &PAGES[j][..PAGE_LEN[j]] } == url {
                return Err("page duplicate");
            }
        }
        unsafe {
            PAGES[i][..url.len()].copy_from_slice(url);
            PAGE_LEN[i] = url.len()
        }
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"mxpage:zh:")?;
        bytes(out, &mut n, wid)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, cid)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, stem)?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b"\",\"index\":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, url)?;
        bytes(out, &mut n, b"}}")?
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let id = text(a, b"pageId")?;
    let mut urlb = [0; 1024];
    let url = uq(required(a, b"url")?, &mut urlb)?;
    if let Some(w) = id.strip_prefix(b"mx:zh:") {
        cover(url, ident(w)?)?
    } else {
        let id = id.strip_prefix(b"mxpage:zh:").ok_or("page namespace")?;
        let mut it = id.split(|c| *c == b':');
        let w = ident(it.next().ok_or("page ID")?)?;
        let c = ident(it.next().ok_or("page ID")?)?;
        let f = ident(it.next().ok_or("page ID")?)?;
        let index = number(it.next().ok_or("page ID")?).ok_or("page index")?;
        if it.next().is_some() || index >= 1000 {
            return Err("page index capacity");
        };
        let mut b = [0; 1024];
        if image_path(url)?
            != make(
                &[
                    w,
                    b"/",
                    c,
                    b"/",
                    f,
                    b".jpg",
                ],
                &mut b,
            )?
        {
            return Err("image ownership");
        };
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(out, &mut n, UA)?;
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
    let s="{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"人气榜\",\"kind\":\"popular\",\"supportsPaging\":false}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let data=if filter_feature(r) {r###"{"filters":[{"id":"notice","type":"header","name":"注意：搜索时不支持分类过滤","scope":"browse"},{"id":"tag","name":"题材","type":"select","scope":"browse","options":[{"label":"全部","value":"全部"},{"label":"青春","value":"青春"},{"label":"性感","value":"性感"},{"label":"长腿","value":"长腿"},{"label":"多人","value":"多人"},{"label":"御姐","value":"御姐"},{"label":"巨乳","value":"巨乳"},{"label":"新婚","value":"新婚"},{"label":"媳妇","value":"媳妇"},{"label":"暧昧","value":"暧昧"},{"label":"清纯","value":"清纯"},{"label":"调教","value":"调教"},{"label":"少妇","value":"少妇"},{"label":"风骚","value":"风骚"},{"label":"同居","value":"同居"},{"label":"淫乱","value":"淫乱"},{"label":"好友","value":"好友"},{"label":"女神","value":"女神"},{"label":"诱惑","value":"诱惑"},{"label":"偷情","value":"偷情"},{"label":"出轨","value":"出轨"},{"label":"正妹","value":"正妹"},{"label":"家教","value":"家教"}]},{"id":"area","name":"地区","type":"select","scope":"browse","options":[{"label":"全部","value":"-1"},{"label":"韩国","value":"1"},{"label":"日本","value":"2"},{"label":"台湾","value":"3"}]},{"id":"status","name":"进度","type":"select","scope":"browse","options":[{"label":"全部","value":"-1"},{"label":"连载","value":"0"},{"label":"完结","value":"1"}]}]}"###.as_bytes()} else {b"{\"filters\":[]}"};
    payload_buf()[..data.len()].copy_from_slice(data);write_success_payload("get_filters",data.len())
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
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"漫小肆\",\"zh-Hant\":\"漫小肆\",\"en\":\"Manga Xiao Si (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":true,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
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
