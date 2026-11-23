// Baozi.kt appChapterUrl/getPageList: selected app origin, one complete
// .comic-contain img[data-src] array. Keep stable chapter/page identities and
// parent CDN/banner contracts. Current app templates returned CF403 in the
// bounded provider capture; source-contract implementation is not live acceptance.
use super::{
    append_json_escaped, attr_into, body_buf, cdn_selection, fetch_html_with_attempts, host,
    html_parse, html_select_all, http_out, log_cdn_result, mirror, payload_buf, probe_image_head,
    write_bytes, write_error, write_fetch_error, write_page_ordinal, write_success_payload,
    write_usize, OwnedDescriptor, PageDescriptors, BODY_CAP, NATIVE_RESULT_CAP,
};

const IMAGES: &[u8] = b".comic-contain img[data-src]";

fn valid_slug(slug: &[u8]) -> bool {
    !slug.is_empty()
        && slug
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
}

fn digits(value: &[u8]) -> bool {
    !value.is_empty() && value.iter().all(u8::is_ascii_digit)
}

// Section zero retains its published numeric slot. Only nonzero sections use
// the new opaque section_slot spelling; never normalize away leading digits.
fn valid_slot(slot: &[u8]) -> bool {
    if digits(slot) {
        return true;
    }
    let Some(split) = slot.iter().position(|b| *b == b'_') else {
        return false;
    };
    let (section, tail) = slot.split_at(split);
    digits(section) && section.iter().any(|b| *b != b'0') && digits(&tail[1..])
}

fn matching_identity(chapter_id: &[u8], slug: &[u8], slot: &[u8]) -> bool {
    chapter_id
        .strip_prefix(b"chapter:")
        .and_then(|id| id.strip_prefix(slug))
        .and_then(|id| id.strip_prefix(b":"))
        .map(|id| id.split(|b| *b == b'#').next() == Some(slot))
        .unwrap_or(false)
}

// Resolve data-src against the actual app document rather than the selected
// web mirror or the cover resizer. No CDN suffix rewrite is introduced here.
fn image_uri(raw: &[u8], document_url: &[u8], app: &[u8], output: &mut [u8]) -> Option<usize> {
    if raw.is_empty() || raw.iter().any(|b| *b <= 32 || *b == 127 || *b == b'\\') {
        return None;
    }
    let base = if raw.starts_with(b"https://") || raw.starts_with(b"http://") {
        b"".as_slice()
    } else if raw.starts_with(b"//") {
        b"https:".as_slice()
    } else if raw.starts_with(b"/") {
        app
    } else {
        // A relative filename is allowed; an unrecognised scheme is not.
        if raw.contains(&b':') {
            return None;
        }
        &document_url[..document_url.iter().rposition(|b| *b == b'/')? + 1]
    };
    mirror::join(output, base, raw)
}

pub(super) fn run(req: &[u8], chapter_id: &[u8], slug: &[u8], slot: &[u8]) -> u32 {
    if !valid_slug(slug) || !valid_slot(slot) || !matching_identity(chapter_id, slug, slot) {
        return write_error(
            "get_pages",
            "invalid_request",
            "invalid chapter owner or slot",
        );
    }
    if host::check_cancel() {
        return write_error("get_pages", "cancelled", "cancelled");
    }
    let app = mirror::selected(req).app;
    let mut url = [0u8; 4352];
    let mut url_len = 0;
    let ok = write_bytes(&mut url, &mut url_len, app)
        && write_bytes(&mut url, &mut url_len, b"/baozimhapp/comic/chapter/")
        && write_bytes(&mut url, &mut url_len, slug)
        && write_bytes(&mut url, &mut url_len, b"/")
        && (slot.contains(&b'_') || write_bytes(&mut url, &mut url_len, b"0_"))
        && write_bytes(&mut url, &mut url_len, slot)
        && write_bytes(&mut url, &mut url_len, b".html");
    if !ok {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    // The parent owns challenge classification and the operation request budget.
    // This branch must not multiply that budget through its own GET retry loop.
    let html_len = match fetch_html_with_attempts(&url[..url_len], req, 1) {
        Ok(length) => length,
        Err(error) => return write_fetch_error("get_pages", error),
    };
    let document = match html_parse(&body_buf()[..html_len]) {
        Ok(descriptor) => OwnedDescriptor(descriptor),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };
    let count = html_select_all(document.0.raw(), IMAGES, &mut []);
    if count <= 0 {
        return write_error("get_pages", "parse_error", "missing app chapter images");
    }
    let total = count as usize;
    let descriptor_bytes = match total.checked_mul(4) {
        Some(bytes) if bytes <= BODY_CAP => bytes,
        _ => {
            return write_error(
                "get_pages",
                "source_error",
                "chapter cannot fit current response capacity",
            )
        }
    };
    // The native DOM owns its HTML storage. Reuse the consumed body buffer for
    // every child handle; no fixed page-count ceiling or partial success.
    let handles = &mut body_buf()[..descriptor_bytes];
    let selected = html_select_all(document.0.raw(), IMAGES, handles);
    if selected < 0 {
        return write_error("get_pages", "parse_error", "image selection failed");
    }
    let descriptors = PageDescriptors {
        bytes: handles,
        count: (selected as usize).min(total),
    };
    if selected as usize != total {
        return write_error("get_pages", "parse_error", "image selection changed");
    }
    let Some(first) = descriptors.get(0) else {
        return write_error("get_pages", "parse_error", "missing image descriptor");
    };
    // This small buffer bounds only the optional first-image CDN probe, as in
    // the existing reader. Full page URIs below retain the Native capacity.
    let mut first_uri_buffer = [0u8; 4096];
    let first_raw = match attr_into(first, b"data-src", &mut http_out()[..NATIVE_RESULT_CAP]) {
        Some(value) if value.len() < NATIVE_RESULT_CAP => value,
        Some(_) => {
            return write_error(
                "get_pages",
                "source_error",
                "image URI cannot fit current response capacity",
            )
        }
        None => return write_error("get_pages", "parse_error", "missing image data-src"),
    };
    let first_len = image_uri(first_raw, &url[..url_len], app, &mut first_uri_buffer).unwrap_or(0);
    if host::check_cancel() {
        return write_error("get_pages", "cancelled", "cancelled");
    }
    let mut probes = 0;
    let mut head_succeeded = false;
    // Keep the existing finite CDN candidates. New upstream .baozicdn.com
    // rewriting and its 404 fallback are not implemented by this older helper;
    // unsupported hosts stay unchanged for the parent image-request path.
    let selection = match cdn_selection::choose(&first_uri_buffer[..first_len], slug, |parts| {
        probes += 1;
        let result = probe_image_head(parts);
        if matches!(result, Ok(true)) {
            head_succeeded = true;
        }
        result
    }) {
        Ok(value) => value,
        Err(()) => {
            log_cdn_result(probes, b"cancelled");
            return write_error("get_pages", "cancelled", "cancelled");
        }
    };
    log_cdn_result(
        probes,
        if selection.is_some() {
            b"fallback"
        } else if probes == 0 {
            b"skipped"
        } else if head_succeeded {
            b"original"
        } else {
            b"original_all_failed"
        },
    );

    // probe_image_head uses the payload as request scratch; publish pages only
    // after selection has finished. Banner processing remains in the parent's
    // get_image_request path, not in this descriptor array.
    let payload = payload_buf();
    let mut cursor = 0;
    if !(write_bytes(payload, &mut cursor, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut cursor, chapter_id)
        && write_bytes(payload, &mut cursor, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let (raw_buffer, uri_buffer) = http_out().split_at_mut(NATIVE_RESULT_CAP);
    for index in 0..total {
        if host::check_cancel() {
            return write_error("get_pages", "cancelled", "cancelled");
        }
        let Some(image) = descriptors.get(index) else {
            return write_error("get_pages", "parse_error", "missing image descriptor");
        };
        let raw = match attr_into(image, b"data-src", raw_buffer) {
            Some(value) if !value.is_empty() && value.len() < NATIVE_RESULT_CAP => value,
            _ => {
                return write_error(
                    "get_pages",
                    "parse_error",
                    "missing or truncated image data-src",
                )
            }
        };
        let Some(uri_len) = image_uri(raw, &url[..url_len], app, uri_buffer) else {
            return write_error(
                "get_pages",
                "parse_error",
                "invalid or oversized image data-src",
            );
        };
        let uri = &uri_buffer[..uri_len];
        let ok = (index == 0 || write_bytes(payload, &mut cursor, b","))
            && write_bytes(payload, &mut cursor, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut cursor, slug)
            && write_bytes(payload, &mut cursor, b":")
            && append_json_escaped(payload, &mut cursor, slot)
            && write_bytes(payload, &mut cursor, b":")
            && write_page_ordinal(payload, &mut cursor, index)
            && write_bytes(payload, &mut cursor, br#"","index":"#)
            && write_usize(payload, &mut cursor, index)
            && write_bytes(payload, &mut cursor, br#","image":{"kind":"url","url":""#)
            && match selection.as_ref().and_then(|chosen| chosen.rewrite(uri)) {
                Some(rewritten) => {
                    append_json_escaped(payload, &mut cursor, rewritten.prefix)
                        && append_json_escaped(payload, &mut cursor, rewritten.suffix)
                        && append_json_escaped(payload, &mut cursor, rewritten.tail)
                }
                None => append_json_escaped(payload, &mut cursor, uri),
            }
            && write_bytes(payload, &mut cursor, br#""}}"#);
        if !ok {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
    }
    if !write_bytes(payload, &mut cursor, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let result = write_success_payload("get_pages", cursor);
    if result == 0 {
        return write_error("get_pages", "internal_error", "response overflow");
    }
    let envelope_len =
        unsafe { core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>()) };
    if envelope_len as usize > NATIVE_RESULT_CAP {
        return write_error(
            "get_pages",
            "source_error",
            "chapter cannot fit current response capacity",
        );
    }
    result
}
