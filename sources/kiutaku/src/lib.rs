#![no_std]

const BASE: &str = "https://kiutaku.com";
const IMAGE_HOST: &str = "mitaku.net";
const NAME: &str = "Kiutaku";
const SOURCE_ID: &str = "com.kiutaku.koma";
const KIUTAKU: bool = true;
const CARD_LINK: &[u8] = b"a.item-link";

include!("../../taku_gallery.rs");

koma_source_sdk::koma_source_exports!("kiutaku_source");
