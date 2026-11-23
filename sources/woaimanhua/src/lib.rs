#![no_std]
extern crate alloc;

use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

#[path = "../../mobile_book_site.rs"]
mod book_site;
#[path = "../../port_support.rs"]
mod port_support;

const SITE: book_site::Site = book_site::Site {
    mirrors: &["https://www.woimh.com"],
    work_prefix: "/book/",
    kind: book_site::Kind::Woai,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.woaimanhua.koma",
    name: "我爱漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "我愛漫畫收錄國漫、日漫、韓漫及歐美漫畫，包含戀愛、校園、古風、奇幻等題材。付費章節保留鎖定狀態。",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = book_site::CAPABILITIES;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload:1024*1024, http_out:4*1024*1024, body:3*1024*1024, http_req:4096, scratch:8192,
}
koma_source_sdk::koma_source_helpers!();
book_site::operations!("woaimanhua");
