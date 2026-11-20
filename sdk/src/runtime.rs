//! Query the executing host, rather than infer capabilities from an app version.
use crate::{host, source_protocol as p};
pub struct Context<'a> {
    request: &'a [u8],
}
impl<'a> Context<'a> {
    pub fn from_request(request: &'a [u8]) -> Self {
        Self { request }
    }
    pub fn supports(&self, feature: &[u8]) -> bool {
        p::feature(self.request, feature)
    }
    /// Absence means unknown on older hosts; never fabricate a supported limit.
    pub fn limit(&self, group: &[u8], name: &[u8]) -> Option<usize> {
        let caps = p::direct(self.request, b"hostCapabilities").ok()??;
        let group = p::direct(caps, group).ok()??;
        core::str::from_utf8(p::direct(group, name).ok()??)
            .ok()?
            .parse()
            .ok()
    }
    pub fn cancelled(&self) -> bool {
        host::check_cancel()
    }
}

#[link(wasm_import_module = "koma_host")]
unsafe extern "C" {
    #[link_name = "unix_time_ms"]
    fn native_time(out: *mut u8, cap: u32) -> i32;
    #[link_name = "random_bytes"]
    fn native_random(out: *mut u8, cap: u32) -> i32;
}
pub fn unix_time_ms() -> Result<u64, i32> {
    let mut bytes = [0; 8];
    let n = unsafe { native_time(bytes.as_mut_ptr(), 8) };
    if n != 8 {
        Err(n)
    } else {
        Ok(u64::from_le_bytes(bytes))
    }
}
pub fn random_bytes(out: &mut [u8]) -> Result<(), i32> {
    if out.len() > 256 {
        return Err(-1);
    }
    let n = unsafe { native_random(out.as_mut_ptr(), out.len() as u32) };
    if n < 0 || n as usize != out.len() {
        Err(n)
    } else {
        Ok(())
    }
}
