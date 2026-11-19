use super::*;

const ORDER: &[(&[u8], &[u8])] = &[
    (b"popular", "热门".as_bytes()),
    (b"latest", "最新".as_bytes()),
    (b"top_rated", "最高评分".as_bytes()),
    (b"top_faved", "收藏最多".as_bytes()),
    (b"top_fapped", "站内热度".as_bytes()),
    (b"top_downloaded", "下载最多".as_bytes()),
];

fn feature(req: &[u8], name: &[u8]) -> bool {
    member(req, b"hostFeatures")
        .ok()
        .flatten()
        .and_then(|raw| raw.strip_prefix(b"[")?.strip_suffix(b"]"))
        .is_some_and(|raw| raw.split(|x| *x == b',').any(|v| quoted(trim_ascii(v)).ok() == Some(name)))
}

fn supports_search_filters(req: &[u8]) -> bool {
    feature(req, b"source_filter_search_without_query_v1")
        && feature(req, b"source_filter_scope_header_v1")
        && feature(req, b"source_filter_fetching_v1")
}

fn filter_values(req: &[u8]) -> R<&[u8]> {
    if !supports_search_filters(req) {
        return Ok(b"{}");
    }
    Ok(member(args(req)?, b"filters")?.unwrap_or(b"{}"))
}

fn tag_slug(slug: &[u8]) -> R<&[u8]> {
    if slug.is_empty()
        || slug.len() > 192
        || !slug.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("invalid tag slug");
    }
    Ok(slug)
}

fn add_term(out: &mut [u8], size: &mut usize, term: &[u8]) -> R<()> {
    let mut clean = [0; 2048];
    let term = normalized(term, &mut clean)?;
    if term.is_empty() {
        return Ok(());
    }
    if *size > 0 {
        bytes(out, size, b"+")?;
    }
    bytes(out, size, term)
}

fn read_tags(filters: &[u8], names: &mut [u8], size: &mut usize, first: &mut [u8]) -> R<(usize, usize)> {
    let raw = member(filters, b"tags")?.unwrap_or(b"[]");
    if raw.first() != Some(&b'[') || raw.last() != Some(&b']') {
        return Err("invalid tag selection");
    }
    let mut cursor = ws(raw, 1);
    let mut count = 0;
    let mut first_len = 0;
    while cursor < raw.len() - 1 {
        let end = json_string_end(raw, cursor).or_else(|| {
            cursor = ws(raw, cursor);
            json_string_end(raw, cursor)
        }).ok_or("invalid tag selection")?;
        let mut buf = [0; 2048];
        let value = uq(&raw[cursor..end], &mut buf)?;
        let split = value.iter().position(|b| *b == b':').ok_or("invalid tag identity")?;
        let slug = tag_slug(&value[..split])?;
        if count == 0 {
            first_len = slug.len();
            first[..first_len].copy_from_slice(slug);
        }
        add_term(names, size, &value[split + 1..])?;
        count += 1;
        cursor = ws(raw, end);
        if raw.get(cursor) == Some(&b',') {
            cursor = ws(raw, cursor + 1);
        } else if cursor + 1 != raw.len() {
            return Err("invalid tag selection");
        }
    }
    Ok((count, first_len))
}

#[derive(Clone, Copy)]
enum Route<'a> {
    Popular,
    Latest,
    Search { terms: &'a [u8], popular: bool },
    Tag { slug: &'a [u8], popular: bool },
}
impl Route<'_> {
    fn url<'a>(self, page: usize, out: &'a mut [u8]) -> R<&'a [u8]> {
        let mut n = 0;
        bytes(out, &mut n, WEB)?;
        match self {
            Self::Search { terms, popular } => {
                bytes(out, &mut n, b"/search/?q=")?;
                query_encoded(out, &mut n, terms)?;
                if popular { bytes(out, &mut n, b"&sort=popular")?; }
                bytes(out, &mut n, b"&page=")?;
                wn(out, &mut n, page)?;
                return Ok(&out[..n]);
            },
            Self::Popular => bytes(out, &mut n, b"/language/chinese/popular")?,
            Self::Latest => bytes(out, &mut n, b"/language/chinese")?,
            Self::Tag { slug, popular } => {
                bytes(out, &mut n, b"/tag/")?;
                bytes(out, &mut n, slug)?;
                if popular { bytes(out, &mut n, b"/popular")?; }
            },
        }
        bytes(out, &mut n, b"/pag/")?;
        wn(out, &mut n, page)?;
        bytes(out, &mut n, b"/")?;
        Ok(&out[..n])
    }

    fn strict_language(self) -> bool {
        matches!(self, Self::Popular | Self::Latest)
    }
}

fn push_card(out: &mut [u8], n: &mut usize, count: &mut usize, d: HtmlDescriptor, req: &[u8]) -> R<()> {
    if *count != 0 {
        bytes(out, n, b",")?;
    }
    emit_card(out, n, d, req)?;
    *count += 1;
    Ok(())
}

fn language_matches(d: HtmlDescriptor, strict: bool) -> R<bool> {
    if !strict {
        let mut raw = [0; 256];
        match host::html_attr(d, b"data-languages", &mut raw) {
            Err(-1) | Ok(0) => return Ok(true),
            Err(_) => return Err("language attribute"),
            Ok(_) => {}
        }
    }
    chinese(d)
}

const END: &[u8] = br#"],"page":{"nextCursor":null,"hasMore":false}}"#;

fn csrf_for_list<'a>(out: &'a mut [u8], guest_cookie: Option<&mut [u8]>) -> R<Option<&'a [u8]>> {
    let doc = fetch_html(b"https://hentaifox.com/language/chinese/popular/pag/1/", None, guest_cookie)?;
    let Ok(meta) = child(doc.0, b"meta[name='csrf-token']") else { return Ok(None); };
    let value = attr(meta.0, b"content", out)?;
    Ok((!value.is_empty()).then_some(value))
}

fn gallery(req: &[u8], requested_id: Option<&[u8]>) -> R<usize> {
    let mut url = [0; 128];
    let doc = match requested_id {
        Some(id) => fetch(make(&[WEB, b"/gallery/", id, b"/"], &mut url)?)?,
        None => fetch(b"https://hentaifox.com/random/")?,
    };
    let mut id_buf = [0; 64];
    let id = ident(input(doc.0, b"gallery_id", &mut id_buf)?)?;
    if requested_id.is_some_and(|expected| expected != id) {
        return Err("detail ownership");
    }
    let mut title_buf = [0; 4096];
    let title = safe_text(doc.0, b".gallery_top h1", &mut title_buf)?;
    let mut image = [0; 1024];
    let image = cover(doc.0, &mut image)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, br#"{"items":["#)?;
    emit_gallery(out, &mut n, id, title, Some(image), req)?;
    bytes(out, &mut n, END)?;
    Ok(n)
}

fn list_xhr(req: &[u8], url: &[u8], form: &[u8]) -> R<Owned> {
    let mut csrf = [0; 512];
    let csrf = csrf_for_list(&mut csrf, None)?;
    match fetch_html(url, Some((form, csrf, None)), None) {
        Err("site session rejected") if feature(req, b"source_anonymous_http_v1") => {
            // The public HTML sets a PHP session tied to its CSRF token. Retry
            // only the site endpoint with the session and token from that GET.
            let mut guest = [0; 512];
            let mut token = [0; 512];
            let csrf = csrf_for_list(&mut token, Some(&mut guest))?;
            let cookie_len = guest.iter().position(|b| *b == 0).unwrap_or(guest.len());
            fetch_html(url, Some((form, csrf, Some(&guest[..cookie_len]))), None)
        },
        response => response,
    }
}

fn sidebar(req: &[u8], sort: &[u8]) -> R<usize> {
    let mut form = [0; 96];
    let form = make(&[b"type=", sort], &mut form)?;
    let doc = list_xhr(req, b"https://hentaifox.com/includes/sidebar.php", form)?;
    let cards = All::new(doc.0, b"div.item")?;
    if cards.len == 0 {
        return Err("missing sidebar list");
    }
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(out, &mut n, br#"{"items":["#)?;
    for i in 0..cards.len {
        let card = cards.at(i);
        let mut href = [0; 256];
        let href = sa(card, b"a", b"href", &mut href)?;
        let mut absolute_buf = [0; 256];
        let id = work_href(absolute(href, &mut absolute_buf)?)?;
        let img = child(card, b"img")?;
        let mut alt = [0; 4096];
        let name = attr(img.0, b"alt", &mut alt)?;
        if name.is_empty() {
            continue;
        }
        let mut raw = [0; 1024];
        let mut url = [0; 1024];
        let image = match imgattr(img.0, &mut raw) {
            Ok(value) => Some(absolute(value, &mut url)?),
            Err("missing image") => None,
            Err(err) => return Err(err),
        };
        if count > 0 {
            bytes(out, &mut n, b",")?;
        }
        emit_gallery(out, &mut n, id, name, image, req)?;
        count += 1;
    }
    if count == 0 {
        return Err("missing sidebar items");
    }
    bytes(out, &mut n, END)?;
    Ok(n)
}

fn emit_cursor(out: &mut [u8], n: &mut usize, current: usize, key: &[u8]) -> R<()> {
    let page = current.checked_add(1).filter(|p| *p <= 10000).ok_or("page capacity")?;
    let mut cursor = [0; 8192];
    let mut k = 0;
    wn(&mut cursor, &mut k, page)?;
    bytes(&mut cursor, &mut k, b":")?;
    if !write_url_encoded(&mut cursor, &mut k, key) {
        return Err("cursor capacity");
    }
    q(out, n, &cursor[..k])?;
    bytes(out, n, br#","hasMore":true}}"#)
}

fn favorites(req: &[u8]) -> R<usize> {
    let key = b"favorites";
    let current = page(req, key)?;
    let mut form = [0; 24];
    let mut f = 0;
    bytes(&mut form, &mut f, b"page=")?;
    wn(&mut form, &mut f, current)?;
    let doc = list_xhr(req, b"https://hentaifox.com/includes/user_favs.php", &form[..f])?;
    if child(doc.0, b"input[value='Login'], form[action*='/login']").is_ok() {
        return Err("auth_required");
    }
    let cards = All::new(doc.0, b"div.thumb")?;
    let next = All::new(doc.0, b".pagination li.active + li:not(.disabled) a.page-link")?;
    if next.len > 1 {
        return Err("ambiguous next");
    }
    if cards.len == 0 && !contains_bytes(unsafe { &body_buf()[..BODY_LEN] }, b"favorites") {
        return Err("auth_required");
    }
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(out, &mut n, br#"{"items":["#)?;
    for i in 0..cards.len {
        let card = cards.at(i);
        if language_matches(card, false)? {
            push_card(out, &mut n, &mut count, card, req)?;
        }
    }
    if next.len == 0 {
        bytes(out, &mut n, END)?;
    } else {
        bytes(out, &mut n, br#"],"page":{"nextCursor":"#)?;
        emit_cursor(out, &mut n, current, key)?;
    }
    Ok(n)
}

pub(super) fn list(req: &[u8], search: bool) -> R<usize> {
    let mut query_buf = [0; 2048];
    let query = if search {
        uq(required(args(req)?, b"query")?, &mut query_buf)?
    } else {
        b""
    };
    let filters = if search { filter_values(req)? } else { b"{}" };
    let sort = member(filters, b"sort")?.map(quoted).transpose()?.unwrap_or(b"popular");
    if !ORDER.iter().any(|(key, _)| *key == sort) {
        return Err("unsupported sort");
    }
    if search {
        if sort.starts_with(b"top_") {
            return sidebar(req, sort);
        }
        if !query.is_empty() && query.iter().all(u8::is_ascii_digit) {
            return gallery(req, Some(ident(query)?));
        }
        if member(filters, b"random")? == Some(b"true") {
            return gallery(req, None);
        }
        if member(filters, b"favorites")? == Some(b"true") {
            return favorites(req);
        }
    }
    let mut terms = [0; 4096];
    let mut terms_len = 0;
    let mut first_tag = [0; 192];
    let (tag_count, first_len) = read_tags(filters, &mut terms, &mut terms_len, &mut first_tag)?;
    let has_keyword = !trim_ascii(query).is_empty();
    if has_keyword {
        add_term(&mut terms, &mut terms_len, query)?;
    }
    let mut key_buf = [0; 8192];
    let route = if !search {
        match text(args(req)?, b"listingId")? {
            b"listing:popular" => Route::Popular,
            b"listing:latest" => Route::Latest,
            _ => return Err("listing identity"),
        }
    } else if tag_count == 1 && !has_keyword {
        Route::Tag { slug: &first_tag[..first_len], popular: sort == b"popular" }
    } else if tag_count != 0 || has_keyword {
        add_term(&mut terms, &mut terms_len, b"chinese")?;
        Route::Search { terms: &terms[..terms_len], popular: sort == b"popular" }
    } else if sort == b"latest" {
        Route::Latest
    } else {
        Route::Popular
    };
    let key = if !search && matches!(route, Route::Popular) {
        b"".as_slice()
    } else if search && tag_count == 0 && has_keyword && sort == b"popular" {
        query
    } else {
        route.url(1, &mut key_buf)?
    };
    let current = page(req, key)?;
    let mut url = [0; 8192];
    let doc = fetch(route.url(current, &mut url)?)?;
    let cards = All::new(doc.0, b".lc_galleries .thumb")?;
    let next = All::new(doc.0, b".pagination li.active + li:not(.disabled) a.page-link")?;
    if next.len > 1 {
        return Err("ambiguous next");
    }
    if cards.len == 0 {
        let mut text = [0; 4096];
        let heading = safe_text(doc.0, b"h1.tag_info", &mut text)?;
        if !search || !contains_bytes(heading, b"- 0 results") {
            return Err("empty or gated listing");
        }
    }
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    let mut seen = [0usize; 1000];
    bytes(out, &mut n, b"{\"items\":[")?;
    for i in 0..cards.len {
        let card = cards.at(i);
        let mut href = [0; 256];
        let id = number(work_href(sa(card, b".inner_thumb a", b"href", &mut href)?)?).ok_or("work number")?;
        if seen[..i].contains(&id) {
            return Err("same page duplicate");
        }
        seen[i] = id;
        if language_matches(card, route.strict_language())? {
            push_card(out, &mut n, &mut count, card, req)?;
        }
    }
    // GalleryAdults retains one item when a page has only other languages and a next page.
    if search && count == 0 && cards.len > 0 && next.len == 1 {
        push_card(out, &mut n, &mut count, cards.at(0), req)?;
    }
    bytes(out, &mut n, b"],\"page\":{")?;
    if next.len == 0 {
        bytes(out, &mut n, br#""nextCursor":null,"hasMore":false}}"#)?;
        return Ok(n);
    }
    let np = current.checked_add(1).filter(|p| *p <= 10000).ok_or("page capacity")?;
    let mut href = [0; 8192];
    let mut absolute_buf = [0; 8192];
    let actual = absolute(attr(next.at(0), b"href", &mut href)?, &mut absolute_buf)?;
    if actual != route.url(np, &mut url)? {
        return Err("next ownership");
    }
    bytes(out, &mut n, br#""nextCursor":"#)?;
    emit_cursor(out, &mut n, current, key)?;
    Ok(n)
}

fn emit_tag_options(out: &mut [u8], n: &mut usize) -> R<()> {
    let mut count = 0;
    for page in 1..=5 {
        let mut url = [0; 128];
        let mut size = 0;
        bytes(&mut url, &mut size, b"https://hentaifox.com/tags/popular/pag/")?;
        wn(&mut url, &mut size, page)?;
        bytes(&mut url, &mut size, b"/")?;
        let Ok(doc) = fetch(&url[..size]) else { continue; };
        let tags = All::new(doc.0, b"a.tag_btn")?;
        for i in 0..tags.len {
            let tag = tags.at(i);
            let mut href = [0; 512];
            let raw = attr(tag, b"href", &mut href)?;
            let path = raw.strip_prefix(WEB).unwrap_or(raw).trim_ascii_end();
            let path = path.strip_suffix(b"/").unwrap_or(path);
            let slug = tag_slug(path.strip_prefix(b"/tag/").ok_or("tag route")?)?;
            let mut title_buf = [0; 1024];
            let label = safe_text(tag, b".list_tag, .tag_name", &mut title_buf)?;
            let mut value_buf = [0; 1536];
            let value = make(&[slug, b":", label], &mut value_buf)?;
            if count != 0 { bytes(out, n, b",")?; }
            bytes(out, n, br#"{"value":"#)?;
            q(out, n, value)?;
            bytes(out, n, br#","label":"#)?;
            q(out, n, label)?;
            bytes(out, n, b"}")?;
            count += 1;
        }
    }
    if count == 0 {
        return Err("no tags returned");
    }
    Ok(())
}

pub(super) fn filter_schema(req: &[u8]) -> R<usize> {
    let out = payload_buf();
    let mut n = 0;
    if !supports_search_filters(req) {
        bytes(out, &mut n, br#"{"filters":[]}"#)?;
        return Ok(n);
    }
    bytes(out, &mut n, r#"{"supportsFilterFetching":true,"useSearchForFilters":true,"filters":[{"type":"header","name":"用双引号进行精确搜索","scope":"search"},{"id":"sort","name":"排序","type":"select","scope":"search","default":"popular","options":["#.as_bytes())?;
    for (i, (value, name)) in ORDER.iter().enumerate() {
        if i != 0 { bytes(out, &mut n, b",")?; }
        bytes(out, &mut n, br#"{"value":"#)?;
        q(out, &mut n, value)?;
        bytes(out, &mut n, br#","label":"#)?;
        q(out, &mut n, name)?;
        bytes(out, &mut n, b"}")?;
    }
    bytes(out, &mut n, r#"]},{"id":"tags","name":"题材","type":"multiselect","scope":"search""#.as_bytes())?;
    if member(args(req)?, b"fetchFilterData")? == Some(b"true") {
        bytes(out, &mut n, br#","options":["#)?;
        emit_tag_options(out, &mut n)?;
        bytes(out, &mut n, b"]")?;
    }
    bytes(out, &mut n, r#"},{"id":"favorites","name":"仅显示站内收藏","type":"boolean","scope":"search","default":false},{"id":"random","name":"随机作品","type":"boolean","scope":"search","default":false}]}"#.as_bytes())?;
    Ok(n)
}
