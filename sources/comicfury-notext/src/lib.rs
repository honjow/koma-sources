#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.notext.koma",
    name: "Comic Fury (No Text)",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Wordless comics in Comic Fury’s No Text section, including Chamerion. Stories are told through images; covers, descriptions or author notes may still contain text.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "all";
const LANG_BYTES: &[u8] = b"all";
const EXPORT_NAME: &str = "comicfury_notext";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_notext"); };
}
include!("../../comicfury_common.rs");
