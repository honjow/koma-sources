#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.ncsoft.bns.zh-hant.koma",
    name: "劍靈官方繁中漫畫",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "The publicly available Chinese-Taiwan comic reading units in the official NCSOFT Taiwan Musung library.",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    json_object_member(v, k).ok_or("parse_error")
}
fn string<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let x = member(v, k)?;
    if x.len() < 2 || x[0] != b'"' || x[x.len() - 1] != b'"' {
        return Err("parse_error");
    };
    Ok(&x[1..x.len() - 1])
}
fn number(v: &[u8]) -> R<usize> {
    if v.is_empty() {
        return Err("parse_error");
    };
    let mut n = 0usize;
    for b in v {
        if !b.is_ascii_digit() {
            return Err("parse_error");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("parse_error")?;
    }
    Ok(n)
}
fn raw_string(v: &[u8], out: &mut [u8]) -> R<usize> {
    let mut i = 0;
    let mut c = 0;
    while i < v.len() {
        let b = v[i];
        i += 1;
        if b != b'\\' {
            if b < 32 || !write_bytes(out, &mut c, &[b]) {
                return Err("parse_error");
            };
            continue;
        }
        if i == v.len() {
            return Err("parse_error");
        };
        let e = v[i];
        i += 1;
        match e {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut c, &[e]) {
                    return Err("capacity_exceeded");
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let b = match e {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut c, &[b]) {
                    return Err("capacity_exceeded");
                }
            }
            b'u' => {
                if i + 4 > v.len() || !v[i..i + 4].iter().all(|b| b.is_ascii_hexdigit()) {
                    return Err("parse_error");
                };
                let mut cp = hex_to_u16(&v[i..i + 4]) as u32;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if i + 6 > v.len()
                        || &v[i..i + 2] != b"\\u"
                        || !v[i + 2..i + 6].iter().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err("parse_error");
                    };
                    let low = hex_to_u16(&v[i + 2..i + 6]) as u32;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("parse_error");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    i += 6;
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("parse_error");
                };
                let mut b = [0; 4];
                let n = encode_utf8(cp, &mut b);
                if !write_bytes(out, &mut c, &b[..n]) {
                    return Err("capacity_exceeded");
                }
            }
            _ => return Err("parse_error"),
        }
    }
    Ok(c)
}
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"")
        && append_json_unescaped_then_escaped(p, c, v)
        && write_bytes(p, c, b"\"")
    {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn w(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn n(p: &mut [u8], c: &mut usize, v: usize) -> R<()> {
    if write_usize(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}

fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false}}";
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let v = match json_object_member(args, b"page") {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        json_object_member(v, b"cursor").unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    };
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}

const MAIN: &[u8] = b"https://tw.ncsoft.com/bns/library/musung/main";
const DATA: &[u8] = b"https://tw.ncsoft.com/bns/js/library_data.js";
// The retained catalogue shares the existing 1 MiB HTTP/body ABI budget, not a book-count limit.
static mut CATALOGUE: [u8; 1024 * 1024] = [0; 1024 * 1024];
static mut CATALOGUE_LEN: usize = 0;
static mut IMAGE_URI: [u8; 1024 * 1024] = [0; 1024 * 1024];
#[derive(Clone, Copy)]
struct Unit<'a> {
    id: &'a [u8],
    title: &'a [u8],
}
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    // SelectAll returns the full count but allocates descriptors only for written slots.
    // A zero-size output counts without acquiring any child handles.
    let mut buf = [];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            buf.as_mut_ptr(),
            0,
        )
    };
    if n < 0 {
        Err("html_error")
    } else {
        Ok(n as usize)
    }
}
fn text(doc: host::HtmlDescriptor, sel: &[u8], dst: &mut [u8]) -> R<usize> {
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_text(node, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    let len = r?;
    if len >= dst.len() {
        Err("capacity_exceeded")
    } else {
        Ok(len)
    }
}
fn attr(doc: host::HtmlDescriptor, sel: &[u8], key: &[u8], dst: &mut [u8]) -> R<usize> {
    // Caller selector requires presence of the same key; Attr never probes absent attributes.
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_attr(node, key, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    let len = r?;
    if len >= dst.len() {
        Err("capacity_exceeded")
    } else {
        Ok(len)
    }
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, None, &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn skip(v: &[u8], p: &mut usize) {
    loop {
        while *p < v.len() && v[*p].is_ascii_whitespace() {
            *p += 1;
        }
        if v.get(*p..*p + 2) == Some(b"//") {
            while *p < v.len() && v[*p] != b'\n' {
                *p += 1;
            }
        } else {
            break;
        }
    }
}
fn jsstr<'a>(v: &'a [u8], p: &mut usize) -> R<&'a [u8]> {
    skip(v, p);
    if v.get(*p) != Some(&b'\'') {
        return Err("parse_error");
    }
    *p += 1;
    let at = *p;
    while *p < v.len() && v[*p] != b'\'' {
        if v[*p] == b'\\' || v[*p] < 32 {
            return Err("parse_error");
        }
        *p += 1;
    }
    if *p == v.len() {
        return Err("parse_error");
    }
    let s = &v[at..*p];
    *p += 1;
    Ok(s)
}
fn array_end(v: &[u8], start: usize) -> R<usize> {
    if v.get(start) != Some(&b'[') {
        return Err("parse_error");
    }
    let mut i = start;
    let mut depth = 0;
    let mut quote = 0;
    while i < v.len() {
        let b = v[i];
        if quote != 0 {
            if b == b'\\' {
                i += 1;
            } else if b == quote {
                quote = 0;
            }
        } else {
            match b {
                b'\'' | b'"' => quote = b,
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(i + 1);
                    }
                }
                b'/' => {
                    if v.get(i + 1) == Some(&b'/') {
                        while i < v.len() && v[i] != b'\n' {
                            i += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    Err("parse_error")
}
fn data_group(v: &[u8]) -> R<&[u8]> {
    let marker = b"var dataNameListArr = [";
    let mut at = find_subslice(v, marker).ok_or("parse_error")? + marker.len();
    for i in 0..3 {
        let name = jsstr(v, &mut at)?;
        if i == 2 && name != b"musung" {
            return Err("wrong_owner");
        }
        skip(v, &mut at);
        if v.get(at) != Some(&b',') {
            return Err("parse_error");
        }
        at += 1;
    }
    let marker = b"var dataArr = [";
    let mut at = find_subslice(v, marker).ok_or("parse_error")? + marker.len();
    let mut selected = None;
    for i in 0..3 {
        skip(v, &mut at);
        let stop = array_end(v, at)?;
        if i == 2 {
            selected = Some(&v[at..stop]);
        }
        at = stop;
        skip(v, &mut at);
        if v.get(at) != Some(&b',') {
            return Err("parse_error");
        }
        at += 1;
    }
    selected.ok_or("parse_error")
}

struct Units<'a> {
    group: &'a [u8],
    at: usize,
}
impl<'a> Units<'a> {
    fn new(group: &'a [u8]) -> Self {
        Self { group, at: 1 }
    }
    fn next(&mut self) -> R<Option<Unit<'a>>> {
        loop {
            skip(self.group, &mut self.at);
            if self.group.get(self.at) == Some(&b']') {
                return Ok(None);
            }
            let stop = array_end(self.group, self.at)?;
            let row = &self.group[self.at..stop];
            self.at = stop;
            skip(self.group, &mut self.at);
            if self.group.get(self.at) == Some(&b',') {
                self.at += 1;
            } else if self.group.get(self.at) != Some(&b']') {
                return Err("parse_error");
            }
            let mut x = 1;
            let id = jsstr(row, &mut x)?;
            skip(row, &mut x);
            if row.get(x) != Some(&b',') {
                return Err("parse_error");
            }
            x += 1;
            let title = jsstr(row, &mut x)?;
            // Match the official consumer's category precedence; other media remain filtered.
            let mut category = b"epicQuest".as_slice();
            for c in [b"epicQuest".as_slice(), b"comic", b"novel", b"musical"] {
                if contains_bytes(id, c) {
                    category = c;
                }
            }
            if category != b"comic" {
                continue;
            }
            skip(row, &mut x);
            // Current musung comic rows use the original raw ID and default route.
            // An explicit route/identity override needs its own contract, not a guessed fallback.
            if row.get(x) != Some(&b',') || trim_ascii(&row[x + 1..row.len() - 1]) != b"null" {
                return Err("unsupported_contract");
            }
            if id.is_empty() || title.is_empty() {
                return Err("wrong_owner");
            }
            return Ok(Some(Unit { id, title }));
        }
    }
}
fn units() -> R<Units<'static>> {
    let data = unsafe { &CATALOGUE[..CATALOGUE_LEN] };
    Ok(Units::new(data_group(data)?))
}
fn validate_data(v: &[u8]) -> R<()> {
    let group = data_group(v)?;
    let mut rows = Units::new(group);
    let mut count = 0;
    while let Some(unit) = rows.next()? {
        // Duplicate route identities cannot silently overwrite or produce duplicate IDs.
        let mut previous = Units::new(group);
        for _ in 0..count {
            if previous.next()?.ok_or("parse_error")?.id == unit.id {
                return Err("wrong_owner");
            }
        }
        count += 1;
    }
    if count == 0 {
        Err("not_found")
    } else {
        Ok(())
    }
}
fn catalogue() -> R<()> {
    let size = fetch(MAIN)?;
    let h = &body_buf()[..size];
    if !contains_bytes(h, b"var currentIdx = 2;")
        || !contains_bytes(h, b"src=\"/bns/js/library_data.js\"")
        || !contains_bytes(h, b"dataNameListArr[currentIdx]")
    {
        return Err("wrong_owner");
    }
    let doc = host::html_parse(h).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 1024];
        let n = text(doc, b".library__name > h1", &mut b)?;
        if trim_ascii(&b[..n]) != "無塵篇".as_bytes() {
            return Err("wrong_owner");
        }
        let n = attr(doc, b"html[lang]", b"lang", &mut b)?;
        if &b[..n] != b"zh-TW" {
            return Err("wrong_owner");
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r?;
    let size = fetch(DATA)?;
    validate_data(&body_buf()[..size])?;
    unsafe {
        CATALOGUE[..size].copy_from_slice(&body_buf()[..size]);
        CATALOGUE_LEN = size;
    }
    Ok(())
}
fn route(unit: Unit, cover: bool, out: &mut [u8]) -> R<usize> {
    let mut c = 0;
    w(
        out,
        &mut c,
        if cover {
            b"https://image.tw.ncsoft.com/bns/pc/official/library/musung/thumb_"
        } else {
            b"https://tw.ncsoft.com/bns/library/musung/"
        },
    )?;
    w(out, &mut c, unit.id)?;
    if cover {
        w(out, &mut c, b".jpg")?;
    }
    Ok(c)
}
fn identity(unit: Unit, prefix: &[u8], out: &mut [u8]) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, prefix)?;
    w(out, &mut c, unit.id)?;
    Ok(c)
}
fn resolve(id: &[u8], prefix: &[u8]) -> R<Unit<'static>> {
    let key = id.strip_prefix(prefix).ok_or("not_found")?;
    let mut rows = units()?;
    while let Some(unit) = rows.next()? {
        if unit.id == key {
            return Ok(unit);
        }
    }
    Err("not_found")
}
fn owner(req: &[u8], unit: Unit) -> R<()> {
    if let Some(raw) = extract_json_string(req, b"mangaUrl") {
        let len = raw_string(raw, body_buf())?;
        let c = route(unit, false, scratch_b())?;
        if body_buf()[..len] != scratch_b()[..c] {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn check_current(h: &[u8], unit: Unit) -> R<()> {
    let mut at =
        find_subslice(h, b"var currentArr").ok_or("wrong_owner")? + b"var currentArr".len();
    skip(h, &mut at);
    if h.get(at) != Some(&b'=') {
        return Err("wrong_owner");
    }
    at += 1;
    skip(h, &mut at);
    if h.get(at) != Some(&b'[') {
        return Err("wrong_owner");
    }
    at += 1;
    skip(h, &mut at);
    let start = at;
    while h.get(at).is_some_and(|b| b.is_ascii_digit()) {
        at += 1;
    }
    if number(&h[start..at])? != 2 {
        return Err("wrong_owner");
    }
    skip(h, &mut at);
    if h.get(at) != Some(&b',') {
        return Err("wrong_owner");
    }
    at += 1;
    if jsstr(h, &mut at)? != b"comic" {
        return Err("wrong_owner");
    }
    skip(h, &mut at);
    if h.get(at) != Some(&b',') {
        return Err("wrong_owner");
    }
    at += 1;
    if jsstr(h, &mut at)? != unit.id {
        return Err("wrong_owner");
    }
    skip(h, &mut at);
    if h.get(at) != Some(&b']') {
        return Err("wrong_owner");
    }
    Ok(())
}
fn with_detail<T>(unit: Unit, f: impl FnOnce(host::HtmlDescriptor) -> R<T>) -> R<T> {
    let len = route(unit, false, scratch_b())?;
    let size = fetch(&scratch_b()[..len])?;
    check_current(&body_buf()[..size], unit)?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let result = (|| {
        let len = attr(doc, b"html[lang]", b"lang", body_buf())?;
        if &body_buf()[..len] != b"zh-TW" {
            return Err("wrong_owner");
        }
        let len = text(doc, b".library-view--head.comic h2", body_buf())?;
        if trim_ascii(&body_buf()[..len]) != unit.title {
            return Err("wrong_owner");
        }
        let len = text(
            doc,
            b".library-view--head.comic .library-view--title > dt",
            body_buf(),
        )?;
        if trim_ascii(&body_buf()[..len]) != "漫畫".as_bytes() {
            return Err("wrong_owner");
        }
        f(doc)
    })();
    let closed = host::html_close(doc).map_err(|_| "html_error");
    closed?;
    result
}
fn pages(doc: host::HtmlDescriptor, mut visit: impl FnMut(usize, &[u8]) -> R<()>) -> R<usize> {
    let count = count(doc, b"#view .viewArea > .page")?;
    if count == 0 || self::count(doc, b"#view .viewArea > .page > img")? != count {
        return Err("incomplete_pages");
    }
    for i in 0..count {
        let mut s = [0; 160];
        let mut c = 0;
        w(&mut s, &mut c, b"#view .viewArea > .page:nth-child(")?;
        n(&mut s, &mut c, i + 1)?;
        w(&mut s, &mut c, b") > img")?;
        let base = c;
        w(&mut s, &mut c, b"[data-src]")?;
        let key = match self::count(doc, &s[..c])? {
            1 => b"data-src".as_slice(),
            0 => {
                c = base;
                w(&mut s, &mut c, b"[src]:not([data-src])")?;
                b"src".as_slice()
            }
            _ => return Err("incomplete_pages"),
        };
        // A page has exactly one image. Otherwise duplicates could hide behind attribute selectors.
        if self::count(doc, &s[..base])? != 1 {
            return Err("incomplete_pages");
        }
        let len = attr(doc, &s[..c], key, body_buf())?;
        let uri = &body_buf()[..len];
        if !(uri.starts_with(b"http://") || uri.starts_with(b"https://"))
            || uri == b"https://image.tw.ncsoft.com/common/spacer.gif"
        {
            return Err("incomplete_pages");
        }
        visit(i, uri)?;
    }
    Ok(count)
}
fn quote_identity(p: &mut [u8], c: &mut usize, unit: Unit, prefix: &[u8]) -> R<()> {
    let len = identity(unit, prefix, scratch_b())?;
    q(p, c, &scratch_b()[..len])
}
fn quote_route(p: &mut [u8], c: &mut usize, unit: Unit, cover: bool) -> R<()> {
    let len = route(unit, cover, scratch_b())?;
    q(p, c, &scratch_b()[..len])
}
fn creators(
    doc: host::HtmlDescriptor,
    sel: &[u8],
    prefix: &[u8],
    p: &mut [u8],
    c: &mut usize,
) -> R<()> {
    let count = count(doc, sel)?;
    w(p, c, b"[")?;
    if count > 1 {
        return Err("parse_error");
    }
    if count == 1 {
        let len = text(doc, sel, body_buf())?;
        let value = trim_ascii(&body_buf()[..len]);
        if let Some(name) = value.strip_prefix(prefix) {
            let name = trim_ascii(name);
            if !name.is_empty() {
                q(p, c, name)?;
            }
        }
    }
    w(p, c, b"]")
}
fn book(p: &mut [u8], c: &mut usize, unit: Unit, detail: bool) -> R<()> {
    with_detail(unit, |doc| {
        w(p, c, b"{\"id\":")?;
        quote_identity(p, c, unit, b"bns:zh-hant:musung:")?;
        w(p, c, b",\"title\":")?;
        q(p, c, unit.title)?;
        w(p, c, b",\"url\":")?;
        quote_route(p, c, unit, false)?;
        w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        quote_route(p, c, unit, true)?;
        w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":")?;
        creators(doc, b".library-view--author > dt", "文：".as_bytes(), p, c)?;
        w(p, c, b",\"artists\":")?;
        creators(doc, b".library-view--author > dd", "圖：".as_bytes(), p, c)?;
        w(
            p,
            c,
            b",\"status\":\"unknown\",\"contentRating\":\"unknown\"",
        )?;
        if detail {
            w(p, c, b",\"description\":")?;
            w(p, c, b"\"")?;
            for part in [
                "《劍靈外傳》官網公開繁中漫畫《".as_bytes(),
                unit.title,
                "》。".as_bytes(),
            ] {
                if !append_json_unescaped_then_escaped(p, c, part) {
                    return Err("capacity_exceeded");
                }
            }
            w(p, c, b"\"")?;
            w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
            quote_route(p, c, unit, false)?;
            w(p, c, b"}]")?;
        }
        w(p, c, b"}")
    })
}
fn listing(query: &[u8]) -> R<usize> {
    catalogue()?;
    let mut rows = units()?;
    let p = payload_buf();
    let mut c = 0;
    let mut emitted = false;
    w(p, &mut c, b"{\"items\":[")?;
    while let Some(unit) = rows.next()? {
        let common = contains_bytes("劍靈 剑灵 Blade Soul BNS".as_bytes(), query);
        let old_alias =
            unit.id == b"comic" && contains_bytes("無塵 无尘 悲運的徒弟，無塵".as_bytes(), query);
        if !query.is_empty() && !common && !old_alias && !contains_bytes(unit.title, query) {
            continue;
        }
        if emitted {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, unit, false)?;
        emitted = true;
    }
    w(p, &mut c, b"],\"page\":{\"hasMore\":false}}")?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let raw = extract_json_string(req, b"query").unwrap_or(b"");
    let mut b = [0; 1024];
    let len = match raw_string(raw, &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    match page(req) {
        Ok(1) => result("search", listing(&b[..len])),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(1) => result("get_manga_list", listing(b"")),
        Ok(_) => empty("get_manga_list"),
        Err(e) => write_error("get_manga_list", e, ""),
    }
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let id = extract_json_string(req, b"mangaId").ok_or("not_found")?;
        if !id.starts_with(b"bns:zh-hant:musung:") {
            return Err("not_found");
        }
        catalogue()?;
        let unit = resolve(id, b"bns:zh-hant:musung:")?;
        owner(req, unit)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, unit, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let id = extract_json_string(req, b"mangaId").ok_or("not_found")?;
        if !id.starts_with(b"bns:zh-hant:musung:") {
            return Err("not_found");
        }
        catalogue()?;
        let unit = resolve(id, b"bns:zh-hant:musung:")?;
        owner(req, unit)?;
        let count = with_detail(unit, |doc| pages(doc, |_, _| Ok(())))?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":")?;
        quote_identity(p, &mut c, unit, b"bnsread:zh-hant:musung:")?;
        w(p, &mut c, b",\"mangaId\":")?;
        quote_identity(p, &mut c, unit, b"bns:zh-hant:musung:")?;
        w(p, &mut c, b",\"title\":")?;
        q(p, &mut c, unit.title)?;
        w(p, &mut c, b",\"url\":")?;
        quote_route(p, &mut c, unit, false)?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":",
        )?;
        quote_identity(p, &mut c, unit, b"bns:zh-hant:musung:")?;
        w(
            p,
            &mut c,
            b",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, count)?;
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let id = extract_json_string(req, b"chapterId").ok_or("not_found")?;
        if !id.starts_with(b"bnsread:zh-hant:musung:") {
            return Err("not_found");
        }
        catalogue()?;
        let unit = resolve(id, b"bnsread:zh-hant:musung:")?;
        if let Some(mid) = extract_json_string(req, b"mangaId") {
            let n = identity(unit, b"bns:zh-hant:musung:", scratch_b())?;
            if mid != &scratch_b()[..n] {
                return Err("not_found");
            }
        }
        owner(req, unit)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        with_detail(unit, |doc| {
            pages(doc, |i, uri| {
                if i > 0 {
                    w(p, &mut c, b",")?;
                }
                w(p, &mut c, b"{\"id\":")?;
                let mut n = identity(unit, b"bnspage:zh-hant:musung:", scratch_b())?;
                w(scratch_b(), &mut n, b":")?;
                self::n(scratch_b(), &mut n, i)?;
                q(p, &mut c, &scratch_b()[..n])?;
                w(p, &mut c, b",\"index\":")?;
                self::n(p, &mut c, i)?;
                w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
                q(p, &mut c, uri)?;
                w(p, &mut c, b"}}")
            })
        })?;
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let raw = extract_json_string(req, b"url").ok_or("invalid_request")?;
        // Preserve the request URI outside body_buf, which is reused by complete DOM traversal.
        let uri = unsafe { &mut *core::ptr::addr_of_mut!(IMAGE_URI) };
        let len = raw_string(raw, uri)?;
        let u = &uri[..len];
        catalogue()?;
        let pid = extract_json_string(req, b"pageId");
        let mut rows = units()?;
        let mut cover = None;
        while let Some(unit) = rows.next()? {
            let len = route(unit, true, scratch_b())?;
            if &scratch_b()[..len] == u {
                cover = Some(unit);
                break;
            }
        }
        if let Some(unit) = cover {
            if let Some(id) = pid {
                let len = identity(unit, b"bns:zh-hant:musung:", scratch_b())?;
                if id != &scratch_b()[..len] {
                    return Err("invalid_request");
                }
            }
            owner(req, unit)?;
        } else {
            let id = pid.ok_or("invalid_request")?;
            let raw = id
                .strip_prefix(b"bnspage:zh-hant:musung:")
                .ok_or("invalid_request")?;
            let sep = raw
                .iter()
                .rposition(|b| *b == b':')
                .ok_or("invalid_request")?;
            let key = &raw[..sep];
            let ordinal = &raw[sep + 1..];
            if ordinal.is_empty() || (ordinal.len() > 1 && ordinal[0] == b'0') {
                return Err("invalid_request");
            }
            let index = number(ordinal).map_err(|_| "invalid_request")?;
            let mut selected = None;
            let mut rows = units()?;
            while let Some(unit) = rows.next()? {
                if unit.id == key {
                    selected = Some(unit);
                    break;
                }
            }
            let unit = selected.ok_or("invalid_request")?;
            owner(req, unit)?;
            let mut matched = false;
            let count = with_detail(unit, |doc| {
                pages(doc, |i, actual| {
                    if i == index {
                        matched = actual == u;
                    }
                    Ok(())
                })
            })?;
            if index >= count || !matched {
                return Err("invalid_request");
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p, &mut c, b",\"headers\":{}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方漫畫\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "")
}
const REQUEST_CAP: usize = 1024 * 1024;
static mut REQUEST: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= REQUEST_CAP || end > cap {
        return None;
    }
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= REQUEST_CAP {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
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
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            let req = match checked_request(ptr, len) {
                Some(r) => r,
                None => return write_error($op, "invalid_request", "invalid ABI range"),
            };
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"operation").ok() != Some($op.as_bytes()) {
                return write_error($op, "invalid_request", "operation mismatch");
            }
            $run(args)
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
