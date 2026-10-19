// Finite host-envelope classification. No raw error text reaches diagnostics.
use koma_source_sdk::json_utils::json_object_member;
pub fn failed(response: &[u8], status: u16) -> Option<&'static str> {
    // Existing numeric HTTP exchanges retain their status classification, even ok:false.
    if (100..=599).contains(&status) {
        return None;
    }
    if json_object_member(response, b"ok") != Some(b"false".as_slice()) {
        return None;
    }
    let code = json_object_member(response, b"error").and_then(|e| json_object_member(e, b"code"));
    Some(match code {
        Some(b"\"timeout\"") => "request timed out",
        Some(b"\"transport_unavailable\"") => "transport unavailable",
        Some(b"\"transport_rejected\"") => "transport rejected",
        Some(b"\"transport_invalid_response\"") => "invalid transport response",
        Some(b"\"network_error\"") | _ => "network request failed",
    })
}
pub fn http_error(status: u16) -> Option<&'static str> {
    match status {
        200..=299 => None,
        404 => Some("not found"),
        429 => Some("rate limited"),
        401 => Some("authentication required"),
        403 => Some("public request rejected"),
        400..=499 => Some("HTTP client error"),
        500..=599 => Some("server error"),
        _ => Some("invalid HTTP status"),
    }
}
