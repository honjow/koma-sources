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

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.koma",
    name: "Example Source",
    language: "en",
    version: "0.1.0",
    api_version: "0.2",
    description: "An example source template.",
    author: "Your Name",
    content_rating: "unknown",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
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

// ═══════════════════════════════════════════════════════════════════
// Buffers — adjust sizes based on your source's needs
// ═══════════════════════════════════════════════════════════════════

const PAYLOAD_CAP: usize = 128 * 1024;
const HTTP_OUT_CAP: usize = 512 * 1024;

static mut RESPONSE: ResultBuffer<{ PAYLOAD_CAP + 256 }> = ResultBuffer::new();
static mut PAYLOAD_BUF: [u8; PAYLOAD_CAP] = [0; PAYLOAD_CAP];
static mut HTTP_OUT: [u8; HTTP_OUT_CAP] = [0; HTTP_OUT_CAP];

// ═══════════════════════════════════════════════════════════════════
// Panic handler (required for no_std)
// ═══════════════════════════════════════════════════════════════════

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn response_buffer() -> &'static mut ResultBuffer<{ PAYLOAD_CAP + 256 }> {
    unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) }
}

fn payload_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(PAYLOAD_BUF) }
}

fn http_out() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(HTTP_OUT) }
}

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
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
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
    // TODO: Parse query, fetch from source, build items array
    // let query = extract_json_string(req, b"query").unwrap_or(b"");
    // let body = http_request(url, http_out());
    write_error("search", "not_implemented", "TODO")
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    write_error("get_manga", "not_implemented", "TODO")
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    write_error("get_chapters", "not_implemented", "TODO")
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    write_error("get_pages", "not_implemented", "TODO")
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
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
