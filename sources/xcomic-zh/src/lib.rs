#![no_std]
extern crate alloc;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
use regex::Regex;
use alloc::{string::{String,ToString},vec::Vec,collections::{BTreeMap,BTreeSet}};
#[path = "../chapter_editions.rs"]
mod chapter_editions;
#[path = "../dedup.rs"]
mod dedup;
#[path = "../probe_batches.rs"]
mod probe_batches;
#[path = "../detail_context.rs"]
mod detail_context;
// Independent modified adaptation of XCOMIC published JSON contract, Apache-2.0.
// Keiyoushi primary1319949ca9bc456908196e3ba2d104e94182c5d6. No auth/premium fallback.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xcomic.zh.koma",
    name: "XCOMIC",
    version: "0.1.11",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "XCOMIC Chinese public editions; anonymous GraphQL queries and direct original images. Usage unknown.",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: true,
    image_request: true,
    credentials: false,
};
const WEB: &[u8] = b"https://xcomic.me";
const MP: &[u8] = b"xcomic:zh:";
const CP: &[u8] = b"xcomicch:zh:";
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:2*1024*1024,body:1024*1024,http_req:65536,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn bytes(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    if write_bytes(b, n, s) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn q(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, s) {
        return Err("payload overflow");
    };
    bytes(b, n, b"\"")
}
fn number(s: &[u8]) -> Option<usize> {
    if s.is_empty() {
        return None;
    };
    let mut n = 0usize;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        };
        n = n.checked_mul(10)?.checked_add((c - b'0') as usize)?;
    }
    Some(n)
}
fn ws(s: &[u8], mut p: usize) -> usize {
    while p < s.len() && matches!(s[p], b' ' | b'\t' | b'\r' | b'\n') {
        p += 1
    }
    p
}
fn json_string_end(s: &[u8], p: usize) -> Option<usize> {
    if s.get(p) != Some(&b'"') {
        return None;
    };
    let mut i = p + 1;
    while i < s.len() {
        match s[i] {
            b'"' => return Some(i + 1),
            0..=31 => return None,
            b'\\' => {
                i += 1;
                match *s.get(i)? {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                    b'u' => {
                        let cp = hex4(s.get(i + 1..i + 5)?)?;
                        i += 4;
                        if (0xd800..=0xdbff).contains(&cp) {
                            if s.get(i + 1..i + 3) != Some(b"\\u") {
                                return None;
                            }
                            let low = hex4(s.get(i + 3..i + 7)?)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return None;
                            }
                            i += 6;
                        } else if (0xdc00..=0xdfff).contains(&cp) {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
fn json_value_end(s: &[u8], p: usize, depth: usize) -> Option<usize> {
    if depth > 32 {
        return None;
    };
    let p = ws(s, p);
    match *s.get(p)? {
        b'"' => json_string_end(s, p),
        b'{' | b'[' => {
            let object = s[p] == b'{';
            let close = if object { b'}' } else { b']' };
            let mut i = ws(s, p + 1);
            if s.get(i) == Some(&close) {
                return Some(i + 1);
            };
            loop {
                if object {
                    i = json_string_end(s, i)?;
                    i = ws(s, i);
                    if s.get(i) != Some(&b':') {
                        return None;
                    };
                    i = ws(s, i + 1)
                }
                i = json_value_end(s, i, depth + 1)?;
                i = ws(s, i);
                if s.get(i) == Some(&close) {
                    return Some(i + 1);
                };
                if s.get(i) != Some(&b',') {
                    return None;
                };
                i = ws(s, i + 1);
            }
        }
        b't' => {
            if s.get(p..p + 4) == Some(b"true") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'f' => {
            if s.get(p..p + 5) == Some(b"false") {
                Some(p + 5)
            } else {
                None
            }
        }
        b'n' => {
            if s.get(p..p + 4) == Some(b"null") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'-' | b'0'..=b'9' => {
            let mut i = p;
            if s[i] == b'-' {
                i += 1
            };
            if s.get(i) == Some(&b'0') {
                i += 1
            } else {
                if !matches!(s.get(i), Some(b'1'..=b'9')) {
                    return None;
                };
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
            }
            if s.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            if matches!(s.get(i), Some(b'e' | b'E')) {
                i += 1;
                if matches!(s.get(i), Some(b'+' | b'-')) {
                    i += 1
                };
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            Some(i)
        }
        _ => None,
    }
}
fn hex4(s: &[u8]) -> Option<u32> {
    if s.len() != 4 {
        return None;
    };
    let mut n = 0;
    for &c in s {
        n = n * 16
            + match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => return None,
            };
    }
    Some(n)
}
fn unescape<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != b'\\' {
            if c < 32 || c == b'"' {
                return Err("invalid JSON string");
            };
            bytes(out, &mut n, &[c])?;
            continue;
        }
        let c = *s.get(i).ok_or("invalid JSON escape")?;
        i += 1;
        let c = match c {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'b' => 8,
            b'f' => 12,
            b'u' => {
                let mut cp = hex4(s.get(i..i + 4).ok_or("invalid unicode escape")?)
                    .ok_or("invalid unicode escape")?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if s.get(i..i + 2) != Some(b"\\u") {
                        return Err("invalid unicode pair");
                    };
                    i += 2;
                    let low = hex4(s.get(i..i + 4).ok_or("invalid unicode pair")?)
                        .ok_or("invalid unicode pair")?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("invalid unicode pair");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("invalid unicode pair");
                };
                let ch = char::from_u32(cp).ok_or("invalid unicode scalar")?;
                let mut tmp = [0; 4];
                bytes(out, &mut n, ch.encode_utf8(&mut tmp).as_bytes())?;
                continue;
            }
            _ => return Err("invalid JSON escape"),
        };
        bytes(out, &mut n, &[c])?;
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}

type E = &'static str;
fn valid_json(s: &[u8]) -> Result<(), E> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    if json_value_end(s, 0, 0).map(|i| ws(s, i)) != Some(s.len()) {
        return Err("invalid complete JSON");
    }
    Ok(())
}
fn member<'a>(obj: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, E> {
    if obj.first() != Some(&b'{') {
        return Err("expected object");
    }
    let mut i = ws(obj, 1);
    let mut found = None;
    while obj.get(i) != Some(&b'}') {
        let end = json_string_end(obj, i).ok_or("invalid object key")?;
        let matches = &obj[i + 1..end - 1] == key;
        i = ws(obj, end);
        if obj.get(i) != Some(&b':') {
            return Err("invalid object member");
        }
        i = ws(obj, i + 1);
        let end = json_value_end(obj, i, 0).ok_or("invalid object value")?;
        if matches {
            if found.is_some() {
                return Err("duplicate object member");
            }
            found = Some(&obj[i..end]);
        }
        i = ws(obj, end);
        if obj.get(i) == Some(&b',') {
            i = ws(obj, i + 1)
        } else if obj.get(i) != Some(&b'}') {
            return Err("invalid object separator");
        }
    }
    Ok(found)
}
fn required<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    member(obj, k)?.ok_or("required field missing")
}
fn quoted(v: &[u8]) -> Result<&[u8], E> {
    if v.first() != Some(&b'"') || json_string_end(v, 0) != Some(v.len()) {
        return Err("expected string");
    }
    Ok(&v[1..v.len() - 1])
}
fn text<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    quoted(required(obj, k)?)
}
fn raw_string<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    let v = required(obj, k)?;
    quoted(v)?;
    Ok(v)
}
struct Array<'a> {
    s: &'a [u8],
    p: usize,
}
impl<'a> Array<'a> {
    fn new(s: &'a [u8]) -> Result<Self, E> {
        if s.first() != Some(&b'[') || s.last() != Some(&b']') {
            return Err("expected array");
        }
        Ok(Self { s, p: ws(s, 1) })
    }
    fn next(&mut self) -> Result<Option<&'a [u8]>, E> {
        if self.s.get(self.p) == Some(&b']') {
            return Ok(None);
        }
        let end = json_value_end(self.s, self.p, 0).ok_or("invalid array entry")?;
        let v = &self.s[self.p..end];
        self.p = ws(self.s, end);
        if self.s.get(self.p) == Some(&b',') {
            self.p = ws(self.s, self.p + 1)
        } else if self.s.get(self.p) != Some(&b']') {
            return Err("invalid array separator");
        }
        Ok(Some(v))
    }
}
fn args(r: &[u8]) -> Result<&[u8], E> {
    valid_json(r)?;
    Ok(member(r, b"args")?.unwrap_or(r))
}
fn integer(v: &[u8]) -> Result<usize, E> {
    number(v).ok_or("invalid integer")
}
fn page(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let v = match member(a, b"page")? {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        member(v, b"cursor")?.unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.first() == Some(&b'"') {
        quoted(v)?
    } else {
        v
    };
    integer(v).map_err(|_| "invalid request").and_then(|n| {
        if n > 0 && n <= 100000 {
            Ok(n)
        } else {
            Err("invalid request")
        }
    })
}
fn qid(b: &mut [u8], n: &mut usize, p: &[u8], a: &[u8], suffix: Option<&[u8]>) -> Result<(), E> {
    bytes(b, n, b"\"")?;
    bytes(b, n, p)?;
    bytes(b, n, a)?;
    if let Some(s) = suffix {
        bytes(b, n, b":")?;
        bytes(b, n, s)?;
    }
    bytes(b, n, b"\"")
}
fn rawq(b: &mut [u8], n: &mut usize, v: &[u8]) -> Result<(), E> {
    quoted(v)?;
    bytes(b, n, v)
}
fn wn(b: &mut [u8], n: &mut usize, v: usize) -> Result<(), E> {
    if write_usize(b, n, v) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn error_code(e: &str) -> &'static str {
    match e {
        "authentication required" => "auth_required",
        "payment or account required" => "auth_required",
        "permission denied" => "permission_denied",
        "not found" => "not_found",
        "rate limited" => "rate_limit",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "network failure" => "network_error",
        "cancelled" => "cancelled",
        "payload overflow" | "catalogue capacity exceeded" => "internal_error",
        "invalid request" | "invalid identity" | "empty query" | "unknown listing"
        | "invalid image URL" => "invalid_request",
        _ => "parse_error",
    }
}
fn execute(op: &str, res: Result<usize, E>) -> u32 {
    match res {
        Ok(n) if n + 256 < 1048576 => write_success_payload(op, n),
        Ok(_) => write_error(op, "internal_error", "payload overflow"),
        Err(e) => {
            host::log_info(error_code(e).as_bytes());
            write_error(op, error_code(e), e)
        }
    }
}
const TITLE_BROWSE_QUERY: &[u8] = br###"
    query get_title_browse($select: Title_Browse_Select) {
        get_title_browse_items(select: $select) {
            id
            data {
                title
                native_title
                romanized_title
                original_language
                translated_languages
                type
                cover_local_url
                cover_url
                comic_ids
                chap_last_public_at
            }
        }
    }
"###;
const TITLE_BROWSE_PAGER_QUERY: &[u8] = br###"
    query get_title_browse_pager($select: Title_Browse_Select) {
        get_title_browse_pager(select: $select) {
            next
            total
        }
    }
"###;
const TITLE_NODE_QUERY: &[u8] = br###"
    query get_title_titleNode($id: ID!) {
        get_title_titleNode(id: $id) {
            id
            data {
                id
                title
                alt_titles
                native_title
                romanized_title
                original_language
                translated_languages
                authors
                artists
                year
                type
                status
                description
                cover_local_url
                cover_local
                cover_url
                urlPath
                total_comics
                total_chapters
                total_follows
                total_reviews
                total_comments
                vote_avg
                vote_users
                vote_bay
                vote_val
                chap_last_public_at
                created_at
                updated_at
                is_merged
                merged_to
                comic_ids
                content_rating_id
                type_id
                demographic_ids
                genre_ids
                format_ids
                tracking_sites {
                    anilist
                    myanimelist
                    mangaupdates
                    kitsu
                    animeplanet
                    shikimori
                    mangabaka
                }
            }
        }
    }
"###;
const COMIC_NODE_QUERY: &[u8] = br###"
    query get_comicNode($id: ID!) {
        get_comicNode(id: $id) {
            id
            data {
                id
                name
                subName
                altNames
                originalLanguage
                translatedLanguage
                originalStatus
                uploadStatus
                type
                demographics
                contentRating
                genres
                tags
                dbStatus
                isPublic
                follows
                reviews
                comments_total
                score_val
                is_hot
                is_new
                originalPubFrom { y m d }
                originalPubTill { y m d }
                originalPubZone
                chaps_normal
                dateUpload
                chapterNode_up_to {
                    id
                    data {
                        dname
                        datePublic
                    }
                }
                summary {
                    text
                }
                extraInfo {
                    text
                }
                readDirection
                trackingSites {
                    anilist
                    myanimelist
                    mangaupdates
                    kitsu
                    animeplanet
                }
                urlPath
                urlCover
                title_titleNode {
                    id
                    data {
                        id
                        title
                        alt_titles
                        native_title
                        romanized_title
                        original_language
                        translated_languages
                        authors
                        artists
                        content_rating_id
                        type_id
                        demographic_ids
                        genre_ids
                        format_ids
                        year
                        type
                        status
                        description
                        cover_local_url
                        cover_url
                        urlPath
                        total_chapters
                        total_follows
                        total_reviews
                        total_comments
                        vote_avg
                        vote_users
                        vote_val
                        chap_last_public_at
                        is_merged
                        merged_to
                        comic_ids
                        tracking_sites {
                            anilist
                            myanimelist
                            mangaupdates
                            kitsu
                            animeplanet
                            shikimori
                            mangabaka
                        }
                    }
                }
            }
        }
    }
"###;
const COMIC_PROBE_QUERY: &[u8] = br###"
    query get_comicNode($id: ID!) {
        get_comicNode(id: $id) {
            id
            data {
                subName
                dbStatus
                isPublic
                translatedLanguage
                chaps_normal
                chapterNode_up_to {
                    data {
                        datePublic
                    }
                }
            }
        }
    }
"###;
const CHAPTER_UNIQ_LIST_QUERY: &[u8] = br###"
    query get_comic_chapterList_uniqList($select: Select_Comic_ChapterList_UniqList) {
        get_comic_chapterList_uniqList(select: $select) {
            paging {
                next
                total
            }
            items {
                id
                data {
                    id
                    dbStatus
                    isFinal
                    volume
                    serial
                    dname
                    title
                    urlPath
                    dateCreate
                    datePublic
                    dateModify
                    chaNum
                    volNum
                    count_images
                    is_new
                    srcName
                    srcTitle
                    srcColor
                    comments_topic
                    comments_total
                    views_login
                    views_guest
                }
            }
        }
    }
"###;
const CHAPTER_PAGES_QUERY: &[u8] = br###"
    query($id: ID!) {
        get_chapterNode(id: $id) {
            id
            data {
                imageUrls
            }
        }
    }
"###;
const MAX: usize = 1048576;
#[derive(Clone, Copy)]
struct Identity {
    data: [u8; 64],
    len: usize,
}
const NO_ID: Identity = Identity {
    data: [0; 64],
    len: 0,
};
impl Identity {
    fn new(s: &[u8]) -> Result<Self, E> {
        id(s)?;
        let mut v = NO_ID;
        v.data[..s.len()].copy_from_slice(s);
        v.len = s.len();
        Ok(v)
    }
    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut COMIC: [u8; MAX] = [0; MAX];
static mut TITLE: [u8; MAX] = [0; MAX];
static mut BROWSE: [u8; MAX] = [0; MAX];
static mut VARIABLES: [u8; 32768] = [0; 32768];
static mut POST_BODY: [u8; 49152] = [0; 49152];
static mut IDS: [Identity; 512] = [NO_ID; 512];
#[derive(Clone, Copy)]
struct Edition {
    id: Identity,
    label: [u8; 512],
    label_len: usize,
}
const NO_EDITION: Edition = Edition {
    id: NO_ID,
    label: [0; 512],
    label_len: 0,
};
static mut EDITIONS: [Edition; 512] = [NO_EDITION; 512];
fn edition(ident: Identity, data: &[u8]) -> Result<Edition, E> {
    let mut v = NO_EDITION;
    v.id = ident;
    if let Some(raw) = optional_text(data, b"subName")? {
        v.label_len = unescape(raw, &mut v.label)?.len();
    }
    Ok(v)
}
static mut CH_BYTES: [u8; MAX] = [0; MAX];
#[derive(Clone, Copy)]
struct Chapter {
    off: usize,
    len: usize,
    id: Identity,
    edition: Identity,
    number: f64,
    numbered: bool,
    ordinal: usize,
}
const NO_CH: Chapter = Chapter {
    off: 0,
    len: 0,
    id: NO_ID,
    edition: NO_ID,
    number: 0.0,
    numbered: false,
    ordinal: 0,
};
static mut CHS: [Chapter; 8192] = [NO_CH; 8192];
fn id(s: &[u8]) -> Result<&[u8], E> {
    if s.is_empty()
        || s.len() > 64
        || s.iter()
            .any(|c| !c.is_ascii_alphanumeric() && *c != b'_' && *c != b'-')
    {
        Err("invalid identity")
    } else {
        Ok(s)
    }
}
fn manga_id(r: &[u8]) -> Result<&[u8], E> {
    id(text(args(r)?, b"mangaId")?
        .strip_prefix(MP)
        .ok_or("invalid request")?)
}
fn chapter_id(r: &[u8]) -> Result<(&[u8], &[u8], &[u8]), E> {
    let s = text(args(r)?, b"chapterId")?
        .strip_prefix(CP)
        .ok_or("invalid request")?;
    let mut it = s.split(|c| *c == b':');
    let w = id(it.next().ok_or("invalid request")?)?;
    let e = id(it.next().ok_or("invalid request")?)?;
    let c = id(it.next().ok_or("invalid request")?)?;
    if it.next().is_some() {
        return Err("invalid request");
    };
    Ok((w, e, c))
}
fn vars_id(value: &[u8]) -> Result<&'static [u8], E> {
    id(value)?;
    unsafe {
        let mut n = 0;
        bytes(&mut VARIABLES, &mut n, b"{\"id\":")?;
        q(&mut VARIABLES, &mut n, value)?;
        bytes(&mut VARIABLES, &mut n, b"}")?;
        Ok(&VARIABLES[..n])
    }
}
fn query(qry: &[u8], vars: &[u8]) -> Result<&'static [u8], E> {
    valid_json(vars)?;
    unsafe {
        let mut n = 0;
        bytes(&mut POST_BODY, &mut n, b"{\"query\":")?;
        q(&mut POST_BODY, &mut n, qry)?;
        bytes(&mut POST_BODY, &mut n, b",\"variables\":")?;
        bytes(&mut POST_BODY, &mut n, vars)?;
        bytes(&mut POST_BODY, &mut n, b"}")?;
        fetch(&POST_BODY[..n])
    }
}
fn append_http_request(b: &mut [u8], n: &mut usize, post: &[u8]) -> Result<(), E> {
    bytes(b,n,b"{\"version\":1,\"method\":\"POST\",\"url\":\"https://xcomic.me/query/\",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://xcomic.me/\",\"Origin\":\"https://xcomic.me\",\"Content-Type\":\"application/json\"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\",\"bodyBase64\":")?;
    q(b, n, post)?;
    bytes(b, n, b"}")?;
    Ok(())
}
fn fetch(post: &[u8]) -> Result<&'static [u8], E> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    let b = http_req_buf();
    let mut n = 0;
    append_http_request(b, &mut n, post)?;
    let len = host::http_request(&b[..n], http_out()).map_err(|_| "network failure")?;
    if len == 0 || len > 2 * MAX {
        return Err("HTTP response capacity exceeded");
    };
    let resp =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT) as *const u8, len) };
    decode_http_response(resp)
}
fn decode_http_response(resp: &[u8]) -> Result<&'static [u8], E> {
    valid_json(resp)?;
    if member(resp, b"ok")? != Some(b"true".as_slice()) {
        return Err("network failure");
    };
    let st = integer(
        member(resp, b"status")?
            .or(member(resp, b"statusCode")?)
            .ok_or("missing HTTP status")?,
    )?;
    match st {
        200..=299 => {}
        401 => return Err("authentication required"),
        402 => return Err("payment or account required"),
        403 => return Err("permission denied"),
        404 => return Err("not found"),
        429 => return Err("rate limited"),
        400..=499 => return Err("client failure"),
        500..=599 => return Err("server failure"),
        _ => return Err("unexpected HTTP status"),
    };
    let body = unescape(text(resp, b"bodyText")?, body_buf())?;
    if body.len() >= MAX {
        return Err("HTTP response capacity exceeded");
    };
    valid_json(body)?;
    if let Some(err) = member(body, b"errors")? {
        if err != b"null" && Array::new(err)?.next()?.is_some() {
            return Err("GraphQL errors");
        }
    };
    let data = required(body, b"data")?;
    if data == b"null" {
        return Err("GraphQL data missing");
    };
    Ok(data)
}
fn optional_text<'a>(obj: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, E> {
    match member(obj, key)? {
        None | Some(b"null") => Ok(None),
        Some(v) => Ok(Some(quoted(v)?)),
    }
}
fn copy_title(work: &[u8]) -> Result<&'static [u8], E> {
    let data = query(TITLE_NODE_QUERY, vars_id(work)?)?;
    let node = required(data, b"get_title_titleNode")?;
    if node == b"null" {
        return Err("not found");
    };
    if text(node, b"id")? != work {
        return Err("title identity mismatch");
    };
    let t = required(node, b"data")?;
    if text(t, b"id")? != work {
        return Err("title identity mismatch");
    };
    if member(t, b"is_merged")? == Some(b"true".as_slice()) {
        return Err("merged title requires explicit identity review");
    };
    unsafe {
        if t.len() >= TITLE.len() {
            return Err("catalogue capacity exceeded");
        };
        TITLE[..t.len()].copy_from_slice(t);
        Ok(&TITLE[..t.len()])
    }
}
fn live_zh(probe: &[u8]) -> Result<bool, E> {
    let lang = optional_text(probe, b"translatedLanguage")?;
    let public = required(probe, b"isPublic")?;
    if public != b"true" && public != b"false" {
        return Err("invalid public flag");
    };
    let state = optional_text(probe, b"dbStatus")?;
    Ok(public == b"true" && state == Some(b"normal".as_slice()) && lang == Some(b"zh".as_slice()))
}
fn accept_probe(cid: Identity, data: &[u8], good: &mut usize) -> Result<(), E> {
    let node = required(data, b"get_comicNode")?;
    if node == b"null" {
        return Err("edition missing");
    };
    if text(node, b"id")? != cid.as_bytes() {
        return Err("edition identity mismatch");
    };
    let probe = required(node, b"data")?;
    if probe == b"null" {
        return Err("edition missing");
    };
    if live_zh(probe)? {
        unsafe { EDITIONS[*good] = edition(cid, probe)? };
        *good += 1
    }
    Ok(())
}
fn resolve(title: &[u8], req: &[u8]) -> Result<usize, E> {
    let mut a = Array::new(required(title, b"comic_ids")?)?;
    let mut count = 0;
    while let Some(v) = a.next()? {
        let idv = Identity::new(quoted(v)?)?;
        unsafe {
            if count == IDS.len() {
                return Err("catalogue capacity exceeded");
            };
            if IDS[..count].iter().any(|x| x.as_bytes() == idv.as_bytes()) {
                return Err("duplicate edition identity");
            };
            IDS[count] = idv
        };
        count += 1
    }
    let mut good = 0;
    if probe_batches::supported(req) {
        // Only independent probes are parallel. Keep the full catalogue and all
        // ownership/language guards; no TTL cache or shortened validation.
        for start in (0..count).step_by(probe_batches::MAX_REQUESTS) {
            let end = core::cmp::min(start + probe_batches::MAX_REQUESTS, count);
            probe_batches::collect(unsafe { &IDS[start..end] }, &mut good)?;
        }
    } else {
        for i in 0..count {
            let cid = unsafe { IDS[i] };
            let data = query(COMIC_PROBE_QUERY, vars_id(cid.as_bytes())?)?;
            accept_probe(cid, data, &mut good)?;
        }
    }
    unsafe {
        EDITIONS[..good].sort_unstable_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    }
    Ok(good)
}
fn rating<'a>(title: &'a [u8], comic: Option<&'a [u8]>) -> Result<&'static [u8], E> {
    let v = optional_text(title, b"content_rating_id")?.or(match comic {
        Some(c) => optional_text(c, b"contentRating")?,
        None => None,
    });
    Ok(match v {
        Some(b"safe") => b"safe",
        Some(b"suggestive") => b"suggestive",
        Some(b"erotica")
        | Some(b"pornographic")
        | Some(b"mature")
        | Some(b"explicit")
        | Some(b"nsfw") => b"nsfw",
        _ => b"unknown",
    })
}
fn valid_image(u: &[u8]) -> Result<(), E> {
    let s = core::str::from_utf8(u).map_err(|_| "invalid image URL")?;
    if !s.starts_with("https://")
        || s.len() > 2048
        || s.bytes()
            .any(|x| x <= 32 || x == b'\\' || x == b'"' || x == b'#')
    {
        return Err("invalid image URL");
    };
    let rest = &s[8..];
    let (h, path) = rest.split_once('/').ok_or("invalid image URL")?;
    let allowed = h == "xcomic.me"
        || h == "cdn.mangabaka.dev"
        || h == "images.mangabaka.dev"
        || h == "s4.anilist.co"
        || ((h.ends_with(".img00.org") || h.ends_with(".img01.org") || h.ends_with(".img02.org"))
            && h.len() == 13
            && h.as_bytes()[0] == b'i'
            && h.as_bytes()[1] == b'0'
            && h.as_bytes()[2].is_ascii_digit());
    if !allowed
        || path.is_empty()
        || path.split('/').any(|x| x == ".." || x == ".")
        || path.contains('%')
        || h.contains('@')
    {
        return Err("invalid image URL");
    };
    Ok(())
}
fn image_url<'a>(raw: &[u8], b: &'a mut [u8]) -> Result<&'a [u8], E> {
    let mut t = [0; 2048];
    let decoded = unescape(raw, &mut t)?;
    let mut n = 0;
    if decoded.starts_with(b"/") && !decoded.starts_with(b"//") {
        bytes(b, &mut n, WEB)?;
        bytes(b, &mut n, decoded)?
    } else {
        bytes(b, &mut n, decoded)?
    };
    valid_image(&b[..n])?;
    Ok(&b[..n])
}
fn emit_strings(b: &mut [u8], n: &mut usize, title: &[u8], key: &[u8]) -> Result<(), E> {
    bytes(b, n, b"[")?;
    if let Some(v) = member(title, key)? {
        if v != b"null" {
            let mut a = Array::new(v)?;
            let mut first = true;
            while let Some(s) = a.next()? {
                if !first {
                    bytes(b, n, b",")?
                };
                rawq(b, n, s)?;
                first = false
            }
        }
    };
    bytes(b, n, b"]")
}
// Fixed Keiyoushi 949dea titleRegex, enabled only by explicit preference.
// Java's (?:(?!]).)* is exactly a class excluding ']' and Java line terminators;
// .+? has the same line exclusions. Only /Official contains letters, so explicit
// I/i/İ/ı preserves Java UNICODE_CASE instead of a different Unicode case-fold.
const VERSION_PATTERN: &str = r"\([^()]*\)|\{[^{}]*\}|\[[^\]\n\r\u{85}\u{2028}\u{2029}]*\]|«[^»]*»|〘[^〙]*〙|「[^」]*」|『[^』]*』|≪[^≫]*≫|﹛[^﹜]*﹜|〖[^〖〗]*〗|\u{1690D}[^\n\r\u{85}\u{2028}\u{2029}]+?\u{1690D}|《[^》]*》|⌜[^\n\r\u{85}\u{2028}\u{2029}]+?⌝|⟨[^⟩]*⟩|/[Oo][Ff][Ff][Iiİı][Cc][Iiİı][Aa][Ll]|/ [Oo][Ff][Ff][Iiİı][Cc][Iiİı][Aa][Ll]";
fn title_cleaner(r: &[u8]) -> Result<Option<Regex>, E> {
    if json_object_member(r, b"settings")
        .and_then(|v| json_object_member(v, b"REMOVE_TITLE_VERSION")) == Some(b"true")
    {
        Regex::new(VERSION_PATTERN).map(Some).map_err(|_| "invalid title pattern")
    } else {
        Ok(None)
    }
}
fn emit_title(b: &mut [u8], n: &mut usize, raw: &[u8], cleaner: Option<&Regex>, fallback: Option<&[u8]>) -> Result<(), E> {
    let Some(cleaner) = cleaner else { return rawq(b, n, raw) };
    let mut decoded = alloc::vec![0; raw.len()];
    let original = core::str::from_utf8(unescape(quoted(raw)?, &mut decoded)?).map_err(|_| "invalid UTF8")?;
    let removed = cleaner.replace_all(original, "");
    // Kotlin Char.isWhitespace = Java isWhitespace || isSpaceChar.
    let trimmed = removed.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || ('\u{1c}'..='\u{1f}').contains(&c));
    let shown = if trimmed.is_empty() { fallback.unwrap_or(b"") } else { trimmed.as_bytes() };
    if !shown.contains(&b'&') { return q(b, n, shown); }
    // Fixed consumers call unescapeHtml after cleanTitleIfNeeded. Reuse the
    // SDK HTML5 entity owner once; literal angle brackets remain plain text.
    // This explicit-enabled branch never changes the original false/rawq path.
    let capacity = shown.len().checked_mul(6).ok_or("title capacity")?;
    let mut raw = alloc::vec![0; capacity];
    let mut raw_len = 0;
    if !append_json_escaped(&mut raw, &mut raw_len, shown) { return Err("title capacity"); }
    let mut decoded = alloc::vec![0; capacity];
    let mut decoded_len = 0;
    if !koma_source_sdk::plain_text::append_entity_decoded_json_content(&mut decoded, &mut decoded_len, &raw[..raw_len]) {
        return Err("title entity decode failed");
    }
    let mut text = alloc::vec![0; decoded_len];
    let value = unescape(&decoded[..decoded_len], &mut text)?;
    let string = core::str::from_utf8(value).map_err(|_| "invalid title UTF8")?;
    let blank = string.chars().all(|c| (c.is_whitespace() && c != '\u{85}') || ('\u{1c}'..='\u{1f}').contains(&c));
    q(b, n, if blank { fallback.unwrap_or(value) } else { value })
}
fn emit_manga(
    b: &mut [u8],
    n: &mut usize,
    title: &[u8],
    comic: Option<&[u8]>,
    work: &[u8],
    cleaner: Option<&Regex>,
    browse: bool,
) -> Result<(), E> {
    id(work)?;
    bytes(b, n, b"{\"id\":")?;
    qid(b, n, MP, work, None)?;
    bytes(b, n, b",\"title\":")?;
    emit_title(b, n, required(title, b"title")?, cleaner, if browse { Some(work) } else { None })?;
    let cover = optional_text(title, b"cover_local_url")?.or(optional_text(title, b"cover_url")?);
    if let Some(c) = cover {
        let mut u = [0; 2048];
        let url = image_url(c, &mut u)?;
        bytes(b, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(b, n, url)?;
        bytes(b, n, b"}")?
    };
    bytes(b, n, b",\"contentRating\":")?;
    q(b, n, rating(title, comic)?)?;
    bytes(b, n, b",\"authors\":")?;
    emit_strings(b, n, title, b"authors")?;
    bytes(b, n, b",\"status\":")?;
    let status = optional_text(title, b"status")?;
    q(
        b,
        n,
        match status {
            Some(b"releasing") | Some(b"ongoing") => b"ongoing",
            Some(b"completed") => b"completed",
            Some(b"hiatus") => b"hiatus",
            Some(b"cancelled") => b"cancelled",
            _ => b"unknown",
        },
    )?;
    bytes(b, n, b",\"description\":")?;
    match member(title, b"description")? {
        Some(v) if v != b"null" => rawq(b, n, v)?,
        _ => q(b, n, b"")?,
    };
    bytes(b, n, b",\"tags\":")?;
    emit_strings(b, n, title, b"genre_ids")?;
    bytes(
        b,
        n,
        b",\"links\":[{\"kind\":\"source\",\"url\":\"https://xcomic.me/title/",
    )?;
    bytes(b, n, work)?;
    bytes(b, n, b"\"}]}")
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let cleaner = title_cleaner(r)?;
    let work = manga_id(r)?;
    let title = copy_title(work)?;
    let good = resolve(title, r)?;
    let mut best = None;
    let mut largest = 0;
    let mut comic_len = 0;
    for i in 0..good {
        let eid = unsafe { EDITIONS[i].id };
        let d = query(COMIC_NODE_QUERY, vars_id(eid.as_bytes())?)?;
        let c = required(required(d, b"get_comicNode")?, b"data")?;
        if text(c, b"id")? != eid.as_bytes() || !live_zh(c)? {
            return Err("edition identity changed");
        };
        let count = integer(required(c, b"chaps_normal")?)?;
        if best.is_none() || count > largest {
            best = Some(eid);
            largest = count;
            unsafe {
                if c.len() >= COMIC.len() {
                    return Err("catalogue capacity exceeded");
                };
                COMIC[..c.len()].copy_from_slice(c);
                comic_len = c.len()
            }
        }
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    if best.is_some() {
        let c = unsafe { &COMIC[..comic_len] };
        emit_manga(b, &mut n, title, Some(c), work, cleaner.as_ref(), false)?
    } else {
        emit_manga(b, &mut n, title, None, work, cleaner.as_ref(), false)?
    };
    detail_context::emit(b,&mut n,r,title,work,good)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
// Old hosts must not expose filters they cannot scope safely.
fn filter_host(r: &[u8]) -> bool {
    member(r,b"hostFeatures").ok().flatten().and_then(|v| Array::new(v).ok()).map_or(false,|mut a| {
        while let Ok(Some(v))=a.next() { if v==br#""source_filter_scope_header_v1""# {return true;} } false
    })
}
fn list(r: &[u8], search: bool) -> Result<usize, E> {
    let cleaner = title_cleaner(r)?;
    let a = args(r)?;
    let p = page(r)?;
    let mut term = [0; 4096];
    let queryword = if search {
        unescape(optional_text(a, b"query")?.unwrap_or(b""), &mut term)?
    } else {
        b""
    };
    let sort = if search {
        b"field_score".as_slice()
    } else {
        match optional_text(a, b"listingId")?.or(optional_text(a, b"listing")?) {
            Some(b"latest") => b"field_update".as_slice(),
            None | Some(b"popular") => b"field_score".as_slice(),
            _ => return Err("unknown listing"),
        }
    };
    // An unselected sort preserves each existing listing's own order.
    let sort = if filter_host(r) {
        match member(a,b"filters")?.and_then(|f| optional_text(f,b"sort").ok().flatten()) {
            Some(v @ (b"field_score" | b"field_update" | b"field_create" | b"field_name_asc" | b"field_name_desc" | b"field_follow" | b"field_review" | b"field_comment" | b"field_chapter")) => v,
            _ => sort,
        }
    } else { sort };
    let mut vars = [0; 8192];
    let mut v = 0;
    bytes(&mut vars, &mut v, b"{\"select\":{\"page\":")?;
    wn(&mut vars, &mut v, p)?;
    bytes(&mut vars, &mut v, b",\"size\":12,\"init\":")?;
    wn(
        &mut vars,
        &mut v,
        (p - 1).checked_mul(12).ok_or("invalid request")?,
    )?;
    bytes(&mut vars, &mut v, b",\"sortby\":")?;
    q(&mut vars, &mut v, sort)?;
    bytes(&mut vars, &mut v, b",\"word\":")?;
    q(&mut vars, &mut v, queryword)?;
    bytes(
        &mut vars,
        &mut v,
        b",\"where\":\"browse\",\"incTLangs\":[\"zh\"]",
    )?;
    // The reference flag is a server-side browse preference. Preserve the old
    // omitted/default request; only explicit true opts out of its genre list.
    if json_object_member(r,b"settings").and_then(|s|json_object_member(s,b"IGNORE_GENRE_BLOCKLIST"))==Some(b"true") {
        bytes(&mut vars,&mut v,b",\"ignoreGlobalGenres\":true")?;
    }
    bytes(&mut vars,&mut v,b"}}")?;
    let data = query(TITLE_BROWSE_QUERY, &vars[..v])?;
    let items = required(data, b"get_title_browse_items")?;
    let len = items.len();
    unsafe {
        if len >= BROWSE.len() {
            return Err("catalogue capacity exceeded");
        };
        BROWSE[..len].copy_from_slice(items)
    };
    let d = query(TITLE_BROWSE_PAGER_QUERY, &vars[..v])?;
    let pager = required(d, b"get_title_browse_pager")?;
    let next = integer(required(pager, b"next")?)?;
    let total = integer(required(pager, b"total")?)?;
    if next != 0 && next != p + 1 {
        return Err("invalid browse pagination");
    };
    let mut seen = [NO_ID; 12];
    let mut count = 0;
    let mut arr = Array::new(unsafe { &BROWSE[..len] })?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    while let Some(node) = arr.next()? {
        if count == 12 {
            return Err("catalogue capacity exceeded");
        };
        let ident = Identity::new(text(node, b"id")?)?;
        if seen[..count]
            .iter()
            .any(|x| x.as_bytes() == ident.as_bytes())
        {
            return Err("duplicate title identity");
        };
        seen[count] = ident;
        let t = required(node, b"data")?;
        let mut langs = Array::new(required(t, b"translated_languages")?)?;
        let mut zh = false;
        while let Some(v) = langs.next()? {
            if v != b"null" && quoted(v)? == b"zh" {
                zh = true
            }
        }
        if !zh {
            return Err("browse language mismatch");
        };
        if count > 0 {
            bytes(b, &mut n, b",")?
        };
        emit_manga(b, &mut n, t, None, ident.as_bytes(), cleaner.as_ref(), true)?;
        count += 1
    }
    if (next != 0 && count != 12) || ((p - 1) * 12 + count > total) {
        return Err("invalid browse pagination");
    };
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if next == 0 {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?
    } else {
        bytes(b, &mut n, b"\"")?;
        wn(b, &mut n, next)?;
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?
    };
    Ok(n)
}
fn numeric(v: Option<&[u8]>) -> Result<Option<(&[u8], f64)>, E> {
    match v {
        None | Some(b"null") => Ok(None),
        Some(s) => {
            if s.first() == Some(&b'"') {
                return Err("invalid chapter number");
            };
            let value = core::str::from_utf8(s)
                .map_err(|_| "invalid UTF8")?
                .parse::<f64>()
                .map_err(|_| "invalid chapter number")?;
            if !value.is_finite() {
                return Err("invalid chapter number");
            };
            Ok(Some((s, value)))
        }
    }
}
fn emit_chapter(
    b: &mut [u8],
    n: &mut usize,
    work: &[u8],
    eid: &[u8],
    label: Option<&[u8]>,
    data: &[u8],
) -> Result<(), E> {
    let cid = id(text(data, b"id")?)?;
    bytes(b, n, b"{\"id\":\"")?;
    bytes(b, n, CP)?;
    bytes(b, n, work)?;
    bytes(b, n, b":")?;
    bytes(b, n, eid)?;
    bytes(b, n, b":")?;
    bytes(b, n, cid)?;
    bytes(b, n, b"\",\"mangaId\":")?;
    qid(b, n, MP, work, None)?;
    bytes(b, n, b",\"title\":")?;
    rawq(b, n, required(data, b"dname")?)?;
    bytes(b, n, b",\"chapterNumber\":")?;
    let number = numeric(member(data, b"chaNum")?)?.or(numeric(member(data, b"serial")?)?);
    bytes(b, n, number.map(|x| x.0).unwrap_or(b"null"))?;
    bytes(b, n, b",\"volumeNumber\":")?;
    bytes(
        b,
        n,
        numeric(member(data, b"volNum")?)?
            .map(|x| x.0)
            .unwrap_or(b"null"),
    )?;
    bytes(b, n, b",\"language\":\"zh\"")?;
    let mut uploader = [0; 512];
    let readable = match label {
        Some(s) if !s.is_empty() => Some(s),
        _ => match optional_text(data, b"srcName")? {
            Some(s) => {
                let s = unescape(s, &mut uploader)?;
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            None => None,
        },
    };
    if let Some(name) = readable {
        bytes(b, n, b",\"scanlator\":")?;
        q(b, n, name)?;
    }
    bytes(b, n, b"}")
}
fn collect(work: &[u8], target: Option<&[u8]>, normalize_labels: bool, deduplicate: bool, r: &[u8]) -> Result<(usize, usize, Vec<String>, usize), E> {
    let mut labels = Vec::new();
    let mut label_bytes = 0usize;
    let mut normalizer = if normalize_labels { Some(chapter_editions::Normalizer::new()?) } else { None };
    let title = copy_title(work)?;
    let good = if let Some(target) = target {
        let mut a = Array::new(required(title, b"comic_ids")?)?;
        let mut count = 0;
        let mut found = false;
        while let Some(v) = a.next()? {
            let ident = Identity::new(quoted(v)?)?;
            unsafe {
                if count == IDS.len() {
                    return Err("catalogue capacity exceeded");
                };
                if IDS[..count]
                    .iter()
                    .any(|x| x.as_bytes() == ident.as_bytes())
                {
                    return Err("duplicate edition identity");
                };
                IDS[count] = ident;
            }
            if ident.as_bytes() == target {
                found = true;
            }
            count += 1;
        }
        if !found {
            return Err("edition ownership mismatch");
        };
        let data = query(COMIC_NODE_QUERY, vars_id(target)?)?;
        let node = required(data, b"get_comicNode")?;
        if text(node, b"id")? != target {
            return Err("edition identity mismatch");
        };
        let comic = required(node, b"data")?;
        if text(comic, b"id")? != target || !live_zh(comic)? {
            return Err("edition identity changed");
        };
        if let Some(wrapper) = member(comic, b"title_titleNode")? {
            if wrapper != b"null" {
                if text(wrapper, b"id")? != work {
                    return Err("edition ownership mismatch");
                };
                let nested = required(wrapper, b"data")?;
                if nested != b"null" && text(nested, b"id")? != work {
                    return Err("edition ownership mismatch");
                };
            }
        }
        unsafe {
            EDITIONS[0] = edition(Identity::new(target)?, comic)?;
        }
        1
    } else if let Some(good)=detail_context::load(r,title,work) {
        good
    } else {
        resolve(title, r)?
    };
    let mut used = 0;
    let mut count = 0;
    for ei in 0..good {
        let current_edition = unsafe { EDITIONS[ei] };
        let eid = current_edition.id;
        let first_count = count;
        let mut p = 1;
        let mut expected = None;
        loop {
            if p > 10000 {
                return Err("catalogue capacity exceeded");
            };
            let mut vars = [0; 256];
            let mut v = 0;
            bytes(&mut vars, &mut v, b"{\"select\":{\"comic_id\":")?;
            q(&mut vars, &mut v, eid.as_bytes())?;
            bytes(&mut vars, &mut v, b",\"page\":")?;
            wn(&mut vars, &mut v, p)?;
            bytes(&mut vars, &mut v, if deduplicate { b",\"size\":1000,\"sortby\":\"chapter_desc\"}}".as_slice() } else { b",\"size\":100,\"sortby\":\"chapter_desc\"}}".as_slice() })?;
            let full_query;
            let (chapter_query, chapter_field) = if deduplicate { (CHAPTER_UNIQ_LIST_QUERY, b"get_comic_chapterList_uniqList".as_slice()) } else {
                full_query = core::str::from_utf8(CHAPTER_UNIQ_LIST_QUERY).map_err(|_| "invalid chapter query")?.replace("get_comic_chapterList_uniqList", "get_comic_chapterList_fullList").replace("Select_Comic_ChapterList_UniqList", "Select_Comic_ChapterList");
                (full_query.as_bytes(), b"get_comic_chapterList_fullList".as_slice())
            };
            let d = query(chapter_query, &vars[..v])?; let list = required(d, chapter_field)?;
            let paging = required(list, b"paging")?;
            let total = integer(required(paging, b"total")?)?;
            let next = integer(required(paging, b"next")?)?;
            if total > 8192 || expected.map(|x| x != total).unwrap_or(false) {
                return Err("inconsistent catalogue total");
            };
            expected = Some(total);
            if next != 0 && next != p + 1 {
                return Err("invalid chapter pagination");
            };
            let mut a = Array::new(required(list, b"items")?)?;
            let mut returned = 0;
            while let Some(item) = a.next()? {
                if count >= 8192 || returned >= if deduplicate { 1000 } else { 100 } {
                    return Err("catalogue capacity exceeded");
                };
                let data = required(item, b"data")?;
                let cid = Identity::new(text(data, b"id")?)?;
                if text(item, b"id")? != cid.as_bytes() {
                    return Err("chapter identity mismatch");
                };
                if optional_text(data, b"dbStatus")? != Some(b"normal".as_slice()) {
                    return Err("chapter not public normal");
                };
                unsafe {
                    if CHS[first_count..count]
                        .iter()
                        .any(|x| x.id.as_bytes() == cid.as_bytes())
                    {
                        return Err("duplicate chapter identity");
                    }
                };
                let num = numeric(member(data, b"chaNum")?)?.or(numeric(member(data, b"serial")?)?);
                if let Some(normalizer) = normalizer.as_mut() {
                    let mut label = normalizer.edition(&current_edition.label[..current_edition.label_len])?;
                    if label.is_empty() { label = normalizer.uploader(optional_text(data, b"srcName")?.unwrap_or(b""))?; }
                    label_bytes = label_bytes.checked_add(label.len()).ok_or("catalogue capacity exceeded")?;
                    if label_bytes > MAX { return Err("catalogue capacity exceeded"); }
                    labels.push(label);
                }
                let start = used;
                unsafe {
                    emit_chapter(
                        &mut CH_BYTES,
                        &mut used,
                        work,
                        eid.as_bytes(),
                        if current_edition.label_len > 0 {
                            Some(&current_edition.label[..current_edition.label_len])
                        } else {
                            None
                        },
                        data,
                    )?;
                    CHS[count] = Chapter {
                        off: start,
                        len: used - start,
                        id: cid,
                        edition: eid,
                        number: num.map(|x| x.1).unwrap_or(0.0),
                        numbered: num.is_some(),
                        ordinal: count,
                    }
                };
                count += 1;
                returned += 1
            }
            let accumulated = count - first_count;
            if accumulated > total
                || (next != 0 && (returned == 0 || (!deduplicate && returned != 100) || accumulated >= total))
                || (next == 0 && accumulated != total)
            {
                return Err("incomplete chapter pagination");
            };
            if next == 0 {
                break;
            };
            p = next
        }
    }
    unsafe {
        CHS[..count].sort_unstable_by(|a, b| {
            b.numbered
                .cmp(&a.numbered)
                .then_with(|| {
                    b.number
                        .partial_cmp(&a.number)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .then_with(|| a.ordinal.cmp(&b.ordinal))
        })
    };
    Ok((count, used, labels, good))
}
fn emit_edition_chapter(b:&mut [u8],n:&mut usize,raw:&[u8],label:&str)->Result<(),E> {
    if let Some(old)=member(raw,b"scanlator")? {
        let offset=old.as_ptr() as usize-raw.as_ptr() as usize;
        bytes(b,n,&raw[..offset])?;q(b,n,label.as_bytes())?;bytes(b,n,&raw[offset+old.len()..])
    } else {
        bytes(b,n,&raw[..raw.len()-1])?;bytes(b,n,b",\"scanlator\":")?;q(b,n,label.as_bytes())?;bytes(b,n,b"}")
    }
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let selected=chapter_editions::selection(r)?;
    // Exact continuation is not a filtered catalogue. Restore only the validated
    // requested original identity; keep the old default branch byte-for-byte.
    let feature = chapter_editions::supported(r);
    let exact=if (feature || !selected.is_empty())&&member(args(r)?,b"chapterId")?.is_some(){Some(chapter_id(r)?)}else{None};
    let work=manga_id(r)?;
    if exact.is_some_and(|v|v.0!=work){return Err("chapter ownership mismatch");}
    let group = matches!(json_object_member(r, b"settings").and_then(|v| json_object_member(v, b"GROUP_BY_SOURCE")), Some(b"true") | Some(br#""true""#));
    // Keep this language's historical default unless the capable host supplies a preference.
    let value = if feature { json_object_member(r,b"settings").and_then(|v|json_object_member(v,b"DEDUPLICATE_CHAPTERS")) } else { None };
    let deduplicate = !matches!(value, Some(b"false") | Some(br#""false""#));
    let (count, _, labels, good) = collect(work, exact.map(|v|v.1), (!selected.is_empty() || group) && exact.is_none(), if exact.is_some() && feature { false } else { deduplicate }, r)?;
    let full_query;
    let dedup_excluded = if feature && deduplicate && exact.is_none() {
        full_query = core::str::from_utf8(CHAPTER_UNIQ_LIST_QUERY).map_err(|_| "invalid chapter query")?
            .replace("get_comic_chapterList_uniqList", "get_comic_chapterList_fullList")
            .replace("Select_Comic_ChapterList_UniqList", "Select_Comic_ChapterList");
        dedup::exclusions(good,count,full_query.as_bytes())?
    } else { Vec::new() };
    let mut collisions:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();
    if !labels.is_empty(){
        for ch in unsafe{&CHS[..count]} {
            let label=&labels[ch.ordinal];if label.is_empty(){continue;}
            collisions.entry(label.to_lowercase()).or_default().insert(core::str::from_utf8(ch.edition.as_bytes()).map_err(|_|"invalid identity")?.to_string());
        }
    }
    if group && exact.is_none() {
        let sort_label = |ch: &Chapter| {
            let label = &labels[ch.ordinal];
            if collisions.get(&label.to_lowercase()).is_some_and(|ids| ids.len() > 1) {
                alloc::format!("{} [{}]", label, core::str::from_utf8(ch.edition.as_bytes()).unwrap_or("")) .to_lowercase()
            } else { label.to_lowercase() }
        };
        unsafe { CHS[..count].sort_unstable_by(|a,b| sort_label(a).cmp(&sort_label(b))
            .then_with(|| b.numbered.cmp(&a.numbered))
            .then_with(|| b.number.partial_cmp(&a.number).unwrap_or(core::cmp::Ordering::Equal))
            .then_with(|| a.ordinal.cmp(&b.ordinal))) };
    }
    // The directory can be sorted by label or globally by number. Keep
    // continuation scoped to the same edition, independent of display order.
    let reading_feature = json_object_member(r,b"hostFeatures").map(|v|v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c|*c==b',').any(|v|v.trim_ascii()==br#""chapter_reading_order_v1""#)).unwrap_or(false);
    let b = payload_buf();let mut n = 0;let mut written=0;let mut excluded=Vec::new();
    bytes(b, &mut n, b"{\"items\":[")?;
    for (index,ch) in unsafe{&CHS[..count]}.iter().enumerate() {
        if let Some((_,edition,chapter))=exact {
            if ch.edition.as_bytes()!=edition||ch.id.as_bytes()!=chapter{continue;}
        }
        let raw=unsafe{&CH_BYTES[ch.off..ch.off+ch.len]};
        let mut label=if labels.is_empty(){String::new()}else{labels[ch.ordinal].clone()};
        if !labels.is_empty(){
            if collisions.get(&label.to_lowercase()).is_some_and(|ids|ids.len()>1){label=alloc::format!("{} [{}]",label,core::str::from_utf8(ch.edition.as_bytes()).map_err(|_|"invalid identity")?);}
            if !selected.is_empty() && !chapter_editions::matches(&selected,&label){excluded.push(index);continue;}
        }
        if written>0{bytes(b,&mut n,b",")?;}
        if labels.is_empty(){bytes(b,&mut n,raw)?;}else{emit_edition_chapter(b,&mut n,raw,&label)?;}
        if reading_feature && exact.is_none() {
            let edition=core::str::from_utf8(ch.edition.as_bytes()).map_err(|_|"invalid edition identity")?;
            n=n.checked_sub(1).ok_or("invalid chapter JSON")?;
            bytes(b,&mut n,b",\"readingGroup\":")?;q(b,&mut n,edition.as_bytes())?;
            let eligible=ch.numbered && ch.number >= 0.0;
            bytes(b,&mut n,b",\"continuationEligible\":")?;bytes(b,&mut n,if eligible{b"true"}else{b"false"})?;
            if eligible {bytes(b,&mut n,b",\"readingOrder\":")?;bytes(b,&mut n,required(raw,b"chapterNumber")?)?;}
            bytes(b,&mut n,b"}")?;
        }
        written+=1;
    }
    if exact.is_some()&&written!=1{return Err("chapter ownership mismatch");}
    bytes(b,&mut n,b"]")?;
    if exact.is_none() && (!selected.is_empty() || !dedup_excluded.is_empty()) {
        bytes(b,&mut n,b",\"excludedChapterIds\":[")?;
        for (i,index) in excluded.iter().enumerate(){
            if i>0{bytes(b,&mut n,b",")?;}
            let ch=unsafe{CHS[*index]};
            let raw=unsafe{&CH_BYTES[ch.off..ch.off+ch.len]};rawq(b,&mut n,required(raw,b"id")?)?;
        }
        for (edition,chapter) in &dedup_excluded {
            if !excluded.is_empty() || n > 0 && b[n-1] != b'[' { bytes(b,&mut n,b",")?; }
            bytes(b,&mut n,b"\"")?;bytes(b,&mut n,CP)?;bytes(b,&mut n,work)?;bytes(b,&mut n,b":")?;bytes(b,&mut n,edition)?;bytes(b,&mut n,b":")?;bytes(b,&mut n,chapter)?;bytes(b,&mut n,b"\"")?;
        }
        bytes(b,&mut n,b"]")?;
    }
    bytes(b,&mut n,b"}")?;Ok(n)
}

fn pages(r: &[u8]) -> Result<usize, E> {
    let (work, eid, cid) = chapter_id(r)?;
    let (count, _, _, _) = collect(work, Some(eid), false, if chapter_editions::supported(r) { false } else { true }, r)?;
    let belongs = unsafe {
        CHS[..count]
            .iter()
            .any(|x| x.edition.as_bytes() == eid && x.id.as_bytes() == cid)
    };
    if !belongs {
        return Err("chapter ownership mismatch");
    };
    let data = query(CHAPTER_PAGES_QUERY, vars_id(cid)?)?;
    let node = required(data, b"get_chapterNode")?;
    if text(node, b"id")? != cid {
        return Err("chapter identity mismatch");
    };
    let imgs = required(required(node, b"data")?, b"imageUrls")?;
    let mut a = Array::new(imgs)?;
    let b = payload_buf();
    let mut n = 0;
    let mut i = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    while let Some(raw) = a.next()? {
        if i >= 8192 {
            return Err("catalogue capacity exceeded");
        };
        let mut url = [0; 2048];
        let u = image_url(quoted(raw)?, &mut url)?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":\"xcomicpage:zh:")?;
        bytes(b, &mut n, eid)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, cid)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}}")?;
        i += 1
    }
    if i == 0 {
        return Err("empty original page list");
    };
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let mut url = [0; 2048];
    let u = unescape(text(args(r)?, b"url")?, &mut url)?;
    valid_image(u)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://xcomic.me/\",\"Origin\":\"https://xcomic.me\"}}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let s="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门漫画\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新漫画\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let schema = if filter_host(r) { br#"{"filters":[{"id":"sort","name":"Order by","type":"select","options":[{"value":"field_score","label":"Rating Score"},{"value":"field_update","label":"Latest Update"},{"value":"field_create","label":"Recently Added"},{"value":"field_name_asc","label":"Name A-Z"},{"value":"field_name_desc","label":"Name Z-A"},{"value":"field_follow","label":"Most Follows"},{"value":"field_review","label":"Most Reviews"},{"value":"field_comment","label":"Most Comments"},{"value":"field_chapter","label":"Most Chapters"}]}]}"#.as_slice() } else { br#"{"filters":[]}"#.as_slice() };
    payload_buf()[..schema.len()].copy_from_slice(schema);
    write_success_payload("get_filters",schema.len())
}
fn run_get_settings(req: &[u8]) -> u32 {
    let schema = r#"{"settings":[{"id":"REMOVE_TITLE_VERSION","name":"Remove Version Information From Entry Titles","kind":"toggle","default":"false","description":"This removes version tags like '(Official)' or '(Yaoi)' from entry titles."},{"id":"IGNORE_GENRE_BLOCKLIST","name":"Ignore WebView Genre Blocklist","kind":"toggle","default":"false"},{"id":"GROUP_BY_SOURCE","name":"Group Chapters By Edition","kind":"toggle","default":"false","description":"Sort chapters by edition label, then chapter number."}]}"#.as_bytes();
    let out=payload_buf();let mut n=0;
    if !chapter_editions::supported(req){out[..schema.len()].copy_from_slice(schema);return write_success_payload("get_settings",schema.len());}
    if bytes(out,&mut n,&schema[..schema.len()-2]).is_err() || bytes(out,&mut n,br#",{"id":"DEDUPLICATE_CHAPTERS","name":"Deduplicate Chapter List","kind":"toggle","default":"true","description":"Use the provider deduplicated list; duplicate uploads from the same edition may be hidden."},{"id":"SOURCE_INCLUDE","name":"Chapter Editions To Include","kind":"text","default":"","description":"Comma-separated edition labels; a base label includes matching editions, while a suffixed label selects one. Empty includes all editions."}]}"#).is_err(){return write_error("get_settings","internal_error","payload overflow");}
    write_success_payload("get_settings",n)
}
// Source-local ABI gate; SDK remains byte-identical. Check before helper forms a slice.
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= MAX || end > cap {
        return None;
    };
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_init(ptr: u32, len: u32) -> i32 {
    if checked_request(ptr, len).is_some() {
        0
    } else {
        -1
    }
}
#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => $run(r),
                None => write_error($op, "invalid_request", "invalid ABI request range"),
            }
        }
    };
}
export_op!(koma_source_search, "search", run_search);
export_op!(koma_source_get_manga, "get_manga", run_get_manga);
export_op!(koma_source_get_chapters, "get_chapters", run_get_chapters);
export_op!(koma_source_get_pages, "get_pages", run_get_pages);
export_op!(koma_source_get_listings, "get_listings", run_get_listings);
export_op!(
    koma_source_get_manga_list,
    "get_manga_list",
    run_get_manga_list
);
export_op!(koma_source_get_home, "get_home", run_get_home);
export_op!(koma_source_get_filters, "get_filters", run_get_filters);
export_op!(koma_source_get_settings, "get_settings", run_get_settings);
export_op!(
    koma_source_get_image_request,
    "get_image_request",
    run_get_image_request
);
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= MAX {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
