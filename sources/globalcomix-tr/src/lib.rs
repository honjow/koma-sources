#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id:"com.globalcomix.tr.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"tr",author:"Koma source contributors",description:"GlobalComix’s Turkish catalogue includes creator releases such as Boşluk. Reading is limited to chapters available free without signing in.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"tr";
const LANG_JSON: &[u8] = br#""tr""#;
const WORK_PREFIX: &[u8] = b"gc:tr:";
const CHAPTER_PREFIX: &[u8] = b"gcch:tr:";
const PAGE_PREFIX: &[u8] = b"gcp:tr:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_tr";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_tr";
include!("../../globalcomix_common.rs");
