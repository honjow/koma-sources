use koma_source_sdk::{localization::Catalog, runtime::Context};
static mut REQUEST: [u8; 65536] = [0; 65536];
static mut LEN: usize = 0;
// Only the current invocation is retained for typed chapter error localization.
// Every exported operation resets this buffer; no host state or network behavior changes.
pub(super) fn remember(r: &[u8]) {
    unsafe {
        LEN = if r.len() <= 65536 { r.len() } else { 0 };
        REQUEST[..LEN].copy_from_slice(&r[..LEN]);
    }
}
pub(super) fn error(op: &str, code: &str, _: &str) -> u32 {
    let key = match code {
        "invalid_request" => "error.filter",
        "permission_denied" | "auth_required" => "error.auth",
        "network_error" | "timeout" | "rate_limit" | "client_error" | "server_error" => {
            "error.network"
        }
        "chapter_locked" => "error.locked",
        "host_feature_required" => "error.host",
        "not_found" => "error.unavailable",
        "cancelled" => "error.cancelled",
        "not_implemented" | "unsupported" => "error.unsupported",
        _ => "error.generic",
    };
    let mut b = [0; 1024];
    let request = unsafe { &REQUEST[..LEN] };
    let value = Catalog::from_metadata(include_bytes!("../metadata.json"))
        .ok()
        .and_then(|c| {
            c.text(
                &Context::from_request(request),
                key,
                "Unable to load this content.",
                &mut b,
            )
            .ok()
        });
    super::write_error(op, code, value.unwrap_or("Unable to load this content."))
}
pub(super) fn write(
    r: &[u8],
    out: &mut [u8],
    n: &mut usize,
    key: &str,
) -> Result<(), &'static str> {
    let mut b = [0; 1024];
    let c = Catalog::from_metadata(include_bytes!("../metadata.json"))
        .map_err(|_| "invalid localization metadata")?;
    let value = c
        .text(&Context::from_request(r), key, key, &mut b)
        .map_err(|_| "localization output failed")?;
    super::q(out, n, value.as_bytes())
}
pub(super) fn raw(r: &[u8], out: &mut [u8], n: &mut usize, key: &str) -> Result<(), &'static str> {
    let mut b = [0; 1024];
    let c = Catalog::from_metadata(include_bytes!("../metadata.json"))
        .map_err(|_| "invalid localization metadata")?;
    let value = c
        .text(&Context::from_request(r), key, key, &mut b)
        .map_err(|_| "localization output failed")?;
    if koma_source_sdk::json_utils::append_json_escaped(out, n, value.as_bytes()) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
