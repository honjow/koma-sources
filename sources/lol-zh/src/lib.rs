#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tencent.lol.zh.koma",
    name: "英雄联盟官方漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "腾讯英雄联盟宇宙的六个已核对中文漫画系列；原站图片合同变更时须重审更新。",
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
// Generated description text is already decoded; escape once without JSON unescaping.
fn q_text(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"") && append_json_escaped(p, c, v) && write_bytes(p, c, b"\"") {
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

const ORIGIN: &str = "https://yz.lol.qq.com";
const REFERER: &[u8] = b"https://yz.lol.qq.com/zh_CN/comic/";
const CATALOG: &[u8] = b"https://yz.lol.qq.com/v1/zh_cn/comics/index.json";
struct Book {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    cover: &'static str,
    chapters: &'static [Chapter],
}
struct Chapter {
    id: &'static str,
    title: &'static str,
    number: usize,
    groups: &'static [usize],
    pages: &'static [&'static str],
}
mod description_text;
mod frozen_contract;
use frozen_contract::BOOKS;
fn fetch(u: &[u8]) -> R<usize> {
    let len =
        build_get_request(http_req_buf(), u, Some(REFERER), &[]).ok_or("capacity_exceeded")?;
    let len =
        host::http_request(&http_req_buf()[..len], http_out()).map_err(|_| "network_error")?;
    if member(&http_out()[..len], b"status")? != b"200" {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..len], body_buf()).map_err(|_| "parse_error")
}
fn mid(dst: &mut [u8], c: &mut usize, b: &Book) -> R<()> {
    w(dst, c, b"lol:zh:")?;
    w(dst, c, b.slug.as_bytes())
}
fn cid(dst: &mut [u8], c: &mut usize, b: &Book, ch: &Chapter) -> R<()> {
    w(dst, c, b"lolread:zh:")?;
    w(dst, c, b.slug.as_bytes())?;
    w(dst, c, b":")?;
    w(dst, c, ch.id.as_bytes())
}
fn pid(dst: &mut [u8], c: &mut usize, b: &Book, ch: &Chapter, index: usize) -> R<()> {
    w(dst, c, b"lolpage:zh:")?;
    w(dst, c, b.slug.as_bytes())?;
    w(dst, c, b":")?;
    w(dst, c, ch.id.as_bytes())?;
    w(dst, c, b":")?;
    n(dst, c, index)
}
fn view(dst: &mut [u8], c: &mut usize, b: &Book, ch: Option<&Chapter>) -> R<()> {
    w(dst, c, ORIGIN.as_bytes())?;
    w(dst, c, b"/zh_cn/comic/")?;
    w(dst, c, b.slug.as_bytes())?;
    if let Some(ch) = ch {
        w(dst, c, b"/")?;
        w(dst, c, ch.id.as_bytes())?;
    }
    Ok(())
}
fn resolve_mid(raw: &[u8]) -> R<&'static Book> {
    let s = raw.strip_prefix(b"lol:zh:").ok_or("invalid_request")?;
    BOOKS
        .iter()
        .find(|b| b.slug.as_bytes() == s)
        .ok_or("not_found")
}
fn resolve_cid(raw: &[u8]) -> R<(&'static Book, &'static Chapter)> {
    let s = raw.strip_prefix(b"lolread:zh:").ok_or("invalid_request")?;
    let i = s.iter().position(|x| *x == b':').ok_or("invalid_request")?;
    let b = BOOKS
        .iter()
        .find(|b| b.slug.as_bytes() == &s[..i])
        .ok_or("not_found")?;
    let ch = b
        .chapters
        .iter()
        .find(|ch| ch.id.as_bytes() == &s[i + 1..])
        .ok_or("not_found")?;
    Ok((b, ch))
}
fn validate_owner(req: &[u8], b: &Book) -> R<()> {
    if json_object_member(req, b"mangaId").is_some() {
        let owner = resolve_mid(string(req, b"mangaId")?)?;
        if owner.slug != b.slug {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn catalogue() -> R<()> {
    let len = fetch(CATALOG)?;
    let json = &body_buf()[..len];
    if string(json, b"locale")? != b"zh_cn" || string(json, b"id")? != b"comics" {
        return Err("wrong_owner");
    }
    let section = member(member(json, b"sections")?, b"series")?;
    let mut iter = JsonArrayIter::from_raw_array(member(section, b"data")?).ok_or("parse_error")?;
    let mut count = 0;
    while let Some(card) = iter.next_object() {
        if count >= BOOKS.len() {
            return Err("official_contract_changed");
        }
        let b = &BOOKS[count];
        let mut url = [0; 256];
        let mut c = 0;
        w(&mut url, &mut c, b"/zh_cn/comic/")?;
        w(&mut url, &mut c, b.slug.as_bytes())?;
        if string(card, b"title")? != b.title.as_bytes()
            || string(card, b"url")? != &url[..c]
            || string(card, b"link-out-type")? != b"comic-series"
        {
            return Err("official_contract_changed");
        }
        let cover = json_object_member(card, b"cover-image")
            .or_else(|| json_object_member(card, b"background"))
            .ok_or("parse_error")?;
        if string(cover, b"uri")? != b.cover.as_bytes() {
            return Err("official_contract_changed");
        }
        count += 1;
    }
    if count != BOOKS.len() {
        return Err("incomplete_catalogue");
    }
    Ok(())
}
fn decoded_equal(raw: &[u8], expected: &[u8]) -> R<bool> {
    let mut decoded = [0; 8192];
    let len = raw_string(raw, &mut decoded)?;
    Ok(&decoded[..len] == expected)
}
fn metadata(b: &Book) -> R<usize> {
    let mut url = [0; 256];
    let mut c = 0;
    w(&mut url, &mut c, ORIGIN.as_bytes())?;
    w(&mut url, &mut c, b"/v1/zh_cn/comics/")?;
    w(&mut url, &mut c, b.slug.as_bytes())?;
    w(&mut url, &mut c, b"/index.json")?;
    let len = fetch(&url[..c])?;
    let json = &body_buf()[..len];
    if string(json, b"id")? != b.slug.as_bytes()
        || string(json, b"locale")? != b"zh_cn"
        || string(json, b"name")? != b.title.as_bytes()
        || !decoded_equal(string(json, b"description")?, b.description.as_bytes())?
    {
        return Err("official_contract_changed");
    }
    let mut iter = JsonArrayIter::from_raw_array(member(json, b"issues")?).ok_or("parse_error")?;
    let mut count = 0;
    while let Some(issue) = iter.next_object() {
        let ch = b.chapters.get(count).ok_or("official_contract_changed")?;
        let mut url = [0; 256];
        let mut c = 0;
        w(&mut url, &mut c, b"/zh_cn/comic/")?;
        w(&mut url, &mut c, b.slug.as_bytes())?;
        w(&mut url, &mut c, b"/")?;
        w(&mut url, &mut c, ch.id.as_bytes())?;
        if string(issue, b"id")? != ch.id.as_bytes()
            || number(member(issue, b"index")?)? != ch.number
            || string(issue, b"title")? != ch.title.as_bytes()
            || string(issue, b"url")? != &url[..c]
            || string(issue, b"link-out-type")? != b"comic"
        {
            return Err("official_contract_changed");
        }
        count += 1;
    }
    if count != b.chapters.len() {
        return Err("incomplete_chapters");
    }
    Ok(len)
}
// Iterate direct JSON array values while respecting quoted strings and nested arrays.
struct Array<'a> {
    raw: &'a [u8],
    pos: usize,
}
impl<'a> Array<'a> {
    fn new(raw: &'a [u8]) -> R<Self> {
        if raw.first() != Some(&b'[') || raw.last() != Some(&b']') {
            return Err("parse_error");
        }
        Ok(Self { raw, pos: 1 })
    }
    fn next(&mut self) -> R<Option<&'a [u8]>> {
        while self.pos < self.raw.len()
            && (self.raw[self.pos].is_ascii_whitespace() || self.raw[self.pos] == b',')
        {
            self.pos += 1;
        }
        if self.raw.get(self.pos) == Some(&b']') {
            return Ok(None);
        }
        let start = self.pos;
        let mut depth = 0usize;
        let mut quoted = false;
        while self.pos < self.raw.len() {
            let c = self.raw[self.pos];
            if quoted {
                if c == b'\\' {
                    self.pos += 1;
                } else if c == b'"' {
                    quoted = false;
                }
            } else {
                match c {
                    b'"' => quoted = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            self.pos += 1;
                            return Ok(Some(&self.raw[start..self.pos]));
                        }
                    }
                    b',' | b']' if depth == 0 => return Err("parse_error"),
                    _ => {}
                }
            }
            self.pos += 1;
        }
        Err("parse_error")
    }
}
fn pages_contract(b: &Book, ch: &Chapter) -> R<()> {
    let mut url = [0; 256];
    let mut c = 0;
    w(&mut url, &mut c, ORIGIN.as_bytes())?;
    w(&mut url, &mut c, b"/comics-data/zh_cn/")?;
    w(&mut url, &mut c, b.slug.as_bytes())?;
    w(&mut url, &mut c, b"/")?;
    w(&mut url, &mut c, ch.id.as_bytes())?;
    w(&mut url, &mut c, b"/index.json")?;
    let len = fetch(&url[..c])?;
    let json = &body_buf()[..len];
    let mut id = [0; 128];
    let mut c = 0;
    w(&mut id, &mut c, b.slug.as_bytes())?;
    w(&mut id, &mut c, b"/")?;
    w(&mut id, &mut c, ch.id.as_bytes())?;
    if string(json, b"id")? != &id[..c] {
        return Err("wrong_owner");
    }
    let mut mobile = Array::new(member(json, b"mobile-pages")?)?;
    if mobile.next()?.is_some() {
        return Err("official_contract_changed");
    }
    let mut groups = Array::new(member(json, b"desktop-pages")?)?;
    let mut group_n = 0;
    let mut count = 0;
    while let Some(group) = groups.next()? {
        let mut entries = Array::new(group)?;
        let mut group_count = 0;
        while let Some(d) = entries.next()? {
            let expected = ch.pages.get(count).ok_or("official_contract_changed")?;
            if string(d, b"uri")? != expected.as_bytes() {
                return Err("official_contract_changed");
            }
            count += 1;
            group_count += 1;
        }
        if ch.groups.get(group_n) != Some(&group_count) {
            return Err("official_contract_changed");
        }
        group_n += 1;
    }
    if group_n != ch.groups.len() || count != ch.pages.len() {
        return Err("incomplete_pages");
    }
    Ok(())
}
fn book(p: &mut [u8], c: &mut usize, b: &Book, detail: bool) -> R<()> {
    let mut id = [0; 256];
    let mut n = 0;
    mid(&mut id, &mut n, b)?;
    w(p, c, b"{\"id\":")?;
    q(p, c, &id[..n])?;
    w(p, c, b",\"title\":")?;
    q(p, c, b.title.as_bytes())?;
    let mut url = [0; 256];
    let mut n = 0;
    view(&mut url, &mut n, b, None)?;
    w(p, c, b",\"url\":")?;
    q(p, c, &url[..n])?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, b.cover.as_bytes())?;
    w(p,c,b"},\"authors\":[],\"language\":\"zh\",\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q_text(p, c, description_text::for_book(b)?.as_bytes())?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, &url[..n])?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn listing(query: &[u8]) -> R<usize> {
    catalogue()?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    let mut count = 0;
    for b in BOOKS {
        if !query.is_empty()
            && !contains_bytes(b.title.as_bytes(), query)
            && !contains_bytes(description_text::for_book(b)?.as_bytes(), query)
        {
            continue;
        }
        if count > 0 {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, b, false)?;
        count += 1;
    }
    w(
        p,
        &mut c,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut query = [0; 1024];
        let len = raw_string(string(req, b"query")?, &mut query)?;
        match page(req)? {
            1 => listing(&query[..len]),
            _ => {
                let v = b"{\"items\":[],\"page\":{\"nextCursor\":null,\"hasMore\":false}}";
                payload_buf()[..v.len()].copy_from_slice(v);
                Ok(v.len())
            }
        }
    }
    result("search", inner(req))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if string(req, b"listingId").ok() != Some(b"listing:official") {
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
        let b = resolve_mid(string(req, b"mangaId")?)?;
        metadata(b)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, b, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let b = resolve_mid(string(req, b"mangaId")?)?;
        metadata(b)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for (i, ch) in b.chapters.iter().enumerate() {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            let mut id = [0; 256];
            let mut n = 0;
            cid(&mut id, &mut n, b, ch)?;
            w(p, &mut c, b"{\"id\":")?;
            q(p, &mut c, &id[..n])?;
            n = 0;
            mid(&mut id, &mut n, b)?;
            w(p, &mut c, b",\"mangaId\":")?;
            q(p, &mut c, &id[..n])?;
            w(p, &mut c, b",\"readingGroup\":")?;
            q(p, &mut c, &id[..n])?;
            w(p, &mut c, b",\"title\":")?;
            q(p, &mut c, ch.title.as_bytes())?;
            let mut url = [0; 256];
            let mut n = 0;
            view(&mut url, &mut n, b, Some(ch))?;
            w(p, &mut c, b",\"url\":")?;
            q(p, &mut c, &url[..n])?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            self::n(p, &mut c, ch.number)?;
            w(
                p,
                &mut c,
                b",\"volumeNumber\":null,\"continuationEligible\":true,\"language\":\"zh\"}",
            )?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let (b, ch) = resolve_cid(string(req, b"chapterId")?)?;
        validate_owner(req, b)?;
        pages_contract(b, ch)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for (i, url) in ch.pages.iter().enumerate() {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            let mut id = [0; 256];
            let mut len = 0;
            pid(&mut id, &mut len, b, ch, i)?;
            w(p, &mut c, b"{\"id\":")?;
            q(p, &mut c, &id[..len])?;
            w(p, &mut c, b",\"index\":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, url.as_bytes())?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 2048];
        let len = raw_string(string(req, b"url")?, &mut url)?;
        let url = &url[..len];
        let page_id = if json_object_member(req, b"pageId").is_some() {
            Some(string(req, b"pageId")?)
        } else {
            None
        };
        let mut found = None;
        for b in BOOKS {
            if url == b.cover.as_bytes() {
                let mut id = [0; 256];
                let mut len = 0;
                mid(&mut id, &mut len, b)?;
                if page_id.is_some() && page_id != Some(&id[..len]) {
                    continue;
                }
                found = Some((b, None));
                break;
            }
            for ch in b.chapters {
                for (i, page) in ch.pages.iter().enumerate() {
                    if url != page.as_bytes() {
                        continue;
                    }
                    let mut id = [0; 256];
                    let mut len = 0;
                    pid(&mut id, &mut len, b, ch, i)?;
                    if page_id == Some(&id[..len]) {
                        found = Some((b, Some(ch)));
                        break;
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        let (b, ch) = found.ok_or("invalid_request")?;
        if json_object_member(req, b"mangaUrl").is_some() {
            let mut route = [0; 256];
            let mut len = 0;
            view(&mut route, &mut len, b, None)?;
            let mut given = [0; 512];
            let n = raw_string(string(req, b"mangaUrl")?, &mut given)?;
            if given[..n] != route[..len] {
                return Err("invalid_request");
            }
        }
        validate_owner(req, b)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        let mut route = [0; 256];
        let mut len = 0;
        view(&mut route, &mut len, b, ch)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, &route[..len])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let v = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方中文系列\"}]}".as_bytes();
    payload_buf()[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
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
            if string(req, b"sourceId").ok() != Some(SOURCE_INFO.id.as_bytes())
                || string(req, b"operation").ok() != Some($op.as_bytes())
            {
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
