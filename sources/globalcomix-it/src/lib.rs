#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo {
 id:"com.globalcomix.it.koma",name:"GlobalComix",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
 language:"it",author:"Koma source contributors",description:"GlobalComix’s Italian catalogue includes creator releases such as Whisper ITA. Read chapters offered free without signing in; subscription and purchase restrictions still apply.",content_rating:"nsfw",
};
const METADATA: &[u8] = include_bytes!("../metadata.json");
const LANG: &[u8] = b"it";
const LANG_JSON: &[u8] = br#""it""#;
const WORK_PREFIX: &[u8] = b"gc:it:";
const CHAPTER_PREFIX: &[u8] = b"gcch:it:";
const PAGE_PREFIX: &[u8] = b"gcp:it:";
const DATA_SAVER_KEY: &[u8] = b"dataSaver_it";
const SHOW_LOCKED_KEY: &[u8] = b"showLockedChapters_it";
include!("../../globalcomix_common.rs");
