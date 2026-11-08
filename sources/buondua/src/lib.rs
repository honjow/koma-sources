#![no_std]
// Independent port of the complete Keiyoushi BuonDua.kt, Apache-2.0,
// ref de743aed846da53e00c45ab26ee0e77f7bb4355b. Host must enforce
// 10 requests/second for buondua.com only. Images/other CDN hosts do not share it.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
mod dom;
use dom::*;
use koma_source_sdk::source_protocol as protocol;
const BASE: &[u8] = b"https://buondua.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.buondua.koma",
    name: "Buon Dua",
    version: "0.1.0",
    api_version: "0.2",
    language: "all",
    author: "Koma",
    description: "Buon Dua public photo galleries. Contains adult material.",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: true,
    image_request: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:3*1024*1024,body:3*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn positive(s: &[u8]) -> R<usize> {
    if s.is_empty() || s[0] == b'0' {
        return Err("invalid_request");
    };
    let mut n = 0usize;
    for b in s {
        if !b.is_ascii_digit() {
            return Err("invalid_request");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("invalid_request")?
    }
    Ok(n)
}
fn absolute<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if v.starts_with(b"https://") {
        w(b, &mut n, v)?
    } else if v.starts_with(b"//") {
        w(b, &mut n, b"https:")?;
        w(b, &mut n, v)?
    } else if v.starts_with(b"/") {
        w(b, &mut n, BASE)?;
        w(b, &mut n, v)?
    } else {
        return Err("parse_error");
    };
    Ok(&b[..n])
}
fn local(v: &[u8]) -> R<&[u8]> {
    let p = v.strip_prefix(BASE).unwrap_or(v);
    if !p.starts_with(b"/")
        || p.starts_with(b"//")
        || p.windows(2).any(|x| x == b"..")
        || p.iter()
            .any(|b| b.is_ascii_control() || matches!(b, b'"' | b'\\' | b'#'))
    {
        return Err("invalid_request");
    };
    Ok(p)
}
fn work(r: &[u8]) -> R<&[u8]> {
    let p = string(args(r), b"mangaId")?
        .strip_prefix(b"buondua:")
        .ok_or("invalid_request")?;
    let p = local(p)?;
    if p.len() < 2 || p.contains(&b'?') {
        return Err("invalid_request");
    };
    Ok(p)
}
fn get(p: &[u8]) -> R<Node> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    let mut u = [0; 8192];
    let u = absolute(p, &mut u)?;
    if !u.starts_with(BASE) || u.get(BASE.len()) != Some(&b'/') {
        return Err("invalid_request");
    };
    let n = koma_source_sdk::build_get_request(http_req_buf(), u, None, &[])
        .ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    let r = &http_out()[..n];
    let n = protocol::body(r, body_buf())?;
    Node::parse(&body_buf()[..n])
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(c) => write_error(
            op,
            c,
            match c {
                "auth_required" => "网站要求登录后访问。",
                "forbidden" => "网站拒绝了本次请求，请在原网页确认访问状态。",
                "not_found" => "网站未提供该相册，请在原网页确认是否已移除。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "chapter_unavailable" => "此网页没有提供相册图片，请在原网页确认。",
                _ => "相册获取失败，请稍后重试或打开原网页核对。",
            },
        ),
    }
}
fn split(r: &[u8]) -> bool {
    let s = json_object_member(r, b"settings").unwrap_or(b"{}");
    !matches!(
        json_object_member(s, b"pref_split_pages"),
        Some(b"false" | b"\"false\"")
    )
}
fn article_cover<'a>(article: &Node, path: &[u8], buf: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut owner_buf = [0; 4096];
    let owner = article.select(br#"head meta[property="og:url"]"#).ok()?;
    let owner = owner.attr(b"content", &mut owner_buf).ok()?;
    let owner = owner.strip_prefix(BASE)?;
    if owner != path && owner.strip_suffix(b"?page=1") != Some(path) {
        return None;
    }
    let image = article.select(br#"head meta[property="og:image"]"#).ok()?;
    let image = image.attr(b"content", buf).ok()?;
    let authority = image.strip_prefix(b"https://")?.split(|b| *b == b'/').next()?;
    if authority == b"buondua.com" || (authority.ends_with(b".buondua.com")
        && authority.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))) {
        Some(image)
    } else {
        None
    }
}

fn write_cover(out: &mut [u8], n: &mut usize, cover: &[u8]) -> R<()> {
    let mut buf = [0; 4096];
    let cover = absolute(cover, &mut buf)?;
    w(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, cover)?;
    w(out, n, b"}")
}

fn card(out: &mut [u8], n: &mut usize, p: &[u8], t: &[u8], cover: Option<&[u8]>) -> R<()> {
    let p = local(p)?;
    if t.is_empty() || p.contains(&b'?') {
        return Err("parse_error");
    };
    w(out, n, b"{\"id\":\"buondua:")?;
    w(out, n, p)?;
    w(out, n, b"\",\"title\":")?;
    q(out, n, t)?;
    w(out, n, b",\"contentRating\":\"nsfw\"")?;
    if let Some(c) = cover {
        write_cover(out, n, c)?
    }
    w(out, n, b"}")
}
fn paging(out: &mut [u8], n: &mut usize, page: usize, more: bool) -> R<()> {
    w(out, n, b"],\"page\":{\"nextCursor\":")?;
    if more {
        w(out, n, b"\"")?;
        num(out, n, page + 1)?;
        w(out, n, b"\"")?
    } else {
        w(out, n, b"null")?
    };
    w(
        out,
        n,
        if more {
            b",\"hasMore\":true}}"
        } else {
            b",\"hasMore\":false}}"
        },
    )
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let a = args(r);
    let page = request_page_number(r);
    if page > 100000 {
        return Err("invalid_request");
    };
    let mut qb = [0; 16384];
    let query = if search {
        protocol::text(a, b"query", &mut qb)?.unwrap_or(b"")
    } else {
        b""
    };
    if query.starts_with(b"https://buondua.com/") {
        let p = local(query)?;
        let d = get(p)?;
        let mut t = [0; 8192];
        let t = d.select(b".article-header")?.text(&mut t)?;
        let t = title(t);
        let out = payload_buf();
        let mut n = 0;
        w(out, &mut n, b"{\"items\":[")?;
        let mut cover_buf = [0; 4096];
        let cover = article_cover(&d, p, &mut cover_buf);
        card(out, &mut n, p, t, cover)?;
        paging(out, &mut n, page, false)?;
        return Ok(n);
    }
    let mut p = [0; 8192];
    let mut n = 0;
    if !query.is_empty() {
        w(&mut p, &mut n, b"/?search=")?;
        if !write_url_encoded(&mut p, &mut n, query) {
            return Err("capacity_exceeded");
        };
        w(&mut p, &mut n, b"&start=")?;
    } else {
        let filter = json_object_member(a, b"filters").unwrap_or(b"{}");
        let tag = protocol::raw_string(filter, b"tag")?.unwrap_or(b"");
        if !tag.is_empty() {
            if !tag
                .iter()
                .all(|x| x.is_ascii_alphanumeric() || matches!(x, b'-' | b'_'))
            {
                return Err("invalid_request");
            };
            w(&mut p, &mut n, b"/tag/")?;
            w(&mut p, &mut n, tag)?;
            w(&mut p, &mut n, b"&start=")?;
        } else {
            w(
                &mut p,
                &mut n,
                if protocol::raw_string(a, b"listingId")? == Some(b"latest") {
                    b"/?start="
                } else {
                    b"/hot?start="
                },
            )?;
        }
    }
    num(&mut p, &mut n, (page - 1) * 20)?;
    let d = get(&p[..n])?;
    let a = Nodes::get(&d, b".blog > div")?;
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    w(out, &mut n, b"{\"items\":[")?;
    for i in 0..a.n {
        let c = a.at(i);
        let Ok(l) = c.select(b".item-content .item-link") else {
            continue;
        };
        let mut h = [0; 4096];
        let h = l.attr(b"href", &mut h)?;
        let mut t = [0; 8192];
        let t = l.text(&mut t)?;
        let mut cb = [0; 4096];
        let im = c.select(b"img");
        let cover = if let Ok(ref im) = im {
            Some(im.attr(b"src", &mut cb)?)
        } else {
            None
        };
        if count > 0 {
            w(out, &mut n, b",")?
        };
        card(out, &mut n, h, t, cover)?;
        count += 1
    }
    let more = d.select(b".pagination-next:not([disabled])").is_ok();
    paging(out, &mut n, page, more)?;
    Ok(n)
}
fn title(s: &[u8]) -> &[u8] {
    if let Some(i) = find_subslice(s, b" - ( Page ") {
        let tail = &s[i + 10..];
        if tail.ends_with(b" )")
            && tail[..tail.len() - 2]
                .iter()
                .all(|c| c.is_ascii_digit() || matches!(c, b' ' | b'/'))
        {
            return trim_ascii(&s[..i]);
        }
    }
    s
}
fn details(r: &[u8]) -> R<usize> {
    let p = work(r)?;
    let d = get(p)?;
    let mut t = [0; 8192];
    let t = d.select(b".article-header")?.text(&mut t)?;
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"manga\":{\"id\":\"buondua:")?;
    w(out, &mut n, p)?;
    w(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, title(t))?;
    let mut cover_buf = [0; 4096];
    if let Some(cover) = article_cover(&d, p, &mut cover_buf) {
        write_cover(out, &mut n, cover)?;
    }
    w(
        out,
        &mut n,
        b",\"contentRating\":\"nsfw\",\"description\":\"",
    )?;
    let info = Nodes::get(&d, b".article-info > strong")?;
    let mut any = false;
    for i in 0..info.n {
        let mut b = [0; 8192];
        let a = info.at(i);
        let t = a.text(&mut b)?;
        let mut pos = 0;
        while pos < t.len() {
            if t[pos..].starts_with(b"Buondua") {
                pos += 7
            } else {
                if !append_json_escaped(out, &mut n, &t[pos..pos + 1]) {
                    return Err("capacity_exceeded");
                };
                any = true;
                pos += 1
            }
        }
    }
    let links = Nodes::get(&d, b".article-links a[href]")?;
    if links.n > 0 && any {
        w(out, &mut n, b"\\n\\n")?
    }
    for i in 0..links.n {
        let a = links.at(i);
        let mut t = [0; 4096];
        let t = a.text(&mut t)?;
        let mut h = [0; 4096];
        let h = a.attr(b"href", &mut h)?;
        if i > 0 {
            w(out, &mut n, b"\\n")?
        };
        for x in [b"[".as_slice(), t, b"](", h, b")"] {
            if !append_json_escaped(out, &mut n, x) {
                return Err("capacity_exceeded");
            }
        }
        any = true
    }
    let codes = Nodes::get(&d, b"code")?;
    if codes.n > 0 && any {
        w(out, &mut n, b"\\n\\n")?
    }
    for i in 0..codes.n {
        let a = codes.at(i);
        let mut b = [0; 4096];
        let t = a.text(&mut b)?;
        if i > 0 {
            w(out, &mut n, b" ")?
        };
        if !append_json_escaped(out, &mut n, t) {
            return Err("capacity_exceeded");
        }
    }
    w(out, &mut n, b"\",\"tags\":[")?;
    let tags = Nodes::get(&d, b".article-tags .tags > .tag")?;
    for i in 0..tags.n {
        let a = tags.at(i);
        let mut b = [0; 2048];
        let t = a.text(&mut b)?;
        let t = if let Some(k) = t.iter().position(|x| *x == b'#') {
            &t[k + 1..]
        } else {
            t
        };
        if i > 0 {
            w(out, &mut n, b",")?
        };
        q(out, &mut n, t)?
    }
    w(out, &mut n, b"],\"links\":[{\"kind\":\"source\",\"url\":\"")?;
    w(out, &mut n, BASE)?;
    w(out, &mut n, p)?;
    w(out, &mut n, b"\"}]}}")?;
    Ok(n)
}
fn last(d: &Node, p: &[u8]) -> R<usize> {
    let a = Nodes::get(d, b"nav.pagination:first-of-type a.pagination-next")?;
    if a.n == 0 {
        return Ok(1);
    }
    let node = a.at(a.n - 1);
    let mut h = [0; 8192];
    let h = node.attr(b"href", &mut h)?;
    let Ok(h) = local(h) else { return Ok(1) };
    let Some(k) = find_subslice(h, b"?page=") else {
        return Ok(1);
    };
    if &h[..k] != p {
        return Err("identity_mismatch");
    };
    let num = &h[k + 6..];
    let n = positive(num).map_err(|_| "parse_error")?;
    if n > 500 {
        return Err("capacity_exceeded");
    };
    Ok(n)
}
fn catalogue_max(d: &Node, p: &[u8]) -> R<usize> {
    let max = last(d, p)?;
    let mut b = [0; 8192];
    let t = d.select(b".article-header")?.text(&mut b)?;
    if let Some(k) = find_subslice(t, b" - ( Page ") {
        let tail = t[k + 10..].strip_suffix(b" )").ok_or("parse_error")?;
        let at = find_subslice(tail, b" / ").ok_or("parse_error")?;
        if positive(&tail[..at]).map_err(|_| "parse_error")? != 1
            || positive(&tail[at + 3..]).map_err(|_| "parse_error")? != max
        {
            return Err("parse_error");
        }
    }
    Ok(max)
}
fn upload_date(d: &Node, out: &mut [u8; 19]) -> R<Option<usize>> {
    let Ok(node) = d.select(b".article-info > small") else {
        return Ok(None);
    };
    let mut b = [0; 128];
    let v = node.text(&mut b)?;
    if v.len() != 16
        || v[2] != b':'
        || v[5] != b' '
        || v[8] != b'-'
        || v[11] != b'-'
        || !v
            .iter()
            .enumerate()
            .all(|(i, c)| [2, 5, 8, 11].contains(&i) || c.is_ascii_digit())
    {
        return Ok(None);
    }
    let mut n = 0;
    w(out, &mut n, &v[12..16])?;
    w(out, &mut n, b"-")?;
    w(out, &mut n, &v[9..11])?;
    w(out, &mut n, b"-")?;
    w(out, &mut n, &v[6..8])?;
    w(out, &mut n, b"T")?;
    w(out, &mut n, &v[..5])?;
    w(out, &mut n, b":00")?;
    Ok(Some(n))
}
fn chapter(
    out: &mut [u8],
    n: &mut usize,
    p: &[u8],
    page: Option<usize>,
    date: Option<&[u8]>,
) -> R<()> {
    w(out, n, b"{\"id\":\"buondua-read:")?;
    w(out, n, p)?;
    w(out, n, b":")?;
    if let Some(x) = page {
        num(out, n, x)?
    } else {
        w(out, n, b"gallery")?
    };
    w(out, n, b"\",\"mangaId\":\"buondua:")?;
    w(out, n, p)?;
    w(out, n, b"\",\"title\":\"")?;
    if let Some(x) = page {
        w(out, n, b"Page ")?;
        num(out, n, x)?
    } else {
        w(out, n, b"Gallery")?
    };
    w(out, n, b"\",\"chapterNumber\":")?;
    num(out, n, page.unwrap_or(0))?;
    w(
        out,
        n,
        b",\"volumeNumber\":null,\"language\":\"all\",\"publishedAt\":",
    )?;
    if let Some(date) = date {
        q(out, n, date)?
    } else {
        w(out, n, b"null")?
    };
    w(out, n, b",\"url\":\"")?;
    w(out, n, BASE)?;
    w(out, n, p)?;
    if let Some(x) = page {
        w(out, n, b"?page=")?;
        num(out, n, x)?
    };
    w(out, n, b"\"}")
}
fn chapters(r: &[u8]) -> R<usize> {
    let p = work(r)?;
    let d = get(p)?;
    d.select(b".article-header")?;
    let max = catalogue_max(&d, p)?;
    let mut db = [0; 19];
    let dn = upload_date(&d, &mut db)?;
    let date = dn.map(|n| &db[..n]);
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"chapters\":[")?;
    if split(r) {
        for x in (1..=max).rev() {
            if x != max {
                w(out, &mut n, b",")?
            };
            chapter(out, &mut n, p, Some(x), date)?
        }
    } else {
        chapter(out, &mut n, p, None, date)?
    };
    w(out, &mut n, b"]")?;
    if protocol::feature(r, b"source_chapter_exclusions_v1") {
        // The setting replaces one complete, verified catalogue representation.
        // Hide only the exact alternate-mode IDs; downloads/progress remain Host-owned.
        w(out, &mut n, b",\"excludedChapterIds\":[")?;
        if split(r) {
            w(out, &mut n, b"\"buondua-read:")?;
            w(out, &mut n, p)?;
            w(out, &mut n, b":gallery\"")?;
        } else {
            for x in 1..=max {
                if x > 1 {
                    w(out, &mut n, b",")?
                };
                w(out, &mut n, b"\"buondua-read:")?;
                w(out, &mut n, p)?;
                w(out, &mut n, b":")?;
                num(out, &mut n, x)?;
                w(out, &mut n, b"\"")?
            }
        }
        w(out, &mut n, b"]")?;
    }
    w(
        out,
        &mut n,
        b",\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn append_images(
    out: &mut [u8],
    n: &mut usize,
    d: &Node,
    p: &[u8],
    mode: &[u8],
    ordinal: &mut usize,
) -> R<()> {
    let a = Nodes::get(d, b".article-fulltext img")?;
    if a.n == 0 {
        d.select(b".article-header")?;
        d.select(b".article-fulltext")?;
        return Err("chapter_unavailable");
    };
    for i in 0..a.n {
        let a = a.at(i);
        let mut u = [0; 8192];
        let u = a.attr(b"src", &mut u)?;
        let mut abs = [0; 8192];
        let u = absolute(u, &mut abs)?;
        if *ordinal > 0 {
            w(out, n, b",")?
        };
        w(out, n, b"{\"id\":\"buondua-page:")?;
        w(out, n, p)?;
        w(out, n, b":")?;
        w(out, n, mode)?;
        w(out, n, b":")?;
        num(out, n, *ordinal)?;
        w(out, n, b"\",\"index\":")?;
        num(out, n, *ordinal)?;
        w(out, n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, n, u)?;
        w(out, n, b"}}")?;
        *ordinal += 1
    }
    Ok(())
}
fn pages(r: &[u8]) -> R<usize> {
    let c = string(args(r), b"chapterId")?
        .strip_prefix(b"buondua-read:")
        .ok_or("invalid_request")?;
    let k = c
        .iter()
        .rposition(|x| *x == b':')
        .ok_or("invalid_request")?;
    let p = local(&c[..k])?;
    if p.contains(&b'?') {
        return Err("invalid_request");
    };
    let mode = &c[k + 1..];
    if let Some(m) = protocol::raw_string(args(r), b"mangaId")? {
        if m.strip_prefix(b"buondua:") != Some(p) {
            return Err("identity_mismatch");
        }
    }
    let mut url = [0; 8192];
    let mut un = 0;
    w(&mut url, &mut un, p)?;
    let gallery = mode == b"gallery";
    if !gallery {
        let x = positive(mode)?;
        w(&mut url, &mut un, b"?page=")?;
        num(&mut url, &mut un, x)?;
    }
    let d = get(&url[..un])?;
    if !gallery {
        // The last page disables its Next/End links; those links cannot prove
        // catalogue size here. Validate the real displayed page, never clamp it.
        let mut title = [0; 8192];
        let text = d.select(b".article-header")?.text(&mut title)?;
        if let Some(k) = find_subslice(text, b" - ( Page ") {
            let t = &text[k + 10..];
            let end = t.strip_suffix(b" )").ok_or("parse_error")?;
            let slash = find_subslice(end, b" / ").ok_or("parse_error")?;
            let current = positive(&end[..slash]).map_err(|_| "parse_error")?;
            let total = positive(&end[slash + 3..]).map_err(|_| "parse_error")?;
            if current != positive(mode)? || current > total {
                return Err("identity_mismatch");
            }
        } else if positive(mode)? != 1 {
            return Err("identity_mismatch");
        }
    }
    let out = payload_buf();
    let mut n = 0;
    let mut ordinal = 0;
    w(out, &mut n, b"{\"pages\":[")?;
    append_images(out, &mut n, &d, p, mode, &mut ordinal)?;
    if gallery {
        let max = catalogue_max(&d, p)?;
        drop(d);
        for x in 2..=max {
            let mut un = 0;
            w(&mut url, &mut un, p)?;
            w(&mut url, &mut un, b"?page=")?;
            num(&mut url, &mut un, x)?;
            let d = get(&url[..un])?;
            append_images(out, &mut n, &d, p, mode, &mut ordinal)?
        }
    }
    w(out, &mut n, b"]}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("search", c, "invalid request");
    }
    if !protocol::feature(r, b"source_rate_limit_buondua_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return write_error(
            "search",
            "host_feature_required",
            "请升级App以支持此书源的请求速率限制。",
        );
    }
    result("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga_list", c, "invalid request");
    }
    if !protocol::feature(r, b"source_rate_limit_buondua_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return write_error(
            "get_manga_list",
            "host_feature_required",
            "请升级App以支持此书源的请求速率限制。",
        );
    }
    result("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga", c, "invalid request");
    }
    if !protocol::feature(r, b"source_rate_limit_buondua_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return write_error(
            "get_manga",
            "host_feature_required",
            "请升级App以支持此书源的请求速率限制。",
        );
    }
    result("get_manga", details(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_chapters", c, "invalid request");
    }
    if !protocol::feature(r, b"source_rate_limit_buondua_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return write_error(
            "get_chapters",
            "host_feature_required",
            "请升级App以支持此书源的请求速率限制。",
        );
    }
    result("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_pages", c, "invalid request");
    }
    if !protocol::feature(r, b"source_rate_limit_buondua_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return write_error(
            "get_pages",
            "host_feature_required",
            "请升级App以支持此书源的请求速率限制。",
        );
    }
    result("get_pages", pages(r))
}
fn fixed(op: &str, p: &[u8]) -> u32 {
    payload_buf()[..p.len()].copy_from_slice(p);
    write_success_payload(op, p.len())
}
fn run_get_listings(_: &[u8]) -> u32 {
    fixed("get_listings",b"{\"listings\":[{\"id\":\"popular\",\"name\":\"Popular\"},{\"id\":\"latest\",\"name\":\"Latest\"}]}")
}
fn run_get_filters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_filters", c, "invalid request");
    }
    fixed(
        "get_filters",
        if protocol::feature(r, b"source_filter_scope_header_v1") {
            b"{\"filters\":[{\"type\":\"header\",\"scope\":\"browse\",\"name\":\"NOTE: Ignored if using text search!\"},{\"id\":\"tag\",\"name\":\"Tag ID\",\"type\":\"text\",\"scope\":\"browse\"}]}"
        } else {
            b"{\"filters\":[{\"id\":\"tag\",\"name\":\"Tag ID\",\"type\":\"text\"}]}"
        },
    )
}
fn run_get_settings(_: &[u8]) -> u32 {
    fixed("get_settings",b"{\"settings\":[{\"id\":\"pref_split_pages\",\"name\":\"Split into multiple pages\",\"kind\":\"toggle\",\"default\":true,\"description\":\"On: Multiple pages; Off: Single gallery\"}]}")
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    write_error(
        "get_image_request",
        "not_implemented",
        "ordinary image pipeline",
    )
}
koma_source_sdk::koma_source_exports!("buondua_source");
