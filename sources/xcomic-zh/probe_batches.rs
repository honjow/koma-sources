//! Fixed949dea resolveTargetComics chunks independent probes by five.
//! This is transport scheduling only: no merged GraphQL, cache or weaker owner checks.
use super::*;
pub const MAX_REQUESTS: usize = 5;
pub fn supported(req: &[u8]) -> bool {
    let Some(raw) = json_object_member(req, b"hostFeatures") else { return false };
    raw.starts_with(b"[") && raw.ends_with(b"]") && raw[1..raw.len()-1].split(|b| *b == b',')
        .any(|v| v.trim_ascii() == br#""source_http_batch_v1""#)
}
pub fn collect(ids: &[Identity], good: &mut usize) -> Result<(), E> {
    if ids.is_empty() || ids.len() > MAX_REQUESTS { return Err("invalid probe batch") }
    if host::check_cancel() { return Err("cancelled") }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, br#"{"version":1,"requests":["#)?;
    for (i, cid) in ids.iter().enumerate() {
        if i > 0 { bytes(b, &mut n, b",")?; }
        let vars = vars_id(cid.as_bytes())?;
        unsafe {
            let mut used = 0;
            bytes(&mut POST_BODY, &mut used, br#"{"query":"#)?;
            q(&mut POST_BODY, &mut used, COMIC_PROBE_QUERY)?;
            bytes(&mut POST_BODY, &mut used, br#","variables":"#)?;
            bytes(&mut POST_BODY, &mut used, vars)?;
            bytes(&mut POST_BODY, &mut used, b"}")?;
            append_http_request(b, &mut n, &POST_BODY[..used])?;
        }
    }
    bytes(b, &mut n, b"]}")?;
    let len = host::http_request(&b[..n], http_out()).map_err(|_| "network failure")?;
    if len == 0 || len > 2 * MAX { return Err("HTTP response capacity exceeded") }
    if host::check_cancel() { return Err("cancelled") }
    let response = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT) as *const u8, len) };
    valid_json(response)?;
    if member(response, b"ok")? != Some(b"true".as_slice()) { return Err("network failure") }
    let mut responses = Array::new(required(response, b"responses")?)?;
    for cid in ids {
        if host::check_cancel() { return Err("cancelled") }
        let child = responses.next()?.ok_or("probe response count mismatch")?;
        let data = decode_http_response(child)?;
        accept_probe(*cid, data, good)?;
    }
    if responses.next()?.is_some() { return Err("probe response count mismatch") }
    Ok(())
}
