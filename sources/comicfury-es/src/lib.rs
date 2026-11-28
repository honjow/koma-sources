#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.es.koma",
    name: "Comic Fury (Spanish)",
    language: "es",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Spanish webcomics self-published by creators on Comic Fury, including Los Cinco Reinos. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "es";
const LANG_BYTES: &[u8] = b"es";
const EXPORT_NAME: &str = "comicfury_es";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_es"); };
}
include!("../../comicfury_common.rs");
