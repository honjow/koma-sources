#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.th.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"th",author:"Koma source contributors",description:"GlobalComix’s Thai catalogue includes creator releases such as Dr. Quench. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"th";
const LANG_JSON: &[u8] = br#""th""#;
const WORK_PREFIX: &[u8] = b"gc:th:";
const CHAPTER_PREFIX: &[u8] = b"gcch:th:";
const PAGE_PREFIX: &[u8] = b"gcp:th:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_th";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_th";
include!("../../globalcomix_common.rs");
