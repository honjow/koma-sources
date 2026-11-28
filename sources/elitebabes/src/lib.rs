#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.elitebabes.koma",
    name: "Elite Babes",
    language: "all",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    author: "Koma source contributors",
    description: "Elite Babes brings together model photo sets from studios such as Photodromm and StasyQ, including Margaret’s Perfect Body Lines and Lissa Key’s Dreamlike GF. Photos retain their original order; video entries are excluded.",
    content_rating: "nsfw",
};
const BASE: &str = "https://www.elitebabes.com";
const IMAGE_HOST: &str = "cdn.elitebabes.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "elitebabes"); };
}
include!("../../masonry_hunter.rs");
