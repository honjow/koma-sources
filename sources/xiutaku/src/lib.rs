#![no_std]

const BASE: &str = "https://xiutaku.com";
const IMAGE_HOST: &str = "i.xiutaku.com";
const NAME: &str = "Xiutaku";
const SOURCE_ID: &str = "com.xiutaku.koma";
const KIUTAKU: bool = false;
const CARD_LINK: &[u8] = b".item-content .item-link";

include!("../../taku_gallery.rs");

koma_source_sdk::koma_source_exports!("xiutaku_source");
