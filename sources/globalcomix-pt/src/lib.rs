#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo {
 id:"com.globalcomix.pt.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"pt",author:"Koma source contributors",description:"GlobalComix’s Portuguese catalogue includes creator releases such as Transorbital and is separate from the site’s Brazilian Portuguese section. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"pt";
const LANG_JSON: &[u8] = br#""pt""#;
const WORK_PREFIX: &[u8] = b"gc:pt:";
const CHAPTER_PREFIX: &[u8] = b"gcch:pt:";
const PAGE_PREFIX: &[u8] = b"gcp:pt:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_pt";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_pt";
include!("../../globalcomix_common.rs");
