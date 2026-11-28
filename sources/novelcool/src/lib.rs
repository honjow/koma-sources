#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.novelcool.koma",name:"NovelCool",language:"en",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",author:"Koma source contributors",description:"NovelCool’s English image-comic catalog, including One Piece. This source excludes prose novels and adult categories and retains the original reader pagination.",content_rating:"safe"};
const BASE: &str = "https://www.novelcool.com";
const LANG_BYTES: &[u8] = b"en";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! exports {()=>{koma_source_sdk::koma_source_exports!(@common "novelcool");};}
include!("runtime.rs");
include!("protocol.rs");
