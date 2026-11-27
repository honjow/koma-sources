#![no_std]
extern crate alloc;
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult, DecimalChapterNumber,
    },
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use md5::{Digest, Md5};
use serde_json::{json, Value};
use url::Url;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use port::{Node, Result};
use port_catalog::Labels;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const SITE: &str = "https://mangataro.org/";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.mangataro.en.koma",name:"MangaTaro",language:"en",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",description:"English manga, manhwa and manhua, including One Piece and Gachiakuta, with catalogues organized by genre, format and publication status.",author:"Koma source contributors",content_rating:"mixed"};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: false,
    home: false,
    credentials: false,
    image_request: true,
};
const SORT: &[(&str, &str)] = &[
    ("post_desc", "Latest updates"),
    ("post_asc", "Oldest updates"),
    ("release_desc", "Newest releases"),
    ("release_asc", "Oldest releases"),
    ("title_asc", "Title A–Z"),
    ("title_desc", "Title Z–A"),
    ("popular_desc", "Most popular"),
    ("popular_asc", "Least popular"),
];
const TYPES: &[(&str, &str)] = &[
    ("", "Any"),
    ("Manga", "Manga"),
    ("Manhwa", "Manhwa"),
    ("Manhua", "Manhua"),
];
const STATUS: &[(&str, &str)] = &[
    ("", "Any"),
    ("Ongoing", "Ongoing"),
    ("Completed", "Completed"),
];
// Stable site tag IDs; keep the original values when translating labels.
const GENRES: &[(&str, &str)] = &[
    ("", "Any"),
    ("7", "Action"),
    ("12", "Adventure"),
    ("20", "Comedy"),
    ("8", "Drama"),
    ("2", "Fantasy"),
    ("44", "Horror"),
    ("3", "Isekai"),
    ("4", "Martial Arts"),
    ("24", "Mystery"),
    ("29", "Romance"),
    ("73", "School Life"),
    ("15", "Sci-Fi"),
    ("47", "Slice of Life"),
    ("25", "Sports"),
    ("9", "Supernatural"),
];
koma_source_sdk::koma_source_buffers! {payload: 2 * 1024 * 1024,http_out: 8 * 1024 * 1024,body:4 * 1024 * 1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn field<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or("parse_error")
}
fn digits(s: &str) -> bool {
    !s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_digit()) && !s.starts_with('0')
}
fn slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn work(args: &Value) -> Result<(&str, &str)> {
    let (id, s) = args["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix("manga:"))
        .and_then(|s| s.split_once(':'))
        .ok_or("invalid_request")?;
    if !digits(id) || !slug(s) {
        return Err("invalid_request");
    }
    Ok((id, s))
}
fn site(raw: &str) -> Result<Url> {
    let u = port::absolute(SITE, raw)?;
    if u.scheme() != "https"
        || u.host_str() != Some("mangataro.org")
        || u.port().is_some()
        || u.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(u)
}
fn media(raw: &str, pages: bool) -> Result<Url> {
    let u = port::absolute(SITE, raw)?;
    let allowed = if pages {
        u.host_str() == Some("mangataro.yachts") && u.path().starts_with("/storage/chapters/")
    } else {
        u.host_str() == Some("mangataro.org") && u.path().starts_with("/content/media/")
    };
    if u.scheme() != "https"
        || !allowed
        || u.port().is_some()
        || u.fragment().is_some()
        || !u
            .path()
            .rsplit_once('.')
            .is_some_and(|(_, e)| matches!(e, "webp" | "jpg" | "jpeg" | "png" | "avif"))
    {
        return Err("parse_error");
    }
    Ok(u)
}
fn api(path: &str, body: Option<&Value>) -> Result<Value> {
    let url = site(path)?;
    let encoded = body
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| "internal_error")?;
    let s = port::request(
        url.as_str(),
        if body.is_some() { "POST" } else { "GET" },
        json!({"Accept":"application/json","Referer":SITE,"Content-Type":"application/json"}),
        encoded.as_deref(),
    )?;
    serde_json::from_str(&s).map_err(|_| "parse_error")
}
fn text(raw: &str) -> Result<String> {
    let mut s = raw.to_string();
    for _ in 0..3 {
        let t = Node::parse(&s)?.text()?;
        if t == s {
            return Ok(t);
        }
        s = t;
    }
    Ok(s)
}
fn card(id: &str, s: &str, title: &str, cover: &str) -> Result<Value> {
    if !digits(id) || !slug(s) {
        return Err("parse_error");
    }
    // The live catalogue uses this external placeholder for missing artwork.
    // Preserve the work without fetching the placeholder host.
    let cover = if cover == "https://via.placeholder.com/300x400" || cover.is_empty() {
        json!({"kind":"none"})
    } else {
        let cover = media(cover, false)?;
        json!({"kind":"url","url":cover.as_str(),"headers":{"Referer":SITE}})
    };
    Ok(
        json!({"id":format!("manga:{id}:{s}"),"title":text(title)?,"url":format!("{SITE}manga/{s}"),"language":"en","contentRating":"mixed","cover":cover}),
    )
}
fn choice<'a>(a: &'a Value, k: &str, opts: &[(&str, &str)], default: &'a str) -> Result<&'a str> {
    match &a["filters"][k] {
        Value::Null => Ok(default),
        Value::String(v) if opts.iter().any(|(s, _)| s == v) => Ok(v),
        _ => Err("invalid_request"),
    }
}
fn page(a: &Value) -> Result<usize> {
    match &a["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(s) => s
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=100_000).contains(n) && s == &n.to_string())
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}
fn list(a: &Value, search: bool) -> Result<Value> {
    let page = page(a)?;
    let listing = a["listingId"].as_str().unwrap_or("popular");
    let listing = listing.strip_prefix("listing:").unwrap_or(listing);
    let sort = if search {
        choice(a, "sort", SORT, "post_desc")?
    } else {
        match listing {
            "popular" => "popular_desc",
            "latest" => "post_desc",
            _ => return Err("invalid_request"),
        }
    };
    let q = if search {
        a["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if q.len() > 512 || q.chars().any(char::is_control) {
        return Err("invalid_request");
    }
    let as_list = |s: &str| {
        if s.is_empty() {
            String::from("[]")
        } else {
            json!([s]).to_string()
        }
    };
    let genre = choice(a, "genre", GENRES, "")?;
    let genre = if genre.is_empty() {
        json!([])
    } else {
        json!([genre.parse::<u64>().map_err(|_| "invalid_request")?])
    };
    let root = api(
        "wp-json/manga/v1/load",
        Some(
            &json!({"page":page,"search":q,"sort":sort,"years":"[]","genres":genre.to_string(),"types":as_list(choice(a,"type",TYPES,"")?),"statuses":as_list(choice(a,"status",STATUS,"")?),"genreMatchMode":"any"}),
        ),
    )?;
    let rows = root.as_array().ok_or("parse_error")?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for r in rows {
        if r["type"].as_str() == Some("Novel") {
            continue;
        }
        let id = field(r, "id")?;
        let u = site(field(r, "url")?)?;
        let s = u
            .path()
            .strip_prefix("/manga/")
            .ok_or("parse_error")?
            .trim_end_matches('/');
        if seen.insert(id) {
            let mut m = card(id, s, field(r, "title")?, field(r, "cover")?)?;
            m["status"] = json!(status(r["status"].as_str().unwrap_or("")));
            items.push(m);
        }
    }
    Ok(port::items(items, (rows.len() == 24).then_some(page + 1)))
}
fn status(s: &str) -> &'static str {
    match s {
        "Ongoing" | "ongoing" => "ongoing",
        "Completed" | "completed" => "completed",
        _ => "unknown",
    }
}
fn detail(a: &Value) -> Result<Value> {
    let (id, s) = work(a)?;
    let r = api(&format!("wp-json/wp/v2/manga/{id}?_embed"), None)?;
    if r["id"].as_u64().map(|n| n.to_string()).as_deref() != Some(id)
        || r["slug"].as_str() != Some(s)
    {
        return Err("not_found");
    }
    let e = &r["_embedded"];
    let cover = e["wp:featuredmedia"][0]["source_url"]
        .as_str()
        .ok_or("parse_error")?;
    let mut m = card(id, s, field(&r["title"], "rendered")?, cover)?;
    m["description"] = json!(text(field(&r["content"], "rendered")?)?);
    for (tax, key) in [("post_tag", "tags"), ("manga_author", "authors")] {
        let tags = e["wp:term"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_array)
            .flatten()
            .filter(|t| t["taxonomy"].as_str() == Some(tax))
            .filter_map(|t| t["name"].as_str())
            .map(text)
            .collect::<Result<Vec<_>>>()?;
        m[key] = json!(tags);
    }
    // WP's `status: publish` is visibility, not serialization status. Read the
    // site's visible status rather than labelling every work ongoing.
    let doc = Node::parse(&port::get(&format!("{SITE}manga/{s}"))?)?;
    m["status"] = json!(doc
        .all(".capitalize")?
        .iter()
        .filter_map(|n| n.text().ok())
        .map(|s| status(&s))
        .find(|s| *s != "unknown")
        .unwrap_or("unknown"));
    Ok(json!({"manga":m}))
}
// Generate the site's public chapter-list token from its current HTTP Date.
// No device clock, cookies or account credentials enter the token or output.
fn clock(date: &str) -> Result<(i64, String)> {
    let p: Vec<_> = date.split_whitespace().collect();
    if p.len() != 6 || p[5] != "GMT" {
        return Err("parse_error");
    }
    let n = |s: &str| s.parse::<i64>().map_err(|_| "parse_error");
    let day = n(p[1])?;
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| *m == p[2])
    .ok_or("parse_error")? as i64
        + 1;
    let year = n(p[3])?;
    let t: Vec<_> = p[4].split(':').collect();
    if t.len() != 3 || !(2000..=2200).contains(&year) || !(1..=31).contains(&day) {
        return Err("parse_error");
    }
    let (h, m, s) = (n(t[0])?, n(t[1])?, n(t[2])?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) || !(0..60).contains(&s) {
        return Err("parse_error");
    }
    let y = year - i64::from(month <= 2);
    let era = y / 400;
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + day - 1 - 719468;
    Ok((
        days * 86400 + h * 3600 + m * 60 + s,
        format!("{year:04}{month:02}{day:02}{h:02}"),
    ))
}
fn chapter_page(id: &str, offset: usize) -> Result<Value> {
    let mut base = site("auth/manga-chapters")?;
    base.query_pairs_mut().extend_pairs([
        ("manga_id", id),
        ("offset", &offset.to_string()),
        ("limit", "500"),
        ("order", "DESC"),
    ]);
    let mut url = base.clone();
    for attempt in 0..2 {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let req=serde_json::to_vec(&json!({"version":1,"method":"GET","url":url.as_str(),"responseKind":"bodyText","responseMetadata":{"headers":true},"headers":{"Accept":"application/json","Referer":SITE},"timeoutMs":15000})).map_err(|_|"internal_error")?;
        let len = host::http_request(&req, http_out()).map_err(|_| "network_error")?;
        let env: Value = serde_json::from_slice(&http_out()[..len]).map_err(|_| "network_error")?;
        if attempt == 0 && env["ok"].as_bool() == Some(true) && env["status"].as_u64() == Some(403)
        {
            let headers = env["headers"].as_object().ok_or("host_feature_required")?;
            let date = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("date"))
                // Koma returns header values as string arrays; the dev runner
                // returns strings. Use the first nonempty server value only,
                // never a local/device-clock fallback for the chapter token.
                .and_then(|(_, v)| match v {
                    Value::String(s) => Some(s.trim()).filter(|s| !s.is_empty()),
                    Value::Array(values) => values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .find(|s| !s.is_empty()),
                    _ => None,
                })
                .ok_or("host_feature_required")?;
            let (ts, hour) = clock(date)?;
            let hash = Md5::digest(format!("{ts}mng_ch_{hour}").as_bytes());
            let token = hash
                .iter()
                .take(8)
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            url = base.clone();
            url.query_pairs_mut()
                .extend_pairs([("_t", token.as_str()), ("_ts", &ts.to_string())]);
            continue;
        }
        let len = source_protocol::body(&http_out()[..len], body_buf())?;
        let v: Value = serde_json::from_slice(&body_buf()[..len]).map_err(|_| "parse_error")?;
        if v["success"].as_bool() != Some(true) {
            return Err("parse_error");
        }
        return Ok(v);
    }
    Err("network_error")
}
fn chapter_key(a: &str) -> Result<(&str, &str)> {
    let (w, c) = a
        .strip_prefix("chapter:")
        .and_then(|s| s.split_once('/'))
        .ok_or("invalid_request")?;
    if !slug(w) || !slug(c) || !c.rsplit_once('-').is_some_and(|(_, id)| digits(id)) {
        return Err("invalid_request");
    }
    Ok((w, c))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let go = || {
        let v = port::input(req)?;
        let a = &v["args"];
        if !a["page"]["cursor"].is_null() {
            return Err("invalid_request");
        };
        let (id, s) = work(a)?;
        let owner = format!("manga:{id}:{s}");
        let mut rows = Vec::new();
        let mut seen = BTreeSet::new();
        let mut offset = 0;
        loop {
            if offset >= 20000 {
                return Err("result_too_large");
            };
            let page = chapter_page(id, offset)?;
            let batch = page["chapters"].as_array().ok_or("parse_error")?;
            let more = page["has_more"].as_bool().ok_or("parse_error")?;
            if more && batch.is_empty() {
                return Err("catalog_changed");
            };
            let mut added = 0;
            for row in batch {
                let u = site(field(row, "url")?)?;
                let prefix = format!("/read/{s}/");
                let c = u
                    .path()
                    .strip_prefix(&prefix)
                    .ok_or("parse_error")?
                    .trim_end_matches('/');
                chapter_key(&format!("chapter:{s}/{c}"))?;
                if seen.insert(u.path().to_string()) {
                    added += 1;
                    if row["language"]
                        .as_str()
                        .is_some_and(|l| l.eq_ignore_ascii_case("en"))
                    {
                        rows.push(row.clone());
                    }
                }
            }
            if more && added == 0 {
                return Err("catalog_changed");
            }
            offset += batch.len();
            if !more {
                break;
            }
        }
        let label = Labels::new(METADATA, req)?.text("chapter.title", "Chapter {number}")?;
        let mut data = Vec::new();
        for row in &rows {
            let u = site(field(row, "url")?)?;
            let key = u
                .path()
                .strip_prefix("/read/")
                .ok_or("parse_error")?
                .trim_end_matches('/');
            let number = field(row, "chapter")?;
            // Gachiakuta's live catalogue includes a whitespace-only title.
            // Treat it as absent before HTML decoding, preserving its chapter.
            let title = field(row, "title").unwrap_or("").trim();
            let title = if title.is_empty() || matches!(title, "N/A" | "—") {
                label.replace("{number}", number)
            } else {
                format!("{}: {}", label.replace("{number}", number), text(title)?)
            };
            data.push((
                format!("chapter:{key}"),
                u,
                title,
                number,
                row["group_name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && !matches!(*s, "N/A" | "—")),
            ));
        }
        let chapters = data
            .iter()
            .map(|(id, u, title, num, group)| {
                let mut c = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
                c.url = Some(u.as_str().as_bytes());
                c.language = Some(b"en");
                c.scanlator = group.map(str::as_bytes);
                c.chapter_number = num
                    .parse::<usize>()
                    .ok()
                    .map(ChapterNumber::Integer)
                    .or_else(|| {
                        DecimalChapterNumber::new(num.as_bytes()).map(ChapterNumber::Decimal)
                    });
                c
            })
            .collect::<Vec<_>>();
        // DESC is provider order across the whole catalogue. Do not derive reading
        // positions from display chapter numbers, which may repeat or be fractional.
        ChapterResponse::ordered(payload_buf(), req, ChapterOrder::NewestFirst, &chapters)
            .map_err(port::chapter_error)
    };
    go().map_err(|c| (c, port_catalog::failure(METADATA, req, c)))
}
fn pages(a: &Value) -> Result<Value> {
    let id = field(a, "chapterId")?;
    let (w, c) = chapter_key(id)?;
    if !a["mangaId"].is_null() && work(a)?.1 != w {
        return Err("invalid_request");
    };
    let cid = c.rsplit_once('-').ok_or("invalid_request")?.1;
    let r = api(&format!("auth/chapter-content?chapter_id={cid}"), None)?;
    if r["success"].as_bool() != Some(true) {
        return Err("chapter_unavailable");
    };
    let rows = r["images"].as_array().ok_or("chapter_unavailable")?;
    if rows.is_empty() {
        return Err("chapter_unavailable");
    };
    let mut seen = BTreeSet::new();
    let mut pages = Vec::new();
    for row in rows {
        let image = media(row.as_str().ok_or("parse_error")?, true)?;
        if seen.insert(image.as_str().to_string()) {
            let mut p = port::page(pages.len(), id, image);
            p["image"]["headers"] = json!({"Referer":SITE});
            pages.push(p);
        }
    }
    Ok(json!({"pages":pages}))
}
fn call(op: &str, r: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let out = port::input(r).and_then(|v| f(&v["args"]));
    port_catalog::write_localized(METADATA, r, op, out)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    call("get_manga_list", r, |a| list(a, false))
}
fn run_search(r: &[u8]) -> u32 {
    call("search", r, |a| list(a, true))
}
fn run_get_manga(r: &[u8]) -> u32 {
    call("get_manga", r, detail)
}
fn run_get_pages(r: &[u8]) -> u32 {
    call("get_pages", r, pages)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    call("get_image_request", r, |a| {
        let raw = field(a, "url")?;
        let u = media(raw, true).or_else(|_| media(raw, false))?;
        Ok(json!({"imageRequest":{"url":u.as_str(),"headers":{"Referer":SITE}}}))
    })
}
fn run_get_listings(r: &[u8]) -> u32 {
    call("get_listings", r, |_| {
        let l = Labels::new(METADATA, r)?;
        Ok(
            json!({"listings":[{"id":"popular","name":l.text("listing.popular","Popular")?,"supportsPaging":true,"supportsFilters":true},{"id":"latest","name":l.text("listing.latest","Latest updates")?,"supportsPaging":true,"supportsFilters":true}]}),
        )
    })
}
fn run_get_filters(r: &[u8]) -> u32 {
    call("get_filters", r, |a| {
        let l = Labels::new(METADATA, r)?;
        let mut out = Vec::new();
        for (id, opts) in [
            ("genre", GENRES),
            ("type", TYPES),
            ("status", STATUS),
            ("sort", SORT),
        ] {
            if id == "sort" && a["listingId"].as_str().is_some_and(|s| !s.is_empty()) {
                continue;
            }
            let options = opts
                .iter()
                .map(|(v, t)| Ok(json!({"value":v,"label":l.text(&format!("option.{id}.{v}"),t)?})))
                .collect::<Result<Vec<_>>>()?;
            out.push(json!({"type":"select","id":id,"name":l.text(&format!("filter.{id}"),id)?,"default":if id=="sort"{"post_desc"}else{""},"options":options}));
        }
        Ok(json!({"filters":out}))
    })
}
fn run_get_settings(r: &[u8]) -> u32 {
    call("get_settings", r, |_| Ok(json!({"settings":[]})))
}
fn run_get_home(r: &[u8]) -> u32 {
    call("get_home", r, |_| Err("not_implemented"))
}
fn write_chapter_failure((c, m): (&str, String)) -> u32 {
    write_error("get_chapters", c, &m)
}
koma_source_sdk::koma_source_exports!("mangataro-en", typed_chapters, write_chapter_failure);
