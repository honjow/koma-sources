//! GigaViewer HTTP payloads and validation shared by site adapters.

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    host, source_protocol,
};

use crate::{
    payload_buf,
    port::{self, Node},
    port_catalog::Labels,
};
use serde_json::{json, Value};
use url::Url;

type Result<T> = core::result::Result<T, &'static str>;

pub(crate) const SERIES_FIELDS: &str = "databaseId title thumbnailUri firstEpisode{permalink} firstVolume{permalink} latestVolume{thumbnailUri}";

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
    // Kurage Bunch's ordinary desktop viewer also uses ttb, including the
    // first Hatarakanai Futari episode. Preserve that provider direction and
    // page order; do not coerce vertical works into a horizontal direction.
    let direction = match structure["readingDirection"].as_str() {
        Some(value @ ("rtl" | "ltr" | "ttb")) => value,
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

const DESKTOP_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
const DECODE: &[u8] = b"source_required_image_decode_wasm_v1";
const EXCLUSIONS: &[u8] = b"source_chapter_exclusions_v1";

pub(crate) struct Site {
    pub origin: &'static str,
    pub page_host: &'static str,
    pub cover_hosts: &'static [&'static str],
    pub work_prefix: &'static str,
    pub chapter_prefix: &'static str,
    pub page_prefix: &'static str,
    pub metadata: &'static [u8],
    pub lock_unavailable_titles: bool,
}

fn chapter_identity(value: &str) -> Result<(&str, ProductKind, &str)> {
    let mut fields = value.split(':');
    let series = fields.next().filter(|s| id(s)).ok_or("invalid_request")?;
    let kind = fields
        .next()
        .and_then(ProductKind::parse)
        .ok_or("invalid_request")?;
    let product = fields.next().filter(|s| id(s)).ok_or("invalid_request")?;
    if fields.next().is_some() {
        return Err("invalid_request");
    }
    Ok((series, kind, product))
}

#[allow(dead_code)]
impl Site {
    fn content(&self, url: &str, request: Option<Value>) -> Result<Value> {
        let origin = self.origin;
        let body = if let Some(query) = request {
            port::request(
                url,
                "POST",
                json!({
                    "Accept":"application/json","Content-Type":"application/json",
                    "Origin":self.origin,"Referer":format!("{origin}/"),"X-Requested-With":"XMLHttpRequest"
                }),
                Some(&query.to_string()),
            )?
        } else {
            port::request(
                url,
                "GET",
                json!({"Accept":"application/json","Referer":format!("{origin}/"),"X-Requested-With":"XMLHttpRequest"}),
                None,
            )?
        };
        serde_json::from_str(&body).map_err(|_| "parse_error")
    }

    pub(crate) fn graphql(&self, query: Value) -> Result<Value> {
        let origin = self.origin;
        let mut response = self.content(&format!("{origin}/graphql"), Some(query))?;
        graphql_data(&response)?;
        Ok(response["data"].take())
    }

    pub(crate) fn document(&self, url: &str) -> Result<Node> {
        Node::parse(&port::request(
            url,
            "GET",
            json!({"Accept":"text/html"}),
            None,
        )?)
    }

    fn image_url(&self, raw: &str) -> Result<Url> {
        let url = port::absolute(self.origin, raw)?;
        if url.scheme() != "https"
            || url.port().is_some()
            || url.fragment().is_some()
            || !url.host_str().is_some_and(|host| {
                host == self.page_host
                    || host == "cdn-scissors.gigaviewer.com"
                    || self.cover_hosts.contains(&host)
            })
        {
            return Err("parse_error");
        }
        Ok(url)
    }

    fn page_image(&self, raw: &str) -> Result<Url> {
        let url = self.image_url(raw)?;
        if url.host_str() != Some(self.page_host) || !url.path().starts_with("/public/page/") {
            return Err("parse_error");
        }
        Ok(url)
    }

    pub(crate) fn thumbnail(&self, raw: Option<&str>) -> Result<Value> {
        let origin = self.origin;
        let Some(raw) = raw.filter(|s| !s.is_empty()) else {
            return Ok(json!({"kind":"none"}));
        };
        let image = self.image_url(raw)?;
        Ok(json!({"kind":"url","url":image.as_str(),"headers":{"Referer":format!("{origin}/")}}))
    }

    pub(crate) fn series_id_from_thumbnail(&self, raw: &str) -> Option<String> {
        let url = self.image_url(raw).ok()?;
        let original = if url.host_str() == Some("cdn-scissors.gigaviewer.com") {
            let encoded = url.path_segments()?.next_back()?;
            let pair = format!("url={encoded}");
            let (_, decoded) = url::form_urlencoded::parse(pair.as_bytes()).next()?;
            self.image_url(&decoded).ok()?
        } else {
            url
        };
        if original.host_str() != Some(self.page_host) {
            return None;
        }
        let mut segments = original.path_segments()?.rev();
        let file = segments.next()?;
        if !segments.next()?.starts_with("series-") {
            return None;
        }
        let (id, _) = file.split_once('-')?;
        self::id(id).then(|| String::from(id))
    }

    pub(crate) fn series_item(&self, series: &Series<'_>) -> Result<Value> {
        let work_prefix = self.work_prefix;
        let url = port::absolute(self.origin, series.permalink)?;
        product_path(self.origin, &url).ok_or("parse_error")?;
        Ok(
            json!({"id":format!("{work_prefix}{}",series.id),"title":series.title,"cover":self.thumbnail(series.thumbnail)?,
        "language":"ja","url":url.as_str(),"links":[{"kind":"source","url":url.as_str()}]}),
        )
    }

    pub(crate) fn search(&self, query: &str) -> Result<Value> {
        let data = self.graphql(search_query(query))?;
        let edges = data["searchSeries"]["edges"]
            .as_array()
            .ok_or("parse_error")?;
        let mut series = Vec::new();
        let mut seen = BTreeSet::new();
        for edge in edges {
            if let Some(item) = Series::read(&edge["node"])? {
                if seen.insert(item.id) {
                    series.push(item);
                }
            }
        }
        series.sort_by_key(|item| item.volume_only);
        let items = series
            .iter()
            .map(|series| self.series_item(series))
            .collect::<Result<Vec<_>>>()?;
        Ok(port::items(items, None))
    }

    fn work<'a>(&self, input: &'a Value) -> Result<(&'a str, String)> {
        let work = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
        let suffix = work
            .strip_prefix(self.work_prefix)
            .ok_or("invalid_request")?;
        if id(suffix) {
            return Ok((work, String::from(suffix)));
        }
        let (kind, id) = suffix.split_once(':').ok_or("invalid_request")?;
        let kind = ProductKind::parse(kind).ok_or("invalid_request")?;
        if !self::id(id) {
            return Err("invalid_request");
        }
        let doc = self.document(&kind.url(self.origin, id))?;
        let series = doc
            .require("script.js-valve[data-giga_series]")?
            .attr("data-giga_series")?;
        if !self::id(&series) {
            return Err("parse_error");
        }
        Ok((work, series))
    }

    fn series_details(&self, id: &str) -> Result<Value> {
        let mut data = self.graphql(series_query(id))?;
        if data["series"].is_null() {
            return Err("not_found");
        }
        if data["series"]["databaseId"].as_str() != Some(id) {
            return Err("parse_error");
        }
        Ok(data["series"].take())
    }

    pub(crate) fn detail(&self, input: &Value) -> Result<Value> {
        let (work, id) = self.work(input)?;
        let node = self.series_details(&id)?;
        let series = Series::read(&node)?.ok_or("not_found")?;
        let mut item = self.series_item(&series)?;
        item["id"] = json!(work);
        item["description"] = json!(node["description"].as_str().unwrap_or(""));
        item["authors"] = json!(node["author"]["name"]
            .as_str()
            .into_iter()
            .collect::<Vec<_>>());
        // This is the format tag consumed by Koma's source reader preferences.
        item["tags"] = json!(["Manga"]);
        Ok(json!({"manga":item}))
    }

    pub(crate) fn chapters(&self, request: &[u8]) -> ChapterResult<'static, &'static str> {
        let chapter_prefix = self.chapter_prefix;
        let input = port::input(request)?;
        let (work, series) = self.work(&input)?;
        let node = self.series_details(&series)?;
        let episode_count = ProductKind::Episode.count(&node)?;
        let volume_count = ProductKind::Volume.count(&node)?;
        let args = &input["args"];
        let exact = args["chapterId"].as_str();
        let (kind, offset, anchor) = match &args["page"]["cursor"] {
            Value::Null if episode_count == 0 => (ProductKind::Volume, 0, None),
            Value::Null => (ProductKind::Episode, 0, None),
            value => {
                let cursor = value.as_str().ok_or("invalid_request")?;
                let mut fields = cursor.split(':');
                let kind = fields
                    .next()
                    .and_then(ProductKind::parse)
                    .ok_or("invalid_request")?;
                let offset = fields
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or("invalid_request")?;
                let anchor = fields.next().filter(|s| id(s));
                if (offset == 0) != anchor.is_none() || fields.next().is_some() {
                    return Err("invalid_request");
                }
                (kind, offset, anchor)
            }
        };
        let mut output =
            GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
        let count = match kind {
            ProductKind::Episode => episode_count,
            ProductKind::Volume => volume_count,
        };
        if count == 0 {
            return output
                .finish(ChapterPage::Complete)
                .map_err(port::chapter_error);
        }
        if offset >= count {
            return Err("catalog_changed");
        }

        // Check the preceding row on continuation. A newly inserted or reordered
        // episode must fail the in-flight catalog instead of silently skipping it.
        let fetch_offset = offset.saturating_sub(1);
        let response = self.content(&kind.catalog_url(self.origin, &series, fetch_offset), None)?;
        let rows = response.as_array().ok_or("parse_error")?;
        let rows = if let Some(anchor) = anchor {
            let (previous, rows) = rows.split_first().ok_or("catalog_changed")?;
            if Product::read(previous, self.origin, kind)?.id != anchor {
                return Err("catalog_changed");
            }
            rows
        } else {
            rows
        };
        if rows.is_empty() || rows.len() > 100 || offset + rows.len() > count {
            return Err("catalog_changed");
        }
        let labels = Labels::new(self.metadata, request)?;
        let can_exclude = source_protocol::feature(request, EXCLUSIONS);
        let mut excluded = Vec::new();
        let mut seen = BTreeSet::new();
        if let Some(anchor) = anchor {
            seen.insert(anchor);
        }
        let mut last = "";
        for (position, row) in rows.iter().enumerate() {
            if host::check_cancel() {
                return Err("cancelled");
            }
            let product = Product::read(row, self.origin, kind)?;
            if !seen.insert(product.id) {
                return Err("catalog_changed");
            }
            last = product.id;
            let id = format!("{chapter_prefix}{series}:{}:{}", kind.name(), product.id);
            if exact.is_some_and(|exact| exact != id) {
                continue;
            }
            let hide = match product.access {
                Access::Locked => input["settings"]["hide_locked"].as_bool() == Some(true),
                Access::Unavailable => {
                    input["settings"]["hide_unavailable"].as_bool() == Some(true)
                }
                Access::Public => false,
            };
            if can_exclude && exact.is_none() && hide {
                excluded.push(id);
                continue;
            }
            let title = match product.access {
                Access::Unavailable => format!(
                    "{} · {}",
                    labels.text("chapter.unavailable", "配信終了")?,
                    product.title
                ),
                Access::Locked | Access::Public => String::from(product.title),
            };
            let title = if kind == ProductKind::Volume {
                format!("{} · {title}", labels.text("chapter.volume", "単行本")?)
            } else {
                title
            };
            // Koma 1.6.3 displays the lock glyph from the title. New sites
            // show it for every inaccessible product while retaining the
            // provider's distinct paid and unavailable settings.
            let show_lock = product.access == Access::Locked
                || (self.lock_unavailable_titles && product.access == Access::Unavailable);
            let title = if show_lock {
                format!("🔒 {title}")
            } else {
                title
            };
            let url = kind.url(self.origin, product.id);
            let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), title.as_bytes());
            chapter.url = Some(url.as_bytes());
            chapter.language = Some(b"ja");
            chapter.published_at = product.published.map(str::as_bytes);
            chapter.locked = Some(product.access != Access::Public);
            let placement = if product.access == Access::Public {
                ChapterPlacement::InGroup {
                    group: kind.name().as_bytes(),
                    position: (count - offset - position - 1) as u64,
                }
            } else {
                ChapterPlacement::ManualOnly
            };
            output
                .push(chapter, placement)
                .map_err(port::chapter_error)?;
            if exact.is_some() {
                return output
                    .finish(ChapterPage::Complete)
                    .map_err(port::chapter_error);
            }
        }
        let next = if offset + rows.len() < count {
            Some(format!("{}:{}:{last}", kind.name(), offset + rows.len()))
        } else if kind == ProductKind::Episode && volume_count > 0 {
            Some(String::from("volume:0"))
        } else {
            None
        };
        let page = next.as_ref().map_or(ChapterPage::Complete, |next| {
            ChapterPage::NextCursor(next.as_bytes())
        });
        if excluded.is_empty() {
            return output.finish(page).map_err(port::chapter_error);
        }
        let mut output = output
            .with_excluded_ids(page)
            .map_err(port::chapter_error)?;
        for id in &excluded {
            output.push(id.as_bytes()).map_err(port::chapter_error)?;
        }
        output.finish().map_err(port::chapter_error)
    }

    fn viewer(&self, kind: ProductKind, id: &str) -> Result<Value> {
        // The same official episode can return usagi on mobile and baku on
        // desktop. The ordinary viewer GET selects the verified desktop variant.
        let body = port::request(
            &kind.url(self.origin, id),
            "GET",
            json!({"Accept":"text/html","User-Agent":DESKTOP_UA}),
            None,
        )?;
        let doc = Node::parse(&body)?;
        let config = doc
            .require("script#episode-json[data-value]")?
            .attr_bounded("data-value", 1024 * 1024)?;
        serde_json::from_str(&config).map_err(|_| "parse_error")
    }

    pub(crate) fn pages(&self, input: &Value) -> Result<Value> {
        let page_prefix = self.page_prefix;
        let chapter = input["args"]["chapterId"]
            .as_str()
            .ok_or("invalid_request")?;
        let key = chapter
            .strip_prefix(self.chapter_prefix)
            .ok_or("invalid_request")?;
        let (series, kind, product) = chapter_identity(key)?;
        let value = self.viewer(kind, product)?;
        let viewer = viewer(&value, self.origin, kind, product, series)?;
        let pages = viewer.pages.iter().enumerate().map(|(index,raw)| -> Result<Value> {
        let url = self.page_image(raw)?;
        Ok(json!({"id":format!("{page_prefix}{key}:{index}"),"index":index,"image":{"kind":"url","url":url.as_str()}}))
    }).collect::<Result<Vec<_>>>()?;
        // Keep the provider's indices intact. On Koma the Manga format tag sets
        // the default RTL reading preference without overriding the user's mode.
        Ok(json!({"chapterId":chapter,"pages":pages,"readingDirection":viewer.direction}))
    }

    pub(crate) fn image_request(&self, request: &[u8], input: &Value) -> Result<Value> {
        let origin = self.origin;
        let args = &input["args"];
        let raw = args["url"].as_str().ok_or("invalid_request")?;
        let url = self.image_url(raw)?;
        let key = args["pageId"]
            .as_str()
            .and_then(|id| id.strip_prefix(self.page_prefix));
        let Some(key) = key else {
            if url.path().starts_with("/public/page/") {
                return Err("invalid_request");
            }
            return Ok(
                json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":format!("{origin}/")}}}),
            );
        };
        let (identity, index) = key.rsplit_once(':').ok_or("invalid_request")?;
        let index = index.parse::<usize>().map_err(|_| "invalid_request")?;
        let (series, kind, product) = chapter_identity(identity)?;
        // A cold resolver must find the render mode and public access from the
        // current episode. The ID only locates the page, with no per-image config.
        let value = self.viewer(kind, product)?;
        let view = viewer(&value, self.origin, kind, product, series)?;
        let current = view.pages.get(index).ok_or("page_unavailable")?;
        if self.page_image(current)? != url {
            return Err("page_unavailable");
        }
        let mut result = json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":kind.url(self.origin,product)}}});
        if matches!(view.tiles, Tiles::Baku) {
            if !source_protocol::feature(request, DECODE) {
                return Err("host_feature_required");
            }
            result["requiredImageDecode"] =
                json!({"kind":"wasm_image_v1","pixels":r#"{"mode":"baku"}"#});
        }
        Ok(result)
    }
}
