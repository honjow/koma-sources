#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id: "com.joymiihub.koma", name: "Joymii Hub", language: "all", version: env!("CARGO_PKG_VERSION"), api_version: "0.2", author: "Koma source contributors", description: "Joymii Hub organizes Joymii photo sets by model, including Lily B’s Lonely Passion and Paula S’s Beautiful Jewel. Photos retain the site’s original order; video entries are excluded.", content_rating: "nsfw",
};
const BASE: &str = "https://www.joymiihub.com";
const IMAGE_HOST: &str = "cdn.joymiihub.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source { () => { koma_source_sdk::koma_source_exports!(@common "joymiihub"); }; }
include!("../../masonry_hunter.rs");
