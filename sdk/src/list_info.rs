//! Optional ordered list-item metadata. Call inside an existing item's object.
//! Unsupported hosts keep byte-identical legacy fields. Validation/capacity errors
//! leave both the cursor and buffer untouched; callers may omit this enhancement.
use crate::{json_utils::write_bytes, source_protocol::feature};

pub const HOST_FEATURE: &[u8] = b"source_manga_list_info_v1";

#[derive(Clone, Copy, Debug)]
pub enum ListInfo<'a> {
    Text {
        text: &'a [u8],
        label: Option<&'a [u8]>,
    },
    Tags {
        items: &'a [&'a [u8]],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    TooManyItems,
    InvalidText,
    InvalidLabel,
    InvalidTags,
    Capacity,
}

fn quoted_len(value: &[u8], max: usize, error: EncodeError) -> Result<usize, EncodeError> {
    if value.len() > max {
        return Err(error);
    }
    let text = core::str::from_utf8(value).map_err(|_| error)?;
    if text.trim().is_empty() {
        return Err(error);
    }
    Ok(2 + value
        .iter()
        .map(|b| match b {
            b'"' | b'\\' => 2,
            0..=31 => 6,
            _ => 1,
        })
        .sum::<usize>())
}

fn quote(dst: &mut [u8], at: &mut usize, text: &[u8]) -> bool {
    const HEX: &[u8] = b"0123456789abcdef";
    if !write_bytes(dst, at, b"\"") {
        return false;
    }
    for &b in text {
        let ok = match b {
            b'"' => write_bytes(dst, at, b"\\\""),
            b'\\' => write_bytes(dst, at, b"\\\\"),
            0..=31 => write_bytes(
                dst,
                at,
                &[
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX[(b >> 4) as usize],
                    HEX[(b & 15) as usize],
                ],
            ),
            _ => write_bytes(dst, at, &[b]),
        };
        if !ok {
            return false;
        }
    }
    write_bytes(dst, at, b"\"")
}

/// Append `,"listInfo":[...]` before the existing item object's closing brace.
/// Ok(false) means no negotiated feature or no entries. Empty/whitespace-only
/// fields, invalid UTF-8 and byte/count limits reject the whole optional field.
/// Text512/label96, tags1..=8/tag96, entries1..=3 are UTF-8 byte limits, not chars.
/// This does not fetch, infer labels/locales, parse HTML or alter existing fields.
/// On Err, omit the optional enhancement rather than dropping an otherwise valid item.
pub fn append(
    dst: &mut [u8],
    at: &mut usize,
    request: &[u8],
    entries: &[ListInfo<'_>],
) -> Result<bool, EncodeError> {
    if !feature(request, HOST_FEATURE) || entries.is_empty() {
        return Ok(false);
    }
    if entries.len() > 3 {
        return Err(EncodeError::TooManyItems);
    }
    let mut needed = b",\"listInfo\":[".len() + 1;
    for (i, entry) in entries.iter().enumerate() {
        needed += usize::from(i > 0);
        needed += match entry {
            ListInfo::Text { text, label } => {
                b"{\"type\":\"text\",\"text\":".len()
                    + quoted_len(text, 512, EncodeError::InvalidText)?
                    + if let Some(label) = label {
                        b",\"label\":".len() + quoted_len(label, 96, EncodeError::InvalidLabel)?
                    } else {
                        0
                    }
                    + 1
            }
            ListInfo::Tags { items } => {
                if items.is_empty() || items.len() > 8 {
                    return Err(EncodeError::InvalidTags);
                }
                let mut len = b"{\"type\":\"tags\",\"items\":[".len() + 2;
                for (i, tag) in items.iter().enumerate() {
                    len += usize::from(i > 0) + quoted_len(tag, 96, EncodeError::InvalidTags)?;
                }
                len
            }
        };
    }
    if at
        .checked_add(needed)
        .filter(|end| *end <= dst.len())
        .is_none()
    {
        return Err(EncodeError::Capacity);
    }
    // Entire input and exact escaped size are checked before the first write.
    let mut cursor = *at;
    let mut ok = write_bytes(dst, &mut cursor, b",\"listInfo\":[");
    for (i, entry) in entries.iter().enumerate() {
        ok = ok && (i == 0 || write_bytes(dst, &mut cursor, b","));
        ok = ok
            && match entry {
                ListInfo::Text { text, label } => {
                    write_bytes(dst, &mut cursor, b"{\"type\":\"text\",\"text\":")
                        && quote(dst, &mut cursor, text)
                        && label.map_or(true, |label| {
                            write_bytes(dst, &mut cursor, b",\"label\":")
                                && quote(dst, &mut cursor, label)
                        })
                        && write_bytes(dst, &mut cursor, b"}")
                }
                ListInfo::Tags { items } => {
                    let mut ok = write_bytes(dst, &mut cursor, b"{\"type\":\"tags\",\"items\":[");
                    for (i, tag) in items.iter().enumerate() {
                        ok = ok
                            && (i == 0 || write_bytes(dst, &mut cursor, b","))
                            && quote(dst, &mut cursor, tag);
                    }
                    ok && write_bytes(dst, &mut cursor, b"]}")
                }
            };
    }
    ok = ok && write_bytes(dst, &mut cursor, b"]");
    debug_assert!(ok && cursor == *at + needed);
    if !ok {
        return Err(EncodeError::Capacity);
    }
    *at = cursor;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MODERN: &[u8] = br#"{"hostFeatures":["source_manga_list_info_v1"]}"#;
    #[test]
    fn ordered_plain_text_and_tags_escape_without_interpreting_markup() {
        let mut b = [0; 1024];
        let mut n = 0;
        let tags = ["冒险".as_bytes(), "奇幻".as_bytes()];
        assert_eq!(
            append(
                &mut b,
                &mut n,
                MODERN,
                &[
                    ListInfo::Text {
                        text: "第123话\n<>&\"\\\0".as_bytes(),
                        label: Some("更新".as_bytes())
                    },
                    ListInfo::Tags { items: &tags },
                    ListInfo::Text {
                        text: b"last",
                        label: None
                    },
                ]
            ),
            Ok(true)
        );
        assert_eq!(core::str::from_utf8(&b[..n]).unwrap(), ",\"listInfo\":[{\"type\":\"text\",\"text\":\"第123话\\u000a<>&\\\"\\\\\\u0000\",\"label\":\"更新\"},{\"type\":\"tags\",\"items\":[\"冒险\",\"奇幻\"]},{\"type\":\"text\",\"text\":\"last\"}]");
    }
    #[test]
    fn legacy_and_actual_top_level_feature_gate_preserve_bytes() {
        for req in [
            br#"{}"#.as_slice(),
            br#"{"hostFeatures":[]}"#,
            br#"{"args":{"hostFeatures":["source_manga_list_info_v1"]}}"#,
            br#"{"hostFeatures":["source_manga_list_info_v1",false]}"#,
        ] {
            let mut b = [7; 64];
            let before = b;
            let mut n = 3;
            assert_eq!(
                append(
                    &mut b,
                    &mut n,
                    req,
                    &[ListInfo::Text {
                        text: b"\xff",
                        label: None
                    }]
                ),
                Ok(false)
            );
            assert_eq!(n, 3);
            assert_eq!(b, before);
        }
        let mut b = [7; 64];
        let mut n = 3;
        assert_eq!(append(&mut b, &mut n, MODERN, &[]), Ok(false));
        assert_eq!(b, [7; 64]);
    }
    #[test]
    fn utf8_empty_and_count_byte_boundaries_are_atomic() {
        let text = [b'x'; 512];
        let label = [b'l'; 96];
        let tag = [b't'; 96];
        let tags = [tag.as_slice(); 8];
        let mut b = [0; 4096];
        let mut n = 0;
        assert_eq!(
            append(
                &mut b,
                &mut n,
                MODERN,
                &[
                    ListInfo::Text {
                        text: &text,
                        label: Some(&label)
                    },
                    ListInfo::Tags { items: &tags }
                ]
            ),
            Ok(true)
        );
        let nine = [b"t".as_slice(); 9];
        let four = [ListInfo::Text {
            text: b"ok",
            label: None,
        }; 4];
        for entries in [
            &[ListInfo::Text {
                text: &[b'x'; 513],
                label: None,
            }][..],
            &[ListInfo::Text {
                text: b"x",
                label: Some(&[b'l'; 97]),
            }],
            &[ListInfo::Tags {
                items: &[&[b't'; 97]],
            }],
            &[ListInfo::Tags { items: &nine }],
            &[ListInfo::Tags { items: &[] }],
            &[ListInfo::Tags { items: &[b" "] }],
            &[ListInfo::Text {
                text: b"\xff",
                label: None,
            }],
            &[ListInfo::Text {
                text: b"\xe4\xb8",
                label: None,
            }],
            &[ListInfo::Text {
                text: b" \n",
                label: None,
            }],
            &[ListInfo::Text {
                text: b"x",
                label: Some(b""),
            }],
            &four,
        ] {
            let mut b = [7; 4096];
            let before = b;
            let mut n = 4;
            assert!(append(&mut b, &mut n, MODERN, entries).is_err());
            assert_eq!(n, 4);
            assert_eq!(b, before);
        }
        // UTF-8 limits count bytes, including a multibyte character at the edge.
        let mut multibyte = [b'x'; 513];
        multibyte[510..].copy_from_slice("话".as_bytes());
        let mut n = 0;
        assert_eq!(
            append(
                &mut b,
                &mut n,
                MODERN,
                &[ListInfo::Text {
                    text: &multibyte,
                    label: None
                }]
            ),
            Err(EncodeError::InvalidText)
        );
    }
    #[test]
    fn exact_escaped_capacity_preflight_keeps_prefix_and_failure_bytes() {
        let entries = [ListInfo::Text {
            text: b"\"\n\\",
            label: None,
        }];
        let mut full = [0; 256];
        let mut n = 0;
        append(&mut full, &mut n, MODERN, &entries).unwrap();
        let mut exact = [9; 256];
        let mut p = 5;
        assert_eq!(
            append(&mut exact[..n + 5], &mut p, MODERN, &entries),
            Ok(true)
        );
        assert_eq!(&exact[..5], &[9; 5]);
        assert_eq!(&exact[5..p], &full[..n]);
        let mut small = [9; 256];
        let before = small;
        let mut p = 5;
        assert_eq!(
            append(&mut small[..n + 4], &mut p, MODERN, &entries),
            Err(EncodeError::Capacity)
        );
        assert_eq!(small, before);
        assert_eq!(p, 5);
    }
}
