#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.olympusscanlation.koma",name:"Olympus Scanlation",language:"es",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",author:"Koma source contributors",description:"Spanish-language comics from Olympus Scanlation, including the fantasy serial El Blinker genio de la academia mágica, with the site’s genre and publication-status catalog. Prose novels are excluded.",content_rating:"safe"};
const BASE: &str = "https://olympusxyz.com";
const LANG_BYTES: &[u8] = b"es";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! exports {()=>{koma_source_sdk::koma_source_exports!(@common "olympusscanlation");};}
include!("runtime.rs");
include!("protocol.rs");
