#![no_std]
extern crate alloc;

use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

#[path = "../../mobile_book_site.rs"]
mod book_site;
#[path = "../../port_support.rs"]
mod port_support;

const SITE: book_site::Site = book_site::Site {
    mirrors: &[
        "https://hxy4.com",
        "https://qimeitj.com",
        "https://hxy10.com",
    ],
    work_prefix: "/manhua/",
    kind: book_site::Kind::Buding,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.budingmanhua.koma",
    name: "布丁漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "布丁漫画。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = book_site::CAPABILITIES;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload:1024*1024, http_out:4*1024*1024, body:3*1024*1024, http_req:4096, scratch:8192,
}
koma_source_sdk::koma_source_helpers!();
book_site::operations!("budingmanhua");
