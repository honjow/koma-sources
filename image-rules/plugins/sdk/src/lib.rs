//! Koma image processor ABI v1: packed RGBA8888 regions and sequential output rows.
#[link(wasm_import_module = "koma_image")]
extern "C" {
    pub fn width() -> i32;
    pub fn height() -> i32;
    pub fn config_len() -> i32;
    fn config_read(ptr: *mut u8, cap: i32) -> i32;
    fn read_region(x: i32, y: i32, w: i32, h: i32, ptr: *mut u8, cap: i32) -> i32;
    fn output_create(w: i32, h: i32) -> i32;
    fn output_write(x: i32, y: i32, w: i32, h: i32, ptr: *const u8, len: i32) -> i32;
    pub fn check_cancel() -> i32;
}
pub fn config() -> Result<Vec<u8>, ()> {
    let n = unsafe { config_len() }; if !(0..=4*1024*1024).contains(&n) { return Err(()); }
    let mut b = vec![0; n as usize]; if unsafe { config_read(b.as_mut_ptr(), n) } != n { return Err(()); } Ok(b)
}
pub fn region(x:i32,y:i32,w:i32,h:i32,b:&mut [u8]) -> Result<(),()> {
    let n = w.checked_mul(h).and_then(|v|v.checked_mul(4)).ok_or(())?;
    if n < 0 || b.len() != n as usize || unsafe { read_region(x,y,w,h,b.as_mut_ptr(),n) } != n { return Err(()); } Ok(())
}
pub fn create(w:i32,h:i32) -> Result<(),()> { if unsafe { output_create(w,h) } == 0 { Ok(()) } else { Err(()) } }
pub fn write(y:i32,w:i32,h:i32,b:&[u8]) -> Result<(),()> {
    let n = w.checked_mul(h).and_then(|v|v.checked_mul(4)).ok_or(())?;
    if n < 0 || b.len() != n as usize || unsafe { output_write(0,y,w,h,b.as_ptr(),n) } != n { Err(()) } else { Ok(()) }
}
pub fn cancelled() -> bool { unsafe { check_cancel() != 0 } }
