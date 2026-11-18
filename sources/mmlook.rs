use super::port_support::{self as port, Node, Result};
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    source::SourceCapabilities,
};
use serde_json::{json, Value};
use url::{form_urlencoded, Url};

pub(crate) struct Site {
    pub desktop: &'static str,
    pub mobile: &'static str,
    pub legacy_work_id: bool,
}

pub(crate) const CAPABILITIES: SourceCapabilities = SourceCapabilities {
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

const RANKS: &[(&str, &str)] = &[
    ("", "不查看"),
    ("1", "精品榜"),
    ("2", "人气榜"),
    ("3", "推荐榜"),
    ("4", "黑马榜"),
    ("5", "最近更新"),
    ("6", "新漫画"),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("", "全部"),
    ("1", "冒险"),
    ("2", "热血"),
    ("3", "都市"),
    ("4", "玄幻"),
    ("5", "悬疑"),
    ("6", "耽美"),
    ("7", "恋爱"),
    ("8", "生活"),
    ("9", "搞笑"),
    ("10", "穿越"),
    ("11", "修真"),
    ("12", "后宫"),
    ("13", "女主"),
    ("14", "古风"),
    ("15", "连载"),
    ("16", "完结"),
];

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

impl Site {
    fn work_id(&self, slug: &str) -> String {
        if self.legacy_work_id {
            format!("/{slug}/")
        } else {
            slug.to_string()
        }
    }

    fn work_slug<'a>(&self, args: &'a Value) -> Result<&'a str> {
        let id = args["mangaId"].as_str().ok_or("invalid_request")?;
        let slug = id.trim_matches('/');
        if !valid_id(slug) || self.work_id(slug) != id {
            return Err("invalid_request");
        }
        Ok(slug)
    }

    fn work_path(&self, value: &str) -> Result<String> {
        let url = port::absolute(self.desktop, value)?;
        let desktop = Url::parse(self.desktop).map_err(|_| "parse_error")?;
        let mobile = Url::parse(self.mobile).map_err(|_| "parse_error")?;
        let slug = url.path().trim_matches('/');
        if !valid_id(slug)
            || (url.host_str() != desktop.host_str() && url.host_str() != mobile.host_str())
        {
            return Err("parse_error");
        }
        Ok(slug.to_string())
    }

    fn post(
        &self,
        path: &str,
        key: &str,
        value: &str,
        referer: &str,
        ajax: bool,
    ) -> Result<String> {
        let form: String = form_urlencoded::Serializer::new(String::new())
            .append_pair(key, value)
            .finish();
        let mut headers = json!({
            "Referer":referer,"Content-Type":"application/x-www-form-urlencoded",
            "Accept":if ajax { "application/json" } else { "text/html" },
        });
        if ajax {
            headers["X-Requested-With"] = json!("XMLHttpRequest");
        }
        port::request(
            &format!("{}{path}", self.desktop),
            "POST",
            headers,
            Some(&form),
        )
    }

    fn cover(&self, node: &Node) -> Result<Option<Value>> {
        let Some(image) = node.one("img") else {
            return Ok(None);
        };
        let raw = image.attr("data-src").unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(
            json!({"kind":"url","url":port::absolute(self.desktop, &raw)?.as_str()}),
        ))
    }

    fn listing(&self, args: &Value, search: bool) -> Result<Value> {
        let query = args["query"].as_str().unwrap_or("").trim();
        let search_text = search && !query.is_empty();
        let listing = args["listingId"].as_str().unwrap_or("popular");
        let body = if search_text {
            let keyword = query.chars().take(12).collect::<String>();
            self.post("/s", "k", &keyword, self.desktop, false)?
        } else {
            let rank = port::filter(args, "rank", RANKS)?;
            let category = port::filter(args, "category", CATEGORIES)?;
            let path = if !rank.is_empty() {
                format!("/rank/{rank}")
            } else if !category.is_empty() {
                format!("/sort/{category}")
            } else {
                match listing {
                    "popular" => "/rank/1".into(),
                    "latest" => "/rank/5".into(),
                    _ => return Err("invalid_request"),
                }
            };
            port::get(&format!("{}{path}", self.desktop))?
        };
        let document = Node::parse(&body)?;
        let selector = if search_text {
            ".item-data > div"
        } else {
            ".likedata"
        };
        let mut results = Vec::new();
        for entry in document.all(selector)? {
            let slug = self.work_path(&entry.require("a[href]")?.attr("href")?)?;
            let title = entry
                .require(if search_text {
                    ".e-title, .title"
                } else {
                    ".le-t"
                })?
                .text()?;
            let author = entry
                .one(if search_text { ".tip" } else { ".likeinfo > p" })
                .map(|node| node.text())
                .transpose()?
                .unwrap_or_default();
            let mut manga = json!({
                "id":self.work_id(&slug),"title":title,
                "authors":[author.trim_start_matches("作者：")],
                "links":[{"kind":"source","url":format!("{}/{slug}/",self.mobile)}]
            });
            if let Some(cover) = self.cover(&entry)? {
                manga["cover"] = cover;
            }
            if let Some(description) = entry.one(".le-j") {
                manga["description"] = json!(description.text()?);
            }
            results.push(manga);
        }
        Ok(port::items(results, None))
    }

    fn details(&self, args: &Value) -> Result<Value> {
        let slug = self.work_slug(args)?;
        let body = port::get(&format!("{}/{slug}/", self.desktop))?;
        let document = Node::parse(&body)?;
        let info = document.require(".comicInfo")?;
        let fields = info.require(".detinfo")?;
        let mut manga = json!({
            "id":self.work_id(slug),"title":fields.require("h1")?.text()?,
            "links":[{"kind":"source","url":format!("{}/{slug}/",self.mobile)}]
        });
        if let Some(cover) = self.cover(&info)? {
            manga["cover"] = cover;
        }
        let mut updated = String::new();
        for field in fields.all("span")? {
            let text = field.text()?;
            let Some((label, value)) = text.split_once('：') else {
                continue;
            };
            match label.replace(' ', "").as_str() {
                "作者" => manga["authors"] = json!([value.trim()]),
                "标签" => manga["tags"] = json!(value.split_whitespace().collect::<Vec<_>>()),
                "状态" => {
                    manga["status"] = json!(match value.trim() {
                        "连载中" => "ongoing",
                        "已完结" => "completed",
                        _ => "unknown",
                    });
                }
                "更新时间" => updated = text,
                _ => {}
            }
        }
        let description = fields.require(".content")?.text()?;
        manga["description"] = json!(if updated.is_empty() {
            description
        } else {
            format!("{updated}\n\n{description}")
        });
        Ok(json!({"manga":manga}))
    }

    pub(crate) fn chapters(&self, request: &[u8]) -> ChapterResult<'static, &'static str> {
        let value = port::input(request)?;
        let args = &value["args"];
        let slug = self.work_slug(args)?;
        let owner = self.work_id(slug);
        let referer = format!("{}/{slug}/", self.desktop);
        let body = port::get(&referer)?;
        let document = Node::parse(&body)?;
        let catalog = document.require(".chapterlistload")?;
        let mut rows = Vec::new();
        for link in document.all(".chapterlistload > :first-child a[href]")? {
            let href = link.attr("href")?;
            let prefix = format!("/{slug}/");
            let id = href
                .strip_prefix(&prefix)
                .and_then(|href| href.strip_suffix(".html"))
                .filter(|id| valid_id(id))
                .ok_or("parse_error")?;
            rows.push((format!("{slug}/{id}"), link.text()?));
        }
        if catalog.one(".chaplist-more").is_some() {
            let body = self.post("/morechapter", "id", slug, &referer, true)?;
            let data: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
            for entry in data["data"].as_array().ok_or("parse_error")? {
                let id = entry["chapterid"]
                    .as_str()
                    .filter(|id| valid_id(id))
                    .ok_or("parse_error")?;
                let title = entry["chaptername"].as_str().ok_or("parse_error")?;
                rows.push((format!("{slug}/{id}"), title.to_string()));
            }
        }
        let mut seen = BTreeSet::new();
        rows.retain(|(id, _)| seen.insert(id.clone()));
        let chapters = rows
            .iter()
            .enumerate()
            .map(|(index, (id, title))| {
                let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
                chapter.index = Some(index);
                chapter
            })
            .collect::<Vec<_>>();
        ChapterResponse::ordered(
            super::payload_buf(),
            request,
            ChapterOrder::NewestFirst,
            &chapters,
        )
        .map_err(port::chapter_error)
    }

    fn pages(&self, args: &Value) -> Result<Value> {
        let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
        let (slug, id) = chapter.split_once('/').ok_or("invalid_request")?;
        if !valid_id(slug)
            || !valid_id(id)
            || args["mangaId"]
                .as_str()
                .is_some_and(|owner| owner != self.work_id(slug))
        {
            return Err("invalid_request");
        }
        let url = format!("{}/{chapter}.html", self.mobile);
        let html = port::get(&url)?;
        let document = Node::parse(&html)?;
        let index = document
            .require(".readerContainer")?
            .attr("data-id")?
            .parse::<usize>()
            .map_err(|_| "parse_error")?;
        let raw_pages = unpack_urls(&html, index)?;
        if raw_pages.is_empty() {
            return Err("chapter_unavailable");
        }
        let pages = raw_pages
            .iter()
            .enumerate()
            .map(|(index, raw)| Ok(port::page(index, chapter, port::absolute(&url, raw)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"pages":pages}))
    }

    pub(crate) fn dispatch(&self, operation: &str, request: &[u8]) -> Result<Value> {
        let value = port::input(request)?;
        let args = &value["args"];
        match operation {
            "get_listings" => Ok(port::listing_list(false, false, true)),
            "get_filters" => Ok(json!({"filters":[
                port::select("rank","排行榜","browse",RANKS),
                port::select("category","分类","browse",CATEGORIES),
            ]})),
            "search" => self.listing(args, true),
            "get_manga_list" => self.listing(args, false),
            "get_manga" => self.details(args),
            "get_pages" => self.pages(args),
            _ => Err("not_implemented"),
        }
    }
}

#[path = "javascript_packer.rs"]
mod javascript_packer;

fn unpack_urls(html: &str, index: usize) -> Result<Vec<String>> {
    let program_expanded = javascript_packer::expand(html)?;
    let value = program_expanded
        .strip_prefix("var __c0rst96=\"")
        .and_then(|v| v.strip_suffix('"'))
        .ok_or("parse_error")?;
    let keys = [
        "smkhy258", "smkd95fv", "md496952", "cdcsdwq", "vbfsa256", "cawf151c", "cd56cvda",
        "8kihnt9", "dso15tlo", "5ko6plhy",
    ];
    let key = keys.get(index).ok_or("parse_error")?.as_bytes();
    let mut bytes = STANDARD.decode(value).map_err(|_| "parse_error")?;
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte ^= key[index % key.len()];
    }
    let urls = STANDARD.decode(bytes).map_err(|_| "parse_error")?;
    serde_json::from_slice(&urls).map_err(|_| "parse_error")
}

macro_rules! operations {
    ($name:literal) => {
        fn run(operation: &str, request: &[u8]) -> u32 {
            port_support::write_result(operation, SITE.dispatch(operation, request))
        }
        fn run_search(r: &[u8]) -> u32 {
            run("search", r)
        }
        fn run_get_manga(r: &[u8]) -> u32 {
            run("get_manga", r)
        }
        fn run_get_chapters(
            r: &[u8],
        ) -> koma_source_sdk::chapters::ChapterResult<'static, &'static str> {
            SITE.chapters(r)
        }
        fn write_chapter_failure(code: &str) -> u32 {
            port_support::write_failure("get_chapters", code)
        }
        fn run_get_pages(r: &[u8]) -> u32 {
            run("get_pages", r)
        }
        fn run_get_listings(r: &[u8]) -> u32 {
            run("get_listings", r)
        }
        fn run_get_manga_list(r: &[u8]) -> u32 {
            run("get_manga_list", r)
        }
        fn run_get_filters(r: &[u8]) -> u32 {
            run("get_filters", r)
        }
        fn run_get_settings(r: &[u8]) -> u32 {
            run("get_settings", r)
        }
        fn run_get_home(r: &[u8]) -> u32 {
            run("get_home", r)
        }
        fn run_get_image_request(r: &[u8]) -> u32 {
            run("get_image_request", r)
        }
        koma_source_sdk::koma_source_exports!($name, typed_chapters, write_chapter_failure);
    };
}
pub(crate) use operations;
