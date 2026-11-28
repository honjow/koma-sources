fn all() -> Result<Vec<Value>> {
    let x = jget(&format!("{BASE}/api/series/list"))?;
    Ok(x["data"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .filter(|m| m["type"].as_str() == Some("comic"))
        .cloned()
        .collect())
}
fn getwork(id: &str) -> Result<Value> {
    all()?
        .into_iter()
        .find(|m| text(&m["id"]) == id)
        .ok_or("not_found")
}
fn slug(id: &str) -> Result<String> {
    Ok(getwork(id)?["slug"].as_str().ok_or("parse_error")?.into())
}
fn as_item(m: &Value) -> Result<Value> {
    Ok(item(
        &text(&m["id"]),
        m["name"].as_str().ok_or("parse_error")?,
        m["cover"].as_str().ok_or("parse_error")?,
    ))
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = pg(v)?;
    let q = if search {
        v["args"]["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    let status = v["args"]["filters"]["status"].as_str().unwrap_or("");
    let genres = v["args"]["filters"]["genres"].as_str().unwrap_or("");
    if !status
        .bytes()
        .chain(genres.bytes())
        .all(|c| c.is_ascii_digit() || c == b',')
    {
        return Err("invalid_request");
    }
    // Filtered keyword searches must use the actual filtered catalog. Scan a
    // bounded window and return the next upstream page, including on empty hits.
    if search && !q.trim().is_empty() && (!status.is_empty() || !genres.is_empty()) {
        let mut items = Vec::new();
        let mut next = None;
        for page in n..n + 5 {
            let j = jget(&format!(
                "{BASE}/api/series?page={page}&type=comic&status={status}&genres={genres}"
            ))?;
            let data = &j["data"]["series"];
            for m in data["data"].as_array().ok_or("parse_error")? {
                if m["type"].as_str().is_none_or(|t| t == "comic")
                    && m["name"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q.trim().to_lowercase())
                {
                    items.push(as_item(m)?);
                }
            }
            next = (page < data["last_page"].as_u64().ok_or("parse_error")? as usize)
                .then_some(page + 1);
            if next.is_none() || items.len() >= 20 {
                break;
            }
        }
        return Ok(list_out(items, next));
    }
    if !q.trim().is_empty() {
        let xs: Vec<Value> = all()?
            .into_iter()
            .filter(|m| {
                m["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q.to_lowercase())
            })
            .collect();
        let more = n * 20 < xs.len();
        return Ok(list_out(
            xs.iter()
                .skip((n - 1) * 20)
                .take(20)
                .map(as_item)
                .collect::<Result<_>>()?,
            more.then_some(n + 1),
        ));
    }
    let endpoint = if search {
        format!(
            "/api/series?page={n}&type=comic&status={}&genres={}",
            v["args"]["filters"]["status"].as_str().unwrap_or(""),
            v["args"]["filters"]["genres"].as_str().unwrap_or("")
        )
    } else if v["args"]["listingId"].as_str() == Some("latest") {
        format!("/api/new-chapters?page={n}")
    } else {
        format!("/api/rankings?page={n}&period=total_ranking")
    };
    let x = jget(&format!("{BASE}{endpoint}"))?;
    let x = if search { &x["data"]["series"] } else { &x };
    let a = x["data"].as_array().ok_or("parse_error")?;
    let xs = a
        .iter()
        .filter(|m| m["type"].as_str().is_none_or(|t| t == "comic"))
        .map(as_item)
        .collect::<Result<Vec<_>>>()?;
    Ok(list_out(
        xs,
        (n < x["last_page"].as_u64().unwrap_or(1) as usize).then_some(n + 1),
    ))
}
fn filters(r: &[u8]) -> Result<Value> {
    let j = jget(&format!("{BASE}/api/genres-statuses"))?;
    let mut xs = Vec::new();
    for (id, key) in [("status", "statuses"), ("genres", "genres")] {
        let mut options = alloc::vec![json!({"value":"","label":label(r,"filter.any")?})];
        let a = j[key]
            .as_array()
            .or_else(|| j["data"][key].as_array())
            .ok_or("parse_error")?;
        for t in a {
            let value = text(&t["id"]);
            let key = format!("filter.{id}.{value}");
            let display = if serde_json::from_slice::<Value>(METADATA)
                .map_err(|_| "internal_error")?["localization"]["messages"]
                .get(&key)
                .is_some()
            {
                label(r, &key)?
            } else {
                t["name"].as_str().unwrap_or("").into()
            };
            options.push(json!({"value":value,"label":display}))
        }
        xs.push(json!({"id":id,"scope":"search","name":label(r,if id=="status"{"filter.status"}else{"filter.genre"})?,"type":"select","default":"","options":options}))
    }
    let mut out = json!({"filters":xs});
    // Keep browse/search filter state unified only after host capability negotiation.
    if koma_source_sdk::source_protocol::feature(r, b"source_filter_search_without_query_v1") {
        out["useSearchForFilters"] = json!(true);
    }
    Ok(out)
}
fn details(v: &Value) -> Result<Value> {
    let id = args_id(v, "mangaId")?;
    let sl = slug(id)?;
    let j = jget(&format!("{BASE}/api/series/{sl}?type=comic"))?;
    let m = &j["data"];
    let mut x = as_item(m)?;
    x["description"] = json!(clean(m["summary"].as_str().unwrap_or("")));
    let status = m["status"]["name"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    x["status"] = json!(if status == "activo" {
        "ongoing"
    } else if status == "finalizado" {
        "completed"
    } else if status.starts_with("pausado") {
        "hiatus"
    } else if status.starts_with("cancelado") || status.starts_with("abandonado") {
        "cancelled"
    } else {
        "unknown"
    });
    x["tags"] = json!(m["genres"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .filter_map(|g| g["name"].as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>());
    if let Some(author) = m["team"]["name"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        x["authors"] = json!([author]);
    }
    Ok(x)
}
fn chapters(v: &Value) -> Result<(Vec<C>, Option<usize>)> {
    let mid = args_id(v, "mangaId")?;
    let sl = slug(mid)?;
    let n = pg(v)?;
    let j = jget(&format!(
        "{BASE}/api/series/{sl}/chapters?page={n}&direction=desc&type=comic"
    ))?;
    let a = j["data"].as_array().ok_or("parse_error")?;
    let total = j["meta"]["total"].as_u64().unwrap_or(a.len() as u64) as usize;
    let per = j["meta"]["per_page"].as_u64().unwrap_or(a.len() as u64) as usize;
    let xs = a
        .iter()
        .enumerate()
        .map(|(i, c)| C {
            id: format!("{mid}/{}", text(&c["id"])),
            title: c["name"].as_str().unwrap_or("").into(),
            url: format!("{BASE}/capitulo/{}/comic-{sl}", text(&c["id"])),
            count: None,
            order: total.saturating_sub((n - 1) * per + i + 1),
        })
        .collect();
    Ok((xs, (n * per < total).then_some(n + 1)))
}
fn reader(v: &Value) -> Result<Vec<String>> {
    let mid = args_id(v, "mangaId")?;
    let c = args_id(v, "chapterId")?;
    let (own, id) = c.split_once('/').ok_or("invalid_request")?;
    if own != mid || !id.bytes().all(|c| c.is_ascii_digit()) {
        return Err("invalid_request");
    };
    let sl = slug(mid)?;
    let d = html(&format!("{BASE}/capitulo/{id}/comic-{sl}"))?;
    if d.all("main.strip div.panel > img")?
        .iter()
        .any(|n| n.attr("src").is_ok_and(|s| s.starts_with("/cp/")))
    {
        return Err("chapter_unavailable");
    }
    d.all("main.strip div.panel > img")?
        .iter()
        .map(|n| n.attr("src").and_then(|s| abs(&s)))
        .collect()
}
fn image_request(raw: &str) -> Result<String> {
    let u = Url::parse(raw).map_err(|_| "invalid_request")?;
    let h = u.host_str().ok_or("invalid_request")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.port().is_some()
        || !(h == "olympusxyz.com"
            || h.ends_with(".imagesolymp.xyz")
            || h.ends_with(".olympusbiblioteca.com"))
        || u.path().ends_with(".avif")
    {
        return Err("invalid_request");
    };
    Ok(raw.into())
}
