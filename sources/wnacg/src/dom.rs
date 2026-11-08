use super::*;
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(d: i32, p: *const u8, n: u32, o: *mut i32, c: u32) -> i32;
}
pub struct Node(host::HtmlDescriptor);
impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
impl Node {
    pub fn parse(b: &[u8]) -> R<Self> {
        host::html_parse(b)
            .map(Self)
            .map_err(|_| "parse_error".into())
    }
    pub fn one(&self, s: &str) -> Option<Self> {
        host::html_select(self.0, s.as_bytes()).ok().map(Self)
    }
    pub fn required(&self, s: &str) -> R<Self> {
        self.one(s).ok_or_else(|| "parse_error".into())
    }
    pub fn whole(&self) -> R<String> {
        let mut b = alloc::vec![0;128*1024];
        let n = host::html_text(self.0, &mut b).map_err(|_| E::from("result_too_large"))?;
        if n >= b.len() {
            return Err("result_too_large".into());
        }
        b.truncate(n);
        String::from_utf8(b).map_err(|_| "parse_error".into())
    }
    pub fn text(&self) -> R<String> {
        Ok(self
            .whole()?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "))
    }
    pub fn attr(&self, k: &str) -> R<Option<String>> {
        let mut b = alloc::vec![0;32768];
        let n = match host::html_attr(self.0, k.as_bytes(), &mut b) {
            Ok(n) => n,
            Err(-1) => return Ok(None),
            Err(_) => return Err("parse_error".into()),
        };
        if n >= b.len() {
            return Err("result_too_large".into());
        }
        b.truncate(n);
        String::from_utf8(b)
            .map(Some)
            .map_err(|_| "parse_error".into())
    }
    pub fn all(&self, s: &str) -> R<Vec<Self>> {
        let mut ids = alloc::vec![0i32;16384];
        let n = unsafe {
            select_all(
                self.0.raw(),
                s.as_ptr(),
                s.len() as u32,
                ids.as_mut_ptr(),
                (ids.len() * 4) as u32,
            )
        };
        if n < 0 {
            return Err("parse_error".into());
        }
        if n as usize >= ids.len() {
            for id in ids.iter().filter(|id| **id > 0) {
                let _ = host::html_close(host::HtmlDescriptor::from_raw(*id));
            }
            return Err("result_too_large".into());
        }
        ids.truncate(n as usize);
        Ok(ids
            .into_iter()
            .map(|id| Self(host::HtmlDescriptor::from_raw(id)))
            .collect())
    }
}
