#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.comicskingdom.koma",name:"Comics Kingdom",language:"en",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",author:"Koma source contributors",description:"Official newspaper strips and editorial cartoons, including Blondie and Hägar the Horrible, with daily publication archives.",content_rating:"safe"};
const BASE: &str = "https://comicskingdom.com";
const LANG_BYTES: &[u8] = b"en";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! exports {()=>{koma_source_sdk::koma_source_exports!(@common "comicskingdom");};}
include!("runtime.rs");
include!("protocol.rs");
