#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.ja.koma",
    name: "Comic Fury (Japanese)",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Japanese webcomics self-published by creators on Comic Fury, including ササフラス・ロード. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "ja";
const LANG_BYTES: &[u8] = b"ja";
const EXPORT_NAME: &str = "comicfury_ja";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_ja"); };
}
include!("../../comicfury_common.rs");
