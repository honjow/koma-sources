use super::port_support::{self as port, Node, Result};
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::SourceCapabilities,
};
use serde_json::{json, Value};
use url::Url;

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Woai,
    Buding,
}

pub(crate) struct Site {
    pub mirrors: &'static [&'static str],
    pub work_prefix: &'static str,
    pub kind: Kind,
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

// These origins serve a different DOM to desktop clients. The mobile HTML
// request profile is separate from any interactive WebView verification.
const MOBILE: &str = "Mozilla/5.0 (Linux; Android 13; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
const WOAI_TAGS: &[(&str, &str)] = &[
    ("", "全部"),
    ("恋爱", "恋爱"),
    ("古风", "古风"),
    ("校园", "校园"),
    ("奇幻", "奇幻"),
    ("大女主", "大女主"),
    ("治愈", "治愈"),
    ("穿越", "穿越"),
    ("励志", "励志"),
    ("爆笑", "爆笑"),
    ("萌系", "萌系"),
    ("玄幻", "玄幻"),
    ("日常", "日常"),
    ("都市", "都市"),
    ("彩虹", "彩虹"),
    ("灵异", "灵异"),
    ("悬疑", "悬疑"),
    ("少年", "少年"),
];
const BUDING_TAGS: &[(&str, &str)] = &[
    ("", "全部"),
    ("恐怖", "恐怖"),
    ("科幻", "科幻"),
    ("悬疑", "悬疑"),
    ("真人", "真人"),
    ("穿越", "穿越"),
    ("战争", "战争"),
    ("后宫", "后宫"),
    ("霸总", "霸总"),
    ("搞笑", "搞笑"),
    ("生活", "生活"),
    ("恋爱", "恋爱"),
    ("校园", "校园"),
    ("修真", "修真"),
    ("玄幻", "玄幻"),
    ("热血", "热血"),
];
const WOAI_AREAS: &[(&str, &str)] = &[
    ("", "全部"),
    ("1", "国产漫画"),
    ("2", "日本漫画"),
    ("3", "韩国漫画"),
    ("5", "欧美漫画"),
    ("7", "其他漫画"),
    ("8", "日韩小说"),
];
const BUDING_AREAS: &[(&str, &str)] = &[("", "全部"), ("3", "韩漫"), ("2", "日漫"), ("1", "国漫")];
const WOAI_STATES: &[(&str, &str)] = &[
    ("", "全部"),
    ("last", "最新"),
    ("0", "连载中"),
    ("1", "已完结"),
];
const BUDING_STATES: &[(&str, &str)] = &[("", "全部"), ("2", "连载"), ("1", "完结")];

fn valid_segment(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

impl Site {
    pub(crate) fn is_buding(&self) -> bool {
        matches!(self.kind, Kind::Buding)
    }

    fn allowed(&self, url: &Url) -> bool {
        url.host_str().is_some_and(|host| {
            self.mirrors.iter().any(|base| {
                Url::parse(base)
                    .ok()
                    .and_then(|url| url.host_str().map(String::from))
                    .is_some_and(|allowed| {
                        host.trim_start_matches("www.") == allowed.trim_start_matches("www.")
                    })
            })
        })
    }

    fn route(&self, raw: &str, prefix: &str) -> Result<String> {
        let url = port::absolute(self.mirrors[0], raw)?;
        let id = url
            .path()
            .strip_prefix(prefix)
            .filter(|id| valid_segment(id))
            .ok_or("invalid_request")?;
        if !self.allowed(&url) {
            return Err("invalid_request");
        }
        Ok(format!("{prefix}{id}"))
    }

    fn fetch(&self, path: &str) -> Result<(Url, String)> {
        let mut failure = "network_error";
        for base in self.mirrors {
            let url = port::absolute(base, path)?;
            if !self.allowed(&url) {
                return Err("invalid_request");
            }
            match port::request(
                url.as_str(),
                "GET",
                json!({"Accept":"text/html","User-Agent":MOBILE}),
                None,
            ) {
                Ok(body) => return Ok((url, body)),
                Err(code @ ("network_error" | "server_error" | "timeout")) => failure = code,
                Err(code) => return Err(code),
            }
        }
        Err(failure)
    }

    fn text_field(document: &Node, label: &str) -> Result<Option<String>> {
        for row in document.all("p.detail-main-info-author")? {
            if row.text()?.trim_start().starts_with(label) {
                let value = row.one("a").map(|a| a.text()).transpose()?;
                return Ok(value.filter(|value| !value.is_empty() && value != "未知"));
            }
        }
        Ok(None)
    }

    fn src(node: Option<Node>, attr: &str, url: &Url) -> Result<Option<Value>> {
        let Some(node) = node else { return Ok(None) };
        let src = node.attr(attr).unwrap_or_default();
        let Ok(image) = port::absolute(url.as_str(), &src) else {
            return Ok(None);
        };
        Ok(Some(json!({"kind":"url","url":image.as_str()})))
    }

    fn card(
        &self,
        loc: &Url,
        link: &Node,
        node: &Node,
        cover_field: &str,
        use_title: bool,
    ) -> Result<Value> {
        let path = self.route(&link.attr("href")?, self.work_prefix)?;
        let title = if use_title {
            link.attr("title").unwrap_or_default()
        } else {
            link.text()?
        };
        if title.is_empty() {
            return Err("parse_error");
        }
        let mut manga = json!({"id":path,"title":title,
            "links":[{"kind":"source","url":port::absolute(loc.as_str(),&path)?.as_str()}]});
        if let Some(cover) = Self::src(node.one("img"), cover_field, loc)? {
            manga["cover"] = cover;
        }
        Ok(manga)
    }

    fn filters(&self) -> [&'static [(&'static str, &'static str)]; 3] {
        if self.is_buding() {
            [BUDING_TAGS, BUDING_AREAS, BUDING_STATES]
        } else {
            [WOAI_TAGS, WOAI_AREAS, WOAI_STATES]
        }
    }

    fn from_link(&self, url: &Url) -> Result<String> {
        if !self.allowed(url) {
            return Err("not_found");
        }
        if url.path().starts_with(self.work_prefix) {
            return self.route(url.as_str(), self.work_prefix);
        }
        let path = self.route(url.as_str(), "/chapter/")?;
        let (_, html) = self.fetch(&path)?;
        let document = Node::parse(&html)?;
        let link = document.require(&format!(
            r#"a.view-fix-bottom-bar-item[href^="{}"]"#,
            self.work_prefix
        ))?;
        self.route(&link.attr("href")?, self.work_prefix)
    }

    fn list(&self, request: &[u8], args: &Value, search: bool) -> Result<Value> {
        let keyword = args["query"].as_str().unwrap_or("").trim();
        let page = request_page_number(request);
        let listing = args["listingId"].as_str().unwrap_or("popular");
        let nonempty = search && !keyword.is_empty();
        let schemas = self.filters();
        let category = port::filter(args, "category", schemas[0])?;
        let area = port::filter(args, "area", schemas[1])?;
        let state = port::filter(args, "state", schemas[2])?;
        let unfiltered = category.is_empty() && area.is_empty() && state.is_empty();
        let ranking = !self.is_buding() && !search && listing == "popular" && unfiltered;
        let (loc, html) = if nonempty {
            if let Ok(url) = Url::parse(keyword) {
                if matches!(url.scheme(), "http" | "https") {
                    return Ok(port::items(
                        vec![self.details(&self.from_link(&url)?)?],
                        None,
                    ));
                }
            }
            let mut url = port::absolute(self.mirrors[0], "/search")?;
            url.query_pairs_mut().append_pair("keyword", keyword);
            self.fetch(&format!(
                "{}?{}",
                url.path(),
                url.query().ok_or("invalid_request")?
            ))?
        } else if ranking {
            self.fetch("/rank")?
        } else {
            let mut url = port::absolute(self.mirrors[0], "/booklist")?;
            let mut params = url.query_pairs_mut();
            if self.is_buding() {
                for (key, value) in [("cate", category), ("area", area), ("end", state)] {
                    if !value.is_empty() {
                        params.append_pair(key, value);
                    }
                }
            } else {
                let latest =
                    state == "last" || (!search && listing == "latest" && state.is_empty());
                params
                    .append_pair(
                        "tag",
                        if category.is_empty() {
                            "全部"
                        } else {
                            category
                        },
                    )
                    .append_pair("area", if area.is_empty() { "-1" } else { area })
                    .append_pair(
                        "end",
                        if latest || state.is_empty() {
                            "-1"
                        } else {
                            state
                        },
                    );
                if latest {
                    params.append_pair("last", "1");
                }
            }
            params.append_pair("page", &page.to_string());
            drop(params);
            self.fetch(&format!(
                "{}?{}",
                url.path(),
                url.query().ok_or("invalid_request")?
            ))?
        };
        let document = Node::parse(&html)?;
        let mut items = Vec::new();
        let next = if nonempty || ranking {
            let selector = if nonempty {
                "div.book-list-cover a[href]"
            } else {
                "#rankList_2 > a[href]"
            };
            for link in document.all(selector)? {
                items.push(self.card(&loc, &link, &link, "data-original", true)?);
            }
            false
        } else {
            let selector = if self.is_buding() {
                "ul.manga-list-2 li"
            } else {
                "ul.manga-list-2 li"
            };
            for node in document.all(selector)? {
                let Some(link) = node.one("p.manga-list-2-title a") else {
                    continue;
                };
                let raw = link.attr("href")?;
                if self.route(&raw, self.work_prefix).is_err() {
                    continue;
                }
                items.push(self.card(
                    &loc,
                    &link,
                    &node,
                    if self.is_buding() {
                        "src"
                    } else {
                        "data-original"
                    },
                    false,
                )?);
            }
            document
                .one(if self.is_buding() {
                    "ul.pagination li:last-child a[href*='page=']"
                } else {
                    "a.paginate-btn[title='下一页']"
                })
                .is_some()
        };
        if !nonempty && items.is_empty() && document.one("ul.manga-list-2, #rankList_2").is_none() {
            return Err("parse_error");
        }
        Ok(port::items(
            items,
            if next { page.checked_add(1) } else { None },
        ))
    }

    fn details(&self, path: &str) -> Result<Value> {
        let (url, html) = self.fetch(path)?;
        let document = Node::parse(&html)?;
        let mut item = json!({"id":path, "title":document.require("p.detail-main-info-title")?.text()?,
            "links":[{"kind":"source","url":url.as_str()}]});
        if let Some(cover) = Self::src(
            document.one("div.detail-main-cover img"),
            if self.is_buding() {
                "data-original"
            } else {
                "src"
            },
            &url,
        )? {
            item["cover"] = cover;
        }
        let mut tags = Vec::new();
        if let Some(region) = Self::text_field(&document, "地区")? {
            tags.push(region);
        }
        for tag in document.all("p.detail-main-info-class a")? {
            tags.push(tag.text()?);
        }
        item["tags"] = json!(tags);
        let mut description = document
            .one("p.detail-desc")
            .map(|p| p.text())
            .transpose()?
            .unwrap_or_default();
        if !self.is_buding() {
            if let Some(author) = Self::text_field(&document, "作者")? {
                item["authors"] = json!([author]);
            }
            if let Some(alt) = Self::text_field(&document, "别名")? {
                if !description.is_empty() {
                    description.push_str("\n\n");
                }
                description.push_str(&format!("别名：{alt}"));
            }
            if let Some(state) = document.one("span.detail-list-title-1") {
                item["status"] = json!(match state.text()?.as_str() {
                    "连载中" => "ongoing",
                    "已完结" => "completed",
                    _ => "unknown",
                });
            }
        }
        item["description"] = json!(description);
        Ok(item)
    }

    pub(crate) fn chapters(&self, request: &[u8]) -> ChapterResult<'static, &'static str> {
        let input = port::input(request)?;
        let path = self.route(
            input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
            self.work_prefix,
        )?;
        let (_, body) = self.fetch(&path)?;
        let document = Node::parse(&body)?;
        let mut rows = Vec::new();
        for entry in document.all("#detail-list-select li")?.into_iter().rev() {
            let Some(anchor) = entry.one("a.chapteritem") else {
                continue;
            };
            let id = self.route(&anchor.attr("href")?, "/chapter/")?;
            let locked = !self.is_buding() && entry.one("img.detail-list-3-info-right").is_some();
            rows.push((id, anchor.text()?, locked));
        }
        if rows.is_empty() && document.one("#detail-list-select").is_none() {
            return Err("parse_error");
        }
        let date = document
            .one("span.detail-list-title-3")
            .map(|n| n.text())
            .transpose()?
            .unwrap_or_default();
        let date = date.trim_end_matches("更新").trim();
        let chapters = rows
            .iter()
            .enumerate()
            .map(|(index, (id, title, locked))| {
                let mut chapter = Chapter::new(id.as_bytes(), path.as_bytes(), title.as_bytes());
                chapter.index = Some(index);
                chapter.locked = locked.then_some(true);
                if index == 0 && date.len() == 10 {
                    chapter.published_at = Some(date.as_bytes());
                }
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
        let chapter = self.route(
            args["chapterId"].as_str().ok_or("invalid_request")?,
            "/chapter/",
        )?;
        let mut path = chapter.clone();
        let mut visited = BTreeSet::new();
        let mut pages = Vec::new();
        loop {
            if !visited.insert(path.clone()) || visited.len() > 100 {
                return Err("parse_error");
            }
            let (loc, html) = self.fetch(&path)?;
            let document = Node::parse(&html)?;
            let images = document.all("div.readForm img")?;
            if images.is_empty() {
                return Err(
                    if document.one("a[href*='/login']").is_some() && pages.is_empty() {
                        "auth_required"
                    } else {
                        "chapter_unavailable"
                    },
                );
            }
            for image in images {
                let key = if self.is_buding() {
                    "data-original"
                } else {
                    "src"
                };
                let mut url = port::absolute(loc.as_str(), &image.attr(key)?)?;
                if !self.is_buding() && url.scheme() == "http" {
                    url.set_scheme("https").map_err(|_| "parse_error")?;
                }
                let mut page = port::page(pages.len(), &chapter, url);
                if self.is_buding() {
                    page["image"]["headers"] = port::mobile_image_headers(loc.as_str());
                }
                pages.push(page);
            }
            if !self.is_buding() {
                break;
            }
            let Some(next) = document.one("a.view-bottom-bar-item[title='下一页'][href]") else {
                break;
            };
            let next = port::absolute(loc.as_str(), &next.attr("href")?)?;
            if !self.allowed(&next) || next.path() != chapter {
                return Err("parse_error");
            }
            let query = next.query().ok_or("parse_error")?;
            path = format!("{}?{query}", next.path());
        }
        Ok(json!({"pages":pages}))
    }

    pub(crate) fn comments(&self, request: &[u8]) -> Result<Value> {
        let request = koma_source_sdk::comments::request(request)?;
        if request.chapter_id.is_some() || request.cursor.is_some() {
            return Err("invalid_request");
        }
        let work = core::str::from_utf8(request.manga_id).map_err(|_| "invalid_request")?;
        let path = self.route(work, self.work_prefix)?;
        let (_, html) = self.fetch(&path)?;
        let document = Node::parse(&html)?;
        document.require("#comment .detail-list-comment")?;
        let mut items = Vec::new();
        let mut repeated = alloc::collections::BTreeMap::new();
        for row in document.all("#comment .detail-list-comment > li")? {
            let title = row.require(".detail-list-comment-title")?;
            let name = title.text()?;
            let likes = title
                .one(".zanbtn")
                .map(|node| node.text())
                .transpose()?
                .unwrap_or_default();
            let author = if likes.is_empty() {
                name.as_str()
            } else {
                name.strip_suffix(&format!(" {likes}"))
                    .unwrap_or(&name)
                    .trim_end()
            };
            let date = row
                .one(".detail-list-comment-subtitle")
                .map(|node| node.text())
                .transpose()?
                .unwrap_or_default();
            let body = row.require(".detail-list-comment-content")?.text()?;
            // Rows lack provider IDs. Exclude mutable vote counts from identity.
            let digest = author
                .bytes()
                .chain([0])
                .chain(date.bytes())
                .chain([0])
                .chain(body.bytes())
                .fold(0xcbf29ce484222325u64, |hash, byte| {
                    (hash ^ byte as u64).wrapping_mul(0x100000001b3)
                });
            let count = repeated.entry(digest).or_insert(0usize);
            let mut item =
                json!({"id":format!("{digest:016x}-{count}"),"author":author,"content":body});
            if !date.is_empty() {
                item["createdAt"] = json!(date);
            }
            items.push(item);
            *count += 1;
        }
        Ok(json!({"items":items}))
    }

    pub(crate) fn dispatch(&self, op: &str, request: &[u8]) -> Result<Value> {
        let input = port::input(request)?;
        let args = &input["args"];
        let schemas = self.filters();
        match op {
            "get_listings" => {
                if self.is_buding() {
                    Ok(
                        json!({"listings":[{"id":"popular","name":"最近更新","supportsPaging":true}]}),
                    )
                } else {
                    Ok(port::listing_list(true, false, true))
                }
            }
            "get_filters" => Ok(
                json!({"filters":[port::select("category","题材","browse",schemas[0]),
                port::select("area","地区","browse",schemas[1]),port::select("state","进度","browse",schemas[2])]}),
            ),
            "get_manga_list" => self.list(request, args, false),
            "search" => self.list(request, args, true),
            "get_manga" => {
                let path = self.route(
                    args["mangaId"].as_str().ok_or("invalid_request")?,
                    self.work_prefix,
                )?;
                Ok(json!({"manga":self.details(&path)?}))
            }
            "get_pages" => self.pages(args),
            _ => Err("not_implemented"),
        }
    }
}

macro_rules! operations {
    ($name:literal) => {
        fn run(operation: &str, request: &[u8]) -> u32 {
            let result = SITE.dispatch(operation, request).map(|mut value| {
                if SITE.is_buding()
                    && matches!(operation, "get_manga_list" | "search" | "get_manga")
                {
                    port_support::mobile_cover_headers(&mut value, SITE.mirrors[0], request);
                }
                value
            });
            port_support::write_result(operation, result)
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
        fn run_get_comments(r: &[u8]) -> u32 {
            port_support::write_result(koma_source_sdk::comments::OPERATION, SITE.comments(r))
        }
        koma_source_sdk::koma_source_exports!($name, typed_chapters, write_chapter_failure);
        koma_source_sdk::koma_source_comments_export!(run_get_comments);
    };
}
pub(crate) use operations;
