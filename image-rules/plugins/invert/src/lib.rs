//! Demonstration processor, not published in the default index.
use koma_image_sdk as host;
#[no_mangle] pub extern "C" fn koma_image_processor_abi() -> i32 { 1 }
#[no_mangle] pub extern "C" fn koma_image_process() -> i32 { run().unwrap_or(-1) }
fn run() -> Result<i32,()> {
    let config: serde_json::Value = serde_json::from_slice(&host::config()?).map_err(|_| ())?;
    let amount = config.pointer("/settings/amount").and_then(serde_json::Value::as_f64).unwrap_or(100.0);
    if !amount.is_finite() || !(0.0..=100.0).contains(&amount) { return Err(()); }
    if amount == 0.0 { return Ok(0); }
    let (w,h) = unsafe { (host::width(),host::height()) }; if w <= 0 || h <= 0 { return Err(()); }
    host::create(w,h)?; let mut row = vec![0; (w as usize).checked_mul(4).ok_or(())?];
    for y in 0..h { if host::cancelled(){return Err(());} host::region(0,y,w,1,&mut row)?; for p in row.chunks_exact_mut(4) { for c in &mut p[..3] { *c=((*c as f64 * (100.0-amount) + (255-*c) as f64 * amount) / 100.0).round() as u8; } } host::write(y,w,1,&row)?; } Ok(1)
}
