#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.pl.koma",
    name: "Comic Fury (Polish)",
    language: "pl",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Polish webcomics published by creators on Comic Fury, including Przygody Krysi. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "pl";
const LANG_BYTES: &[u8] = b"pl";
const EXPORT_NAME: &str = "comicfury_pl";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_pl"); };
}
include!("../../comicfury_common.rs");
