#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo {
 id:"com.globalcomix.es.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"es",author:"Koma source contributors",description:"GlobalComix’s Spanish catalogue includes creator releases such as DBS: BLACK F. Read chapters offered free without signing in; subscription and purchase restrictions still apply.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"es";
const LANG_JSON: &[u8] = br#""es""#;
const WORK_PREFIX: &[u8] = b"gc:es:";
const CHAPTER_PREFIX: &[u8] = b"gcch:es:";
const PAGE_PREFIX: &[u8] = b"gcp:es:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_es";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_es";
include!("../../globalcomix_common.rs");
