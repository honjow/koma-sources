use super::Result;
use alloc::{string::String, vec, vec::Vec};
use koma_source_sdk::host;

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(node: i32, sel: *const u8, sel_len: u32, out: *mut i32, cap: u32) -> i32;
}

pub struct Node(host::HtmlDescriptor);

impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
impl Node {
    pub fn parse(html: &[u8]) -> Result<Self> {
        host::html_parse(html).map(Self).map_err(|_| "parse_error")
    }
    pub fn one(&self, selector: &str) -> Option<Self> {
        host::html_select(self.0, selector.as_bytes())
            .ok()
            .map(Self)
    }
    pub fn require(&self, selector: &str) -> Result<Self> {
        self.one(selector).ok_or("parse_error")
    }
    pub fn all(&self, selector: &str) -> Result<Vec<Self>> {
        let mut ids = vec![0; 4096];
        let n = unsafe {
            select_all(
                self.0.raw(),
                selector.as_ptr(),
                selector.len() as u32,
                ids.as_mut_ptr(),
                ids.len() as u32,
            )
        };
        if n < 0 || n as usize >= ids.len() {
            return Err("parse_error");
        }
        ids.truncate(n as usize);
        Ok(ids
            .into_iter()
            .map(|id| Self(host::HtmlDescriptor::from_raw(id)))
            .collect())
    }
    pub fn whole(&self) -> Result<String> {
        let mut out = vec![0; 65536];
        let n = host::html_text(self.0, &mut out).map_err(|_| "parse_error")?;
        if n >= out.len() {
            return Err("result_too_large");
        }
        out.truncate(n);
        String::from_utf8(out).map_err(|_| "parse_error")
    }
    pub fn text(&self) -> Result<String> {
        Ok(self
            .whole()?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "))
    }
    pub fn attr(&self, name: &str) -> Result<Option<String>> {
        let mut out = vec![0; 16384];
        let n = match host::html_attr(self.0, name.as_bytes(), &mut out) {
            Ok(n) => n,
            Err(-1) => return Ok(None),
            Err(_) => return Err("parse_error"),
        };
        if n >= out.len() {
            return Err("result_too_large");
        }
        out.truncate(n);
        Ok(Some(String::from_utf8(out).map_err(|_| "parse_error")?))
    }
}
