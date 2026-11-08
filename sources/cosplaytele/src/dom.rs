use super::*;
pub(super) type R<T> = Result<T, &'static str>;
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn all(d: i32, s: *const u8, n: u32, o: *mut i32, c: u32) -> i32;
}
pub(super) struct Node(host::HtmlDescriptor);
impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
impl Node {
    pub(super) fn parse(b: &[u8]) -> R<Self> {
        host::html_parse(b).map(Self).map_err(|_| "parse_error")
    }
    pub(super) fn select(&self, s: &[u8]) -> R<Self> {
        host::html_select(self.0, s)
            .map(Self)
            .map_err(|_| "parse_error")
    }
    pub(super) fn text<'a>(&self, b: &'a mut [u8]) -> R<&'a [u8]> {
        let n = host::html_text(self.0, b).map_err(|_| "capacity_exceeded")?;
        if n >= b.len() {
            return Err("capacity_exceeded");
        };
        Ok(trim_ascii(&b[..n]))
    }
    pub(super) fn attr<'a>(&self, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
        let n = host::html_attr(self.0, k, b).map_err(|_| "parse_error")?;
        if n >= b.len() {
            return Err("capacity_exceeded");
        };
        Ok(&b[..n])
    }
}
pub(super) struct Nodes {
    ids: [i32; 8192],
    pub(super) n: usize,
}
impl Nodes {
    pub(super) fn get(d: &Node, s: &[u8]) -> R<Self> {
        let mut a = Self {
            ids: [0; 8192],
            n: 0,
        };
        let n = unsafe {
            all(
                d.0.raw(),
                s.as_ptr(),
                s.len() as u32,
                a.ids.as_mut_ptr(),
                (a.ids.len() * 4) as u32,
            )
        };
        if n < 0 || n as usize >= a.ids.len() {
            return Err("capacity_exceeded");
        };
        a.n = n as usize;
        Ok(a)
    }
    pub(super) fn at(&self, i: usize) -> core::mem::ManuallyDrop<Node> {
        core::mem::ManuallyDrop::new(Node(host::HtmlDescriptor::from_raw(self.ids[i])))
    }
}
impl Drop for Nodes {
    fn drop(&mut self) {
        for id in &self.ids[..self.n] {
            let _ = host::html_close(host::HtmlDescriptor::from_raw(*id));
        }
    }
}
pub(super) fn w(o: &mut [u8], n: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(o, n, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
pub(super) fn q(o: &mut [u8], n: &mut usize, v: &[u8]) -> R<()> {
    w(o, n, b"\"")?;
    if !append_json_escaped(o, n, v) {
        return Err("capacity_exceeded");
    };
    w(o, n, b"\"")
}
pub(super) fn num(o: &mut [u8], n: &mut usize, v: usize) -> R<()> {
    if write_usize(o, n, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
pub(super) fn args(r: &[u8]) -> &[u8] {
    json_object_member(r, b"args").unwrap_or(r)
}
pub(super) fn string<'a>(r: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    protocol::raw_string(r, k)?.ok_or("invalid_request")
}
