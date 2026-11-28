#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.globalcomix.ru.koma", name: "GlobalComix", version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2", language: "ru", author: "Koma source contributors",
    description: "GlobalComix’s Russian catalogue includes creator releases such as трансорбитальный. Reading is limited to chapters available free without signing in.", content_rating: "nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"ru";
const LANG_JSON: &[u8] = br#""ru""#;
const WORK_PREFIX: &[u8] = b"gc:ru:";
const CHAPTER_PREFIX: &[u8] = b"gcch:ru:";
const PAGE_PREFIX: &[u8] = b"gcp:ru:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_ru";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_ru";
include!("../../globalcomix_common.rs");
