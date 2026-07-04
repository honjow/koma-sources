#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, build_post_request, fetch_error_code};

const API_BASE: &[u8] = b"http://mangaapi.manhuaren.com";
const WEB_BASE: &[u8] = b"https://www.manhuaren.com";
const GSN_SALT: &[u8] = b"4e0a48e1c0b54041bce9c8f0e036124d";
const PAGE_SIZE: usize = 20;

koma_source_sdk::koma_source_buffers! {
    payload: 512 * 1024,
    http_out: 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 16 * 1024,
    scratch: 4096,
}
koma_source_sdk::koma_source_helpers!();
const JSON_BUF_CAP: usize = 1024 * 1024;
const URL_BUF_CAP: usize = 8192;

static mut JSON_BUF: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];
static mut URL_BUF: [u8; URL_BUF_CAP] = [0; URL_BUF_CAP];
static mut AUTH_USER_ID: [u8; 32] = [0; 32];
static mut AUTH_TOKEN: [u8; 2048] = [0; 2048];
static mut AUTH_USER_ID_LEN: usize = 0;
static mut AUTH_TOKEN_LEN: usize = 0;

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuaren.koma",
    name: "漫画人",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫画人 (manhuaren.com) API source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: true,
    filters: true,
    settings: false,
    image_request: true,
    credentials: false,
};

fn auth_user_id_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(AUTH_USER_ID) }
}
fn auth_token_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(AUTH_TOKEN) }
}
fn auth_user_id_slice() -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(
            core::ptr::addr_of!(AUTH_USER_ID) as *const u8,
            AUTH_USER_ID_LEN,
        )
    }
}
fn auth_token_slice() -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(AUTH_TOKEN) as *const u8, AUTH_TOKEN_LEN)
    }
}
fn json_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(JSON_BUF) as *const u8, len) }
}

fn json_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(JSON_BUF) }
}

fn url_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(URL_BUF) }
}

fn url_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(URL_BUF) as *const u8, len) }
}

#[derive(Copy, Clone)]
struct Param<'a> {
    key: &'static [u8],
    value: &'a [u8],
}

const IMEI: &[u8] = b"860000000000006";
const LAST_USED_TIME: &[u8] = b"1700000000000";
const GTS: &[u8] = b"2026-05-25+00:00:00";
const USER_ID_FALLBACK: &[u8] = b"-1";

fn set_auth(user_id: &[u8], token: &[u8]) -> bool {
    if user_id.is_empty() || token.is_empty() {
        return false;
    }
    if user_id.len() > 32 || token.len() > 2048 {
        return false;
    }
    auth_user_id_buf()[..user_id.len()].copy_from_slice(user_id);
    auth_token_buf()[..token.len()].copy_from_slice(token);
    unsafe {
        AUTH_USER_ID_LEN = user_id.len();
        AUTH_TOKEN_LEN = token.len();
    }
    true
}

fn common_params<'a>(user_id: &'a [u8]) -> [Param<'a>; 34] {
    [
        Param {
            key: b"gsm",
            value: b"md5",
        },
        Param {
            key: b"gft",
            value: b"json",
        },
        Param {
            key: b"gak",
            value: b"android_manhuaren2",
        },
        Param {
            key: b"gat",
            value: b"",
        },
        Param {
            key: b"gui",
            value: user_id,
        },
        Param {
            key: b"gts",
            value: GTS,
        },
        Param {
            key: b"gut",
            value: b"0",
        },
        Param {
            key: b"gem",
            value: b"1",
        },
        Param {
            key: b"gaui",
            value: user_id,
        },
        Param {
            key: b"gln",
            value: b"",
        },
        Param {
            key: b"gcy",
            value: b"US",
        },
        Param {
            key: b"gle",
            value: b"zh",
        },
        Param {
            key: b"gcl",
            value: b"dm5",
        },
        Param {
            key: b"gos",
            value: b"1",
        },
        Param {
            key: b"gov",
            value: b"33_13",
        },
        Param {
            key: b"gav",
            value: b"7.0.1",
        },
        Param {
            key: b"gdi",
            value: IMEI,
        },
        Param {
            key: b"gfcl",
            value: b"dm5",
        },
        Param {
            key: b"gfut",
            value: LAST_USED_TIME,
        },
        Param {
            key: b"glut",
            value: LAST_USED_TIME,
        },
        Param {
            key: b"gpt",
            value: b"com.mhr.mangamini",
        },
        Param {
            key: b"gciso",
            value: b"us",
        },
        Param {
            key: b"glot",
            value: b"",
        },
        Param {
            key: b"glat",
            value: b"",
        },
        Param {
            key: b"gflot",
            value: b"",
        },
        Param {
            key: b"gflat",
            value: b"",
        },
        Param {
            key: b"glbsaut",
            value: b"0",
        },
        Param {
            key: b"gac",
            value: b"",
        },
        Param {
            key: b"gcut",
            value: b"GMT+8",
        },
        Param {
            key: b"gfcc",
            value: b"",
        },
        Param {
            key: b"gflg",
            value: b"",
        },
        Param {
            key: b"glcn",
            value: b"",
        },
        Param {
            key: b"glcc",
            value: b"",
        },
        Param {
            key: b"gflcc",
            value: b"",
        },
    ]
}

fn bytes_cmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        if a[i] < b[i] {
            return -1;
        }
        if a[i] > b[i] {
            return 1;
        }
        i += 1;
    }
    if a.len() < b.len() {
        -1
    } else if a.len() > b.len() {
        1
    } else {
        0
    }
}

fn write_query_param(dst: &mut [u8], cursor: &mut usize, first: &mut bool, p: Param<'_>) -> bool {
    if *first {
        if !write_bytes(dst, cursor, b"?") {
            return false;
        }
        *first = false;
    } else if !write_bytes(dst, cursor, b"&") {
        return false;
    }
    write_bytes(dst, cursor, p.key)
        && write_bytes(dst, cursor, b"=")
        && write_url_encoded(dst, cursor, p.value)
}

fn append_sorted_gsn_material(
    dst: &mut [u8],
    cursor: &mut usize,
    method: &[u8],
    params: &[Param<'_>],
) -> bool {
    if !(write_bytes(dst, cursor, GSN_SALT) && write_bytes(dst, cursor, method)) {
        return false;
    }
    let mut used = [false; 64];
    let mut used_count = 0usize;
    while used_count < params.len() {
        let mut best: Option<usize> = None;
        let mut i = 0usize;
        while i < params.len() {
            if !used[i] {
                if let Some(bi) = best {
                    if bytes_cmp(params[i].key, params[bi].key) < 0 {
                        best = Some(i);
                    }
                } else {
                    best = Some(i);
                }
            }
            i += 1;
        }
        let Some(idx) = best else {
            break;
        };
        used[idx] = true;
        used_count += 1;
        if !(write_bytes(dst, cursor, params[idx].key)
            && write_url_encoded(dst, cursor, params[idx].value))
        {
            return false;
        }
    }
    write_bytes(dst, cursor, GSN_SALT)
}

fn build_api_url(path: &[u8], endpoint_params: &[Param<'_>]) -> Option<usize> {
    build_api_url_with_user(b"GET", path, endpoint_params, USER_ID_FALLBACK, None)
}

fn build_api_url_with_user(
    method: &[u8],
    path: &[u8],
    endpoint_params: &[Param<'_>],
    uid: &[u8],
    body: Option<&[u8]>,
) -> Option<usize> {
    let common = common_params(uid);
    let mut all = [Param {
        key: b"",
        value: b"" as &[u8],
    }; 48];
    let mut count = 0usize;
    for p in endpoint_params {
        all[count] = *p;
        count += 1;
    }
    for p in common.iter() {
        all[count] = *p;
        count += 1;
    }
    if let Some(body) = body {
        all[count] = Param {
            key: b"body",
            value: body,
        };
        count += 1;
    }

    let sign_scratch = scratch_b();
    let mut sc = 0usize;
    append_sorted_gsn_material(sign_scratch, &mut sc, method, &all[..count]).then_some(())?;
    let mut digest = [0u8; 16];
    md5(&sign_scratch[..sc], &mut digest);
    let mut gsn_hex = [0u8; 32];
    hex_encode(&digest, &mut gsn_hex);

    let out = url_buf();
    let mut c = 0usize;
    write_bytes(out, &mut c, API_BASE).then_some(())?;
    write_bytes(out, &mut c, path).then_some(())?;
    let mut first = true;
    for p in endpoint_params {
        write_query_param(out, &mut c, &mut first, *p).then_some(())?;
    }
    for p in common.iter() {
        write_query_param(out, &mut c, &mut first, *p).then_some(())?;
    }
    write_query_param(
        out,
        &mut c,
        &mut first,
        Param {
            key: b"gsn",
            value: &gsn_hex,
        },
    )
    .then_some(())?;
    Some(c)
}

fn build_authed_get_request(dst: &mut [u8], url: &[u8], auth: &[u8]) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"version":1,"method":"GET","url":""#).then_some(())?;
    append_json_escaped(dst, &mut c, url).then_some(())?;
    write_bytes(dst, &mut c, br#"","headers":{"User-Agent":"Dalvik/2.1.0 (Linux; U; Android 13; Koma Build/Koma)","Authorization":""#).then_some(())?;
    append_json_escaped(dst, &mut c, auth).then_some(())?;
    write_bytes(dst, &mut c, br#"","X-Yq-Yqci":"{\"at\":-1,\"av\":\"7.0.1\",\"ciso\":\"us\",\"cl\":\"dm5\",\"cy\":\"US\",\"di\":\"860000000000006\",\"dm\":\"Koma\",\"fcl\":\"dm5\",\"ft\":\"mhr\",\"fut\":\"1700000000000\",\"installation\":\"dm5\",\"le\":\"zh\",\"ln\":\"\",\"lut\":\"1700000000000\",\"nt\":3,\"os\":1,\"ov\":\"33_13\",\"pt\":\"com.mhr.mangamini\",\"rn\":\"1080x1920\",\"st\":0}"},"timeoutMs":15000,"responseKind":"bodyText"}"#).then_some(())?;
    Some(c)
}

fn fetch_json(url: &[u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"manhuaren: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    let resp = &http_out()[..resp_len];
    // Manhuaren API uses {"response":{...}} for success, {"errorResponse":{...}} for error
    if contains_bytes(resp, br#""errorResponse""#) {
        return Err(FetchError::ClientError);
    }
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let out = json_buf();
    let mut c = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            match next {
                b'"' => out[c] = b'"',
                b'\\' => out[c] = b'\\',
                b'/' => out[c] = b'/',
                b'n' => out[c] = b'\n',
                b'r' => out[c] = b'\r',
                b't' => out[c] = b'\t',
                _ => out[c] = next,
            }
            c += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            break;
        }
        if c >= out.len() {
            return Err(FetchError::Network);
        }
        out[c] = b;
        c += 1;
        i += 1;
    }
    Ok(json_slice(c))
}

const ANONY_BODY: &[u8] = br#"{"keys":[{"key":"Xzx0eOT9S0PvTAgEc+1ArTpY/7QkbuTcgZRrcj+F3jXawag6aK7jsOos9n8cMDZJTTRFpDZBLrgr81WvOLv5S6tydZnH+hwYFnNzP26sMnO5jLJ1H12JfQI1lA3QUWnAEE2Mxxu/PaLK6kJi0w3ZkVa/5VeCwm8ucRyAVHU2WKJsGt0i7hLs2WUrMNpXAAlRTFFt6nHH7afw0hUFkuXAmIozaHnWi/xxt6ueTWqfM6zs4x4FunXzEOLjDy1864bIrHPyM5Kz9kWNDG2JuxNQabl0rwclOeGKd1Am+ZC0ikBmPdXdfu6esdZtRtbXarNoA5d35Q6C9HhIT2snXpC4kQ==","keyType":"0"},{"key":"EbKgPL1r273TYaF9rlsh9wHhgmEF7M9BR6hCYim7tQvbnTImnrzU5+PpcIQZ6be630vb0T2WyPvIQb5BPI8riy6o2pUHRi2AS1WZPQcGxOkWU1Kl9JEeQMTVTz2iTKctBK5Ozh5eZMUJfJ3XIpfTkQCJ5G1O6CrmRUbTc5NGl3mVVqotThkpTtwSNym9O+a5fXnP8jiu3ozQ/LnSStq7RMrjHy+Mc8t/jAEXJpuzz+QemPW6EcRbW7JJXRROQPsljn0Ov8Jq499xP1cEMosIcr2G3ouOmNu4BKLTQHfQNti8lBaEE8CiEVuhIZ9BqS+hCj0JtJEpebL+jYtK9lAoDA==","keyType":"2"},{"key":"CjgpSmGEoSDw0f1/+s59b9XqZ6tvnTx5m36MCWGghjV85Vi8Dph4mw0YwK0WbyTjQvHuYJ88ixKAGBg1BnextgxoEAfHBmCa38MjSEpF5FqlzGLECaeA1OQBJehLHUukdfvqK7odgYre80DMbyf4XWZ0OQXbVbY4fI8DelZGZznZAKaIvrGCYrzpQx+O73NecxkdYsJw+NeH4h1GDqEbKV70XsXtb3nxFY4PbfaI0xfBB7WrsnaIUJvkMZuj851QsNBKMbohFFAB+WPq9+lZtnYCBcSjYh7Ghak/tAZMh9d4DF2MW1UQjt0WJsA0/KO12S5n83dX3gTc7pe+F71sxQ==","keyType":"-1"}]}"#;

fn fetch_anony_user() -> Result<(), FetchError> {
    let len = build_api_url_with_user(
        b"POST",
        b"/v1/user/createAnonyUser2",
        &[],
        USER_ID_FALLBACK,
        Some(ANONY_BODY),
    )
    .ok_or(FetchError::Network)?;
    let url = url_slice(len);
    let req_len = build_post_request(http_req_buf(), url, ANONY_BODY, b"application/json", None)
        .ok_or(FetchError::Network)?;
    let resp_len =
        http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
        return Err(FetchError::Network);
    }
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let out = json_buf();
    let mut c = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            match next {
                b'"' => out[c] = b'"',
                b'\\' => out[c] = b'\\',
                b'/' => out[c] = b'/',
                b'n' => out[c] = b'\n',
                b'r' => out[c] = b'\r',
                b't' => out[c] = b'\t',
                _ => out[c] = next,
            }
            c += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            break;
        }
        if c >= out.len() {
            return Err(FetchError::Network);
        }
        out[c] = b;
        c += 1;
        i += 1;
    }
    let api_json = json_slice(c);
    if response_has_error(api_json) {
        return Err(FetchError::ClientError);
    }
    let user_id = extract_json_number(api_json, b"userId").ok_or(FetchError::Network)?;
    let scheme = extract_json_string(api_json, b"scheme").ok_or(FetchError::Network)?;
    let parameter = extract_json_string(api_json, b"parameter").ok_or(FetchError::Network)?;
    let token_buf = scratch_a();
    let mut tc = 0usize;
    if !(write_bytes(token_buf, &mut tc, scheme)
        && write_bytes(token_buf, &mut tc, b" ")
        && write_bytes(token_buf, &mut tc, parameter))
    {
        return Err(FetchError::Network);
    }
    if !set_auth(user_id, &token_buf[..tc]) {
        return Err(FetchError::Network);
    }
    Ok(())
}

fn ensure_auth() -> Result<(), FetchError> {
    unsafe {
        if AUTH_USER_ID_LEN > 0 && AUTH_TOKEN_LEN > 0 {
            return Ok(());
        }
    }
    fetch_anony_user()
}

fn fetch_json_authed(url: &[u8], auth: &[u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_authed_get_request(http_req_buf(), url, auth).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"manhuaren: authed http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    let resp = &http_out()[..resp_len];
    // Manhuaren API uses {"response":{...}} for success, {"errorResponse":{...}} for error
    if contains_bytes(resp, br#""errorResponse""#) {
        return Err(FetchError::ClientError);
    }
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let out = json_buf();
    let mut c = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            match next {
                b'"' => out[c] = b'"',
                b'\\' => out[c] = b'\\',
                b'/' => out[c] = b'/',
                b'n' => out[c] = b'\n',
                b'r' => out[c] = b'\r',
                b't' => out[c] = b'\t',
                _ => out[c] = next,
            }
            c += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            break;
        }
        if c >= out.len() {
            return Err(FetchError::Network);
        }
        out[c] = b;
        c += 1;
        i += 1;
    }
    Ok(json_slice(c))
}

fn parse_usize(bytes: &[u8]) -> usize {
    let mut n = 0usize;
    for &b in bytes {
        if b >= b'0' && b <= b'9' {
            n = n * 10 + (b - b'0') as usize;
        }
    }
    n
}

fn is_json_ws(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn json_value_start(data: &[u8], key: &[u8]) -> Option<usize> {
    let mut pat = [0u8; 64];
    let need = key.len() + 2;
    if need > pat.len() {
        return None;
    }
    pat[0] = b'"';
    pat[1..1 + key.len()].copy_from_slice(key);
    pat[1 + key.len()] = b'"';

    let mut search_from = 0usize;
    while search_from < data.len() {
        let rel = find_subslice(&data[search_from..], &pat[..need])?;
        let mut i = search_from + rel + need;
        while i < data.len() && is_json_ws(data[i]) {
            i += 1;
        }
        if i < data.len() && data[i] == b':' {
            i += 1;
            while i < data.len() && is_json_ws(data[i]) {
                i += 1;
            }
            return Some(i);
        }
        search_from = search_from + rel + 1;
    }
    None
}

fn extract_json_string<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = json_value_start(data, key)?;
    if i >= data.len() || data[i] != b'"' {
        return None;
    }
    i += 1;
    let start = i;
    while i < data.len() {
        if data[i] == b'\\' {
            i += 2;
            continue;
        }
        if data[i] == b'"' {
            return Some(&data[start..i]);
        }
        i += 1;
    }
    None
}

fn extract_json_number<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = json_value_start(data, key)?;
    let start = i;
    while i < data.len() && data[i] >= b'0' && data[i] <= b'9' {
        i += 1;
    }
    if i == start {
        return None;
    }
    Some(&data[start..i])
}

fn response_has_error(api_json: &[u8]) -> bool {
    contains_bytes(api_json, br#""errorResponse""#)
}

fn status_from_i32(v: usize) -> &'static [u8] {
    match v {
        1 => b"completed",
        0 => b"ongoing",
        _ => b"unknown",
    }
}

fn object_array_iter_start(data: &[u8], key: &[u8]) -> Option<usize> {
    let i = json_value_start(data, key)?;
    if i < data.len() && data[i] == b'[' {
        Some(i + 1)
    } else {
        None
    }
}

struct ObjectArrayIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ObjectArrayIter<'a> {
    fn new(data: &'a [u8], key: &[u8]) -> Option<Self> {
        Some(Self {
            data,
            pos: object_array_iter_start(data, key)?,
        })
    }

    fn next_object(&mut self) -> Option<&'a [u8]> {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' | b',' => self.pos += 1,
                b']' => return None,
                b'{' => break,
                _ => return None,
            }
        }
        let start = self.pos;
        let mut depth = 0i32;
        let mut in_string = false;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if in_string {
                if b == b'\\' {
                    self.pos += 1;
                } else if b == b'"' {
                    in_string = false;
                }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            self.pos += 1;
                            return Some(&self.data[start..self.pos]);
                        }
                    }
                    _ => {}
                }
            }
            self.pos += 1;
        }
        None
    }
}

fn string_array_iter_start(data: &[u8], key: &[u8]) -> Option<usize> {
    object_array_iter_start(data, key)
}

fn write_author_array(payload: &mut [u8], c: &mut usize, data: &[u8], key: &[u8]) -> bool {
    let Some(mut i) = string_array_iter_start(data, key) else {
        return true;
    };
    let mut count = 0usize;
    while i < data.len() {
        while i < data.len()
            && (data[i] == b' ' || data[i] == b',' || data[i] == b'\n' || data[i] == b'\r')
        {
            i += 1;
        }
        if i >= data.len() || data[i] == b']' {
            break;
        }
        if data[i] != b'"' {
            break;
        }
        i += 1;
        let start = i;
        while i < data.len() {
            if data[i] == b'\\' {
                i += 2;
                continue;
            }
            if data[i] == b'"' {
                break;
            }
            i += 1;
        }
        if count > 0 && !write_bytes(payload, c, b",") {
            return false;
        }
        if !(write_bytes(payload, c, b"\"")
            && append_json_unescaped_then_escaped(payload, c, &data[start..i])
            && write_bytes(payload, c, b"\""))
        {
            return false;
        }
        count += 1;
        i += 1;
    }
    true
}

fn write_tags(payload: &mut [u8], c: &mut usize, tags: &[u8]) -> bool {
    let mut i = 0usize;
    let mut count = 0usize;
    while i < tags.len() {
        while i < tags.len()
            && (tags[i] == b' ' || tags[i] == b',' || tags[i] == b'/' || tags[i] == b'|')
        {
            i += 1;
        }
        let start = i;
        while i < tags.len()
            && tags[i] != b' '
            && tags[i] != b','
            && tags[i] != b'/'
            && tags[i] != b'|'
        {
            i += 1;
        }
        if i > start {
            if count > 0 && !write_bytes(payload, c, b",") {
                return false;
            }
            if !(write_bytes(payload, c, b"\"")
                && append_json_unescaped_then_escaped(payload, c, &tags[start..i])
                && write_bytes(payload, c, b"\""))
            {
                return false;
            }
            count += 1;
        }
    }
    true
}

fn write_manga_item(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let id_num = extract_json_number(obj, b"mangaId").unwrap_or(b"0");
    let title = extract_json_string(obj, b"mangaName").unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"mangaCoverimageUrl").unwrap_or(b"");
    let author = extract_json_string(obj, b"mangaAuthor").unwrap_or(b"");
    let theme = extract_json_string(obj, b"mangaTheme").unwrap_or(b"");
    let status_num = extract_json_number(obj, b"mangaIsOver")
        .map(parse_usize)
        .unwrap_or(9);
    write_bytes(payload, c, br#"{"id":"mhr:"#)
        && append_json_escaped(payload, c, id_num)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":"#)
        && if cover.is_empty() {
            write_bytes(payload, c, br#"{"kind":"none"}"#)
        } else {
            write_bytes(payload, c, br#"{"kind":"url","url":""#)
                && append_json_unescaped_then_escaped(payload, c, cover)
                && write_bytes(payload, c, br#""}"#)
        }
        && write_bytes(payload, c, br#","authors":["#)
        && if author.is_empty() {
            true
        } else {
            write_bytes(payload, c, b"\"")
                && append_json_unescaped_then_escaped(payload, c, author)
                && write_bytes(payload, c, b"\"")
        }
        && write_bytes(payload, c, br#"],"status":""#)
        && write_bytes(payload, c, status_from_i32(status_num))
        && write_bytes(
            payload,
            c,
            br#"","contentRating":"safe","sourceTags":["manhuaren""#,
        )
        && if theme.is_empty() {
            true
        } else {
            write_bytes(payload, c, b",") && write_tags(payload, c, theme)
        }
        && write_bytes(payload, c, b"]}")
}

fn write_manga_items_from_array(
    operation: &str,
    api_json: &[u8],
    key_a: &[u8],
    key_b: Option<&[u8]>,
) -> u32 {
    log_info(b"manhuaren api_json start");
    log_info(&api_json[..api_json.len().min(200)]);
    log_info(b"manhuaren api_json end");
    if response_has_error(api_json) {
        return write_error(
            operation,
            "source_error",
            "server returned errorResponse; configure userId/token",
        );
    }
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "overflow");
    }
    let mut iter = ObjectArrayIter::new(api_json, key_a)
        .or_else(|| key_b.and_then(|k| ObjectArrayIter::new(api_json, k)));
    let mut written = 0usize;
    if let Some(ref mut it) = iter {
        while let Some(obj) = it.next_object() {
            if written >= PAGE_SIZE {
                break;
            }
            if written > 0 && !write_bytes(payload, &mut c, b",") {
                break;
            }
            if !write_manga_item(payload, &mut c, obj) {
                break;
            }
            written += 1;
        }
    }
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":""#)
        || !write_usize(payload, &mut c, PAGE_SIZE)
        || !write_bytes(payload, &mut c, br#"","hasMore":"#)
    {
        return write_error(operation, "internal_error", "overflow");
    }
    if written == 0 {
        if !write_bytes(payload, &mut c, b"false}}") {
            return write_error(operation, "internal_error", "overflow");
        }
    } else if !write_bytes(payload, &mut c, b"true}}") {
        return write_error(operation, "internal_error", "overflow");
    }
    write_success_payload(operation, c)
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let page = extract_json_number(req, b"page")
        .map(parse_usize)
        .unwrap_or(1)
        .max(1);
    let start = (page - 1) * PAGE_SIZE;
    let mut start_buf = [0u8; 20];
    let mut limit_buf = [0u8; 20];
    let mut sc = 0usize;
    let mut lc = 0usize;
    write_usize(&mut start_buf, &mut sc, start);
    write_usize(&mut limit_buf, &mut lc, PAGE_SIZE);
    let params = [
        Param {
            key: b"start",
            value: &start_buf[..sc],
        },
        Param {
            key: b"limit",
            value: &limit_buf[..lc],
        },
        Param {
            key: b"keywords",
            value: query,
        },
    ];
    let len = match build_api_url(b"/v1/search/getSearchManga", &params) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "url overflow"),
    };
    let url = url_slice(len);
    let api_json = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    write_manga_items_from_array("search", api_json, b"result", Some(b"mangas"))
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let prefix = b"mhr:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_manga", "invalid_request", "bad mangaId");
    }
    let id = &manga_id[prefix.len()..];
    let params = [Param {
        key: b"mangaId",
        value: id,
    }];
    if let Err(e) = ensure_auth() {
        let (c, m) = fetch_error_code(e);
        return write_error("get_manga", c, m);
    }
    let auth_uid = auth_user_id_slice();
    let auth_token = auth_token_slice();
    let len = match build_api_url_with_user(b"GET", b"/v1/manga/getDetail", &params, auth_uid, None)
    {
        Some(v) => v,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let url = url_slice(len);
    let api_json = match fetch_json_authed(url, auth_token) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    if response_has_error(api_json) {
        return write_error(
            "get_manga",
            "source_error",
            "server returned errorResponse; configure userId/token",
        );
    }

    let title = extract_json_string(api_json, b"mangaName").unwrap_or(b"Unknown");
    let mut cover = extract_json_string(api_json, b"mangaCoverimageUrl").unwrap_or(b"");
    if cover.is_empty() || cover == b"http://mhfm5.tel.cdndm5.com/tag/category/nopic.jpg" {
        cover = extract_json_string(api_json, b"mangaPicimageUrl").unwrap_or(cover);
    }
    if cover.is_empty() {
        cover = extract_json_string(api_json, b"shareIcon").unwrap_or(cover);
    }
    let desc = extract_json_string(api_json, b"mangaIntro").unwrap_or(b"");
    let theme = extract_json_string(api_json, b"mangaTheme").unwrap_or(b"");
    let status_num = extract_json_number(api_json, b"mangaIsOver")
        .map(parse_usize)
        .unwrap_or(9);

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"mhr:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_unescaped_then_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":"#)
        && if cover.is_empty() {
            write_bytes(payload, &mut c, br#"{"kind":"none"}"#)
        } else {
            write_bytes(payload, &mut c, br#"{"kind":"url","url":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, cover)
                && write_bytes(payload, &mut c, br#""}"#)
        }
        && write_bytes(payload, &mut c, br#","authors":["#)
        && if let Some(author) = extract_json_string(api_json, b"mangaAuthor") {
            write_bytes(payload, &mut c, b"\"")
                && append_json_unescaped_then_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_author_array(payload, &mut c, api_json, b"mangaAuthors")
        }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status_from_i32(status_num))
        && write_bytes(
            payload,
            &mut c,
            br#"","contentRating":"safe","language":"zh","tags":["#,
        )
        && write_tags(payload, &mut c, theme)
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, API_BASE)
        && write_bytes(payload, &mut c, b"/v1/manga/getDetail?mangaId=")
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#""},{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, WEB_BASE)
        && write_bytes(payload, &mut c, b"/manhua/")
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "overflow");
    }
    write_success_payload("get_manga", c)
}

fn write_chapters_from_array(
    payload: &mut [u8],
    c: &mut usize,
    api_json: &[u8],
    key: &[u8],
    extra: bool,
    written: &mut usize,
    manga_id: &[u8],
) -> bool {
    let Some(mut iter) = ObjectArrayIter::new(api_json, key) else {
        return true;
    };
    while let Some(obj) = iter.next_object() {
        let ch_id = extract_json_number(obj, b"sectionId").unwrap_or(b"0");
        let name = extract_json_string(obj, b"sectionName").unwrap_or(b"");
        let title = extract_json_string(obj, b"sectionTitle").unwrap_or(b"");
        let sort = extract_json_number(obj, b"sectionSort").unwrap_or(b"0");
        let release_time = extract_json_string(obj, b"releaseTime").unwrap_or(b"");
        let locked = extract_json_number(obj, b"isMustPay")
            .map(parse_usize)
            .unwrap_or(0)
            == 1;
        if *written > 0 && !write_bytes(payload, c, b",") {
            return false;
        }
        if !(write_bytes(payload, c, br#"{"id":""#)
            && append_json_escaped(payload, c, ch_id)
            && write_bytes(payload, c, br#"","mangaId":"mhr:"#)
            && append_json_escaped(payload, c, manga_id)
            && write_bytes(payload, c, br#"","title":""#))
        {
            return false;
        }
        if locked && !write_bytes(payload, c, "(锁) ".as_bytes()) {
            return false;
        }
        if extra && !write_bytes(payload, c, "[番外] ".as_bytes()) {
            return false;
        }
        if !(append_json_unescaped_then_escaped(payload, c, name)
            && if title.is_empty() { true } else {
                write_bytes(payload, c, b": ") && append_json_unescaped_then_escaped(payload, c, title)
            }
            && write_bytes(payload, c, br#"","chapterNumber":""#)
            && append_json_escaped(payload, c, sort)
            && write_bytes(payload, c, br#"","volumeNumber":null,"language":"zh","publishedAt":"#)
            && if release_time.is_empty() {
                write_bytes(payload, c, b"null")
            } else {
                write_bytes(payload, c, b"\"")
                    && append_json_unescaped_then_escaped(payload, c, release_time)
                    && write_bytes(payload, c, b"\"")
            }
            && write_bytes(payload, c, br#","updatedAt":null,"pageCount":null}"#))
        {
            return false;
        }
        *written += 1;
    }
    true
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let prefix = b"mhr:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_chapters", "invalid_request", "bad mangaId");
    }
    let id = &manga_id[prefix.len()..];
    let params = [Param {
        key: b"mangaId",
        value: id,
    }];
    if let Err(e) = ensure_auth() {
        let (c, m) = fetch_error_code(e);
        return write_error("get_chapters", c, m);
    }
    let auth_uid = auth_user_id_slice();
    let auth_token = auth_token_slice();
    let len = match build_api_url_with_user(b"GET", b"/v1/manga/getDetail", &params, auth_uid, None)
    {
        Some(v) => v,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let url = url_slice(len);
    let api_json = match fetch_json_authed(url, auth_token) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    if response_has_error(api_json) {
        return write_error(
            "get_chapters",
            "source_error",
            "server returned errorResponse; configure userId/token",
        );
    }
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    let mut written = 0usize;
    if !(write_chapters_from_array(
        payload,
        &mut c,
        api_json,
        b"mangaEpisode",
        true,
        &mut written,
        id,
    ) && write_chapters_from_array(
        payload,
        &mut c,
        api_json,
        b"mangaWords",
        false,
        &mut written,
        id,
    ) && write_chapters_from_array(
        payload,
        &mut c,
        api_json,
        b"mangaRolls",
        false,
        &mut written,
        id,
    )) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    if chapter_id.is_empty() {
        return write_error("get_pages", "invalid_request", "bad chapterId");
    }
    let legacy_prefix = b"mhrch:";
    let id = if chapter_id.len() > legacy_prefix.len()
        && &chapter_id[..legacy_prefix.len()] == legacy_prefix
    {
        &chapter_id[legacy_prefix.len()..]
    } else {
        chapter_id
    };
    let params = [
        Param {
            key: b"mangaSectionId",
            value: id,
        },
        Param {
            key: b"netType",
            value: b"4",
        },
        Param {
            key: b"loadreal",
            value: b"1",
        },
        Param {
            key: b"imageQuality",
            value: b"2",
        },
    ];
    if let Err(e) = ensure_auth() {
        let (c, m) = fetch_error_code(e);
        return write_error("get_pages", c, m);
    }
    let auth_uid = auth_user_id_slice();
    let auth_token = auth_token_slice();
    let len = match build_api_url_with_user(b"GET", b"/v1/manga/getRead", &params, auth_uid, None) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let url = url_slice(len);
    let api_json = match fetch_json_authed(url, auth_token) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    if response_has_error(api_json) {
        return write_error(
            "get_pages",
            "source_error",
            "server returned errorResponse; configure userId/token",
        );
    }
    let host = first_string_in_array(api_json, b"hostList").unwrap_or(b"");
    let query = extract_json_string(api_json, b"query").unwrap_or(b"");
    let mut i = match string_array_iter_start(api_json, b"mangaSectionImages") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "no images"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "overflow");
    }
    let mut page_idx = 0usize;
    while i < api_json.len() {
        while i < api_json.len()
            && (api_json[i] == b' '
                || api_json[i] == b','
                || api_json[i] == b'\n'
                || api_json[i] == b'\r')
        {
            i += 1;
        }
        if i >= api_json.len() || api_json[i] == b']' {
            break;
        }
        if api_json[i] != b'"' {
            break;
        }
        i += 1;
        let start = i;
        while i < api_json.len() {
            if api_json[i] == b'\\' {
                i += 2;
                continue;
            }
            if api_json[i] == b'"' {
                break;
            }
            i += 1;
        }
        let path = &api_json[start..i];
        i += 1;
        if page_idx > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, host)
            && append_json_unescaped_then_escaped(payload, &mut c, path)
            && append_json_unescaped_then_escaped(payload, &mut c, query)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            break;
        }
        page_idx += 1;
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

fn first_string_in_array<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = string_array_iter_start(data, key)?;
    while i < data.len() && data[i] != b'"' {
        if data[i] == b']' {
            return None;
        }
        i += 1;
    }
    i += 1;
    let start = i;
    while i < data.len() {
        if data[i] == b'\\' {
            i += 2;
            continue;
        }
        if data[i] == b'"' {
            return Some(&data[start..i]);
        }
        i += 1;
    }
    None
}

fn run_category_list(
    operation: &str,
    sort: &[u8],
    sub_type: &[u8],
    sub_id: &[u8],
    page: usize,
) -> u32 {
    let start = (page.max(1) - 1) * PAGE_SIZE;
    let mut start_buf = [0u8; 20];
    let mut limit_buf = [0u8; 20];
    let mut sc = 0usize;
    let mut lc = 0usize;
    write_usize(&mut start_buf, &mut sc, start);
    write_usize(&mut limit_buf, &mut lc, PAGE_SIZE);
    let params = [
        Param {
            key: b"subCategoryType",
            value: sub_type,
        },
        Param {
            key: b"subCategoryId",
            value: sub_id,
        },
        Param {
            key: b"start",
            value: &start_buf[..sc],
        },
        Param {
            key: b"limit",
            value: &limit_buf[..lc],
        },
        Param {
            key: b"sort",
            value: sort,
        },
    ];
    let len = match build_api_url(b"/v2/manga/getCategoryMangas", &params) {
        Some(v) => v,
        None => return write_error(operation, "internal_error", "url overflow"),
    };
    let url = url_slice(len);
    let api_json = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    write_manga_items_from_array(operation, api_json, b"mangas", None)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    run_category_list("get_listings", b"0", b"0", b"0", 1)
}

fn append_home_section(payload: &mut [u8], c: &mut usize, title: &[u8], sort: &[u8]) -> bool {
    if !(write_bytes(payload, c, br#"{"title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","items":["#))
    {
        return false;
    }

    let mut start_buf = [0u8; 20];
    let mut limit_buf = [0u8; 20];
    let mut sc = 0usize;
    let mut lc = 0usize;
    write_usize(&mut start_buf, &mut sc, 0);
    write_usize(&mut limit_buf, &mut lc, 10);
    let params = [
        Param {
            key: b"subCategoryType",
            value: b"0",
        },
        Param {
            key: b"subCategoryId",
            value: b"0",
        },
        Param {
            key: b"start",
            value: &start_buf[..sc],
        },
        Param {
            key: b"limit",
            value: &limit_buf[..lc],
        },
        Param {
            key: b"sort",
            value: sort,
        },
    ];
    let Some(len) = build_api_url(b"/v2/manga/getCategoryMangas", &params) else {
        return false;
    };
    let url = url_slice(len);
    if let Ok(api_json) = fetch_json(url) {
        if let Some(mut iter) = ObjectArrayIter::new(api_json, b"mangas") {
            let mut written = 0usize;
            while let Some(obj) = iter.next_object() {
                if written >= 10 {
                    break;
                }
                if written > 0 && !write_bytes(payload, c, b",") {
                    return false;
                }
                if !write_manga_item(payload, c, obj) {
                    return false;
                }
                written += 1;
            }
        }
    }

    write_bytes(payload, c, b"]}")
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page")
        .map(parse_usize)
        .or_else(|| extract_json_string(req, b"cursor").map(parse_usize))
        .unwrap_or(1)
        .max(1);
    let sort = extract_json_string(req, b"sort").unwrap_or(b"0");
    let sub_type = extract_json_string(req, b"subCategoryType").unwrap_or(b"0");
    let sub_id = extract_json_string(req, b"subCategoryId")
        .unwrap_or_else(|| extract_json_string(req, b"category").unwrap_or(b"0"));
    run_category_list("get_manga_list", sort, sub_type, sub_id, page)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":["#)
        || !append_home_section(payload, &mut c, "热门".as_bytes(), b"0")
        || !write_bytes(payload, &mut c, b",")
        || !append_home_section(payload, &mut c, "更新".as_bytes(), b"1")
        || !write_bytes(payload, &mut c, b"]}")
    {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    const FILTERS_JSON: &str = "{\"filters\":[{\"id\":\"sort\",\"name\":\"状态\",\"kind\":\"select\",\"options\":[{\"value\":\"0\",\"label\":\"热门\"},{\"value\":\"1\",\"label\":\"更新\"},{\"value\":\"2\",\"label\":\"新作\"},{\"value\":\"3\",\"label\":\"完结\"}],\"default\":\"0\"},{\"id\":\"subCategoryId\",\"name\":\"分类\",\"kind\":\"select\",\"options\":[{\"value\":\"0\",\"label\":\"全部\"},{\"value\":\"31\",\"label\":\"热血\"},{\"value\":\"26\",\"label\":\"恋爱\"},{\"value\":\"1\",\"label\":\"校园\"},{\"value\":\"3\",\"label\":\"百合\"},{\"value\":\"27\",\"label\":\"耽美\"},{\"value\":\"2\",\"label\":\"冒险\"},{\"value\":\"17\",\"label\":\"悬疑\"},{\"value\":\"37\",\"label\":\"搞笑\"},{\"value\":\"14\",\"label\":\"奇幻\"},{\"value\":\"29\",\"label\":\"恐怖\"},{\"value\":\"4\",\"label\":\"历史\"},{\"value\":\"34\",\"label\":\"运动\"},{\"value\":\"36\",\"label\":\"绅士\"},{\"value\":\"61\",\"label\":\"限制级\"}],\"default\":\"0\"}]}";
    let bytes = FILTERS_JSON.as_bytes();
    let payload = payload_buf();
    if bytes.len() > payload.len() {
        return write_error("get_filters", "internal_error", "overflow");
    }
    payload[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_filters", bytes.len())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS_JSON: &str = "{\"settings\":[{\"id\":\"userId\",\"name\":\"用户ID\",\"kind\":\"text\",\"default\":\"\"},{\"id\":\"token\",\"name\":\"令牌(Token)\",\"kind\":\"text\",\"default\":\"\"}]}";
    let bytes = SETTINGS_JSON.as_bytes();
    let payload = payload_buf();
    if bytes.len() > payload.len() {
        return write_error("get_settings", "internal_error", "overflow");
    }
    payload[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_settings", bytes.len())
}

fn run_image_request(req: &[u8], operation: &str) -> u32 {
    let url = extract_json_string(req, b"url")
        .or_else(|| extract_json_string(req, b"imageUrl"))
        .unwrap_or(b"");
    if url.is_empty() {
        return write_error(operation, "invalid_request", "missing url");
    }
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"http://www.dm5.com/dm5api/","User-Agent":"Dalvik/2.1.0 (Linux; U; Android 13; Koma Build/Koma)"}}"#);
    if !ok {
        return write_error(operation, "internal_error", "overflow");
    }
    write_success_payload(operation, c)
}

fn hex_encode(input: &[u8], out: &mut [u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut i = 0usize;
    while i < input.len() {
        out[i * 2] = HEX[(input[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(input[i] & 0x0f) as usize];
        i += 1;
    }
}

fn md5(input: &[u8], out: &mut [u8; 16]) {
    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;
    let bit_len = (input.len() as u64) * 8;
    let mut offset = 0usize;
    let mut block = [0u8; 64];
    while offset + 64 <= input.len() {
        md5_block(
            &input[offset..offset + 64],
            &mut a0,
            &mut b0,
            &mut c0,
            &mut d0,
        );
        offset += 64;
    }
    let rem = input.len() - offset;
    let mut i = 0usize;
    while i < rem {
        block[i] = input[offset + i];
        i += 1;
    }
    block[rem] = 0x80;
    if rem >= 56 {
        md5_block(&block, &mut a0, &mut b0, &mut c0, &mut d0);
        block = [0u8; 64];
    }
    block[56..64].copy_from_slice(&bit_len.to_le_bytes());
    md5_block(&block, &mut a0, &mut b0, &mut c0, &mut d0);
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
}

fn md5_block(block: &[u8], a0: &mut u32, b0: &mut u32, c0: &mut u32, d0: &mut u32) {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut m = [0u32; 16];
    let mut i = 0usize;
    while i < 16 {
        let j = i * 4;
        m[i] = u32::from_le_bytes([block[j], block[j + 1], block[j + 2], block[j + 3]]);
        i += 1;
    }
    let mut a = *a0;
    let mut b = *b0;
    let mut c = *c0;
    let mut d = *d0;
    i = 0;
    while i < 64 {
        let (f, g) = if i < 16 {
            ((b & c) | ((!b) & d), i)
        } else if i < 32 {
            ((d & b) | ((!d) & c), (5 * i + 1) % 16)
        } else if i < 48 {
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            (c ^ (b | (!d)), (7 * i) % 16)
        };
        let tmp = d;
        d = c;
        c = b;
        b = b.wrapping_add(
            a.wrapping_add(f)
                .wrapping_add(K[i])
                .wrapping_add(m[g])
                .rotate_left(S[i]),
        );
        a = tmp;
        i += 1;
    }
    *a0 = a0.wrapping_add(a);
    *b0 = b0.wrapping_add(b);
    *c0 = c0.wrapping_add(c);
    *d0 = d0.wrapping_add(d);
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"manhuaren source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("search", "invalid_request", "empty"),
    };
    log_info(b"manhuaren search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_listings", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_manga_list", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_home", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_filters", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_settings", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_modify_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("image_request", "invalid_request", "empty"),
    };
    log_info(b"manhuaren modify_image_request");
    run_image_request(req, "image_request")
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "empty"),
    };
    log_info(b"manhuaren get_image_request");
    run_image_request(req, "get_image_request")
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
