#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id: "com.ftvhunter.koma", name: "FTV Hunter", language: "all", version: env!("CARGO_PKG_VERSION"), api_version: "0.2", author: "Koma source contributors", description: "FTV Hunter focuses on FTV Girls photo sets, including Lara’s Shy First Timer and Cecee’s Kitchen Duty. Galleries are organized by model; video entries are excluded.", content_rating: "nsfw",
};
const BASE: &str = "https://www.ftvhunter.com";
const IMAGE_HOST: &str = "cdn.ftvhunter.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source { () => { koma_source_sdk::koma_source_exports!(@common "ftvhunter"); }; }
include!("../../masonry_hunter.rs");
