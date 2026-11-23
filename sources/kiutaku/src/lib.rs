#![no_std]

const BASE: &str = "https://kiutaku.com";
const IMAGE_HOST: &str = "mitaku.net";
const NAME: &str = "Kiutaku";
const SOURCE_ID: &str = "com.kiutaku.koma";
const KIUTAKU: bool = true;
const CARD_LINK: &[u8] = b"a.item-link";

include!("../../taku_gallery.rs");

mod source_metadata {
    use super::*;

    // Only this source's display copy overrides the shared metadata template.
    const SOURCE_INFO: SourceInfo = SourceInfo {
        description: "Kiutaku 的多語言圖集，包含成人內容；每冊按原站頁次編排，標籤與相關圖集可協助尋找相近作品。",
        ..super::SOURCE_INFO
    };

    koma_source_sdk::koma_source_exports!("kiutaku_source");
}
