use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::host;
use serde_json::{json, Value};
use url::Url;

use crate::{port, Result, OUTPUT, SITE};

const MAX_METADATA: usize = 2 * 1024 * 1024;
const MAX_JSIA: usize = 2 * 1024 * 1024;
const MAX_SCANS: usize = 24;

pub(super) fn json_get(url: &str) -> Result<Value> {
    let body = port::request(
        url,
        "GET",
        json!({"Accept":"application/json,application/x-javascript"}),
        None,
    )?;
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub(super) fn strings(value: &Value) -> Vec<&str> {
    match value {
        Value::String(s) => Vec::from([s.as_str()]),
        Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

fn unmarked(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "" | "false" | "0" | "no"
        ),
        Value::Number(n) => n.as_u64() == Some(0),
        Value::Array(values) => values.iter().all(unmarked),
        _ => false,
    }
}

pub(super) fn restricted(value: &Value) -> bool {
    [
        "access-restricted-item",
        "is_dark",
        "dark",
        "private",
        "login-required",
        "lending-required",
        "printdisabled",
    ]
    .iter()
    .any(|key| !unmarked(&value[*key]))
        || strings(&value["collection"]).iter().any(|s| {
            matches!(
                s.trim().to_ascii_lowercase().as_str(),
                "inlibrary" | "printdisabled" | "lendinglibrary"
            )
        })
}

fn archive_node(value: &str) -> bool {
    let Some((node, domain)) = value.split_once('.') else {
        return false;
    };
    let Some(number) = node.strip_prefix("ia") else {
        return false;
    };
    !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && matches!(domain, "archive.org" | "us.archive.org")
}

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 320
        && value != "."
        && value != ".."
        && !value.contains(['/', '\\'])
        && !value.chars().any(char::is_control)
}

pub(super) fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

pub(super) struct Scan {
    pub prefix: String,
    archive: String,
    directory: String,
    format: &'static str,
}

pub(super) struct Book {
    pub value: Value,
    id: String,
    node: String,
    path: String,
    nodes: BTreeSet<String>,
    pub scans: Vec<Scan>,
}

fn scan_from_file(file: &Value) -> Option<Scan> {
    let format = match file["format"].as_str()? {
        "Single Page Processed JP2 ZIP" => "jp2",
        "Single Page Processed JPEG ZIP" => "jpg",
        _ => return None,
    };
    let name = file["name"].as_str().filter(|s| safe_component(s))?;
    let suffix = if format == "jp2" {
        "_jp2.zip"
    } else {
        "_jpg.zip"
    };
    let prefix = name
        .strip_suffix(suffix)
        .or_else(|| {
            if format == "jpg" {
                name.strip_suffix("_jpeg.zip")
            } else {
                None
            }
        })
        .filter(|s| safe_component(s))?;
    if !unmarked(&file["private"]) || !unmarked(&file["encrypted"]) {
        return None;
    }
    Some(Scan {
        prefix: String::from(prefix),
        archive: String::from(name),
        directory: String::from(name.strip_suffix(".zip")?),
        format,
    })
}

pub(super) fn metadata(id: &str) -> Result<Book> {
    let body = port::request(
        &format!("{SITE}/metadata/{id}"),
        "GET",
        json!({"Accept":"application/json"}),
        None,
    )?;
    if body.len() > MAX_METADATA {
        return Err("result_too_large");
    }
    let value: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    if value["metadata"]["identifier"].as_str() != Some(id) {
        return Err("not_found");
    }
    let metadata = &value["metadata"];
    if restricted(&value) || restricted(metadata) {
        return Err("archive_restricted");
    }
    if metadata["mediatype"] != "texts" {
        return Err("no_public_scan");
    }
    let files = value["files"].as_array().ok_or("no_public_scan")?;
    if files.iter().any(|file| {
        !unmarked(&file["encrypted"])
            || file["format"]
                .as_str()
                .is_some_and(|s| s.to_ascii_lowercase().contains("encrypted"))
    }) {
        return Err("archive_restricted");
    }
    let mut scans = Vec::new();
    let mut private_stack = false;
    for file in files {
        if matches!(
            file["format"].as_str(),
            Some("Single Page Processed JP2 ZIP" | "Single Page Processed JPEG ZIP")
        ) && !unmarked(&file["private"])
        {
            private_stack = true;
        }
        if let Some(scan) = scan_from_file(file) {
            if let Some(previous) = scans
                .iter_mut()
                .find(|existing: &&mut Scan| existing.prefix == scan.prefix)
            {
                if scan.format == "jp2" {
                    *previous = scan;
                }
            } else {
                scans.push(scan);
            }
        }
    }
    if scans.is_empty() {
        return Err(if private_stack {
            "archive_restricted"
        } else {
            "no_public_scan"
        });
    }
    if scans.len() > MAX_SCANS {
        return Err("result_too_large");
    }
    let node = value["server"]
        .as_str()
        .filter(|s| archive_node(s))
        .ok_or("no_public_scan")?;
    let path = value["dir"].as_str().ok_or("no_public_scan")?;
    let ending = format!("/items/{id}");
    if !path
        .strip_suffix(&ending)
        .and_then(|s| s.strip_prefix('/'))
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("no_public_scan");
    }
    let mut nodes = BTreeSet::from([String::from(node)]);
    for candidate in strings(&value["workable_servers"]).into_iter().chain(
        [value["d1"].as_str(), value["d2"].as_str()]
            .into_iter()
            .flatten(),
    ) {
        if archive_node(candidate) {
            nodes.insert(String::from(candidate));
        }
    }
    Ok(Book {
        node: String::from(node),
        path: String::from(path),
        id: String::from(id),
        nodes,
        scans,
        value,
    })
}

impl Book {
    pub(super) fn verify_any(&self) -> Result<String> {
        let mut refusal = "no_public_scan";
        for scan in &self.scans {
            match self.open(scan) {
                Ok(volume) => return Ok(volume.title),
                Err(reason @ ("archive_restricted" | "no_public_scan" | "result_too_large")) => {
                    refusal = reason
                }
                Err(reason) => return Err(reason),
            }
        }
        Err(refusal)
    }

    pub(super) fn open(&self, scan: &Scan) -> Result<Volume> {
        let mut url = Url::parse(&format!(
            "https://{}/BookReader/BookReaderJSIA.php",
            self.node
        ))
        .map_err(|_| "parse_error")?;
        url.query_pairs_mut()
            .append_pair("id", &self.id)
            .append_pair("itemPath", &self.path)
            .append_pair("server", &self.node)
            .append_pair("format", "json")
            .append_pair("subPrefix", &scan.prefix);
        let body = port::request(
            url.as_str(),
            "GET",
            json!({"Accept":"application/json,application/x-javascript"}),
            None,
        )?;
        if body.len() > MAX_JSIA {
            return Err("result_too_large");
        }
        let response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
        let data = &response["data"];
        let lending = &data["lendingInfo"];
        if [
            "isLendingRequired",
            "isArchiveOrgLending",
            "shouldProtectImages",
        ]
        .iter()
        .any(|key| lending[*key].as_bool() != Some(false))
            || [
                "isOpenLibraryLending",
                "isPrintDisabledOnly",
                "isBrowserBorrowable",
                "userHasBorrowed",
                "userHasBrowsed",
            ]
            .iter()
            .any(|key| !unmarked(&lending[*key]))
            || !lending["lendingStatus"].is_null()
            || restricted(&data["metadata"])
        {
            return Err("archive_restricted");
        }
        let options = &data["brOptions"];
        if options["bookId"].as_str() != Some(&self.id)
            || options["subPrefix"].as_str() != Some(&scan.prefix)
            || options["bookPath"] != format!("{}/{}", self.path, scan.prefix)
            || data["metadata"]["identifier"].as_str() != Some(&self.id)
            || !options["server"]
                .as_str()
                .is_some_and(|node| self.nodes.contains(node))
        {
            return Err("no_public_scan");
        }
        let archive = format!("{}/{}", self.path, scan.archive);
        if options["zip"].as_str() != Some(&archive) {
            return Err("no_public_scan");
        }
        let spreads = options["data"].as_array().ok_or("no_public_scan")?;
        let title = options["bookTitle"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("parse_error")?;
        let mut pages = Vec::new();
        let mut seen = BTreeSet::new();
        let mut byte_budget = OUTPUT - 2048;
        let identity_bytes = self.id.len() + encode(&scan.prefix).len();
        for spread in spreads {
            if host::check_cancel() {
                return Err("cancelled");
            }
            for page in spread.as_array().ok_or("no_public_scan")? {
                if !matches!(page["viewable"], Value::Null | Value::Bool(true)) {
                    return Err("archive_restricted");
                }
                let leaf = page["leafNum"].as_u64().ok_or("no_public_scan")?;
                if !seen.insert(leaf) {
                    return Err("no_public_scan");
                }
                let uri = self.image(
                    page["uri"].as_str().ok_or("no_public_scan")?,
                    &archive,
                    scan,
                )?;
                // The host consumes one complete page response. Refuse a
                // volume that cannot fit rather than returning an incomplete book.
                let page_bytes = 144 + identity_bytes + uri.len();
                byte_budget = byte_budget
                    .checked_sub(page_bytes)
                    .ok_or("result_too_large")?;
                pages.push(Leaf { id: leaf, uri });
            }
        }
        if pages.is_empty() {
            return Err("no_public_scan");
        }
        Ok(Volume {
            title: String::from(title),
            pages,
        })
    }

    fn image(&self, raw: &str, archive: &str, scan: &Scan) -> Result<String> {
        if raw.len() > 2048 || !raw.starts_with("https://") {
            return Err("no_public_scan");
        }
        let url = port::absolute(SITE, raw)?;
        if url.port().is_some()
            || url.fragment().is_some()
            || !url.host_str().is_some_and(|node| self.nodes.contains(node))
            || !url
                .path()
                .eq_ignore_ascii_case("/BookReader/BookReaderImages.php")
        {
            return Err("archive_restricted");
        }
        let mut zip = None;
        let mut file = None;
        let mut id = None;
        for (key, value) in url.query_pairs() {
            let target = match key.as_ref() {
                "zip" => &mut zip,
                "file" => &mut file,
                "id" => &mut id,
                _ => return Err("no_public_scan"),
            };
            if target.replace(value).is_some() {
                return Err("no_public_scan");
            }
        }
        if zip.as_deref() != Some(archive) || id.as_deref() != Some(self.id.as_str()) {
            return Err("no_public_scan");
        }
        let Some((dir, name)) = file.as_deref().and_then(|file| file.split_once('/')) else {
            return Err("no_public_scan");
        };
        if dir != scan.directory
            || !safe_component(name)
            || !(name
                .rsplit_once('.')
                .is_some_and(|(_, ext)| ext == scan.format)
                || (scan.format == "jpg" && name.ends_with(".jpeg")))
        {
            return Err("no_public_scan");
        }
        Ok(url.into())
    }
}

pub(super) struct Leaf {
    id: u64,
    uri: String,
}

pub(super) struct Volume {
    pub title: String,
    pub pages: Vec<Leaf>,
}

impl Volume {
    pub(super) fn payload(&self, chapter: &str, identity: &str) -> Result<Value> {
        let pages = self
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                json!({"id":format!("ia-page:{identity}:{}",page.id),
                "index":index,"image":{"kind":"url","url":page.uri}})
            })
            .collect::<Vec<_>>();
        Ok(json!({"chapterId":chapter,"pages":pages}))
    }
}
