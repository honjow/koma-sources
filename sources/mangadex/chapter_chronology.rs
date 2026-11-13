//! MangaDex uses UUIDs for editions and decimal volume/chapter coordinates.

use koma_source_sdk::chapters::ChapterPlacement;
use koma_source_sdk::json_utils::{json_object_member, JsonArrayIter};

fn string<'a>(object: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let raw = json_object_member(object, key)?;
    raw.strip_prefix(b"\"")?.strip_suffix(b"\"")
}

fn uuid(value: &[u8]) -> bool {
    value.len() == 36
        && value.iter().enumerate().all(|(i, byte)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

// Keep arithmetic exact. Values outside this representation remain manual.
fn hundredths(value: &[u8]) -> Option<u32> {
    let (whole, fraction) = match value.iter().position(|byte| *byte == b'.') {
        Some(i) => (&value[..i], &value[i + 1..]),
        None => (value, b"".as_slice()),
    };
    if whole.is_empty() || !whole.iter().chain(fraction).all(u8::is_ascii_digit) {
        return None;
    }
    let whole = core::str::from_utf8(whole).ok()?.parse::<u32>().ok()?;
    let hundredths = match fraction {
        [] => 0,
        [a] => u32::from(*a - b'0') * 10,
        [a, b] => u32::from(*a - b'0') * 10 + u32::from(*b - b'0'),
        _ => return None,
    };
    whole.checked_mul(100)?.checked_add(hundredths)
}

fn place<'a>(row: &[u8], manga: &[u8], group: &'a mut [u8; 296]) -> Option<ChapterPlacement<'a>> {
    let attrs = json_object_member(row, b"attributes")?;
    let chapter = hundredths(string(attrs, b"chapter")?)?;
    if chapter >= 1_000_000 {
        return None;
    }
    let volume = match json_object_member(attrs, b"volume") {
        None | Some(b"null" | b"\"\"") => 0,
        Some(_) => hundredths(string(attrs, b"volume")?)?,
    };
    // Allow decimal volumes without treating an ordinal as a displayed number.
    let position = u64::from(volume) * 1_000_000 + u64::from(chapter);
    let mut relationships =
        JsonArrayIter::from_raw_array(json_object_member(row, b"relationships")?)?;
    let mut groups: [&[u8]; 8] = [b""; 8];
    let mut count = 0;
    let mut uploader = None;
    let mut owned = false;
    while let Some(relation) = relationships.next_object() {
        match string(relation, b"type")? {
            b"manga" => owned |= string(relation, b"id") == Some(manga),
            b"scanlation_group" => {
                let id = string(relation, b"id").filter(|value| uuid(value))?;
                if count == groups.len() || groups[..count].contains(&id) {
                    return None;
                }
                groups[count] = id;
                count += 1;
            }
            b"user" => {
                uploader = string(relation, b"id").filter(|value| uuid(value));
            }
            _ => {}
        }
    }
    if !owned {
        return None;
    }
    if count == 0 {
        group[0] = b'u';
        group[1..37].copy_from_slice(uploader?);
        return Some(ChapterPlacement::InGroup {
            group: &group[..37],
            position,
        });
    }
    groups[..count].sort_unstable();
    group[0] = b'g';
    let mut end = 1;
    for id in &groups[..count] {
        group[end..end + 36].copy_from_slice(id);
        end += 36;
    }
    Some(ChapterPlacement::InGroup {
        group: &group[..end],
        position,
    })
}

pub fn placement<'a>(row: &[u8], manga: &[u8], group: &'a mut [u8; 296]) -> ChapterPlacement<'a> {
    place(row, manga, group).unwrap_or(ChapterPlacement::ManualOnly)
}
