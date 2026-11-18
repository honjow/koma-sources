// Read-only source comments v1. Provider IDs and cursors belong to each source.
use crate::source_protocol::{direct, feature, raw_string};

pub const OPERATION: &str = "get_comments";

pub struct Request<'a> {
    pub manga_id: &'a [u8],
    pub chapter_id: Option<&'a [u8]>,
    pub cursor: Option<&'a [u8]>,
}

pub fn request(input: &[u8]) -> Result<Request<'_>, &'static str> {
    if !feature(input, b"source_comments_v1") {
        return Err("host_feature_required");
    }
    let args = direct(input, b"args")?.ok_or("invalid_request")?;
    let manga_id = raw_string(args, b"mangaId")?
        .filter(|id| !id.is_empty())
        .ok_or("invalid_request")?;
    let chapter_id = raw_string(args, b"chapterId")?;
    match (raw_string(args, b"scope")?, chapter_id) {
        (Some(b"manga"), None) => {}
        (Some(b"chapter"), Some(id)) if !id.is_empty() => {}
        _ => return Err("invalid_request"),
    }
    Ok(Request {
        manga_id,
        chapter_id,
        cursor: raw_string(args, b"cursor")?,
    })
}

pub fn position(cursor: Option<&[u8]>) -> Result<Option<usize>, &'static str> {
    cursor
        .map(|raw| {
            if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) {
                return Err("invalid_request");
            }
            core::str::from_utf8(raw)
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or("invalid_request")
        })
        .transpose()
}

// Convert a Unix millisecond timestamp to ISO 8601 for the host.
pub fn utc_iso_millis(raw: &[u8], out: &mut [u8; 20]) -> Option<usize> {
    let stamp = core::str::from_utf8(raw).ok()?.parse::<u64>().ok()?;
    let seconds = stamp / 1000;
    let days = seconds / 86_400;
    if days > 2_932_896 {
        return None;
    }
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = era * 400 + yoe + u64::from(month <= 2);
    *out = *b"0000-00-00T00:00:00Z";
    for (start, width, mut value) in [
        (0, 4, year),
        (5, 2, month),
        (8, 2, day),
        (11, 2, (seconds / 3600) % 24),
        (14, 2, (seconds / 60) % 60),
        (17, 2, seconds % 60),
    ] {
        for index in (start..start + width).rev() {
            out[index] = b'0' + (value % 10) as u8;
            value /= 10;
        }
    }
    Some(out.len())
}

#[macro_export]
macro_rules! koma_source_comments_export {
    ($handler:path) => {
        #[no_mangle]
        pub extern "C" fn koma_source_get_comments(req_ptr: u32, req_len: u32) -> u32 {
            match read_request(req_ptr, req_len) {
                Some(req) => $handler(req),
                None => write_error(
                    $crate::comments::OPERATION,
                    "invalid_request",
                    "invalid comments request",
                ),
            }
        }
    };
}
