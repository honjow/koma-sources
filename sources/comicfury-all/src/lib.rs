#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.all.koma",
    name: "Comic Fury (All Languages)",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Creator comics across Comic Fury’s language sections, including Golden Shrike. This catalogue is not limited to English and preserves original content and author notes.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "all";
const LANG_BYTES: &[u8] = b"all";
const EXPORT_NAME: &str = "comicfury_all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_all"); };
}
include!("../../comicfury_common.rs");
