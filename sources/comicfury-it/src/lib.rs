#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.it.koma",
    name: "Comic Fury (Italian)",
    language: "it",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Italian webcomics published by creators on Comic Fury, including Tangled River-  versione italiana. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "it";
const LANG_BYTES: &[u8] = b"it";
const EXPORT_NAME: &str = "comicfury_it";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_it"); };
}
include!("../../comicfury_common.rs");
