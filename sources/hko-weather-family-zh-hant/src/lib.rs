#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hko.weatherfamily.zh-hant.koma",
    name: "天氣家族漫畫廊・精選六期",
    version: "0.1.2", api_version: "0.2", language: "zh-Hant", author: "Koma",
    description: "香港天文台《談天說地》天氣家族漫畫廊，精選第77至82期。較早PDF及其他期刊不收錄，作品評級尚未確認。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:512*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn whitespace(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn end_value(s: &[u8], start: usize) -> R<usize> {
    parse_value(s, start, 0)
}
fn parse_value(s: &[u8], start: usize, depth: usize) -> R<usize> {
    if depth > 64 {
        return Err("capacity_exceeded");
    }
    let start = whitespace(s, start);
    let first = *s.get(start).ok_or("parse_error")?;
    if first == b'"' {
        let mut i = start + 1;
        while i < s.len() {
            match s[i] {
                b'"' => return Ok(i + 1),
                b'\\' => {
                    i += 1;
                    let e = *s.get(i).ok_or("parse_error")?;
                    if e == b'u' {
                        if i + 5 > s.len() || !s[i + 1..i + 5].iter().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("parse_error");
                        }
                        i += 4;
                    } else if !matches!(e, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') {
                        return Err("parse_error");
                    }
                }
                b if b < 32 => return Err("parse_error"),
                _ => {}
            }
            i += 1;
        }
        return Err("parse_error");
    }
    if matches!(first, b'{' | b'[') {
        let end = if first == b'{' { b'}' } else { b']' };
        let mut i = whitespace(s, start + 1);
        if s.get(i) == Some(&end) {
            return Ok(i + 1);
        }
        loop {
            if first == b'{' {
                if s.get(i) != Some(&b'"') {
                    return Err("parse_error");
                }
                i = whitespace(s, parse_value(s, i, depth + 1)?);
                if s.get(i) != Some(&b':') {
                    return Err("parse_error");
                }
                i = whitespace(s, i + 1);
            }
            i = whitespace(s, parse_value(s, i, depth + 1)?);
            match s.get(i) {
                Some(v) if *v == end => return Ok(i + 1),
                Some(b',') => {
                    i = whitespace(s, i + 1);
                    if s.get(i) == Some(&end) {
                        return Err("parse_error");
                    }
                }
                _ => return Err("parse_error"),
            }
        }
    }
    for v in [b"true".as_slice(), b"false".as_slice(), b"null".as_slice()] {
        if s[start..].starts_with(v) {
            return Ok(start + v.len());
        }
    }
    let mut i = start;
    if s.get(i) == Some(&b'-') {
        i += 1;
    }
    match s.get(i) {
        Some(b'0') => {
            i += 1;
            if s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                return Err("parse_error");
            }
        }
        Some(b'1'..=b'9') => {
            i += 1;
            while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                i += 1;
            }
        }
        _ => return Err("parse_error"),
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    if matches!(s.get(i), Some(b'e') | Some(b'E')) {
        i += 1;
        if matches!(s.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    Ok(i)
}
fn direct<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'{') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    let mut found = None;
    if s.get(i) == Some(&b'}') {
        if whitespace(s, i + 1) != s.len() {
            return Err("parse_error");
        }
        return Ok(None);
    }
    loop {
        if s.get(i) != Some(&b'"') {
            return Err("parse_error");
        }
        let e = end_value(s, i)?;
        let k = &s[i + 1..e - 1];
        if k.contains(&b'\\') {
            return Err("parse_error");
        }
        i = whitespace(s, e);
        if s.get(i) != Some(&b':') {
            return Err("parse_error");
        }
        i = whitespace(s, i + 1);
        let e = end_value(s, i)?;
        if k == key {
            if found.is_some() {
                return Err("duplicate_identity");
            }
            found = Some(&s[i..e]);
        }
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => i = whitespace(s, i + 1),
            Some(b'}') => {
                if whitespace(s, i + 1) != s.len() {
                    return Err("parse_error");
                }
                return Ok(found);
            }
            _ => return Err("parse_error"),
        }
    }
}
fn arg_string<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    match direct(s, key).map_err(|_| "invalid_request")? {
        None => Ok(None),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
            Ok(Some(&v[1..v.len() - 1]))
        }
        _ => Err("invalid_request"),
    }
}

fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    direct(v, k)?.ok_or("parse_error")
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
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
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

fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = req;
    let v = match direct(args, b"page").map_err(|_| "invalid_request")? {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        direct(v, b"cursor")
            .map_err(|_| "invalid_request")?
            .unwrap_or(b"null")
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
    let n = positive(v)?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    let mut buf = [0; 128];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n < 0 {
        return Err("html_error");
    }
    for i in 0..core::cmp::min(n as usize, 32) {
        let at = i * 4;
        let d = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        host::html_close(host::HtmlDescriptor::from_raw(d)).map_err(|_| "html_error")?;
    }
    Ok(n as usize)
}
fn text(doc: host::HtmlDescriptor, sel: &[u8], dst: &mut [u8]) -> R<usize> {
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_text(node, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
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
    r
}

const ORIGIN: &[u8] = b"https://www.hko.gov.hk";
const MID: &[u8] = b"hkofamily:zh-hant:issue:";
const CID: &[u8] = b"hkofamilyread:zh-hant:issue:";
const PID: &[u8] = b"hkofamilypage:zh-hant:issue:";
struct SelectedIssue {
    issue: &'static [u8],
    uri: &'static [u8],
    record: &'static [u8],
    media: &'static [u8],
}
// Six selected original navigation hrefs. Never synthesized from focus.html.
const SELECTED: [SelectedIssue; 6] = [
    SelectedIssue {
        issue: b"82",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol82/comics.html",
        record: b"2021072909093420517",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol82/images/",
    },
    SelectedIssue {
        issue: b"81",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol81/comics.html",
        record: b"2021041216011637415",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol81/images/",
    },
    SelectedIssue {
        issue: b"80",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol80/comics.html",
        record: b"2021012209352956087",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol80/images/",
    },
    SelectedIssue {
        issue: b"79",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol79/comics.html",
        record: b"2020110317221032687",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol79/images/",
    },
    SelectedIssue {
        issue: b"78",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol78/comics.html",
        record: b"2020080317264650605",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol78/images/",
    },
    SelectedIssue {
        issue: b"77",
        uri: b"https://www.hko.gov.hk/tc/education/friends_hko/e-newsletter/vol77/comics.html",
        record: b"2020072917342082021",
        media: b"/tc/education/friends_hko/e-newsletter/res/vol77/images/",
    },
];
const MAX_IMAGES: usize = 24;
static mut TITLE: [u8; 512] = [0; 512];
static mut TL: usize = 0;
static mut IMAGES: [[u8; 512]; MAX_IMAGES] = [[0; 512]; MAX_IMAGES];
static mut IL: [usize; MAX_IMAGES] = [0; MAX_IMAGES];
static mut IC: usize = 0;
// Original77 has direct children; later issues have one plain div. Navigation is .row.
const BODY_ALL: &[u8] = b"#mainContent > .border_blank > img:not(:first-child), #mainContent > .border_blank > a > img, #mainContent > .border_blank > div:not([class]) > img, #mainContent > .border_blank > div:not([class]) > a > img";
const BODY_PRESENT: &[u8] = b"#mainContent > .border_blank > img[src][alt]:not(:first-child), #mainContent > .border_blank > a > img[src][alt], #mainContent > .border_blank > div:not([class]) > img[src][alt], #mainContent > .border_blank > div:not([class]) > a > img[src][alt]";
fn positive(v: &[u8]) -> R<usize> {
    if v.is_empty() || (v.len() > 1 && v[0] == b'0') {
        return Err("invalid_request");
    }
    number(v).map_err(|_| "invalid_request")
}
fn selected_id(v: &[u8], prefix: &[u8]) -> R<usize> {
    let suffix = v.strip_prefix(prefix).ok_or("invalid_request")?;
    SELECTED
        .iter()
        .position(|x| x.issue == suffix)
        .ok_or("invalid_request")
}
fn identifier(p: &mut [u8], c: &mut usize, prefix: &[u8], i: usize) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, SELECTED[i].issue)
}
fn owner(req: &[u8], i: usize) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaId")? {
        if selected_id(v, MID)? != i {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        if v != SELECTED[i].uri {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let i = selected_id(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, i)?;
    Ok(i)
}
fn image_owner(u: &[u8]) -> R<usize> {
    let path = u.strip_prefix(ORIGIN).ok_or("invalid_request")?;
    for (i, selected) in SELECTED.iter().enumerate() {
        if let Some(file) = path.strip_prefix(selected.media) {
            if file.is_empty() || file.len() > 160 || !file.ends_with(b".png") {
                return Err("invalid_request");
            }
            let mut at = 0;
            while at < file.len() {
                let b = file[at];
                if b == b'%' {
                    if file.get(at..at + 3) != Some(b"%20") {
                        return Err("invalid_request");
                    }
                    at += 3;
                } else if b.is_ascii_alphanumeric() || b"_.-()".contains(&b) {
                    at += 1;
                } else {
                    return Err("invalid_request");
                }
            }
            if file.windows(2).any(|x| x == b"..") {
                return Err("invalid_request");
            }
            return Ok(i);
        }
    }
    Err("invalid_request")
}
fn resolve_image(raw: &[u8], i: usize, dst: &mut [u8]) -> R<usize> {
    let mut absolute = [0; 512];
    let mut c = 0;
    if raw.starts_with(ORIGIN) {
        w(&mut absolute, &mut c, raw)?;
    } else if raw.starts_with(b"/") {
        w(&mut absolute, &mut c, ORIGIN)?;
        w(&mut absolute, &mut c, raw)?;
    } else if let Some(path) = raw.strip_prefix(b"../../../../../") {
        w(&mut absolute, &mut c, ORIGIN)?;
        w(&mut absolute, &mut c, b"/")?;
        w(&mut absolute, &mut c, path)?;
    } else {
        return Err("parse_error");
    }
    let mut n = 0;
    for b in &absolute[..c] {
        if *b == b' ' {
            w(dst, &mut n, b"%20")?;
        } else {
            w(dst, &mut n, core::slice::from_ref(b))?;
        }
    }
    if image_owner(&dst[..n]).map_err(|_| "parse_error")? != i {
        return Err("parse_error");
    }
    Ok(n)
}
fn fetch(i: usize) -> R<usize> {
    let len = build_get_request(http_req_buf(), SELECTED[i].uri, Some(SELECTED[i].uri), &[])
        .ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..len], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    let n = decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")?;
    if n == 0 || n >= 512 * 1024 {
        return Err("capacity_exceeded");
    }
    Ok(n)
}
fn article(i: usize) -> R<()> {
    unsafe {
        IC = 0;
        TL = 0;
    }
    let len = fetch(i)?;
    let doc = host::html_parse(&body_buf()[..len]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut buf = [0; 512];
        let n = attr(
            doc,
            b"head meta[property='og:url'][content]",
            b"content",
            &mut buf,
        )?;
        if &buf[..n] != SELECTED[i].uri {
            return Err("identity_mismatch");
        }
        let n = attr(
            doc,
            b"head meta[name='recordId'][content]",
            b"content",
            &mut buf,
        )?;
        if &buf[..n] != SELECTED[i].record {
            return Err("identity_mismatch");
        }
        let n = text(doc, b"#mainContent > h1", &mut buf)?;
        let title = trim_ascii(&buf[..n]);
        if title.is_empty()
            || title.len() >= 512
            || core::str::from_utf8(title).is_err()
            || !contains_bytes(title, "天氣家族".as_bytes())
            || !contains_bytes(title, "漫畫廊".as_bytes())
        {
            return Err("identity_mismatch");
        }
        unsafe {
            TITLE[..title.len()].copy_from_slice(title);
            TL = title.len();
        }
        if count(doc,"#mainContent > .border_blank .desktopmenu a[href='comics.html'] > img[alt='天氣家族漫畫廊']".as_bytes())?!=1
            || count(doc,"#mainContent > .border_blank select#navigation > option[value='comics.html']".as_bytes())?!=1 {return Err("identity_mismatch");}
        let n = text(
            doc,
            b"#mainContent > .border_blank select#navigation > option[value='comics.html']",
            &mut buf,
        )?;
        if trim_ascii(&buf[..n]) != "天氣家族漫畫廊".as_bytes() {
            return Err("identity_mismatch");
        }
        let n = text(
            doc,
            b"#mainContent > .border_blank .row > div:not([class])",
            &mut buf,
        )?;
        if trim_ascii(&buf[..n]) != "編劇：黃婷婷".as_bytes() {
            return Err("identity_mismatch");
        }
        let all = count(doc, BODY_ALL)?;
        let present = count(doc, BODY_PRESENT)?;
        if all == 0 || all > MAX_IMAGES || all != present {
            return Err("parse_error");
        }
        let nav = count(doc, b"#mainContent > .border_blank .desktopmenu img")?;
        if count(doc, b"#mainContent > .border_blank img")? != all + nav + 1 {
            return Err("parse_error");
        }
        // Exactly one original newsletter banner; it never becomes a manga cover.
        if count(
            doc,
            "#mainContent > .border_blank > img:first-child[alt='談天說地'][src]".as_bytes(),
        )? != 1
        {
            return Err("parse_error");
        }
        let mut raw = [0; MAX_IMAGES * 4];
        let n = unsafe {
            select_all_raw(
                doc.raw(),
                BODY_PRESENT.as_ptr(),
                BODY_PRESENT.len() as u32,
                raw.as_mut_ptr(),
                raw.len() as u32,
            )
        };
        if n != all as i32 {
            return Err("html_error");
        }
        let mut failure = None;
        for at in 0..all {
            let off = at * 4;
            let id = i32::from_le_bytes([raw[off], raw[off + 1], raw[off + 2], raw[off + 3]]);
            let node = host::HtmlDescriptor::from_raw(id);
            let parsed = (|| {
                // Both attributes are present-qualified by BODY_PRESENT.
                let l = host::html_attr(node, b"alt", &mut buf).map_err(|_| "html_error")?;
                if &buf[..l] != "天氣家族漫畫廊".as_bytes() {
                    return Err("parse_error");
                }
                let l = host::html_attr(node, b"src", &mut buf).map_err(|_| "html_error")?;
                if l == 0 || l >= buf.len() {
                    return Err("capacity_exceeded");
                }
                let size = resolve_image(&buf[..l], i, unsafe { &mut IMAGES[at] })?;
                for before in 0..at {
                    if unsafe { &IMAGES[before][..IL[before]] } == unsafe { &IMAGES[at][..size] } {
                        return Err("parse_error");
                    }
                }
                unsafe {
                    IL[at] = size;
                }
                Ok(())
            })();
            let closed = host::html_close(node).map_err(|_| "html_error");
            if let Err(e) = parsed.and(closed) {
                if failure.is_none() {
                    failure = Some(e);
                }
            }
        }
        if let Some(e) = failure {
            return Err(e);
        }
        unsafe {
            IC = all;
        }
        Ok(())
    })();
    let closed = host::html_close(doc).map_err(|_| "html_error");
    r.and(closed)
}
fn book(p: &mut [u8], c: &mut usize, i: usize, full: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    identifier(p, c, MID, i)?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLE[..TL] })?;
    w(p, c, b",\"url\":")?;
    q(p, c, SELECTED[i].uri)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &IMAGES[0][..IL[0]] })?;
    w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":[")?;
    q(p, c, "黃婷婷".as_bytes())?;
    w(
        p,
        c,
        b"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"",
    )?;
    if full {
        w(p, c, b",\"description\":")?;
        q(p,c,"精選香港天文台《談天說地》第77至82期的天氣家族漫畫廊，每期獨立閱讀。編劇：黃婷婷；繪者和作品評級尚未確認。較早PDF及其他期刊不收錄。".as_bytes())?;
        w(
            p,
            c,
            b",\"tags\":[],\"links\":[{\"kind\":\"source\",\"url\":",
        )?;
        q(p, c, SELECTED[i].uri)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:selected-six\",\"name\":\"精選六期・第77至82期\"}]}"
        .as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if arg_string(req, b"listingId")? != Some(b"listing:selected-six") {
            return Err("invalid_request");
        }
        let page = page(req)?;
        if !(1..=3).contains(&page) {
            return Err("invalid_request");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"items\":[")?;
        for i in (page - 1) * 2..page * 2 {
            article(i)?;
            if i % 2 != 0 {
                w(p, &mut c, b",")?;
            }
            book(p, &mut c, i, false)?;
        }
        w(p, &mut c, b"],\"hasNextPage\":")?;
        w(p, &mut c, if page < 3 { b"true" } else { b"false" })?;
        w(p, &mut c, b",\"page\":{\"nextCursor\":")?;
        if page < 3 {
            w(p, &mut c, b"\"")?;
            n(p, &mut c, page + 1)?;
            w(p, &mut c, b"\"")?;
        } else {
            w(p, &mut c, b"null")?;
        }
        w(p, &mut c, b",\"hasMore\":")?;
        w(p, &mut c, if page < 3 { b"true" } else { b"false" })?;
        w(p, &mut c, b"}}")?;
        Ok(c)
    }
    result("get_manga_list", inner(req))
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", MID)?;
        article(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, i, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", MID)?;
        if page(req)? != 1 {
            return Err("invalid_request");
        }
        article(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        identifier(p, &mut c, CID, i)?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        identifier(p, &mut c, MID, i)?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLE[..TL] })?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, SELECTED[i].uri)?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        identifier(p, &mut c, MID, i)?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, unsafe { IC })?;
        w(
            p,
            &mut c,
            b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", CID)?;
        article(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for at in 0..unsafe { IC } {
            if at > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            identifier(p, &mut c, PID, i)?;
            w(p, &mut c, b":")?;
            n(p, &mut c, at)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, at)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, unsafe { &IMAGES[at][..IL[at]] })?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn page_id(v: &[u8]) -> R<(usize, usize)> {
    let rest = v.strip_prefix(PID).ok_or("invalid_request")?;
    let at = rest
        .iter()
        .position(|b| *b == b':')
        .ok_or("invalid_request")?;
    let issue = &rest[..at];
    let i = SELECTED
        .iter()
        .position(|x| x.issue == issue)
        .ok_or("invalid_request")?;
    let index = positive(&rest[at + 1..])?;
    if index >= MAX_IMAGES {
        return Err("invalid_request");
    }
    Ok((i, index))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let u = arg_string(req, b"url")?.ok_or("invalid_request")?;
        let i = image_owner(u)?;
        let ordinal = match arg_string(req, b"pageId")? {
            None => None,
            Some(v) if v.starts_with(MID) => {
                if selected_id(v, MID)? != i {
                    return Err("invalid_request");
                }
                None
            }
            Some(v) => {
                let (owner, at) = page_id(v)?;
                if owner != i {
                    return Err("invalid_request");
                }
                Some(at)
            }
        };
        owner(req, i)?;
        article(i)?;
        let at = ordinal.unwrap_or(0);
        if at >= unsafe { IC } || u != unsafe { &IMAGES[at][..IL[at]] } {
            return Err("invalid_request");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p,&mut c,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":")?;
        q(p, &mut c, SELECTED[i].uri)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_implemented", "此來源不提供搜尋")
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
            let args = match direct(req, b"args") {
                Ok(Some(a)) if a.first() == Some(&b'{') => a,
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
