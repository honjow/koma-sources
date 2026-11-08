#![no_std]
// Independent port of Keiyoushi CosplayTele.kt/Dto.kt/Filters.kt, Apache-2.0,
// ref de743aed846da53e00c45ab26ee0e77f7bb4355b. NSFW is the reference's rating.
// Public galleries use the ordinary image pipeline, no account/custom headers.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
mod dom;
use dom::*;
use koma_source_sdk::source_protocol as protocol;
const BASE: &[u8] = b"https://cosplaytele.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.cosplaytele.koma",
    name: "CosplayTele",
    version: "0.1.0",
    api_version: "0.2",
    language: "all",
    author: "Koma",
    description: "CosplayTele public photo galleries. Contains adult material.",
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
    settings: false,
    image_request: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:4*1024*1024,body:4*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
const DEFAULT_CATEGORIES: [(&[u8], &[u8]); 4] = [
    (b"", b"All"),
    (b"category/nude", b"Cosplay Nude"),
    (b"category/no-nude", b"Cosplay Ero"),
    (b"category/cosplay", b"Cosplay"),
];
fn absolute<'a>(v: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if v.starts_with(b"https://") {
        w(out, &mut n, v)?
    } else if v.starts_with(b"//") {
        w(out, &mut n, b"https:")?;
        w(out, &mut n, v)?
    } else if v.starts_with(b"/") {
        w(out, &mut n, BASE)?;
        w(out, &mut n, v)?
    } else {
        return Err("parse_error");
    };
    Ok(&out[..n])
}
fn safe_path(v: &[u8]) -> bool {
    v.starts_with(b"/")
        && !v.starts_with(b"//")
        && !v.windows(2).any(|x| x == b"..")
        && !v
            .iter()
            .any(|b| b.is_ascii_control() || matches!(b, b'"' | b'\\' | b'?' | b'#'))
}
fn local(v: &[u8]) -> R<&[u8]> {
    let p = v
        .strip_prefix(BASE)
        .or_else(|| v.strip_prefix(b"https://www.cosplaytele.com"))
        .unwrap_or(v);
    if safe_path(p) {
        Ok(p)
    } else {
        Err("invalid_request")
    }
}
fn work<'a>(r: &'a [u8], key: &[u8], prefix: &[u8]) -> R<&'a [u8]> {
    let id = string(args(r), key)?
        .strip_prefix(prefix)
        .ok_or("invalid_request")?;
    if !safe_path(id) {
        return Err("invalid_request");
    };
    if id == b"/"
        || id.starts_with(b"/category/")
        || id.starts_with(b"/tag/")
        || id.starts_with(b"/page/")
    {
        return Err("invalid_request");
    };
    Ok(id)
}
fn get(p: &[u8]) -> R<usize> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    let mut u = [0; 8192];
    let u = absolute(p, &mut u)?;
    let q = koma_source_sdk::build_get_request(http_req_buf(), u, Some(BASE), &[])
        .ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..q], http_out()).map_err(|_| "network_error")?;
    protocol::body(&http_out()[..n], body_buf())
}
fn fetch(p: &[u8]) -> R<Node> {
    let n = get(p)?;
    Node::parse(&body_buf()[..n])
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(c) => write_error(
            op,
            c,
            match c {
                "forbidden" => "网站拒绝了本次请求，请在原网页确认访问状态。",
                "auth_required" => "此内容需要登录，请先打开原网页登录。",
                "not_found" => "网站未提供该相册，请在原网页确认是否已移除。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "chapter_unavailable" => "相册网页没有提供图片，请打开原网页确认。",
                _ => "相册获取失败，请稍后重试或打开原网页核对。",
            },
        ),
    }
}
fn card(o: &mut [u8], n: &mut usize, p: &[u8], title: &[u8], cover: Option<&[u8]>) -> R<()> {
    let p = local(p)?;
    if title.is_empty() {
        return Err("parse_error");
    };
    w(o, n, b"{\"id\":\"cosplaytele:")?;
    w(o, n, p)?;
    w(o, n, b"\",\"title\":")?;
    q(o, n, title)?;
    w(o, n, b",\"contentRating\":\"nsfw\"")?;
    if let Some(c) = cover {
        let mut b = [0; 4096];
        let c = absolute(c, &mut b)?;
        w(o, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(o, n, c)?;
        w(o, n, b"}")?
    }
    w(o, n, b"}")
}
fn paging(o: &mut [u8], n: &mut usize, page: usize, more: bool) -> R<()> {
    w(o, n, b"],\"page\":{\"nextCursor\":")?;
    if more {
        w(o, n, b"\"")?;
        num(o, n, page + 1)?;
        w(o, n, b"\"")?
    } else {
        w(o, n, b"null")?
    };
    w(
        o,
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
    let pop = !search && protocol::raw_string(a, b"listingId")? == Some(b"popular");
    if pop {
        return popular(page);
    }
    let filters = json_object_member(a, b"filters").unwrap_or(b"{}");
    let mut category = protocol::raw_string(filters, b"category")?.unwrap_or(b"");
    let mut qb = [0; 16384];
    let mut query = if search {
        protocol::text(a, b"query", &mut qb)?.unwrap_or(b"")
    } else {
        b""
    };
    if query.starts_with(b"https://cosplaytele.com/")
        || query.starts_with(b"https://www.cosplaytele.com/")
    {
        let route = local(query)?;
        if route.starts_with(b"/tag/") || route.starts_with(b"/category/") {
            let route = if let Some(i) = find_subslice(route, b"/page/") {
                &route[..i]
            } else {
                route
            };
            category = &route[1..];
            query = b"";
        } else {
            let doc = fetch(route)?;
            let mut title = [0; 8192];
            let title = doc.select(b".entry-title")?.text(&mut title)?;
            let mut cover = [0; 4096];
            let cover_n = optional_cover(&doc, route, &mut cover)?;
            let out = payload_buf();
            let mut n = 0;
            w(out, &mut n, b"{\"items\":[")?;
            card(out, &mut n, route, title, cover_n.map(|n| &cover[..n]))?;
            paging(out, &mut n, page, false)?;
            return Ok(n);
        }
    }
    let mut p = [0; 8192];
    let mut pn = 0;
    if !category.is_empty() {
        if !category.starts_with(b"category/") && !category.starts_with(b"tag/") {
            return Err("invalid_request");
        };
        w(&mut p, &mut pn, b"/")?;
        w(&mut p, &mut pn, category)?;
        if !safe_path(&p[..pn]) {
            return Err("invalid_request");
        };
        if !p[..pn].ends_with(b"/") {
            w(&mut p, &mut pn, b"/")?
        }
    }
    if pn == 0 {
        w(&mut p, &mut pn, b"/")?
    };
    w(&mut p, &mut pn, b"page/")?;
    num(&mut p, &mut pn, page)?;
    w(&mut p, &mut pn, b"/")?;
    if !query.is_empty() {
        w(&mut p, &mut pn, b"?s=")?;
        if !write_url_encoded(&mut p, &mut pn, query) {
            return Err("capacity_exceeded");
        }
    }
    let d = fetch(&p[..pn])?;
    let cards = Nodes::get(&d, b"main div.box")?;
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let c = cards.at(i);
        let l = c.select(b"h5 a")?;
        let mut h = [0; 4096];
        let h = l.attr(b"href", &mut h)?;
        let mut t = [0; 8192];
        let t = l.text(&mut t)?;
        let mut cb = [0; 4096];
        let image = c.select(b"img");
        let cover = if let Ok(ref im) = image {
            Some(im.attr(b"src", &mut cb)?)
        } else {
            None
        };
        if i > 0 {
            w(out, &mut n, b",")?
        };
        card(out, &mut n, h, t, cover)?;
    }
    let more = d.select(b".next.page-number").is_ok();
    paging(out, &mut n, page, more)?;
    Ok(n)
}
fn decoded<'a>(s: &[u8], key: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let raw = extract_json_string(s, key).ok_or("parse_error")?;
    koma_source_sdk::plain_text::decode_json_content(raw, b)
}
fn popular(page: usize) -> R<usize> {
    let mut p = [0; 1024];
    let mut pn = 0;
    w(
        &mut p,
        &mut pn,
        b"/wp-json/wordpress-popular-posts/v1/popular-posts?offset=",
    )?;
    // Preserve the fixed reference: first popular page starts at offset 20.
    num(&mut p, &mut pn, page * 20)?;
    w(&mut p,&mut pn,b"&limit=20&range=last7days&embed=true&_embed=wp%3Afeaturedmedia&_fields=title%2Clink%2C_embedded%2C_links.wp%3Afeaturedmedia")?;
    let len = get(&p[..pn])?;
    let body = &body_buf()[..len];
    if body.first() != Some(&b'[') {
        return Err("parse_error");
    };
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    w(out, &mut n, b"{\"items\":[")?;
    // JsonArrayIter expects an object member. Use a separate bounded wrapper.
    static mut WRAPPED: [u8; 4 * 1024 * 1024] = [0; 4 * 1024 * 1024];
    let wrap = unsafe { &mut *core::ptr::addr_of_mut!(WRAPPED) };
    let mut wn = 0;
    w(wrap, &mut wn, b"{\"items\":")?;
    w(wrap, &mut wn, body)?;
    w(wrap, &mut wn, b"}")?;
    let mut iter = JsonArrayIter::new(&wrap[..wn], b"items").ok_or("parse_error")?;
    while let Some(v) = iter.next_object() {
        let mut h = [0; 4096];
        let h = decoded(v, b"link", &mut h)?;
        let title = json_object_member(v, b"title").ok_or("parse_error")?;
        let mut t = [0; 8192];
        let t = decoded(title, b"rendered", &mut t)?;
        let td = Node::parse(t)?;
        let mut tt = [0; 8192];
        let title = td.text(&mut tt)?;
        let mut cb = [0; 4096];
        let cover = if extract_json_string(v, b"source_url").is_some() {
            Some(decoded(v, b"source_url", &mut cb)?)
        } else {
            None
        };
        if count > 0 {
            w(out, &mut n, b",")?
        };
        card(out, &mut n, h, title, cover)?;
        count += 1;
    }
    paging(out, &mut n, page, count >= 20)?;
    Ok(n)
}
// A URL-opened article has no listing thumbnail. Resolve only its declared
// WordPress post endpoint and that post's actual featured_media selection.
// This is an approved cover completion beyond the reference's detail parser;
// never substitute a gallery image or a related article. Auxiliary failures
// preserve the previously available metadata and do not gate chapters/pages.
fn strict_member<'a>(doc: &'a [u8], key: &[u8]) -> R<&'a [u8]> {
    protocol::direct(doc, key)
        .map_err(|_| "parse_error")?
        .ok_or("parse_error")
}
fn strict_text<'a>(doc: &[u8], key: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    protocol::text(doc, key, out)
        .map_err(|_| "parse_error")?
        .ok_or("parse_error")
}
fn strict_number<'a>(doc: &'a [u8], key: &[u8]) -> R<&'a [u8]> {
    let n = strict_member(doc, key)?.trim_ascii();
    if n.is_empty() || n.len() > 20 || !n.iter().all(u8::is_ascii_digit) {
        return Err("parse_error");
    }
    Ok(n)
}
fn same_work(a: &[u8], b: &[u8]) -> R<bool> {
    let a = local(a).map_err(|_| "parse_error")?;
    Ok(a.strip_suffix(b"/").unwrap_or(a) == b.strip_suffix(b"/").unwrap_or(b))
}
fn cover_metadata(d: &Node, p: &[u8], out: &mut [u8]) -> R<Option<usize>> {
    let declarations = Nodes::get(d, b"link[rel='alternate'][type='application/json']")?;
    if declarations.n == 0 {
        return Ok(None);
    }
    let mut api = [0; 4096];
    let mut an = 0;
    for i in 0..declarations.n {
        let mut href = [0; 4096];
        let h = declarations.at(i).attr(b"href", &mut href)?;
        let path = local(h).map_err(|_| "parse_error")?;
        let id = path
            .strip_prefix(b"/wp-json/wp/v2/posts/")
            .ok_or("parse_error")?;
        if id.is_empty() || id.len() > 20 || !id.iter().all(u8::is_ascii_digit) {
            return Err("parse_error");
        }
        if an > 0 && &api[..an] != path {
            return Err("parse_error");
        }
        an = 0;
        w(&mut api, &mut an, path)?;
    }
    let path = &api[..an];
    let declared_id = &path[b"/wp-json/wp/v2/posts/".len()..];
    let mut request = [0; 8192];
    let mut rn = 0;
    w(&mut request, &mut rn, path)?;
    // Same embedding relation already used by the adopted popular-list API.
    w(&mut request, &mut rn, b"?_embed=wp%3Afeaturedmedia")?;
    let size = get(&request[..rn])?;
    let data = &body_buf()[..size];
    protocol::complete(data).map_err(|_| "parse_error")?;
    if strict_number(data, b"id")? != declared_id {
        return Err("parse_error");
    }
    let mut link = [0; 4096];
    let link = strict_text(data, b"link", &mut link)?;
    if !same_work(link, p)? {
        return Err("parse_error");
    }
    let mut slug = [0; 1024];
    let slug = strict_text(data, b"slug", &mut slug)?;
    let route = p.strip_suffix(b"/").unwrap_or(p);
    if route.rsplit(|c| *c == b'/').next() != Some(slug) {
        return Err("parse_error");
    }
    let featured = strict_number(data, b"featured_media")?;
    if featured == b"0" {
        return Ok(None);
    }
    let embedded = strict_member(data, b"_embedded")?;
    let media = strict_member(embedded, b"wp:featuredmedia")?;
    let mut items = JsonArrayIter::from_raw_array(media).ok_or("parse_error")?;
    let mut matched = None;
    while let Some(item) = items.next_object() {
        if strict_number(item, b"id")? != featured {
            continue;
        }
        if matched.is_some() {
            return Err("parse_error");
        }
        let mut typ = [0; 128];
        if strict_text(item, b"media_type", &mut typ)? != b"image" {
            return Err("parse_error");
        }
        let mut image = [0; 4096];
        let image = strict_text(item, b"source_url", &mut image)?;
        if image.is_empty() {
            return Err("parse_error");
        }
        let url = absolute(image, out)?;
        matched = Some(url.len());
        // wp:attached-to identifies the original upload's parent. A different
        // article may legitimately reuse this image as its featured cover;
        // post id/link + featured_media == embedded id are the binding.
    }
    matched.map(Some).ok_or("parse_error")
}
fn optional_cover(d: &Node, p: &[u8], out: &mut [u8]) -> R<Option<usize>> {
    match cover_metadata(d, p, out) {
        Ok(v) => Ok(v),
        Err("cancelled") => Err("cancelled"),
        Err(_) => {
            host::log_info(b"cosplaytele cover_metadata_unavailable");
            Ok(None)
        }
    }
}
fn details(r: &[u8]) -> R<usize> {
    let p = work(r, b"mangaId", b"cosplaytele:")?;
    let d = fetch(p)?;
    let mut t = [0; 8192];
    let t = d.select(b".entry-title")?.text(&mut t)?;
    if t.is_empty() {
        return Err("parse_error");
    };
    let mut cover = [0; 4096];
    let cover_n = optional_cover(&d, p, &mut cover)?;
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"manga\":{\"id\":\"cosplaytele:")?;
    w(out, &mut n, p)?;
    w(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, t)?;
    if let Some(cn) = cover_n {
        w(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, &cover[..cn])?;
        w(out, &mut n, b"}")?;
    }
    w(out, &mut n, b",\"description\":")?;
    q(out, &mut n, t)?;
    w(
        out,
        &mut n,
        b",\"status\":\"completed\",\"contentRating\":\"nsfw\",\"tags\":[",
    )?;
    let tags = Nodes::get(&d, b"#main a")?;
    let mut count = 0;
    for i in 0..tags.n {
        let a = tags.at(i);
        let mut h = [0; 4096];
        let h = a.attr(b"href", &mut h).unwrap_or(b"");
        if let Ok(h) = local(h) {
            if !h.starts_with(b"/tag/") && !h.starts_with(b"/category/") {
                continue;
            };
            let mut t = [0; 2048];
            let t = a.text(&mut t)?;
            if count > 0 {
                w(out, &mut n, b",")?
            };
            q(out, &mut n, t)?;
            count += 1
        }
    }
    w(out, &mut n, b"],\"links\":[{\"kind\":\"source\",\"url\":\"")?;
    w(out, &mut n, BASE)?;
    w(out, &mut n, p)?;
    w(out, &mut n, b"\"}]}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> R<usize> {
    let p = work(r, b"mangaId", b"cosplaytele:")?;
    let d = fetch(p)?;
    d.select(b".entry-title")?;
    let mut date = [0; 128];
    let date = if let Ok(t) = d.select(b"time.updated") {
        let v = t.attr(b"datetime", &mut date)?;
        if v.len() >= 10 && v[4] == b'-' && v[7] == b'-' {
            Some(&v[..10])
        } else {
            None
        }
    } else {
        None
    };
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"chapters\":[{\"id\":\"cosplaytele-read:")?;
    w(out, &mut n, p)?;
    w(out, &mut n, b"\",\"mangaId\":\"cosplaytele:")?;
    w(out, &mut n, p)?;
    w(out,&mut n,b"\",\"title\":\"Gallery\",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"all\",\"publishedAt\":")?;
    if let Some(date) = date {
        q(out, &mut n, date)?
    } else {
        w(out, &mut n, b"null")?
    };
    w(out, &mut n, b",\"url\":\"")?;
    w(out, &mut n, BASE)?;
    w(out, &mut n, p)?;
    w(
        out,
        &mut n,
        b"\"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> R<usize> {
    let p = work(r, b"chapterId", b"cosplaytele-read:")?;
    if let Some(m) = protocol::raw_string(args(r), b"mangaId")? {
        if m.strip_prefix(b"cosplaytele:") != Some(p) {
            return Err("invalid_request");
        }
    }
    let d = fetch(p)?;
    let a = Nodes::get(&d, b".gallery-item img")?;
    if a.n == 0 {
        d.select(b".entry-title")?;
        d.select(b".entry-content")?;
        return Err("chapter_unavailable");
    };
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"pages\":[")?;
    for i in 0..a.n {
        let a = a.at(i);
        let mut u = [0; 4096];
        let u = a.attr(b"src", &mut u)?;
        let mut abs = [0; 4096];
        let u = absolute(u, &mut abs)?;
        if i > 0 {
            w(out, &mut n, b",")?
        };
        w(out, &mut n, b"{\"id\":\"cosplaytele-page:")?;
        w(out, &mut n, p)?;
        w(out, &mut n, b":")?;
        num(out, &mut n, i)?;
        w(out, &mut n, b"\",\"index\":")?;
        num(out, &mut n, i)?;
        w(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, u)?;
        w(out, &mut n, b"}}")?;
    }
    w(out, &mut n, b"]}")?;
    Ok(n)
}
#[derive(Clone, Copy)]
struct FilterOption {
    label: [u8; 1024],
    ln: usize,
    value: [u8; 1024],
    vn: usize,
}
const EMPTY_OPTION: FilterOption = FilterOption {
    label: [0; 1024],
    ln: 0,
    value: [0; 1024],
    vn: 0,
};
static mut OPTIONS: [FilterOption; 512] = [EMPTY_OPTION; 512];
fn option_store(
    options: &mut [FilterOption],
    used: &mut usize,
    label: &[u8],
    value: &[u8],
) -> R<()> {
    if label.len() >= 1024 || value.len() >= 1024 {
        return Err("capacity_exceeded");
    }
    let i = options[..*used]
        .iter()
        .position(|x| &x.label[..x.ln] == label)
        .unwrap_or(*used);
    if i >= options.len() {
        return Err("capacity_exceeded");
    };
    if i == *used {
        *used += 1
    };
    options[i].label[..label.len()].copy_from_slice(label);
    options[i].ln = label.len();
    options[i].value[..value.len()].copy_from_slice(value);
    options[i].vn = value.len();
    Ok(())
}
fn filters(r: &[u8]) -> R<usize> {
    let fetching = protocol::feature(r, b"source_filter_fetching_v1");
    let scope = protocol::feature(r, b"source_filter_scope_header_v1");
    let mut dynamic = None;
    if fetching && json_object_member(args(r), b"fetchFilterData") == Some(b"true") {
        dynamic = Some(fetch(b"/explore-categories/")?)
    }
    let options = unsafe { &mut *core::ptr::addr_of_mut!(OPTIONS) };
    let mut used = 0;
    for (v, l) in DEFAULT_CATEGORIES {
        option_store(options, &mut used, l, v)?;
    }
    if let Some(d) = dynamic {
        let tags = Nodes::get(&d, b"#main a")?;
        for i in 0..tags.n {
            let a = tags.at(i);
            let mut h = [0; 4096];
            let h = a.attr(b"href", &mut h).unwrap_or(b"");
            let Ok(h) = local(h) else { continue };
            if !h.starts_with(b"/category/") && !h.starts_with(b"/tag/") {
                continue;
            };
            let mut t = [0; 2048];
            let t = a.text(&mut t)?;
            if !t.is_empty() {
                option_store(options, &mut used, t, &h[1..])?
            }
        }
    }
    let out = payload_buf();
    let mut n = 0;
    w(
        out,
        &mut n,
        b"{\"supportsFilterFetching\":true,\"filters\":[",
    )?;
    if scope {
        w(
            out,
            &mut n,
            b"{\"type\":\"header\",\"name\":\"NOTE: Only one filter will be applied!\"},",
        )?
    }
    w(out,&mut n,b"{\"id\":\"category\",\"name\":\"Category\",\"type\":\"select\",\"default\":\"\",\"options\":[")?;
    for (i, x) in options[..used].iter().enumerate() {
        if i > 0 {
            w(out, &mut n, b",")?
        };
        w(out, &mut n, b"{\"value\":")?;
        q(out, &mut n, &x.value[..x.vn])?;
        w(out, &mut n, b",\"label\":")?;
        q(out, &mut n, &x.label[..x.ln])?;
        w(out, &mut n, b"}")?
    }
    w(out, &mut n, b"]}]}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("search", c, "invalid request");
    }
    result("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga_list", c, "invalid request");
    }
    result("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga", c, "invalid request");
    }
    result("get_manga", details(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_chapters", c, "invalid request");
    }
    result("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_pages", c, "invalid request");
    }
    result("get_pages", pages(r))
}
fn run_get_filters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_filters", c, "invalid request");
    }
    result("get_filters", filters(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let s=b"{\"listings\":[{\"id\":\"popular\",\"name\":\"Popular\"},{\"id\":\"latest\",\"name\":\"Latest\"}]}";
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    write_error(
        "get_image_request",
        "not_implemented",
        "ordinary image pipeline",
    )
}
koma_source_sdk::koma_source_exports!("cosplaytele_source");
