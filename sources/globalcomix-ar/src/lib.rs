#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.ar.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"ar",author:"Koma source contributors",description:"GlobalComix’s Arabic catalogue includes creator releases such as \u{2068}خطوة واحدة نحوي\u{2069}. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"ar";
const LANG_JSON: &[u8] = br#""ar""#;
const WORK_PREFIX: &[u8] = b"gc:ar:";
const CHAPTER_PREFIX: &[u8] = b"gcch:ar:";
const PAGE_PREFIX: &[u8] = b"gcp:ar:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_ar";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_ar";
include!("../../globalcomix_common.rs");
