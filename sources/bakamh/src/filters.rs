// SPDX-License-Identifier: Apache-2.0
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::source_protocol;
use serde_json::{json, Value};
use url::form_urlencoded::Serializer;

use crate::{genre_links, get, port, Result, BASE};

pub(crate) type Form = Serializer<'static, String>;

const STATUS: &[(&str, &str)] = &[
    ("end", "已完结"),
    ("on-going", "连载中"),
    ("canceled", "已取消"),
    ("on-hold", "暂停"),
];
const SORT: &[(&str, &str)] = &[
    ("", "相关度"),
    ("latest", "最近更新"),
    ("alphabet", "标题"),
    ("rating", "评分"),
    ("trending", "近期热门"),
    ("views", "观看次数"),
    ("new-manga", "新作"),
];
const ADULT: &[(&str, &str)] = &[("", "全部"), ("0", "隐藏成人内容"), ("1", "仅成人内容")];
const GENRE_CONDITION: &[(&str, &str)] = &[("", "任一"), ("1", "全部")];
const TEXT: &[(&str, &str, &str)] = &[
    ("author", "作者", "wp-manga-author"),
    ("artist", "画师", "wp-manga-artist"),
    ("year", "年份", "wp-manga-release"),
];

pub(crate) fn schema(args: &Value, request: &[u8]) -> Result<Value> {
    if !source_protocol::feature(request, b"source_filter_scope_header_v1")
        || !source_protocol::feature(request, b"source_filter_search_without_query_v1")
    {
        return Ok(json!({"filters":[]}));
    }
    let mut filters: Vec<Value> = TEXT.iter().map(|(id, label, _)| {
        json!({"type":"text","scope":"search","id":id,"name":label,"default":""})
    }).collect();
    let status = STATUS
        .iter()
        .map(|(value, label)| json!({"value":value,"label":label}))
        .collect::<Vec<_>>();
    filters.extend([
        json!({"type":"multiselect","scope":"search","id":"status","name":"状态","default":[],"options":status}),
        port::select("sort", "排序", "search", SORT),
        port::select("adult", "成人内容", "search", ADULT),
    ]);
    let dynamic = source_protocol::feature(request, b"source_filter_fetching_v1");
    if dynamic {
        let mut genres =
            json!({"type":"multiselect","scope":"search","id":"genres","name":"题材","default":[]});
        if args["fetchFilterData"] == true {
            let url = port::absolute(BASE, "/manga/")?;
            let doc = get(&url)?;
            let options = genre_links(&doc, "div.genres a[href*='/manga-genre/']", &url)?
                .into_iter()
                .map(|(slug, name)| json!({"value":slug,"label":name}))
                .collect::<Vec<_>>();
            if options.is_empty() {
                return Err("parse_error");
            }
            genres["options"] = json!(options);
        }
        filters.extend([
            json!({"type":"header","scope":"search","name":"题材"}),
            port::select("genreCondition", "题材匹配", "search", GENRE_CONDITION),
            genres,
        ]);
    }
    Ok(json!({"filters":filters,"useSearchForFilters":true,"supportsFilterFetching":dynamic}))
}

pub(crate) fn sort(form: &mut Form, key: &str) {
    form.append_pair("vars[orderby]", "meta_value_num")
        .append_pair("vars[meta_key]", key)
        .append_pair("vars[order]", "DESC");
}

pub(crate) fn search(form: &mut Form, args: &Value) -> Result<()> {
    if let Some(query) = args["query"]
        .as_str()
        .filter(|query| !query.trim().is_empty())
    {
        form.append_pair("vars[s]", query);
    }
    let mut tax = 0;
    for (id, _, taxonomy) in TEXT {
        let Some(value) = args["filters"].get(id) else {
            continue;
        };
        let term = value.as_str().ok_or("invalid_request")?;
        if !term.trim().is_empty() {
            form.append_pair(&format!("vars[tax_query][{tax}][taxonomy]"), taxonomy)
                .append_pair(&format!("vars[tax_query][{tax}][field]"), "name")
                .append_pair(&format!("vars[tax_query][{tax}][terms]"), term);
            tax += 1;
        }
    }
    let mut meta = 1;
    if let Some(status) = args["filters"].get("status") {
        let array = status.as_array().ok_or("invalid_request")?;
        if !array.is_empty() {
            form.append_pair("vars[meta_query][1][key]", "_wp_manga_status")
                .append_pair("vars[meta_query][1][compare]", "IN");
            for (index, value) in array.iter().enumerate() {
                let value = value
                    .as_str()
                    .filter(|value| STATUS.iter().any(|(id, _)| id == value))
                    .ok_or("invalid_request")?;
                form.append_pair(&format!("vars[meta_query][1][value][{index}]"), value);
            }
            meta += 1;
        }
    }
    match port::filter(args, "sort", SORT)? {
        "latest" => sort(form, "_latest_update"),
        "trending" => sort(form, "_wp_manga_week_views_value"),
        "views" => sort(form, "_wp_manga_views"),
        "alphabet" => {
            form.append_pair("vars[orderby]", "post_title")
                .append_pair("vars[order]", "ASC");
        }
        "new-manga" => {
            form.append_pair("vars[orderby]", "date")
                .append_pair("vars[order]", "DESC");
        }
        "rating" => {
            form.append_pair(
                "vars[meta_query][query_average_reviews][key]",
                "_manga_avarage_reviews",
            )
            .append_pair("vars[meta_query][query_average_reviews][compare]", "EXISTS")
            .append_pair(
                "vars[meta_query][query_total_reviews][key]",
                "_manga_total_votes",
            )
            .append_pair("vars[meta_query][query_total_reviews][compare]", "EXISTS")
            .append_pair("vars[orderby][query_average_reviews]", "DESC")
            .append_pair("vars[orderby][query_total_reviews]", "DESC");
        }
        _ => {}
    }
    let adult = port::filter(args, "adult", ADULT)?;
    if !adult.is_empty() {
        form.append_pair(
            &format!("vars[meta_query][{meta}][key]"),
            "manga_adult_content",
        )
        .append_pair(
            &format!("vars[meta_query][{meta}][compare]"),
            if adult == "0" { "not exists" } else { "exists" },
        );
    }
    if let Some(value) = args["filters"].get("genres") {
        let genres = value
            .as_array()
            .ok_or("invalid_request")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|slug| crate::path_segment(slug))
                    .ok_or("invalid_request")
            })
            .collect::<Result<Vec<_>>>()?;
        if !genres.is_empty() {
            if port::filter(args, "genreCondition", GENRE_CONDITION)? == "1" {
                form.append_pair(&format!("vars[tax_query][{tax}][operation]"), "AND");
            }
            add_genres(form, tax, genres.into_iter());
        }
    }
    Ok(())
}

pub(crate) fn add_genres<'a>(form: &mut Form, index: usize, genres: impl Iterator<Item = &'a str>) {
    form.append_pair(
        &format!("vars[tax_query][{index}][taxonomy]"),
        "wp-manga-genre",
    )
    .append_pair(&format!("vars[tax_query][{index}][field]"), "slug");
    for (position, slug) in genres.enumerate() {
        form.append_pair(
            &format!("vars[tax_query][{index}][terms][{position}]"),
            slug,
        );
    }
}

pub(crate) fn archive_form(page: usize) -> Form {
    let mut form = Serializer::new(String::new());
    form.append_pair("action", "madara_load_more")
        .append_pair("page", &page.to_string())
        .append_pair("template", "madara-core/content/content-archive")
        .append_pair("vars[template]", "archive")
        .append_pair("vars[posts_per_page]", &crate::PAGE_SIZE.to_string())
        .append_pair("vars[post_type]", "wp-manga")
        .append_pair("vars[post_status]", "publish")
        .append_pair("vars[manga_archives_item_layout]", "big_thumbnail");
    form
}
