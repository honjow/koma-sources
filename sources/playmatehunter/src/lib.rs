#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.playmatehunter.koma",
    name: "Playmate Hunter",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    author: "Koma source contributors",
    description: "Playmate Hunter organizes Playboy photo sets by model, including Hunter McCloud’s Blissful Time and Katya Clover’s White Lace. Photo galleries retain the site’s image order and exclude video entries.",
    content_rating: "nsfw",
};
const BASE: &str = "https://pmatehunter.com";
const IMAGE_HOST: &str = "cdn.pmatehunter.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "playmatehunter"); };
}
include!("../../masonry_hunter.rs");
