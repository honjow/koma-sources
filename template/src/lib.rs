#![no_std]

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_string, write_bytes,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

// ═══════════════════════════════════════════════════════════════════
// Configuration — edit these for your source
// ═══════════════════════════════════════════════════════════════════

const SOURCE_ID: &[u8] = b"com.example.koma";
const SOURCE_NAME: &[u8] = b"Example Source";
const SOURCE_LANG: &[u8] = b"en";
const SOURCE_DESC: &[u8] = b"An example source template.";
const SOURCE_AUTHOR: &[u8] = b"Your Name";
const SOURCE_VERSION: &[u8] = b"0.1.0";
const BASE_URL: &[u8] = b"https://example.com";

// ═══════════════════════════════════════════════════════════════════
// Buffers
// ═══════════════════════════════════════════════════════════════════

static mut RESPONSE_BUF: [u8; 4096] = [0u8; 4096];
static mut PAYLOAD_BUF: [u8; 262144] = [0u8; 262144];

fn response_buffer() -> &'static mut ResultBuffer {
    unsafe { &mut *(RESPONSE_BUF.as_mut_ptr() as *mut ResultBuffer) }
}

fn payload_buf() -> &'static mut [u8] {
    unsafe { &mut PAYLOAD_BUF }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn write_success_payload(operation: &str, payload_len: usize) -> u32 {
    let payload = unsafe { &PAYLOAD_BUF[..payload_len] };
    response_buffer().write_success(operation, payload)
}

fn write_error(operation: &str, code: &str, message: &str) -> u32 {
    response_buffer().write_error(operation, code, message)
}

// ═══════════════════════════════════════════════════════════════════
// Source Info & Init
// ═══════════════════════════════════════════════════════════════════

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    let info = SourceInfo {
        id: SOURCE_ID,
        name: SOURCE_NAME,
        language: SOURCE_LANG,
        version: SOURCE_VERSION,
        api_version: b"0.2",
        description: SOURCE_DESC,
        author: SOURCE_AUTHOR,
        content_rating: b"unknown",
    };
    let caps = SourceCapabilities {
        search: true,
        manga_detail: true,
        chapters: true,
        pages: true,
        listings: false,
        manga_list: false,
        filters: false,
        settings: false,
        home: false,
        image_request: true,
    };
    response_buffer().write_source_info(&info, &caps)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"example source init");
    if manifest_len > 0 { 0 } else { -1 }
}

// ═══════════════════════════════════════════════════════════════════
// Operations — implement these for your source
// ═══════════════════════════════════════════════════════════════════

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("search", "invalid_request", "empty"),
    };
    // TODO: Parse query from request, fetch from your source, build response
    // let query = extract_json_string(req, b"query").unwrap_or(b"");
    write_error("search", "not_implemented", "TODO: implement search")
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    // TODO: Extract mangaId, fetch details, build response
    write_error("get_manga", "not_implemented", "TODO: implement get_manga")
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    // TODO: Extract mangaId, fetch chapter list, build response
    write_error("get_chapters", "not_implemented", "TODO: implement get_chapters")
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    // TODO: Extract chapterId, fetch page URLs, build response
    write_error("get_pages", "not_implemented", "TODO: implement get_pages")
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty"),
    };
    // Passthrough — most sources don't need to modify image URLs
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

// ═══════════════════════════════════════════════════════════════════
// Memory management
// ═══════════════════════════════════════════════════════════════════

#[no_mangle]
pub extern "C" fn koma_source_alloc(size: u32) -> u32 {
    response_buffer().alloc(size)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
