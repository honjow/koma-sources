use alloc::string::String;
use koma_source_sdk::{localization::Catalog, runtime::Context};

use crate::port::Result;

pub(crate) struct Labels<'a> {
    catalog: Catalog<'a>,
    context: Context<'a>,
}

impl<'a> Labels<'a> {
    pub(crate) fn new(metadata: &'a [u8], request: &'a [u8]) -> Result<Self> {
        Ok(Self {
            catalog: Catalog::from_metadata(metadata).map_err(|_| "internal_error")?,
            context: Context::from_request(request),
        })
    }

    pub(crate) fn text(&self, key: &str, default: &str) -> Result<String> {
        let mut out = [0; 1024];
        self.catalog
            .text(&self.context, key, default, &mut out)
            .map(String::from)
            .map_err(|_| "internal_error")
    }
}

// Translate source-owned operational feedback without changing the stable ABI code.
#[allow(dead_code)]
pub(crate) fn failure(metadata: &[u8], request: &[u8], code: &str) -> String {
    let key = match code {
        "chapter_locked" => "error.locked",
        "chapter_unavailable" | "page_unavailable" | "not_found" => "error.unavailable",
        "forbidden" | "rate_limit" | "network_error" | "upstream_error" => "error.network",
        "host_feature_required" => "error.host",
        "unsupported_scramble" => "error.scramble",
        "catalog_changed" => "error.catalog",
        "unknown_filter" | "invalid_request" => "error.filter",
        _ => "error.generic",
    };
    Labels::new(metadata, request)
        .and_then(|labels| labels.text(key, "Unable to load this content."))
        .unwrap_or_else(|_| String::from("Unable to load this content."))
}

#[allow(dead_code)]
pub(crate) fn write_localized(
    metadata: &[u8],
    request: &[u8],
    operation: &str,
    result: Result<serde_json::Value>,
) -> u32 {
    match result {
        Ok(value) => crate::port::write_result(operation, Ok(value)),
        Err(code) => super::write_error(operation, code, &failure(metadata, request, code)),
    }
}
