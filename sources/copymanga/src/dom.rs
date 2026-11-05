use super::*;
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
}
