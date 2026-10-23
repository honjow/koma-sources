// A reader-link slot is an opaque identity, not the displayed chapter number.
// Only the observed leading labels supply numeric metadata; all other titles
// remain unknown. This deliberately does not scan for digits elsewhere.
pub(crate) fn title_chapter_number(title: &[u8]) -> Option<&[u8]> {
    if title.first().is_some_and(u8::is_ascii_digit) {
        return numbered_title_prefix(title);
    }
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

// Baozimh's observed 紹宋 catalog labels use "215 如之奈何". Keep the
// request slot opaque: only this complete, delimited title prefix is metadata.
fn numbered_title_prefix(title: &[u8]) -> Option<&[u8]> {
    let mut end = 0;
    while title.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
    if end == 0 { return None; }
    if title.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while title.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
        if end == start { return None; }
    }
    let number = &title[..end];
    let value = core::str::from_utf8(number).ok()?.parse::<f64>().ok()?;
    // Avoid silently turning a huge decimal label into rounded/infinite metadata.
    if !value.is_finite() || value > 9_007_199_254_740_991.0 { return None; }
    let rest = &title[end..];
    // Two additional real catalog forms: "045" and "006扶腰壯膽韓良臣".
    // Only three-digit zero-padded integers qualify, not arbitrary bare numbers.
    if number.len() == 3 && number[0] == b'0' && number.iter().all(u8::is_ascii_digit) {
        if rest.is_empty() { return Some(number); }
        let first = core::str::from_utf8(rest).ok()?.chars().next()?;
        if ('\u{3400}'..='\u{4dbf}').contains(&first) || ('\u{4e00}'..='\u{9fff}').contains(&first) {
            // These prefixes describe dates, editions or units, not a title.
            if !matches!(first, '年' | '月' | '日' | '卷' | '季' | '頁' | '页' | '冊' | '册' | '部' | '集' | '第') {
                return Some(number);
            }
        }
    }
    if !title.get(end).is_some_and(u8::is_ascii_whitespace) { return None; }
    if rest.iter().all(u8::is_ascii_whitespace) { return None; }
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
    fn observed_shaosong_labels_supply_numbers_without_reordering_catalog() {
        let titles = ["215 如之奈何", "224 潰敗", "223 你不明白", "222 添亂", "221 你們也配？！",
            "220 掏後", "219 車輪子", "214 鷹揚", "218 決戰時機", "217 牽扯", "216 奇兵"];
        let numbers: Vec<u32> = titles.iter().map(|title|
            core::str::from_utf8(title_chapter_number(title.as_bytes()).unwrap()).unwrap().parse().unwrap()).collect();
        assert_eq!(numbers, [215, 224, 223, 222, 221, 220, 219, 214, 218, 217, 216]);
        let mut descending = numbers.clone();
        descending.sort_by(|left, right| right.cmp(left));
        assert_eq!(descending, [224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214]);
        assert_eq!(title_chapter_number("12.5 番外(2)".as_bytes()), Some(b"12.5".as_slice()));
    }

    #[test]
    fn numeric_prefix_requires_complete_delimiter_title_and_representable_number() {
        for title in ["215", "215 ", "215\t\r\n", "215如之奈何", "chapter 215 标题", "215A 标题",
            "2026-10-23 更新", "2026/10/23 更新", "2026.10.23 更新", "215e2 标题", "-215 标题",
            "215. 标题", "215.2.3 标题", "9007199254740992 标题"] {
            assert_eq!(title_chapter_number(title.as_bytes()), None, "{title}");
        }
        let huge = format!("{} 标题", "9".repeat(400));
        assert_eq!(title_chapter_number(huge.as_bytes()), None);
    }

    #[test]
    fn observed_zero_padded_catalog_numbers_do_not_need_a_space_or_title() {
        assert_eq!(title_chapter_number("006扶腰壯膽韓良臣".as_bytes()), Some(b"006".as_slice()));
        assert_eq!(title_chapter_number(b"045"), Some(b"045".as_slice()));
        let labels = ["005 标题", "006扶腰壯膽韓良臣", "007 标题", "045"];
        let numbers: Vec<u32> = labels.iter().map(|title|
            core::str::from_utf8(title_chapter_number(title.as_bytes()).unwrap()).unwrap().parse().unwrap()).collect();
        assert_eq!(numbers, [5, 6, 7, 45]);
    }

    #[test]
    fn zero_padded_extension_does_not_guess_dates_editions_or_other_numeric_titles() {
        for title in ["006年01月02日", "006月刊", "006日更新", "006卷", "006季", "006頁", "006页",
            "006冊", "006册", "006部", "006集", "006第2季", "006-01-02", "006/01/02", "006.01.02",
            "20261023", "2026年更新", "45", "170", "215如之奈何", "6扶腰壯膽韓良臣", "0006标题",
            "006A标题", "006_标题", "006.5标题", "标题006", "006（番外）"] {
            assert_eq!(title_chapter_number(title.as_bytes()), None, "{title}");
        }
        assert_eq!(title_chapter_number(b"006\xff"), None);
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
