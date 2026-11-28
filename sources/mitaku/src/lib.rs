#![no_std]
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.mitaku.koma",name:"Mitaku",language:"all",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",author:"Koma source contributors",description:"Mitaku hosts cosplay galleries such as Hana Bunny – Bela Dimitrescu, organized by characters, models and tags. Mixed posts retain photos; video-only posts are excluded.",content_rating:"nsfw"};
const BASE: &str = "https://mitaku.net";
const IMAGE_HOST: &str = "mitaku.net";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = false;
const POPULAR_KEY: &str = "listing.cosplay";
macro_rules! export_source{()=>{koma_source_sdk::koma_source_exports!(@common "mitaku");};}
include!("protocol.rs");
