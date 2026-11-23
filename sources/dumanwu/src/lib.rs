#![no_std]
extern crate alloc;

use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const SITE: mmlook::Site = mmlook::Site {
    desktop: "http://www.dumanwu1.com",
    mobile: "http://m.dumanwu1.com",
    legacy_work_id: false,
};

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dumanwu.koma",
    name: "读漫屋",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "讀漫屋收錄都市、玄幻、冒險、戀愛及古風等題材漫畫，書庫包含連載與完結作品。",
    content_rating: "safe",
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[path = "../../mmlook.rs"]
mod mmlook;
#[path = "../../port_support.rs"]
mod port_support;

const SOURCE_CAPS: SourceCapabilities = mmlook::CAPABILITIES;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 4 * 1024 * 1024,
    body: 3 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
mmlook::operations!("dumanwu");
