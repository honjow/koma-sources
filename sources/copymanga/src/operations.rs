use super::*;
fn page(v: &Value) -> R<u32> {
    match &v["args"]["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(s) => {
            let n = s.parse::<u32>().map_err(|_| E::from("invalid_request"))?;
            if n == 0 || n > 100000 || n.to_string() != *s {
                return Err("invalid_request".into());
            }
            Ok(n)
        }
        _ => Err("invalid_request".into()),
    }
}
fn browse(
    values: &Value,
    limit: u32,
    offset: u32,
    latest: bool,
) -> R<(String, Vec<(&'static str, String)>, bool)> {
    let key = |k: &str| {
        if latest {
            format!("latest_{}", k)
        } else {
            k.to_string()
        }
    };
    let mut rank = setting(values, &key("ranking"), "")?;
    let mut channel = setting(values, &key("ranking_channel"), "male")?;
    // Persisted latest preferences use the reference's allowedValue fallback.
    // Explicit search-filter input remains strict and is never silently changed.
    if latest {
        if !["", "day", "week", "month", "total"].contains(&rank) {
            rank = "";
        }
        if !["male", "female"].contains(&channel) {
            channel = "male";
        }
    }
    if !["", "day", "week", "month", "total"].contains(&rank)
        || !["male", "female"].contains(&channel)
    {
        return Err("invalid_request".into());
    }
    if !rank.is_empty() {
        return Ok((
            "ranks".to_string(),
            alloc::vec![
                ("date_type", rank.to_string()),
                ("audience_type", channel.to_string()),
                ("limit", limit.to_string()),
                ("offset", offset.to_string())
            ],
            false,
        ));
    }
    let mut region = setting(values, &key("region"), "")?.to_string();
    let mut status = setting(values, &key("status"), "")?.to_string();
    if latest {
        if let Some(top) = values["latest_top"].as_str() {
            let (a, b) = match top {
                "japan" => ("0", ""),
                "korea" => ("1", ""),
                "west" => ("2", ""),
                "finish" => ("", "1"),
                _ => ("", ""),
            };
            region = a.to_string();
            status = b.to_string();
        }
    }
    let mut theme = setting(values, &key("theme"), "")?;
    if latest {
        if !["", "0", "1", "2"].contains(&region.as_str()) {
            region.clear();
        }
        if !["", "0", "1", "2"].contains(&status.as_str()) {
            status.clear();
        }
        let choices = options()?;
        if !choices["LATEST_THEME_OPTIONS"]
            .as_array()
            .ok_or_else(|| E::from("internal_error"))?
            .iter()
            .any(|entry| entry["value"].as_str() == Some(theme))
        {
            theme = "";
        }
    }
    let ordering = if latest {
        if let Some(ui) = values["latest_sort_ui"].as_str() {
            match ui {
                "datetime_updated_ascending" => "datetime_updated",
                "popular_descending" => "-popular",
                "popular_ascending" => "popular",
                _ => "-datetime_updated",
            }
        } else {
            match (
                setting(values, "latest_sort", "datetime_updated")?,
                setting(values, "latest_sort_direction", "descending")?,
            ) {
                ("popular", "ascending") => "popular",
                ("popular", _) => "-popular",
                (_, "ascending") => "datetime_updated",
                _ => "-datetime_updated",
            }
        }
    } else {
        setting(values, "sort", "-datetime_updated")?
    };
    if ![
        "-datetime_updated",
        "datetime_updated",
        "-popular",
        "popular",
    ]
    .contains(&ordering)
        || !["", "0", "1", "2"].contains(&region.as_str())
        || !["", "0", "1", "2"].contains(&status.as_str())
    {
        return Err("invalid_request".into());
    }
    let top = if region.is_empty() && status == "1" {
        Some("finish")
    } else {
        None
    };
    let html = top.is_none() && (!region.is_empty() || !status.is_empty() || !theme.is_empty());
    let mut pairs = alloc::vec![
        ("ordering", ordering.to_string()),
        ("_update", "true".to_string()),
        ("limit", limit.to_string()),
        ("offset", offset.to_string())
    ];
    if html {
        if !region.is_empty() {
            pairs.push(("region", region))
        }
        if !status.is_empty() {
            pairs.push(("status", status))
        }
    } else if let Some(top) = top {
        pairs.push(("top", top.to_string()))
    }
    if !theme.is_empty() {
        pairs.push(("theme", theme.to_string()))
    }
    Ok(("comics".to_string(), pairs, html))
}
fn html_page(cx: &Context, pairs: &[(&str, String)]) -> R<ApiData> {
    let query = pairs
        .iter()
        .filter(|(k, _)| *k != "_update" && *k != "top")
        .map(|(k, v)| format!("{}={}", encoded(k), encoded(v)))
        .collect::<Vec<_>>()
        .join("&");
    let (_, b) = cx.http(&format!("{}/comics?{}", cx.web, query), false)?;
    let d = Node::parse(&b)?;
    let box_ = d.required(".exemptComicList .exemptComic-box")?;
    let total = box_
        .attr("total")?
        .ok_or_else(|| E::from("parse_error"))?
        .parse::<i64>()
        .map_err(|_| E::from("parse_error"))?;
    let raw = box_.attr("list")?.ok_or_else(|| E::from("parse_error"))?;
    if raw.trim().is_empty() {
        return Err("parse_error".into());
    }
    let values = literal::literal(&raw)?;
    let mut rows = Vec::new();
    for v in values.as_array().ok_or_else(|| E::from("parse_error"))? {
        let Some(path) = v["path_word"].as_str().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let Some(name) = v["name"].as_str().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let mut authors = Vec::new();
        if let Some(a) = v["author"].as_array() {
            for author in a {
                if let Some(name) = author["name"].as_str() {
                    authors.push(
                        json!({"name":name,"path_word":author["path_word"].as_str().unwrap_or("")}),
                    )
                }
            }
        }
        let mut row = json!({"path_word":path,"name":name,"author":authors,"cover":v["cover"].as_str().unwrap_or("")});
        let status = v["status"]
            .as_i64()
            .or_else(|| v["status"].as_f64().map(|n| n as i64))
            .or_else(|| v["status"].as_str().and_then(|s| s.parse().ok()));
        if let Some(status) = status {
            row["status"] = json!({"value":status,"display":match status{0=>"連載中",1=>"已完結",2=>"短篇",_=>""}})
        }
        rows.push(row)
    }
    Ok(ApiData {
        json: json!({"list":rows,"total":total,"offset":pairs.iter().find(|(k,_)|*k=="offset").and_then(|(_,v)|v.parse::<u32>().ok()).unwrap_or(0)}),
        groups: None,
    })
}
fn list(cx: &Context, v: &Value, search: bool) -> R<Value> {
    let p = page(v)?;
    let offset = (p - 1)
        .checked_mul(21)
        .ok_or_else(|| E::from("invalid_request"))?;
    let query = if search {
        v["args"]["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    if Url::parse(query).is_ok_and(|u| matches!(u.scheme(), "http" | "https")) {
        let slug = by_url(query)?;
        let result = cx.api(&format!("comic2/{}", slug), &[])?;
        if encoded(field(&result["comic"], "path_word")?) != slug {
            return Err("parse_error".into());
        }
        return Ok(
            json!({"items":[comic(cx,&result["comic"])?],"page":{"hasMore":false,"nextCursor":null}}),
        );
    }
    let (path, pairs, html) = if search && !query.trim().is_empty() {
        (
            "search/comic".to_string(),
            alloc::vec![
                ("q", query.to_string()),
                ("q_type", "".to_string()),
                ("_update", "true".to_string()),
                ("limit", "21".to_string()),
                ("offset", offset.to_string())
            ],
            false,
        )
    } else if !search && v["args"]["listingId"].as_str() == Some("popular") {
        (
            "recs".to_string(),
            alloc::vec![
                ("pos", "3200102".to_string()),
                ("limit", "21".to_string()),
                ("offset", offset.to_string())
            ],
            false,
        )
    } else {
        if !search
            && v["args"]["listingId"]
                .as_str()
                .is_some_and(|s| s != "latest")
        {
            return Err("invalid_request".into());
        }
        browse(
            if search {
                &v["args"]["filters"]
            } else {
                &v["settings"]
            },
            21,
            offset,
            !search,
        )?
    };
    let result = if html {
        html_page(cx, &pairs)?
    } else {
        cx.api(&path, &pairs)?
    };
    let mut items = Vec::new();
    for item in array(&result, "list")? {
        items.push(comic(
            cx,
            if path == "recs" || path == "ranks" {
                &item["comic"]
            } else {
                item
            },
        )?)
    }
    let actual = result["offset"]
        .as_u64()
        .ok_or_else(|| E::from("parse_error"))?;
    let total = result["total"]
        .as_u64()
        .ok_or_else(|| E::from("parse_error"))?;
    if actual != offset as u64 {
        return Err("parse_error".into());
    }
    let more = items.len() >= 21 && actual + (items.len() as u64) < total;
    Ok(
        json!({"items":items,"page":{"hasMore":more,"nextCursor":if more{Some((p+1).to_string())}else{None}}}),
    )
}
fn chapters(cx: &Context, v: &Value) -> R<Value> {
    let slug = manga_id(v)?;
    let result = cx.api(&format!("comic2/{}", slug), &[])?;
    let root = field(&result["comic"], "path_word")?;
    if encoded(root) != slug {
        return Err("parse_error".into());
    }
    let groups = result
        .groups
        .as_ref()
        .ok_or_else(|| E::from("parse_error"))?;
    let mut order = alloc::vec![("default".to_string(), String::new())];
    for g in groups {
        let path = field(g, "path_word")?;
        if path != "default" {
            order.push((path.to_string(), field(g, "name")?.to_string()))
        }
    }
    let mut out = Vec::new();
    for (group, name) in order {
        if group.trim().is_empty() {
            continue;
        }
        let mut offset = 0;
        let mut entries = Vec::new();
        loop {
            let r = cx.api(
                &format!("comic/{}/group/{}/chapters", slug, encoded(&group)),
                &[
                    ("limit", "500".to_string()),
                    ("offset", offset.to_string()),
                    ("_update", "true".to_string()),
                ],
            )?;
            let list = array(&r, "list")?;
            if list.is_empty() {
                break;
            }
            for c in list {
                let uuid = encoded(field(c, "uuid")?);
                path_id(&uuid).map_err(|_| E::from("parse_error"))?;
                let raw_name = cx.display(field(c, "name")?);
                let title = if name.trim().is_empty() {
                    raw_name
                } else {
                    format!("{}：{}", cx.display(&name), raw_name)
                };
                let mut row = json!({"id":format!("copymanga-read:{}:{}",slug,uuid),"mangaId":format!("copymanga:{}",slug),"title":title,"url":format!("{}/comic/{}/chapter/{}",cx.web,slug,uuid)});
                if let Some(t) = date(field(c, "datetime_created")?) {
                    row["dateUpload"] = json!(t)
                }
                entries.push(row)
            }
            offset += list.len();
            let total = r["total"].as_u64().ok_or_else(|| E::from("parse_error"))?;
            if list.len() < 500 || offset as u64 >= total {
                break;
            }
        }
        entries.reverse();
        out.extend(entries);
    }
    for (i, c) in out.iter_mut().enumerate() {
        c["index"] = json!(i)
    }
    Ok(json!({"chapters":out,"page":{"hasMore":false,"nextCursor":null}}))
}
fn pages(cx: &Context, v: &Value) -> R<Value> {
    let (slug, cid) = chapter_id(v)?;
    let r = cx.api(
        &format!("comic/{}/chapter2/{}", slug, cid),
        &[("platform", "1".to_string())],
    )?;
    let c = &r["chapter"];
    let words = array(c, "words")?;
    let contents = array(c, "contents")?;
    // The upstream zip/mapNotNull can silently truncate/drop bad provider rows.
    // Koma's adopted no-page-loss contract requires a complete valid pair table.
    if words.len() != contents.len() {
        return Err("parse_error".into());
    }
    let mut ordered = Vec::new();
    for (i, (word, content)) in words.iter().zip(contents).enumerate() {
        let n = word
            .as_i64()
            .filter(|n| *n >= i32::MIN as i64 && *n <= i32::MAX as i64)
            .ok_or_else(|| E::from("parse_error"))?;
        ordered.push((n, i, content))
    }
    ordered.sort_by_key(|(n, _, _)| *n);
    let re = Regex::new(r"(?i)\d+(x\.(?:jpe?g|png|webp)(?:\?|$))")
        .map_err(|_| E::from("internal_error"))?;
    let mut out = Vec::new();
    for (index, (_, _, content)) in ordered.iter().enumerate() {
        let raw = field(content, "url")?;
        if raw.trim().is_empty() {
            return Err("parse_error".into());
        }
        let url = re
            .replace_all(raw, |c: &regex::Captures| format!("1500{}", &c[1]))
            .into_owned();
        let parsed = Url::parse(&url).map_err(|_| E::from("parse_error"))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("parse_error".into());
        }
        out.push(json!({"id":format!("copymanga-page:{}:{}:{}",slug,cid,index),"index":index,"image":{"kind":"url","url":url}}))
    }
    if out.is_empty() {
        return Err("chapter_unavailable".into());
    }
    Ok(json!({"pages":out}))
}
fn image_request(cx: &Context, v: &Value) -> R<Value> {
    let raw = v["args"]["pageId"]
        .as_str()
        .and_then(|s| s.strip_prefix("copymanga-page:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let p = raw.split(':').collect::<Vec<_>>();
    if p.len() != 3 {
        return Err("invalid_request".into());
    }
    let slug = path_id(p[0])?;
    let cid = path_id(p[1])?;
    let n = p[2]
        .parse::<u64>()
        .map_err(|_| E::from("invalid_request"))?;
    if n.to_string() != p[2] {
        return Err("invalid_request".into());
    }
    if let Some(m) = v["args"].get("mangaId") {
        if m.as_str() != Some(&format!("copymanga:{}", slug)) {
            return Err("invalid_request".into());
        }
    }
    if let Some(c) = v["args"].get("chapterId") {
        if c.as_str() != Some(&format!("copymanga-read:{}:{}", slug, cid)) {
            return Err("invalid_request".into());
        }
    }
    if let Some(m) = v["args"].get("mangaUrl") {
        if by_url(m.as_str().ok_or_else(|| E::from("invalid_request"))?)? != slug {
            return Err("invalid_request".into());
        }
    }
    let raw = v["args"]["url"]
        .as_str()
        .ok_or_else(|| E::from("invalid_request"))?;
    let u = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    // Upstream image traffic uses the ordinary client and source headers; API-only
    // cookies/pool/protocol semantics must not be imposed on an unrelated CDN.
    Ok(
        json!({"url":raw,"headers":{"User-Agent":cx.ua,"Referer":format!("{}/",cx.web),"Accept":"application/json","Version":"2025.11.21","Webp":"1","platform":"1"}}),
    )
}
fn options() -> R<Value> {
    serde_json::from_str(include_str!("options.json")).map_err(|_| "internal_error".into())
}
fn choice(id: &str, name: &str, values: &Value, default: &str) -> Value {
    json!({"id":id,"name":name,"kind":"select","default":default,"options":values})
}
fn settings(cx: &Context) -> R<Value> {
    let o = options()?;
    let domains = DOMAINS
        .iter()
        .map(|(api, _)| json!({"value":api,"label":api.trim_start_matches("https://")}))
        .collect::<Vec<_>>();
    let mut theme = o["LATEST_THEME_OPTIONS"].clone();
    for row in theme
        .as_array_mut()
        .ok_or_else(|| E::from("internal_error"))?
    {
        row["label"] = json!(cx.display(field(row, "label")?))
    }
    let mut rows = alloc::vec![
        choice("domain", "域名", &json!(domains), DOMAINS[0].0),
        choice(
            "display_language",
            "繁簡設定",
            &json!([{"value":"traditional","label":"繁體"},{"value":"simplified","label":"簡體"}]),
            "traditional"
        ),
        json!({"id":"user_agent","name":"User-Agent","kind":"text","default":DEFAULT_UA,
            "validation":{"kind":"non_blank","message":"User-Agent 不能為空白。"}}),
        choice("latest_ranking", "最新模式", &o["LATEST_MODE_OPTIONS"], ""),
        choice(
            "latest_ranking_channel",
            "榜單頻道",
            &o["RANKING_CHANNELS"],
            "male"
        ),
        choice(
            "latest_sort_ui",
            "排序",
            &o["LATEST_SORT_UI"],
            "datetime_updated_descending"
        ),
        choice("latest_region", "地區", &o["REGION_OPTIONS"], ""),
        choice("latest_status", "狀態", &o["STATUS_OPTIONS"], ""),
        choice("latest_theme", "題材", &theme, "")
    ];
    for row in &mut rows {
        match row["id"].as_str() {
            Some("latest_ranking_channel") => {
                row["visibleWhen"] = json!({"settingId":"latest_ranking","values":["day","week","month","total"]});
            }
            Some("latest_sort_ui" | "latest_region" | "latest_status" | "latest_theme") => {
                row["visibleWhen"] = json!({"settingId":"latest_ranking","values":[""]});
            }
            _ => {}
        }
    }
    Ok(json!({"settings":rows}))
}
fn filters(cx: &Context, r: &[u8], v: &Value) -> R<Value> {
    let o = options()?;
    let mut theme = alloc::vec![json!({"value":"","label":"全部"})];
    if protocol::feature(r, b"source_filter_fetching_v1") && v["args"]["fetchFilterData"] == true {
        let data = cx.api("theme/comic/count", &[("limit", "500".to_string())])?;
        let mut seen = alloc::collections::BTreeSet::new();
        for item in array(&data, "list")? {
            let path = field(item, "path_word")?;
            if seen.insert(path.to_string()) {
                theme.push(json!({"value":path,"label":cx.display(field(item,"name")?)}))
            }
        }
    }
    let mut rows = Vec::new();
    if protocol::feature(r, b"source_filter_scope_header_v1") {
        rows.push(json!({"type":"header","name":"排行榜啟用時，榜單頻道生效，並忽略下方一般篩選"}))
    }
    for (id, name, options, default) in [
        ("ranking", "排行榜", o["RANKING_PERIODS"].clone(), ""),
        (
            "ranking_channel",
            "榜單頻道",
            o["RANKING_CHANNELS"].clone(),
            "male",
        ),
        (
            "sort",
            "排序",
            o["FILTER_SORT"].clone(),
            "-datetime_updated",
        ),
        ("region", "地區", o["REGION_OPTIONS"].clone(), ""),
        ("status", "狀態", o["STATUS_OPTIONS"].clone(), ""),
        ("theme", "題材", json!(theme), ""),
    ] {
        rows.push(json!({"id":id,"name":name,"type":"select","options":options,"default":default}))
    }
    Ok(json!({"supportsFilterFetching":true,"filters":rows}))
}
pub fn dispatch(op: &str, r: &[u8], v: &Value) -> R<Value> {
    let cx = Context::new(
        v,
        op == "get_settings" && protocol::feature(r, b"source_settings_context_v1"),
        protocol::feature(r, b"source_http_copymanga_api_v1"),
    )?;
    match op {
        "get_listings" => Ok(
            json!({"listings":[{"id":"latest","name":"最新","supportsPaging":true},{"id":"popular","name":"热门","supportsPaging":true}]}),
        ),
        "get_settings" => settings(&cx),
        "get_filters" => filters(&cx, r, v),
        "search" => list(&cx, v, true),
        "get_manga_list" => list(&cx, v, false),
        "get_manga" => {
            let slug = manga_id(v)?;
            let data = cx.api(&format!("comic2/{}", slug), &[])?;
            if encoded(field(&data["comic"], "path_word")?) != slug {
                return Err("parse_error".into());
            }
            Ok(json!({"manga":comic(&cx,&data["comic"])?}))
        }
        "get_chapters" => chapters(&cx, v),
        "get_pages" => pages(&cx, v),
        "get_image_request" => image_request(&cx, v),
        _ => Err("not_implemented".into()),
    }
}
