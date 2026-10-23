// Native Text/Attr return min(actual bytes, output capacity). A full buffer
// therefore cannot prove a complete fact; retaining that row is conservative.
pub(crate) fn complete_fact(value: Option<&[u8]>, output_capacity: usize) -> Option<&[u8]> {
    value.filter(|bytes| bytes.len() < output_capacity)
}

// Keep only byte-identical, complete chapter facts in the current response.
// This is not chapter-identity migration: conflicting links/titles are retained.
pub(crate) fn remember_exact_duplicate(
    seen: &mut [u8],
    used: &mut usize,
    href: Option<&[u8]>,
    slot: Option<&[u8]>,
    title: Option<&[u8]>,
) -> bool {
    let (href, slot, title) = match (href, slot, title) {
        (Some(href), Some(slot), Some(title))
            if !href.is_empty() && !slot.is_empty() && !title.is_empty()
                && href.len() <= u16::MAX as usize && slot.len() <= u16::MAX as usize
                && title.len() <= u16::MAX as usize => (href, slot, title),
        _ => return false,
    };
    if *used > seen.len() { return false; }
    let mut offset = 0;
    while offset + 6 <= *used {
        let h = u16::from_le_bytes([seen[offset], seen[offset + 1]]) as usize;
        let s = u16::from_le_bytes([seen[offset + 2], seen[offset + 3]]) as usize;
        let t = u16::from_le_bytes([seen[offset + 4], seen[offset + 5]]) as usize;
        let start = offset + 6;
        let end = start + h + s + t;
        if end > *used { return false; }
        if &seen[start..start + h] == href && &seen[start + h..start + h + s] == slot
            && &seen[start + h + s..end] == title {
            return true;
        }
        offset = end;
    }
    let bytes = 6 + href.len() + slot.len() + title.len();
    if bytes > seen.len() - *used {
        // Keep the row rather than imposing another limit on an otherwise valid catalog.
        return false;
    }
    let offset = *used;
    seen[offset..offset + 2].copy_from_slice(&(href.len() as u16).to_le_bytes());
    seen[offset + 2..offset + 4].copy_from_slice(&(slot.len() as u16).to_le_bytes());
    seen[offset + 4..offset + 6].copy_from_slice(&(title.len() as u16).to_le_bytes());
    let mut at = offset + 6;
    for part in [href, slot, title] {
        seen[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    }
    *used = at;
    false
}

#[cfg(test)]
mod tests {
    use super::{complete_fact, remember_exact_duplicate};

    #[test]
    fn saturated_raw_text_is_not_made_complete_by_trimming() {
        let mut raw = [b'x'; 256];
        raw[255] = b' ';
        assert!(complete_fact(Some(&raw), 256).is_none());
        assert_eq!(complete_fact(Some(&raw[..255]), 256), Some(&raw[..255]));
    }

    #[test]
    fn only_exact_facts_repeat() {
        let mut seen = [0; 2048];
        let mut used = 0;
        let rows: [(&[u8], &[u8], &[u8]); 6] = [
            (b"/chapter?chapter_slot=184&section_slot=0", b"184", b"Chapter 184"),
            (b"/chapter?chapter_slot=183&section_slot=0", b"183", b"Same title"),
            (b"/chapter?chapter_slot=184&section_slot=0", b"184", b"Chapter 184"),
            (b"/chapter?chapter_slot=184&section_slot=0", b"184", b"Different title"),
            (b"/chapter?chapter_slot=184&section_slot=1", b"184", b"Chapter 184"),
            (b"/chapter?chapter_slot=182&section_slot=0", b"182", b"Same title"),
        ];
        let kept: std::vec::Vec<usize> = rows.iter().enumerate().filter_map(|(i, (href, slot, title))| {
            (!remember_exact_duplicate(&mut seen, &mut used, Some(href), Some(slot), Some(title))).then_some(i)
        }).collect();
        assert_eq!(kept, [0, 1, 3, 4, 5]);
    }

    #[test]
    fn unknown_slot_and_missing_fields_never_merge_into_fallback_zero() {
        let mut seen = [0; 256];
        let mut used = 0;
        for _ in 0..2 {
            assert!(!remember_exact_duplicate(&mut seen, &mut used, Some(b"/chapter"), None, Some(b"Title")));
            assert!(!remember_exact_duplicate(&mut seen, &mut used, None, Some(b"0"), Some(b"Title")));
            assert!(!remember_exact_duplicate(&mut seen, &mut used, Some(b"/chapter"), Some(b""), Some(b"Title")));
            assert!(!remember_exact_duplicate(&mut seen, &mut used, Some(b"/chapter"), Some(b"0"), None));
        }
        assert_eq!(used, 0);
    }

    #[test]
    fn full_tracking_storage_keeps_rows_without_failing_catalog() {
        let mut seen = [0; 10];
        let mut used = 0;
        for _ in 0..2 {
            assert!(!remember_exact_duplicate(&mut seen, &mut used, Some(b"/chapter"), Some(b"1"), Some(b"Title")));
        }
        assert_eq!(used, 0);
    }
}
