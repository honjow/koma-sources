#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.globalcomix.id.koma", name: "GlobalComix", version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2", language: "id", author: "Koma source contributors",
    description: "GlobalComix’s Indonesian catalogue includes creator releases such as in/habitants. Reading is limited to chapters available free without signing in.", content_rating: "nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"id";
const LANG_JSON: &[u8] = br#""id""#;
const WORK_PREFIX: &[u8] = b"gc:id:";
const CHAPTER_PREFIX: &[u8] = b"gcch:id:";
const PAGE_PREFIX: &[u8] = b"gcp:id:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_id";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_id";
include!("../../globalcomix_common.rs");
