use super::{doc, image_url, manga_route, mid, selected, Nodes};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde_json::{json, Value};
type Result<T> = core::result::Result<T, &'static str>;
const GENRES: &[(&str, Option<&str>, Option<&str>)] = &[
    ("fantasy", Some("/de/genres/fantasy"), Some("FANTASY")),
    ("romance", Some("/de/genres/romance"), Some("ROMANCE")),
    ("drama", Some("/de/genres/drama"), Some("DRAMA")),
    ("action", Some("/de/genres/action"), Some("ACTION")),
    (
        "historical",
        Some("/de/genres/historical"),
        Some("HISTORICAL"),
    ),
    ("thriller", Some("/de/genres/thriller"), Some("THRILLER")),
    ("horror", Some("/de/genres/horror"), Some("HORROR")),
    ("comedy", Some("/de/genres/comedy"), Some("COMEDY")),
    (
        "slice_of_life",
        Some("/de/genres/slice_of_life"),
        Some("SLICE_OF_LIFE"),
    ),
    ("sports", Some("/de/genres/sports"), Some("SPORTS")),
    ("tiptoon", Some("/de/genres/tiptoon"), Some("TIPTOON")),
    ("sf", None, Some("SF")),
    ("super_hero", None, Some("SUPER_HERO")),
    ("heartwarming", None, Some("HEARTWARMING")),
    ("supernatural", None, Some("SUPERNATURAL")),
    ("short_story", None, Some("SHORT_STORY")),
    ("post_apocalyptic", None, Some("POST_APOCALYPTIC")),
    ("zombies", None, Some("ZOMBIES")),
    ("school", None, Some("SCHOOL")),
    ("animals", None, Some("ANIMALS")),
    ("crime_mystery", None, Some("CRIME_MYSTERY")),
    ("inspirational", None, Some("INSPIRATIONAL")),
    ("all_ages", None, Some("ALL_AGES")),
    ("romantic_fantasy", None, Some("ROMANTIC_FANTASY")),
    ("bl_gl", None, Some("BL_GL")),
];
fn label(r: &[u8], key: &str) -> Result<String> {
    let c =
        koma_source_sdk::localization::Catalog::from_metadata(include_bytes!("../metadata.json"))
            .map_err(|_| "invalid localization metadata")?;
    let mut b = [0; 1024];
    c.text(
        &koma_source_sdk::runtime::Context::from_request(r),
        key,
        key,
        &mut b,
    )
    .map(String::from)
    .map_err(|_| "localization output failed")
}
fn put(v: Value) -> Result<usize> {
    let b = serde_json::to_vec(&v).map_err(|_| "payload overflow")?;
    if b.len() > super::payload_buf().len() {
        return Err("payload overflow");
    }
    super::payload_buf()[..b.len()].copy_from_slice(&b);
    Ok(b.len())
}
fn opt(r: &[u8], value: &str, key: &str) -> Result<Value> {
    Ok(json!({"value":value,"label":label(r,key)?}))
}
pub(super) fn filters(r: &[u8]) -> Result<usize> {
    let mut f = Vec::new();
    let mut t = Vec::new();
    for v in ["all", "originals", "canvas"] {
        t.push(opt(r, v, &format!("option.type.{v}"))?);
    }
    f.push(json!({"id":"searchType","type":"select","scope":"search","name":label(r,"filter.type")?,"description":label(r,"filter.hint")?,"default":"all","options":t}));
    let mut days = alloc::vec![opt(r, "", "option.any")?];
    for v in [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "complete",
    ] {
        days.push(opt(r, v, &format!("option.day.{v}"))?);
    }
    f.push(json!({"id":"day","type":"select","name":label(r,"filter.day")?,"default":"","options":days}));
    let mut gs = alloc::vec![opt(r, "", "option.any")?];
    for (k, original, canvas) in GENRES {
        let mut name = label(r, &format!("option.genre.{k}"))?;
        if original.is_none() {
            name.push_str(&label(r, "option.canvas_only")?);
        }
        if canvas.is_none() {
            name.push_str(&label(r, "option.originals_only")?);
        }
        gs.push(json!({"value":k,"label":name}));
    }
    f.push(json!({"id":"genre","type":"select","name":label(r,"filter.genre")?,"default":"","options":gs}));
    let mut sort = Vec::new();
    for v in ["MANA", "LIKEIT", "UPDATE"] {
        sort.push(opt(r, v, &format!("option.sort.{v}"))?);
    }
    f.push(json!({"id":"sort","type":"select","name":label(r,"filter.sort")?,"default":"MANA","options":sort}));
    let mut out = json!({"filters":f});
    if super::empty_filter_search_supported(r) {
        out["useSearchForFilters"] = json!(true);
    }
    put(out)
}
fn value<'a>(a: &'a Value, key: &str, default: &'a str) -> Result<&'a str> {
    match &a["filters"][key] {
        Value::Null => Ok(default),
        Value::String(v) => Ok(v),
        _ => Err("invalid request"),
    }
}
pub(super) fn list(r: &[u8]) -> Result<Option<usize>> {
    let req: Value = serde_json::from_slice(r).map_err(|_| "invalid request")?;
    let a = &req["args"];
    if a["query"].as_str().is_some_and(|q| !q.trim().is_empty()) || a["listingId"] == "latest" {
        return Ok(None);
    }
    let typ = value(a, "searchType", "all")?;
    if !["all", "originals", "canvas"].contains(&typ) {
        return Err("invalid request");
    }
    let day = value(a, "day", "")?;
    let genre = value(a, "genre", "")?;
    let sort = value(a, "sort", "MANA")?;
    if !["MANA", "LIKEIT", "UPDATE"].contains(&sort) {
        return Err("invalid request");
    }
    let path = if !day.is_empty() {
        if ![
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "complete",
        ]
        .contains(&day)
        {
            return Err("invalid request");
        }
        format!("/de/originals/{day}")
    } else if !genre.is_empty() {
        let (_, original, canvas) = GENRES
            .iter()
            .find(|(id, _, _)| *id == genre)
            .ok_or("invalid request")?;
        if (typ == "canvas" && canvas.is_some()) || original.is_none() {
            format!(
                "/de/canvas/list?genreTab={}",
                canvas.ok_or("invalid request")?
            )
        } else {
            original.ok_or("invalid request")?.into()
        }
    } else {
        match typ {
            "originals" => "/de/ranking/originals".into(),
            "canvas" => "/de/canvas/list?genreTab=ALL".into(),
            _ => return Ok(None),
        }
    };
    let pn = super::page(r)?;
    let canvas = path.contains("/canvas/");
    if !canvas && pn != 1 {
        return Err("invalid page cursor");
    }
    let sep = if path.contains('?') { "&" } else { "?" };
    let url = format!("https://www.webtoons.com{path}{sep}sortOrder={sort}&page={pn}");
    let root = doc(url.as_bytes())?;
    let nodes = Nodes::get(&root, b".webtoon_list li a, .challenge_lst li a")?;
    if nodes.n == 0 {
        root.select(b".no_data, .challenge_lst, .webtoon_list")?;
    }
    let mut items = Vec::new();
    for i in 0..nodes.n {
        let card = selected(&nodes, i);
        let mut href = [0; 4096];
        let href = card.attr(b"href", &mut href)?;
        let (kind, id, path) = manga_route(href)?;
        let mut buf = [0; 8192];
        let mut n = 0;
        mid(&mut buf, &mut n, kind, id, path)?;
        let mid: String = serde_json::from_slice(&buf[..n]).map_err(|_| "invalid request")?;
        let mut title_buf = [0; 4096];
        let title_node = card.select(b".title, .subj")?;
        let title = title_node.text(&mut title_buf)?;
        let mut src = [0; 8192];
        let src = card.select(b"img")?.attr(b"src", &mut src)?;
        image_url(src)?;
        items.push(json!({"id":mid,"title":core::str::from_utf8(title).map_err(|_|"invalid UTF8")?,"cover":{"kind":"url","url":core::str::from_utf8(src).map_err(|_|"invalid UTF8")?},"contentRating":"safe"}));
    }
    let next = canvas && root.select(b".paginate a[aria-current='true'] + a").is_ok();
    Ok(Some(put(
        json!({"items":items,"page":{"hasMore":next,"nextCursor":if next{Some((pn+1).to_string())}else{None}}}),
    )?))
}
