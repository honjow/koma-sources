#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.ru.koma",
    name: "Comic Fury (Russian)",
    language: "ru",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Russian webcomics published by creators on Comic Fury, including Изгнанный. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "ru";
const LANG_BYTES: &[u8] = b"ru";
const EXPORT_NAME: &str = "comicfury_ru";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_ru"); };
}
include!("../../comicfury_common.rs");
