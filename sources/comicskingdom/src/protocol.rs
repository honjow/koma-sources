const API: &str = "https://wp.comicskingdom.com/wp-json/wp/v2";
fn num(s: &str) -> Result<usize> {
    s.parse().ok().filter(|n| *n > 0).ok_or("invalid_request")
}
fn manga(id: &str) -> Result<Value> {
    num(id)?;
    jget(&format!("{API}/ck_feature/{id}"))
}
fn cover(m: &Value) -> String {
    m["yoast_head_json"]["og_image"][0]["url"]
        .as_str()
        .unwrap_or("")
        .into()
}
fn all() -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    for p in 1..20 {
        let v=jget(&format!("{API}/ck_feature?per_page=100&ck_language=english&page={p}&_fields=id,title,link,ck_favorite_count,modified,yoast_head_json"))?;
        let a = v.as_array().ok_or("parse_error")?;
        rows.extend(a.iter().cloned());
        if a.len() < 100 {
            return Ok(rows);
        }
    }
    Err("result_too_large")
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = pg(v)?;
    let args = &v["args"];
    let order = if search {
        args["filters"]["order"].as_str().unwrap_or("popular")
    } else {
        args["listingId"].as_str().unwrap_or("popular")
    };
    if !["popular", "latest", "title"].contains(&order) {
        return Err("invalid_request");
    };
    let q = if search {
        args["query"].as_str().unwrap_or("").to_lowercase()
    } else {
        String::new()
    };
    let mut rows = all()?;
    rows.retain(|m| {
        clean(m["title"]["rendered"].as_str().unwrap_or(""))
            .to_lowercase()
            .contains(&q)
    });
    rows.sort_by(|a, b| match order {
        "latest" => b["modified"].as_str().cmp(&a["modified"].as_str()),
        "title" => a["title"]["rendered"]
            .as_str()
            .cmp(&b["title"]["rendered"].as_str()),
        _ => {
            let val = |v: &Value| text(&v["ck_favorite_count"]).parse::<u64>().unwrap_or(0);
            val(b)
                .cmp(&val(a))
                .then_with(|| a["id"].as_u64().cmp(&b["id"].as_u64()))
        }
    });
    let more = n * 20 < rows.len();
    let xs = rows
        .into_iter()
        .skip((n - 1) * 20)
        .take(20)
        .map(|m| {
            item(
                &text(&m["id"]),
                &clean(m["title"]["rendered"].as_str().unwrap_or("")),
                &cover(&m),
            )
        })
        .collect();
    Ok(list_out(xs, more.then_some(n + 1)))
}
fn filters(r: &[u8]) -> Result<Value> {
    Ok(
        json!({"filters":[select(r,"order","filter.order",&[("popular","filter.popular"),("latest","filter.latest"),("title","filter.title")],"popular")?]}),
    )
}
fn details(v: &Value) -> Result<Value> {
    let id = args_id(v, "mangaId")?;
    let m = manga(id)?;
    let mut out = item(
        id,
        &clean(m["title"]["rendered"].as_str().unwrap_or("")),
        &cover(&m),
    );
    out["description"] = json!(clean(m["content"]["rendered"].as_str().unwrap_or("")));
    let author = m["meta"]["ck_byline_on_app"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_start_matches("By ");
    if !author.is_empty() {
        out["authors"] = json!([author]);
    }
    let tags = jget(&format!("{API}/ck_genre?post={id}&_fields=name"))?;
    out["tags"] = json!(tags
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .filter_map(|t| t["name"].as_str())
        .map(clean)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>());
    let until = m["meta"]["ck_active_until"].as_str().unwrap_or("");
    let latest = m["ck_latest_comic"]["date"].as_str().unwrap_or("");
    if m["meta"]["ck_is_vintage"].as_bool() == Some(true) {
        out["status"] = json!("completed");
    } else if !latest.is_empty() && until >= latest {
        out["status"] = json!("ongoing");
    }
    Ok(out)
}
fn slug(id: &str) -> Result<String> {
    let m = manga(id)?;
    let u = Url::parse(m["link"].as_str().ok_or("parse_error")?).map_err(|_| "parse_error")?;
    Ok(u.path().trim_matches('/').into())
}
fn api_ch(slug: &str, p: usize) -> String {
    format!("{API}/ck_comic?per_page=100&ck_feature={slug}&page={p}&order=desc&_fields=id,title,assets,date")
}
// Production 1.5.5 does not expose HTTP response headers. The official
// Next.js page carries the exact public archive count in its body; select only
// the ck_comic query for this feature, never unrelated posts/empty pageInfo.
fn archive_total(sl: &str) -> Result<usize> {
    let d = html(&format!("{BASE}/{sl}"))?;
    let data: Value = serde_json::from_str(&d.require("script#__NEXT_DATA__")?.text()?)
        .map_err(|_| "parse_error")?;
    let fallback = data["props"]["pageProps"]["fallback"]
        .as_object()
        .ok_or("parse_error")?;
    let feature = format!("ck_feature:\"{sl}\"");
    let mut totals = fallback
        .iter()
        .filter(|(key, _)| key.contains("postType:\"ck_comic\"") && key.contains(&feature))
        .filter_map(|(_, value)| value["pageInfo"]["totalItems"].as_u64());
    let total = totals
        .next()
        .filter(|n| *n <= 1_000_000)
        .ok_or("parse_error")?;
    if totals.any(|n| n != total) {
        return Err("parse_error");
    }
    Ok(total as usize)
}
fn chapters(v: &Value) -> Result<(Vec<C>, Option<usize>)> {
    let id = args_id(v, "mangaId")?;
    let sl = slug(id)?;
    let total = archive_total(&sl)?;
    let count = (total + 99) / 100;
    let mut xs = Vec::new();
    for p in (1..=count).rev() {
        xs.push(C {
            id: format!("{id}:{p}"),
            title: format!("{}–{}", (p - 1) * 100 + 1, (p * 100).min(total)),
            url: api_ch(&sl, p),
            count: Some(if p == count {
                total - (p - 1) * 100
            } else {
                100
            }),
            order: p - 1,
        })
    }
    Ok((xs, None))
}
fn reader(v: &Value) -> Result<Vec<String>> {
    let mid = args_id(v, "mangaId")?;
    let c = args_id(v, "chapterId")?;
    let (own, p) = c.split_once(':').ok_or("invalid_request")?;
    if own != mid {
        return Err("invalid_request");
    };
    let sl = slug(mid)?;
    let total = archive_total(&sl)?;
    let p = num(p)?;
    let start = (p - 1).checked_mul(100).ok_or("invalid_request")?;
    if start >= total {
        return Err("not_found");
    }
    let end = (start + 100).min(total);
    // Keep upstream's ascending 100-strip archive IDs and exact boundaries.
    // The public API rejects deep ascending pages with 429; request the same
    // range in descending date order, then reverse the selected strips.
    let offset = total - end;
    let mut rows = Vec::new();
    for page in offset / 100 + 1..=(offset + end - start - 1) / 100 + 1 {
        let a = jget(&api_ch(&sl, page))?;
        rows.extend(a.as_array().ok_or("parse_error")?.iter().cloned());
    }
    let mut selected: Vec<_> = rows
        .into_iter()
        .skip(offset % 100)
        .take(end - start)
        .collect();
    if selected.len() != end - start {
        return Err("parse_error");
    }
    selected.reverse();
    selected
        .iter()
        .map(|c| {
            c["assets"]["single"]["url"]
                .as_str()
                .map(String::from)
                .ok_or("chapter_locked")
        })
        .collect()
}
fn image_request(raw: &str) -> Result<String> {
    let u = Url::parse(raw).map_err(|_| "invalid_request")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || !matches!(
            u.host_str(),
            Some("wp.comicskingdom.com" | "comicskingdom.com")
        )
    {
        return Err("invalid_request");
    };
    Ok(raw.into())
}
