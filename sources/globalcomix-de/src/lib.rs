#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.globalcomix.de.koma", name: "GlobalComix", version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2", language: "de", author: "Koma source contributors",
    description: "GlobalComix’s German catalogue includes creator releases such as Gottseibeiuns. Reading is limited to chapters available free without signing in.", content_rating: "nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"de";
const LANG_JSON: &[u8] = br#""de""#;
const WORK_PREFIX: &[u8] = b"gc:de:";
const CHAPTER_PREFIX: &[u8] = b"gcch:de:";
const PAGE_PREFIX: &[u8] = b"gcp:de:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_de";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_de";
include!("../../globalcomix_common.rs");
