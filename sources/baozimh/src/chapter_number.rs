// A reader-link slot is an opaque identity, not the displayed chapter number.
// Only the observed leading labels supply numeric metadata; all other titles
// remain unknown. This deliberately does not scan for digits elsewhere.
pub(crate) fn title_chapter_number(title: &[u8]) -> Option<&[u8]> {
    let (label, bracketed) = match title.strip_prefix(b"[") {
        Some(label) => (label, true),
        None => (title, false),
    };
    let label = label.strip_prefix("第".as_bytes())?;
    let mut end = 0;
    while end < label.len() && label[end].is_ascii_digit() { end += 1; }
    if end == 0 { return None; }
    if label.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while end < label.len() && label[end].is_ascii_digit() { end += 1; }
        if end == start { return None; }
    }
    let number = &label[..end];
    let value = core::str::from_utf8(number).ok()?.parse::<f64>().ok()?;
    if !value.is_finite() { return None; }
    let rest = label[end..].strip_prefix("話".as_bytes())
        .or_else(|| label[end..].strip_prefix("话".as_bytes()))?;
    if bracketed {
        rest.strip_prefix(b"]")?;
    } else if !rest.is_empty()
        && !matches!(rest[0], b' ' | b'\t' | b'\r' | b'\n' | b':' | b'-' | b'(')
        && !rest.starts_with("：".as_bytes()) && !rest.starts_with("（".as_bytes()) {
        return None;
    }
    Some(number)
}

#[cfg(test)]
mod tests {
    use super::title_chapter_number;

    #[test]
    fn explicit_title_number_never_uses_part_number_or_request_slot() {
        assert_eq!(title_chapter_number("[第170話]鏢行目標(1)".as_bytes()), Some(b"170".as_slice()));
        assert_eq!(title_chapter_number("[第198話]般屍(2)".as_bytes()), Some(b"198".as_slice()));
        assert_eq!(title_chapter_number("第184話".as_bytes()), Some(b"184".as_slice()));
        assert_eq!(title_chapter_number("第0话 起点".as_bytes()), Some(b"0".as_slice()));
        assert_eq!(title_chapter_number("[第12.5話]番外(2)".as_bytes()), Some(b"12.5".as_slice()));
        assert_eq!(title_chapter_number("第1.5话：番外".as_bytes()), Some(b"1.5".as_slice()));
    }

    #[test]
    fn unknown_or_ambiguous_labels_remain_unknown() {
        for title in ["第2季第170話", "第12卷", "第12頁", "预告(3P)", "Extra 2", "170", "",
            "[第170話", "[第170話}正文", "第170話]正文", "第170話數", "第1.話", "第1.2.3話",
            "第-1話", "第+1話", "第1e2話", "第話", "第１２話", "标题[第170話]"] {
            assert_eq!(title_chapter_number(title.as_bytes()), None, "{title}");
        }
    }
}
