//! GigaViewer HTTP payloads and validation shared by site adapters.

use alloc::{format, string::String, vec::Vec};
use serde_json::{json, Value};
use url::Url;

type Result<T> = core::result::Result<T, &'static str>;

const SERIES_FIELDS: &str = "databaseId title thumbnailUri firstEpisode{permalink} firstVolume{permalink} latestVolume{thumbnailUri}";

pub(crate) fn search_query(keyword: &str) -> Value {
    json!({"query":format!("query Search($keyword:String!){{searchSeries(keyword:$keyword){{edges{{node{{{SERIES_FIELDS}}}}}}}}}"),"variables":{"keyword":keyword}})
}

pub(crate) fn series_query(id: &str) -> Value {
    json!({
        "query":format!("query Series($id:String!){{series(databaseId:$id){{{SERIES_FIELDS} author{{name}} description episodes:readableProducts(types:[EPISODE],first:0){{totalCount}} volumes:readableProducts(types:[VOLUME],first:0){{totalCount}}}}}}"),
        "variables":{"id":id}
    })
}

pub(crate) fn graphql_data(body: &Value) -> Result<&Value> {
    if !body["errors"].is_null() && !body["errors"].as_array().is_some_and(Vec::is_empty) {
        return Err("upstream_error");
    }
    body.get("data")
        .filter(|v| v.is_object())
        .ok_or("parse_error")
}

pub(crate) fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 24 && value.bytes().all(|b| b.is_ascii_digit())
}

pub(crate) struct Series<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub permalink: &'a str,
    pub thumbnail: Option<&'a str>,
    pub volume_only: bool,
}

impl<'a> Series<'a> {
    pub(crate) fn read(node: &'a Value) -> Result<Option<Self>> {
        let series_id = node["databaseId"]
            .as_str()
            .filter(|s| id(s))
            .ok_or("parse_error")?;
        let title = node["title"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("parse_error")?;
        let episode = node["firstEpisode"]["permalink"].as_str();
        let Some(permalink) = episode.or_else(|| node["firstVolume"]["permalink"].as_str()) else {
            return Ok(None);
        };
        Ok(Some(Self {
            id: series_id,
            title,
            permalink,
            thumbnail: node["thumbnailUri"]
                .as_str()
                .or_else(|| node["latestVolume"]["thumbnailUri"].as_str()),
            volume_only: episode.is_none(),
        }))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductKind {
    Episode,
    Volume,
}

impl ProductKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "episode" => Some(Self::Episode),
            "volume" => Some(Self::Volume),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Episode => "episode",
            Self::Volume => "volume",
        }
    }

    pub(crate) fn url(self, site: &str, product: &str) -> String {
        format!("{site}/{}/{product}", self.name())
    }

    pub(crate) fn count(self, series: &Value) -> Result<usize> {
        let key = match self {
            Self::Episode => "episodes",
            Self::Volume => "volumes",
        };
        series[key]["totalCount"]
            .as_u64()
            .filter(|n| *n <= 20_000)
            .map(|n| n as usize)
            .ok_or("parse_error")
    }

    pub(crate) fn catalog_url(self, site: &str, series: &str, offset: usize) -> String {
        format!("{site}/api/viewer/pagination_readable_products?type={}&aggregate_id={series}&sort_order=desc&offset={offset}", self.name())
    }
}

pub(crate) fn product_path<'a>(site: &str, url: &'a Url) -> Option<(ProductKind, &'a str)> {
    if url.origin().ascii_serialization() != site
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let (kind, product) = url.path().strip_prefix('/')?.split_once('/')?;
    id(product).then_some((ProductKind::parse(kind)?, product))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    Public,
    Locked,
    Unavailable,
}

pub(crate) struct Product<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub published: Option<&'a str>,
    pub access: Access,
}

impl<'a> Product<'a> {
    pub(crate) fn read(value: &'a Value, site: &str, kind: ProductKind) -> Result<Self> {
        let id = value["readable_product_id"]
            .as_str()
            .filter(|s| self::id(s))
            .ok_or("parse_error")?;
        let link = value["viewer_uri"]
            .as_str()
            .and_then(|s| Url::parse(s).ok())
            .ok_or("parse_error")?;
        if product_path(site, &link) != Some((kind, id)) {
            return Err("parse_error");
        }
        let purchase = &value["purchase_info"];
        let access = match (
            purchase["can_read"].as_bool(),
            purchase["unavailable"].as_bool(),
        ) {
            (Some(_), Some(true)) => Access::Unavailable,
            (Some(true), Some(false)) => Access::Public,
            (Some(false), Some(false)) => Access::Locked,
            _ => return Err("parse_error"),
        };
        Ok(Self {
            id,
            access,
            published: value["display_open_at"].as_str(),
            title: value["title"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or("parse_error")?,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Tiles {
    Plain,
    Baku,
}

pub(crate) struct Viewer<'a> {
    pub pages: Vec<&'a str>,
    pub direction: &'a str,
    pub tiles: Tiles,
}

pub(crate) fn viewer<'a>(
    value: &'a Value,
    site: &str,
    kind: ProductKind,
    id: &str,
    series: &str,
) -> Result<Viewer<'a>> {
    let product = &value["readableProduct"];
    let link = product["permalink"]
        .as_str()
        .and_then(|s| Url::parse(s).ok())
        .ok_or("parse_error")?;
    if product["id"].as_str() != Some(id)
        || product["typeName"].as_str() != Some(kind.name())
        || product["series"]["id"].as_str() != Some(series)
        || product_path(site, &link) != Some((kind, id))
    {
        return Err("parse_error");
    }
    // Only the provider's anonymous full reader grants access. Locked
    // products may still have a 200 HTML response or a promotional preview.
    if product["isPublic"].as_bool() != Some(true) {
        return Err("chapter_locked");
    }
    let structure = &product["pageStructure"];
    let tiles = match structure["choJuGiga"].as_str() {
        Some("baku") => Tiles::Baku,
        Some("none" | "") => Tiles::Plain,
        _ => return Err("unsupported_scramble"),
    };
    let direction = match structure["readingDirection"].as_str() {
        Some(value @ ("rtl" | "ltr")) => value,
        _ => return Err("parse_error"),
    };
    let main = structure["pages"].as_array().ok_or("chapter_unavailable")?;
    let pages = main
        .iter()
        .filter(|p| p["type"] == "main")
        .map(|page| {
            page["src"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("parse_error")
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(Viewer {
        pages,
        direction,
        tiles,
    })
}
