#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.pt-br.koma",
    name: "Comic Fury (Brazilian Portuguese)",
    language: "pt-BR",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "Brazilian Portuguese webcomics published by creators on Comic Fury, including Puara Vereda. Some comics include notes posted by their authors alongside the originals.",
    author: "Koma source contributors",
    content_rating: "unknown",
};
const LANG: &str = "pt-BR";
const LANG_BYTES: &[u8] = b"pt-BR";
const EXPORT_NAME: &str = "comicfury_pt_br";
const METADATA: &[u8] = include_bytes!("../metadata.json");
macro_rules! export_source {
    () => { koma_source_sdk::koma_source_exports!(@common "comicfury_pt_br"); };
}
include!("../../comicfury_common.rs");
