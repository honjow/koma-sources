#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.pl.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"pl",author:"Koma source contributors",description:"GlobalComix’s Polish catalogue includes creator releases such as Transorbital. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"pl";
const LANG_JSON: &[u8] = br#""pl""#;
const WORK_PREFIX: &[u8] = b"gc:pl:";
const CHAPTER_PREFIX: &[u8] = b"gcch:pl:";
const PAGE_PREFIX: &[u8] = b"gcp:pl:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_pl";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_pl";
include!("../../globalcomix_common.rs");
