#![no_std]
use koma_source_sdk::{
    build_get_request, host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
const MAX: usize = 1048576;
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.iqiyi.koma",name:"Iqiyi",version:"0.1.2",api_version:"0.2",language:"zh-Hans",author:"Koma",description:"Iqiyi Bada public metadata API; requires real host clock and entropy. Usage and per-work rating unknown.",content_rating:"safe"};
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
include!("parser.rs");
include!("md5.rs");
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut URL: [u8; 32768] = [0; 32768];
static mut DETAIL: [u8; MAX] = [0; MAX];
static mut EPOS: [(usize, usize); 4096] = [(0, 0); 4096];
static mut QIYI_ID: [u8; 32] = [0; 32];
static mut QIYI_READY: bool = false;
const WEB: &[u8] = b"https://bud.m.iqiyi.com";
#[link(wasm_import_module = "koma_host")]
unsafe extern "C" {
    #[link_name = "unix_time_ms"]
    fn host_unix_time_ms(p: *mut u8, n: u32) -> i32;
    #[link_name = "random_bytes"]
    fn host_random_bytes(p: *mut u8, n: u32) -> i32;
}
fn hex(x: &[u8], o: &mut [u8]) {
    let a = b"0123456789abcdef";
    for (i, v) in x.iter().enumerate() {
        o[i * 2] = a[(v >> 4) as usize];
        o[i * 2 + 1] = a[(v & 15) as usize];
    }
}
fn environment() -> Result<([u8; 32], u64), E> {
    unsafe {
        if !QIYI_READY {
            let mut entropy = [0; 16];
            if host_random_bytes(entropy.as_mut_ptr(), 16) != 16 {
                return Err("host entropy unavailable");
            }
            hex(&entropy, &mut *core::ptr::addr_of_mut!(QIYI_ID));
            QIYI_READY = true;
        }
        let mut t = [0; 8];
        if host_unix_time_ms(t.as_mut_ptr(), 8) != 8 {
            return Err("host time unavailable");
        }
        let ms = u64::from_le_bytes(t);
        if ms == 0 {
            return Err("host time unavailable");
        }
        Ok((QIYI_ID, ms))
    }
}
fn u64write(b: &mut [u8], n: &mut usize, mut x: u64) -> Result<(), E> {
    let mut rev = [0; 20];
    let mut k = 0;
    loop {
        rev[k] = b'0' + (x % 10) as u8;
        k += 1;
        x /= 10;
        if x == 0 {
            break;
        }
    }
    for c in rev[..k].iter().rev() {
        bytes(b, n, &[*c])?;
    }
    Ok(())
}
fn api(path: &[u8], params: &[(&[u8], &[u8])]) -> Result<usize, E> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let (id, time) = environment()?;
    let u = unsafe { &mut *core::ptr::addr_of_mut!(URL) };
    let mut n = 0;
    bytes(u, &mut n, b"https://comic.iqiyi.com")?;
    let start = n;
    bytes(u, &mut n, path)?;
    let path_end = n;
    bytes(u, &mut n, b"?")?;
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            bytes(u, &mut n, b"&")?;
        }
        bytes(u, &mut n, k)?;
        bytes(u, &mut n, b"=")?;
        if !write_url_encoded(u, &mut n, v) {
            return Err("URL capacity");
        }
    }
    bytes(u, &mut n, b"&qiyiId=")?;
    bytes(u, &mut n, &id)?;
    bytes(u, &mut n, b"&timeStamp=")?;
    u64write(u, &mut n, time)?;
    bytes(
        u,
        &mut n,
        b"&srcPlatform=15&appVer=3.0.0&agentVersion=h5&agentType=115",
    )?;
    let mut sign_input = [0; 32768];
    let mut sn = 0;
    bytes(&mut sign_input, &mut sn, &u[start..path_end])?;
    bytes(&mut sign_input, &mut sn, &u[path_end + 1..n])?;
    bytes(
        &mut sign_input,
        &mut sn,
        b"3sj8xof48xjf4tk9f4tk9ypgk9ypg5up",
    )?;
    let digest = md5(&sign_input[..sn]);
    let mut signature = [0; 32];
    hex(&digest, &mut signature);
    let req = build_get_request(
        http_req_buf(),
        &u[..n],
        Some(b"https://bud.m.iqiyi.com/"),
        &[(b"Origin", WEB), (b"md5", &signature)],
    )
    .ok_or("request capacity")?;
    let len = host::http_request(&http_req_buf()[..req], http_out())
        .map_err(|_| "HTTP transport failure")?;
    if len == 0 || len > MAX {
        return Err("HTTP response capacity");
    }
    valid_json(&http_out()[..len])?;
    let status = field_num(&http_out()[..len], b"status")?;
    if !(200..300).contains(&status) {
        return Err("HTTP status failure");
    }
    let size = koma_source_sdk::decode_json_body_into(&http_out()[..len], body_buf())
        .map_err(|_| "HTTP body failure")?;
    if size == 0 || size > MAX {
        return Err("HTTP body capacity");
    }
    valid_json(&body_buf()[..size])?;
    required(&body_buf()[..size], b"data")?;
    Ok(size)
}
fn data(n: usize) -> Result<&'static [u8], E> {
    let b = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, n) };
    required(b, b"data")
}
fn numeric(s: &[u8]) -> Result<u64, E> {
    if s.is_empty() || s.len() > 20 || s.len() > 1 && s[0] == b'0' {
        return Err("invalid numeric identity");
    }
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return Err("invalid numeric identity");
        }
        n = n
            .checked_mul(10)
            .and_then(|x| x.checked_add((c - b'0') as u64))
            .ok_or("numeric identity overflow")?;
    }
    if n == 0 {
        return Err("invalid numeric identity");
    }
    Ok(n)
}
fn field_id(v: &[u8], k: &[u8]) -> Result<u64, E> {
    numeric(scalar(required(v, k)?)?)
}
fn work(id: &[u8]) -> Result<&[u8], E> {
    let p = b"manga:/detail_";
    let x = id
        .strip_prefix(p)
        .and_then(|x| x.strip_suffix(b".html"))
        .ok_or("unsupported legacy manga identity")?;
    numeric(x)?;
    Ok(x)
}
fn work_id(b: &mut [u8], n: &mut usize, id: &[u8]) -> Result<(), E> {
    bytes(b, n, b"manga:/detail_")?;
    bytes(b, n, id)?;
    bytes(b, n, b".html")
}
fn work_route(b: &mut [u8], n: &mut usize, id: &[u8]) -> Result<(), E> {
    bytes(b, n, WEB)?;
    bytes(b, n, b"/detail/")?;
    bytes(b, n, id)
}
fn owned_detail(id: &[u8]) -> Result<usize, E> {
    numeric(id)?;
    let len = api(b"/views/1.0/comicDetail", &[(b"comicId", id)])?;
    let d = data(len)?;
    if field_id(d, b"comicId")? != numeric(id)? {
        return Err("detail owner mismatch");
    }
    let size = d.len();
    unsafe {
        DETAIL[..size].copy_from_slice(d);
    }
    Ok(size)
}
fn dt(n: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(DETAIL) as *const u8, n) }
}
fn image_url(url: &[u8]) -> Result<(), E> {
    core::str::from_utf8(url).map_err(|_| "image URL UTF8")?;
    let rest = url
        .strip_prefix(b"https://")
        .or_else(|| url.strip_prefix(b"http://"))
        .ok_or("image scheme")?;
    let slash = rest.iter().position(|x| *x == b'/').ok_or("image path")?;
    let h = &rest[..slash];
    if h != b"iqiyipic.com" && !h.ends_with(b".iqiyipic.com") {
        return Err("image host");
    }
    if h.is_empty()
        || h.starts_with(b".")
        || h.ends_with(b".")
        || h.windows(2).any(|x| x == b"..")
        || h.iter()
            .any(|x| !x.is_ascii_alphanumeric() && *x != b'.' && *x != b'-')
        || rest
            .iter()
            .any(|x| *x < 33 || matches!(*x, b'\\' | b'#' | b'@'))
    {
        return Err("image URL invalid");
    }
    Ok(())
}
fn emit_string_field(
    v: &[u8],
    key: &[u8],
    outkey: &[u8],
    b: &mut [u8],
    n: &mut usize,
) -> Result<(), E> {
    match member(v, key)? {
        None | Some(b"null") => Ok(()),
        Some(x) => {
            quoted(x)?;
            bytes(b, n, b",")?;
            q(b, n, outkey)?;
            bytes(b, n, b":")?;
            bytes(b, n, x)
        }
    }
}
fn card(v: &[u8], search: bool, b: &mut [u8], n: &mut usize) -> Result<(), E> {
    let ik = if search { b"id" as &[u8] } else { b"comicId" };
    let id = scalar(required(v, ik)?)?;
    numeric(id)?;
    bytes(b, n, b"{\"id\":\"")?;
    work_id(b, n, id)?;
    bytes(b, n, b"\",\"url\":\"")?;
    work_route(b, n, id)?;
    bytes(b, n, b"\",\"title\":")?;
    let title = required(v, b"title")?;
    quoted(title)?;
    bytes(b, n, title)?;
    bytes(b, n, b",\"contentRating\":\"unknown\"")?;
    let ck = if search {
        b"image_url" as &[u8]
    } else {
        b"pic"
    };
    if let Some(c) = member(v, ck)? {
        if c != b"null" {
            let mut u = [0; 4096];
            image_url(unescape(quoted(c)?, &mut u)?)?;
            bytes(b, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
            bytes(b, n, c)?;
            bytes(b, n, b"}")?;
        }
    }
    emit_string_field(
        v,
        if search { b"description" } else { b"brief" },
        b"description",
        b,
        n,
    )?;
    let a = if search {
        b"author_name" as &[u8]
    } else {
        b"authorsName"
    };
    if let Some(x) = member(v, a)? {
        if x != b"null" {
            quoted(x)?;
            bytes(b, n, b",\"authors\":[")?;
            bytes(b, n, x)?;
            bytes(b, n, b"]")?;
        }
    }
    let status_key = if search {
        b"serialize_status" as &[u8]
    } else {
        b"serializeStatus"
    };
    let status = match member(v, status_key)? {
        Some(x) => match num(x)? {
            1 => b"ongoing" as &[u8],
            2 => b"completed",
            _ => b"unknown",
        },
        None => b"unknown",
    };
    bytes(b, n, b",\"status\":")?;
    q(b, n, status)?;
    if !search {
        if let Some(tags) = member(v, b"comicTags")? {
            if tags.first() == Some(&b'[') {
                let mut it = Array::new(tags)?;
                while let Some(t) = it.next()? {
                    quoted(t)?;
                }
                bytes(b, n, b",\"tags\":")?;
                bytes(b, n, tags)?;
            }
        }
    }
    bytes(b, n, b"}")
}
fn first_page(a: &[u8]) -> Result<(), E> {
    if let Some(p) = member(a, b"page")? {
        if p != b"null" {
            if p.first() != Some(&b'{') {
                return Err("page format");
            }
            if let Some(c) = member(p, b"cursor")? {
                if c != b"null" && c != b"\"\"" {
                    return Err("unsupported cursor");
                }
            }
        }
    }
    Ok(())
}
fn listing(r: &[u8], search: bool) -> Result<usize, E> {
    let a = args(r)?;
    first_page(a)?;
    let len = if search {
        let mut query = [0; 8192];
        let x = unescape(text(a, b"query")?, &mut query)?;
        if x.is_empty() {
            return Err("empty query");
        }
        api(b"/views/1.0/search", &[(b"key", x), (b"page_num", b"1")])?
    } else {
        api(
            b"/views/1.0/classify/popularity_list",
            &[(b"type", b"1"), (b"pageSize", b"100"), (b"pageNo", b"1")],
        )?
    };
    let d = data(len)?;
    let arr = required(
        d,
        if search {
            b"docinfos"
        } else {
            b"popularityList"
        },
    )?;
    let mut it = Array::new(arr)?;
    let b = payload_buf();
    let mut n = 0;
    let mut ids = [0u64; 4096];
    let mut count = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    while let Some(v) = it.next()? {
        let v = if search {
            match member(v, b"albumDocInfo")? {
                None | Some(b"null") => continue,
                Some(x) => match member(x, b"comics")? {
                    None | Some(b"null") => continue,
                    Some(v) => v,
                },
            }
        } else {
            v
        };
        let id = field_id(v, if search { b"id" } else { b"comicId" })?;
        if count == 4096 || ids[..count].contains(&id) {
            return Err("list duplicate/capacity");
        }
        ids[count] = id;
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        card(v, search, b, &mut n)?;
        count += 1;
    }
    if !search && required(d, b"isEnd")? != b"true" {
        return Err("popular incomplete");
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let id = work(text(a, b"mangaId")?)?;
    let len = owned_detail(id)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    card(dt(len), false, b, &mut n)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn catalogue(id: &[u8]) -> Result<(usize, usize), E> {
    let len = owned_detail(id)?;
    let d = dt(len);
    let arr = required(d, b"episodes")?;
    let mut it = Array::new(arr)?;
    let mut count = 0;
    let mut ids = [0u64; 4096];
    while let Some(e) = it.next()? {
        if count == 4096 {
            return Err("chapter capacity");
        }
        if field_id(e, b"comicId")? != numeric(id)? {
            return Err("chapter owner");
        }
        let eid = field_id(e, b"episodeId")?;
        if eid == 0 || ids[..count].contains(&eid) {
            return Err("chapter duplicate");
        }
        ids[count] = eid;
        unsafe {
            EPOS[count] = (e.as_ptr() as usize - d.as_ptr() as usize, e.len());
        }
        count += 1;
    }
    if field_num(d, b"episodeCount")? != count || count == 0 {
        return Err("chapter completeness");
    }
    Ok((len, count))
}
fn episode(d: &[u8], i: usize) -> &[u8] {
    let (off, len) = unsafe { EPOS[i] };
    &d[off..off + len]
}
fn chapter_parts(id: &[u8]) -> Result<(&[u8], &[u8]), E> {
    let x = id
        .strip_prefix(b"chapter:/detail_")
        .ok_or("chapter identity")?;
    let pos = x
        .windows(14)
        .position(|x| x == b".html:/reader/")
        .ok_or("chapter identity")?;
    let w = &x[..pos];
    let rest = &x[pos + 14..];
    let (cw, e) = rest
        .strip_suffix(b".html")
        .and_then(|x| {
            x.iter()
                .position(|c| *c == b'_')
                .map(|i| (&x[..i], &x[i + 1..]))
        })
        .ok_or("chapter identity")?;
    numeric(w)?;
    numeric(e)?;
    if w != cw {
        return Err("chapter work mismatch");
    }
    Ok((w, e))
}
fn chapter_id(b: &mut [u8], n: &mut usize, w: &[u8], e: &[u8]) -> Result<(), E> {
    bytes(b, n, b"chapter:/detail_")?;
    bytes(b, n, w)?;
    bytes(b, n, b".html:/reader/")?;
    bytes(b, n, w)?;
    bytes(b, n, b"_")?;
    bytes(b, n, e)?;
    bytes(b, n, b".html")
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let id = work(text(a, b"mangaId")?)?;
    let (len, count) = catalogue(id)?;
    let d = dt(len);
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in (0..count).rev() {
        let e = episode(d, i);
        if i != count - 1 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"")?;
        chapter_id(b, &mut n, id, scalar(required(e, b"episodeId")?)?)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"")?;
        work_id(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        let t = required(e, b"episodeTitle")?;
        let literal = quoted(t)?;
        bytes(b, &mut n, b"\"")?;
        wn(b, &mut n, field_num(e, b"episodeOrder")?)?;
        bytes(b, &mut n, b" ")?;
        bytes(b, &mut n, literal)?;
        bytes(b, &mut n, b"\"")?;
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        wn(b, &mut n, field_num(e, b"episodeOrder")?)?;
        bytes(
            b,
            &mut n,
            b",\"volumeNumber\":null,\"language\":\"zh-Hans\"}",
        )?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn read(w: &[u8], e: &[u8]) -> Result<usize, E> {
    let (len, count) = catalogue(w)?;
    let d = dt(len);
    let mut selected = None;
    for i in 0..count {
        let x = episode(d, i);
        if field_id(x, b"episodeId")? == numeric(e)? {
            selected = Some(x);
        }
    }
    let selected = selected.ok_or("chapter absent")?;
    let order = field_num(selected, b"episodeOrder")?;
    let title = required(selected, b"episodeTitle")?;
    let len = api(b"/read/pcw/1.0/read", &[(b"episodeId", e)])?;
    let d = data(len)?;
    if field_num(d, b"authPass")? != 1 {
        return Err("chapter permission");
    }
    if field_num(d, b"episodeOrder")? != order || required(d, b"episodeTitle")? != title {
        return Err("reader owner");
    }
    Ok(len)
}
fn validate_page_data(d: &[u8]) -> Result<usize, E> {
    let mut it = Array::new(required(d, b"content")?)?;
    let mut count = 0;
    let mut refs = [(0usize, 0usize); 4096];
    while let Some(v) = it.next()? {
        if count >= 4096 || field_num(v, b"pageOrder")? != count + 1 {
            return Err("page sequence/capacity");
        }
        let u = required(v, b"imageUrl")?;
        let mut buf = [0u8; 4096];
        let decoded = unescape(quoted(u)?, &mut buf)?;
        image_url(decoded)?;
        for &(off, len) in &refs[..count] {
            let mut old = [0u8; 4096];
            if unescape(quoted(&d[off..off + len])?, &mut old)? == decoded {
                return Err("duplicate page URL");
            }
        }
        refs[count] = (u.as_ptr() as usize - d.as_ptr() as usize, u.len());
        count += 1;
    }
    if count == 0 || field_num(d, b"pageCount")? != count {
        return Err("page completeness");
    }
    Ok(count)
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let (w, e) = chapter_parts(text(a, b"chapterId")?)?;
    let len = read(w, e)?;
    let d = data(len)?;
    validate_page_data(d)?;
    let mut it = Array::new(required(d, b"content")?)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    let mut i = 0;
    while let Some(v) = it.next()? {
        if i >= 4096 || field_num(v, b"pageOrder")? != i + 1 {
            return Err("page sequence/capacity");
        }
        let u = required(v, b"imageUrl")?;
        let mut out = [0; 4096];
        image_url(unescape(quoted(u)?, &mut out)?)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"page:")?;
        bytes(b, &mut n, w)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, e)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        bytes(b, &mut n, u)?;
        bytes(b, &mut n, b"}}")?;
        i += 1;
    }
    if i == 0 || field_num(d, b"pageCount")? != i {
        return Err("page completeness");
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let p = text(a, b"pageId")?;
    let mut ub = [0; 4096];
    let u = unescape(text(a, b"url")?, &mut ub)?;
    image_url(u)?;
    if p.starts_with(b"manga:") {
        let id = work(p)?;
        let len = owned_detail(id)?;
        let mut c = [0; 4096];
        if unescape(text(dt(len), b"pic")?, &mut c)? != u {
            return Err("cover owner");
        }
    } else {
        let x = p.strip_prefix(b"page:").ok_or("page identity")?;
        let mut parts = x.split(|c| *c == b':');
        let w = parts.next().ok_or("page identity")?;
        let e = parts.next().ok_or("page identity")?;
        let i = number(parts.next().ok_or("page identity")?).ok_or("page index")?;
        if parts.next().is_some() {
            return Err("page identity");
        }
        let len = read(w, e)?;
        validate_page_data(data(len)?)?;
        let mut it = Array::new(required(data(len)?, b"content")?)?;
        let mut at = 0;
        let mut found = false;
        while let Some(v) = it.next()? {
            if at == i {
                let mut out = [0; 4096];
                if unescape(text(v, b"imageUrl")?, &mut out)? != u {
                    return Err("page URL owner");
                }
                found = true;
            }
            at += 1;
        }
        if !found {
            return Err("page index");
        }
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"imageRequest\":{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b,&mut n,b",\"method\":\"GET\",\"headers\":{\"Referer\":\"https://bud.m.iqiyi.com/\",\"Origin\":\"https://bud.m.iqiyi.com\"}}}")?;
    Ok(n)
}
fn execute(op: &str, r: Result<usize, E>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", listing(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", listing(r, false))
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
    let s="{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"人气榜\",\"kind\":\"popular\",\"supportsPaging\":false}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
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
    response_buffer().write_success_parts("source_info",&[b"{\"sourceInfo\":{\"id\":\"com.iqiyi.koma\",\"name\":\"Iqiyi\",\"version\":\"0.1.2\",\"apiVersion\":\"0.2\",\"language\":\"zh-Hans\",\"author\":\"Koma\",\"contentRating\":\"safe\",\"localizedNames\":{", "\"zh-Hans\":\"爱奇艺叭嗒\",\"zh-Hant\":\"愛奇藝叭嗒\",\"en\":\"Iqiyi Bada\"".as_bytes(), b"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}"])
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
