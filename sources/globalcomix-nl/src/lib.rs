#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.nl.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"nl",author:"Koma source contributors",description:"GlobalComix’s Dutch catalogue includes creator releases such as Jolene postermagazines. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"nl";
const LANG_JSON: &[u8] = br#""nl""#;
const WORK_PREFIX: &[u8] = b"gc:nl:";
const CHAPTER_PREFIX: &[u8] = b"gcch:nl:";
const PAGE_PREFIX: &[u8] = b"gcp:nl:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_nl";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_nl";
include!("../../globalcomix_common.rs");
