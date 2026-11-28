#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xarthunter.koma",
    name: "XArt Hunter",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    author: "Koma source contributors",
    description: "XArt Hunter collects X-Art photo sets such as Nikki Hill’s Darling Nikki and Isabella’s Sweet Memories. It presents still-photo galleries and excludes the site’s video entries.",
    content_rating: "nsfw",
};
const BASE: &str = "https://www.xarthunter.com";
const IMAGE_HOST: &str = "cdn.xarthunter.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "xarthunter"); };
}
include!("../../masonry_hunter.rs");
