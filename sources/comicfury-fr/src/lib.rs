#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.fr.koma",
    name: "Comic Fury (French)",
    language: "fr",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "French webcomics self-published by creators on Comic Fury, including Quotidien Absurde. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "fr";
const LANG_BYTES: &[u8] = b"fr";
const EXPORT_NAME: &str = "comicfury_fr";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_fr"); };
}
include!("../../comicfury_common.rs");
