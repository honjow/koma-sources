#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.femjoyhunter.koma",name:"Femjoy Hunter",language:"all",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",author:"Koma source contributors",description:"Femjoy Hunter organizes adult photography by model and studio, including Corinna F – Makeup. Galleries preserve the site’s photo order and exclude video entries.",content_rating:"nsfw"};
const BASE: &str = "https://www.femjoyhunter.com";
const IMAGE_HOST: &str = "cdn.femjoyhunter.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source{()=>{koma_source_sdk::koma_source_exports!(@common "femjoyhunter");};}
include!("../../masonry_hunter.rs");
