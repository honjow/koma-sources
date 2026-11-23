#![no_std]

const BASE: &str = "https://xiutaku.com";
const IMAGE_HOST: &str = "i.xiutaku.com";
const NAME: &str = "Xiutaku";
const SOURCE_ID: &str = "com.xiutaku.koma";
const KIUTAKU: bool = false;
const CARD_LINK: &[u8] = b".item-content .item-link";

include!("../../taku_gallery.rs");

mod source_metadata {
    use super::*;

    // Only this source's display copy overrides the shared metadata template.
    const SOURCE_INFO: SourceInfo = SourceInfo {
        description: "Xiutaku 的多語言圖集，包含成人內容；各冊保留題材標籤、發布日期與原站分頁編排。",
        ..super::SOURCE_INFO
    };

    koma_source_sdk::koma_source_exports!("xiutaku_source");
}
