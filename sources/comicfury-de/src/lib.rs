#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.de.koma",
    name: "Comic Fury (German)",
    language: "de",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "German webcomics published by creators on Comic Fury, including [DE] Pokemon Mystery Dungeon: Clockwork Journeys. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "de";
const LANG_BYTES: &[u8] = b"de";
const EXPORT_NAME: &str = "comicfury_de";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_de"); };
}
include!("../../comicfury_common.rs");
