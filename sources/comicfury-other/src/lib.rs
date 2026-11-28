#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.other.koma",
    name: "Comic Fury (Other)",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Comic Fury’s Other-language catalogue includes Louhi, alongside works filed outside the named language sections. Some entries include author notes.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "all";
const LANG_BYTES: &[u8] = b"all";
const EXPORT_NAME: &str = "comicfury_other";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_other"); };
}
include!("../../comicfury_common.rs");
