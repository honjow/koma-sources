#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.vi.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"vi",author:"Koma source contributors",description:"GlobalComix’s Vietnamese catalogue includes creator releases such as HAPPY the Series. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"vi";
const LANG_JSON: &[u8] = br#""vi""#;
const WORK_PREFIX: &[u8] = b"gc:vi:";
const CHAPTER_PREFIX: &[u8] = b"gcch:vi:";
const PAGE_PREFIX: &[u8] = b"gcp:vi:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_vi";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_vi";
include!("../../globalcomix_common.rs");
