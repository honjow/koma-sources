const API: &str = "https://api.novelcool.com";
fn path(raw: &str, prefix: &str) -> Result<String> {
    let u = port_support::absolute(BASE, raw)?;
    if u.scheme() != "https"
        || u.host_str() != Some("www.novelcool.com")
        || !u.path().starts_with(prefix)
        || u.query().is_some()
    {
        return Err("invalid_request");
    };
    Ok(u.path().into())
}
fn safe(s: &str) -> bool {
    let s = s.to_lowercase();
    ![
        "hentai",
        "smut",
        "adult",
        "erotica",
        "mature",
        "ecchi",
        "pornographic",
        "fetish",
        "milf",
        "netorare",
    ]
    .iter()
    .any(|k| s.contains(k))
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = pg(v)?;
    let q = if search {
        v["args"]["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    // Search owns filter state; listing defaults must never override a board.
    // Conversely search must not inherit the board's stale listingId.
    let order = if search {
        v["args"]["filters"]["order"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("popular")
    } else {
        v["args"]["listingId"].as_str().unwrap_or("popular")
    };
    if !["popular", "latest"].contains(&order) {
        return Err("invalid_request");
    }
    let genre = port_support::filter(
        &v["args"],
        "genre",
        &[
            ("", ""),
            ("4", "Action"),
            ("11", "Comedy"),
            ("2", "Fantasy"),
            ("1", "Adventure"),
            ("9", "Romance"),
            ("3", "Sci-Fi"),
        ],
    )?;
    let status = port_support::filter(
        &v["args"],
        "status",
        &[("", ""), ("ongoing", "NO"), ("completed", "YES")],
    )?;
    let endpoint = if search && !q.trim().is_empty() {
        "/book/search/"
    } else if order == "latest" {
        "/elite/latest/"
    } else {
        "/elite/hot/"
    };
    let mut out = Vec::new();
    let mut next = None;
    // The App API has no documented genre/status parameters. Filter its real
    // category IDs/completed flags instead of inventing ignored API parameters.
    // Keep the upstream page cursor so sparse windows never drop later matches.
    let window = if genre.is_empty() && status.is_empty() {
        1
    } else {
        5
    };
    for page in n..n + window {
        let payload = json!({"appId":"202201290625004","lang":"en","keyword":q,"lc_type":"manga","page":page.to_string(),"page_size":"20","secret":"c73a8590641781f203660afca1d37ada"});
        let raw = http(&format!("{API}{endpoint}"), Some(payload))?;
        let data: Value = serde_json::from_str(&raw).map_err(|_| "parse_error")?;
        let xs = data["list"].as_array().ok_or("parse_error")?;
        for m in xs {
            if !matches!(text(&m["is_novel"]).as_str(), "" | "0" | "false")
                || !safe(&format!(
                    "{} {}",
                    text(&m["category_str"]),
                    text(&m["warning"])
                ))
            {
                continue;
            }
            if !genre.is_empty()
                && !m["category"]
                    .as_str()
                    .unwrap_or("")
                    .split(',')
                    .any(|id| id.trim() == genre)
            {
                continue;
            }
            if !status.is_empty()
                && m["completed"].as_str() != Some(if status == "completed" { "YES" } else { "NO" })
            {
                continue;
            }
            let id = path(m["url"].as_str().ok_or("parse_error")?, "/novel/")?;
            let mut entry = item(
                &id,
                m["name"].as_str().ok_or("parse_error")?,
                m["cover"].as_str().ok_or("parse_error")?,
            );
            entry["status"] = json!(match m["completed"].as_str() {
                Some("YES") => "completed",
                Some("NO") => "ongoing",
                _ => "unknown",
            });
            entry["tags"] = json!(m["category_list"]
                .as_array()
                .map(|a| a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>())
                .unwrap_or_default());
            out.push(entry);
        }
        next = (xs.len() == 20).then_some(page + 1);
        if next.is_none() || out.len() >= 20 {
            break;
        }
    }
    Ok(list_out(out, next))
}
fn filters(r: &[u8]) -> Result<Value> {
    let mut out = json!({"filters":[select(r,"order","filter.order",&[("popular","filter.popular"),("latest","filter.latest")],"popular")?, select(r,"genre","filter.genre",&[("","filter.any"),("4","filter.action"),("11","filter.comedy"),("2","filter.fantasy"),("1","filter.adventure"),("9","filter.romance"),("3","filter.scifi")],"")?, select(r,"status","filter.status",&[("","filter.any"),("ongoing","filter.ongoing"),("completed","filter.completed")],"")?]} );
    for filter in out["filters"].as_array_mut().ok_or("internal_error")? {
        filter["scope"] = json!("search");
    }
    // Opt in only on hosts which route empty-query search/filter state together.
    if koma_source_sdk::source_protocol::feature(r, b"source_filter_search_without_query_v1") {
        out["useSearchForFilters"] = json!(true);
    }
    Ok(out)
}
fn book(v: &Value) -> Result<(String, Node)> {
    let id = path(args_id(v, "mangaId")?, "/novel/")?;
    let d = html(&format!("{BASE}{id}"))?;
    if !safe(
        &d.one(".bk-cates")
            .map(|n| n.text())
            .transpose()?
            .unwrap_or_default(),
    ) {
        return Err("forbidden");
    };
    Ok((id, d))
}
fn details(v: &Value) -> Result<Value> {
    let (id, d) = book(v)?;
    let title = d.require("h1.bookinfo-title")?.text()?;
    let cover = abs(&d.require(".bookinfo-pic-img")?.attr("src")?)?;
    let mut x = item(&id, &title, &cover);
    x["description"] = json!(d
        .one(".bk-summary-txt")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default());
    let author = d
        .one(".bookinfo-author")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    let author = author.trim().trim_start_matches("Author:").trim();
    if !author.is_empty() {
        x["authors"] = json!([author]);
    }
    let status = d
        .one(".bk-status")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default()
        .to_lowercase();
    x["status"] = json!(if status.contains("ongoing") {
        "ongoing"
    } else if status.contains("completed") {
        "completed"
    } else {
        "unknown"
    });
    let mut seen = BTreeSet::new();
    let mut tags = Vec::new();
    // Desktop/mobile genre blocks repeat; status links are not genre tags.
    for node in d.all(".bk-cates .bk-cate-item a[title]")? {
        let tag = node.text()?.trim().to_string();
        if !tag.is_empty() && seen.insert(tag.clone()) {
            tags.push(tag);
        }
    }
    x["tags"] = json!(tags);
    Ok(x)
}
fn chapters(v: &Value) -> Result<(Vec<C>, Option<usize>)> {
    let (_, d) = book(v)?;
    let links = d.all(".chapter-item-list a")?;
    let total = links.len();
    let n = pg(v)?;
    let mut seen = BTreeSet::new();
    let mut xs = Vec::new();
    for (i, a) in links.into_iter().enumerate() {
        let id = path(&a.attr("href")?, "/chapter/")?;
        if !seen.insert(id.clone()) {
            continue;
        };
        xs.push(C {
            id: id.clone(),
            title: a.attr("title")?,
            url: format!("{BASE}{id}"),
            count: None,
            order: total - i - 1,
        });
    }
    let more = n * 100 < xs.len();
    Ok((
        xs.into_iter().skip((n - 1) * 100).take(100).collect(),
        more.then_some(n + 1),
    ))
}
fn reader(v: &Value) -> Result<Vec<String>> {
    let c = path(args_id(v, "chapterId")?, "/chapter/")?;
    let d = html(&format!("{BASE}{c}"))?;
    let mut seen = BTreeSet::new();
    let mut xs = Vec::new();
    for o in d.all(".mangaread-pagenav .sl-page option")? {
        let u = abs(&o.attr("value")?)?;
        if seen.insert(u.clone()) {
            xs.push(u)
        }
    }
    if xs.is_empty() && d.one(".mangaread-manga-pic").is_some() {
        xs.push(format!("{BASE}{c}"))
    }
    Ok(xs)
}
fn image_request(raw: &str) -> Result<String> {
    let u = Url::parse(raw).map_err(|_| "invalid_request")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
    {
        return Err("invalid_request");
    };
    let h = u.host_str().ok_or("invalid_request")?;
    if h == "www.novelcool.com" && u.path().starts_with("/chapter/") {
        let d = html(raw)?;
        let img = d.require(".mangaread-manga-pic")?.attr("src")?;
        return image_request(&img);
    }
    if !(h == "novelcool.com"
        || h.ends_with(".novelcool.com")
        || h.ends_with(".movietop.cc")
        || h.ends_with(".mangapicgallery.com"))
        || u.path().ends_with(".avif")
    {
        return Err("invalid_request");
    };
    Ok(raw.into())
}
