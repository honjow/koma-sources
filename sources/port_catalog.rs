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
