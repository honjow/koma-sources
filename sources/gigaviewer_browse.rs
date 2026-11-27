//! Shared browse and filter plumbing for sites using GigaViewer's serial groups.

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

use crate::{
    giga::{Series, Site, SERIES_FIELDS},
    port::Result,
    port_catalog::Labels,
};

pub(crate) type Collection = (&'static str, &'static [&'static str]);

#[derive(Clone, Copy)]
pub(crate) enum Sort {
    Site,
    Likes,
    Published,
}

pub(crate) fn series_items<'a>(
    site: &Site,
    nodes: impl IntoIterator<Item = &'a Value>,
    sort: Sort,
) -> Result<Vec<Value>> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for node in nodes {
        if let Some(series) = Series::read(node)? {
            if seen.insert(series.id) {
                rows.push((series, node));
            }
        }
    }
    match sort {
        Sort::Site => (),
        Sort::Likes => rows.sort_by(|(_, node), (_, other)| {
            other["likeCount"]
                .as_u64()
                .unwrap_or(0)
                .cmp(&node["likeCount"].as_u64().unwrap_or(0))
        }),
        Sort::Published => rows.sort_by(|(_, node), (_, other)| {
            other["latestEpisode"]["publishedAt"]
                .as_str()
                .cmp(&node["latestEpisode"]["publishedAt"].as_str())
        }),
    }
    rows.iter()
        .map(|(series, _)| site.series_item(series))
        .collect()
}

pub(crate) fn collection(site: &Site, ids: &[&str], sort: Sort) -> Result<Vec<Value>> {
    let mut fields = String::new();
    let mut params = Vec::new();
    let mut variables = Map::new();
    for (i, id) in ids.iter().enumerate() {
        let key = format!("id{i}");
        let node = if id.contains(':') {
            String::from(*id)
        } else {
            format!("SerialGroup:{id}")
        };
        variables.insert(key.clone(), json!(STANDARD.encode(node)));
        params.push(format!("${key}:ID!"));
        fields.push_str(&format!("{key}:node(id:${key}){{...Collection}} "));
    }
    let data = site.graphql(json!({
        "query":format!("query Collection({}){{{fields}}}
            fragment Collection on Node {{
                ... on SerialGroup {{ series {{ edges {{ node {{ ...Entry }} }} }} }}
                ... on Genre {{ series {{ edges {{ node {{ ...Entry }} }} }} }}
            }}
            fragment Entry on Series {{ {SERIES_FIELDS} likeCount latestEpisode{{publishedAt}} }}",params.join(",")),
        "variables":variables
    }))?;
    let mut nodes = Vec::new();
    for i in 0..ids.len() {
        nodes.extend(edges(&data[format!("id{i}")]["series"])?);
    }
    series_items(site, nodes, sort)
}

pub(crate) fn edges(connection: &Value) -> Result<impl Iterator<Item = &Value>> {
    Ok(connection["edges"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|edge| &edge["node"]))
}

/// Preserve the provider's combined order while emitting a bounded host page.
/// The preceding identity catches a changed upstream ordering on continuation.
pub(crate) fn page(items: Vec<Value>, cursor: &Value) -> Result<Value> {
    const LIMIT: usize = 80;
    let offset = if cursor.is_null() {
        0
    } else {
        let (offset, anchor) = cursor
            .as_str()
            .and_then(|text| text.split_once(':'))
            .ok_or("invalid_request")?;
        let offset = offset
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0 && *n % LIMIT == 0)
            .ok_or("invalid_request")?;
        if items.get(offset - 1).and_then(|row| row["id"].as_str()) != Some(anchor) {
            return Err("catalog_changed");
        }
        offset
    };
    let mut rows = items.into_iter().skip(offset);
    let items: Vec<_> = rows.by_ref().take(LIMIT).collect();
    let next = if rows.next().is_some() {
        Some(format!(
            "{}:{}",
            offset + items.len(),
            items
                .last()
                .and_then(|v| v["id"].as_str())
                .ok_or("parse_error")?
        ))
    } else {
        None
    };
    Ok(json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next}}))
}

pub(crate) fn selected<'a>(args: &Value, collections: &'a [Collection]) -> Result<&'a [&'a str]> {
    let id = match &args["filters"]["collection"] {
        Value::Null => collections[0].0,
        value => value.as_str().ok_or("invalid_request")?,
    };
    collections
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, ids)| *ids)
        .ok_or("invalid_request")
}

pub(crate) fn filters(labels: &Labels<'_>, collections: &[Collection]) -> Result<Value> {
    let options = collections
        .iter()
        .map(|(id, _)| Ok(json!({"value":id,"label":labels.text(&format!("collection.{id}"),id)?})))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"filters":[{"id":"collection","type":"select","default":collections[0].0,
        "name":labels.text("filter.collection","作品一覧")?,"options":options}]}),
    )
}

pub(crate) fn listings(labels: &Labels<'_>, ids: &[&str]) -> Result<Value> {
    let items = ids
        .iter()
        .map(|id| Ok(json!({
            "id":id,"name":labels.text(&format!("listing.{id}"),id)?,
            "supportsPaging":true,"supportsFilters":matches!(*id,"popular" | "latest" | "serial")
        })))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"listings":items}))
}

pub(crate) fn settings(labels: &Labels<'_>) -> Result<Value> {
    let settings = ["hide_locked", "hide_unavailable"]
        .iter()
        .map(|id| {
            Ok(json!({"id":id,"kind":"boolean","default":false,
            "name":labels.text(&format!("setting.{id}"),id)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"settings":settings}))
}

pub(crate) fn search(site: &Site, args: &Value, collections: &[Collection]) -> Result<Value> {
    let query = args["query"].as_str().unwrap_or("").trim();
    if query.is_empty() {
        let ids = selected(args, collections)?;
        return page(collection(site, ids, Sort::Site)?, &args["page"]["cursor"]);
    }
    // The reference applies serial-group and genre filters only to empty searches.
    if !args["page"]["cursor"].is_null() {
        return Err("invalid_request");
    }
    site.search(query)
}

// Both adapters use the same request, error, and export contracts. The site
// crate defines only its identity, GraphQL collections and discover listings.
macro_rules! gigaviewer_exports {
    ($name:literal) => {
        use koma_source_sdk::chapters::ChapterResult;
        use port_catalog::Labels;

        fn dispatch(
            operation: &str,
            request: &[u8],
            f: impl FnOnce(&Value) -> Result<Value>,
        ) -> u32 {
            port_catalog::write_localized(
                METADATA,
                request,
                operation,
                port::input(request).and_then(|input| f(&input)),
            )
        }
        fn run_search(req: &[u8]) -> u32 {
            dispatch("search", req, |input| {
                browse::search(&VIEWER, &input["args"], COLLECTIONS)
            })
        }
        fn run_get_manga_list(req: &[u8]) -> u32 {
            dispatch("get_manga_list", req, |input| {
                let args = &input["args"];
                browse::page(listing(args)?, &args["page"]["cursor"])
            })
        }
        fn run_get_manga(req: &[u8]) -> u32 {
            dispatch("get_manga", req, |input| VIEWER.detail(input))
        }
        fn run_get_pages(req: &[u8]) -> u32 {
            dispatch("get_pages", req, |input| VIEWER.pages(input))
        }
        fn run_get_image_request(req: &[u8]) -> u32 {
            dispatch("get_image_request", req, |input| {
                VIEWER.image_request(req, input)
            })
        }
        fn run_get_chapters(
            req: &[u8],
        ) -> ChapterResult<'static, (&'static str, alloc::string::String)> {
            VIEWER
                .chapters(req)
                .map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
        }
        fn write_chapter_failure((code, message): (&str, alloc::string::String)) -> u32 {
            write_error("get_chapters", code, &message)
        }
        fn run_get_listings(req: &[u8]) -> u32 {
            dispatch("get_listings", req, |_| {
                browse::listings(&Labels::new(METADATA, req)?, LISTINGS)
            })
        }
        fn run_get_filters(req: &[u8]) -> u32 {
            dispatch("get_filters", req, |_| {
                browse::filters(&Labels::new(METADATA, req)?, COLLECTIONS)
            })
        }
        fn run_get_settings(req: &[u8]) -> u32 {
            dispatch("get_settings", req, |_| {
                browse::settings(&Labels::new(METADATA, req)?)
            })
        }
        fn run_get_home(req: &[u8]) -> u32 {
            dispatch("get_home", req, |_| Err("not_implemented"))
        }

        koma_source_sdk::koma_source_exports!($name, typed_chapters, write_chapter_failure);
    };
}
pub(crate) use gigaviewer_exports;
