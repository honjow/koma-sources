//! Shared fixed GalleryAdults title preference; no arbitrary regular expressions.
//! Reference: Keiyoushi949dea GalleryAdults.shortenTitleRegex:
//! `(\[[^]]*]|[({][^)}]*[)}])`, then Kotlin.trim(). Matching deliberately permits
//! mixed closing delimiters and nested opening bytes exactly as that expression.
use crate::json_utils::json_object_member;

pub fn enabled(request: &[u8]) -> bool {
    json_object_member(request, b"settings")
        .and_then(|s| json_object_member(s, b"pref_short_title"))
        == Some(b"true")
}

pub fn display<'a>(
    request: &[u8],
    title: &'a [u8],
    output: &'a mut [u8],
) -> Result<&'a [u8], &'static str> {
    display_short(enabled(request), title, output)
}

/// Same fixed algorithm for sources whose real preference uses another key or
/// polarity (3Hentai full_title). Do not fabricate an HTTP/request envelope.
pub fn display_short<'a>(short: bool, title: &'a [u8], output: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    if !short {
        return Ok(title);
    }
    core::str::from_utf8(title).map_err(|_| "invalid title UTF8")?;
    let (mut at, mut written) = (0, 0);
    while at < title.len() {
        let closing = match title[at] {
            b'[' => title[at + 1..].iter().position(|&b| b == b']'),
            b'(' | b'{' => title[at + 1..].iter().position(|&b| b == b')' || b == b'}'),
            _ => None,
        };
        if let Some(end) = closing {
            at += end + 2;
            continue;
        }
        if written == output.len() {
            return Err("short title capacity");
        }
        output[written] = title[at];
        written += 1;
        at += 1;
    }
    let text = core::str::from_utf8(&output[..written]).map_err(|_| "invalid title UTF8")?;
    // Kotlin Char.isWhitespace is Java isWhitespace || isSpaceChar.
    Ok(text
        .trim_matches(|c: char| {
            (c.is_whitespace() && c != '\u{85}') || ('\u{1c}'..='\u{1f}').contains(&c)
        })
        .as_bytes())
}
