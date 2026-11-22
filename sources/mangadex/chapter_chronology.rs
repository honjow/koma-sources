//! Provider positions define chronology; decimal coordinates only identify alternatives.

use koma_source_sdk::chapters::ChapterPlacement;
use koma_source_sdk::json_utils::{json_object_member, write_bytes, write_usize, JsonArrayIter};

fn string<'a>(object: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    json_object_member(object, key)?.strip_prefix(b"\"")?.strip_suffix(b"\"")
}

fn uuid(value: &[u8]) -> bool {
    value.len() == 36 && value.iter().enumerate().all(|(i, byte)| {
        if matches!(i, 8 | 13 | 18 | 23) { *byte == b'-' } else { byte.is_ascii_hexdigit() }
    })
}

// Equality only, never a manufactured chapter number or a sorting key. Avoid
// f64 rounding and the old two-decimal gate. Unknown text still gets a slot.
fn decimal(value: &[u8], out: &mut [u8], n: &mut usize) -> Option<()> {
    let (negative, value) = match value.strip_prefix(b"-") {
        Some(v) => (true, v), None => (false, value),
    };
    let (whole, fraction) = match value.iter().position(|b| *b == b'.') {
        Some(i) => (&value[..i], &value[i + 1..]), None => (value, b"".as_slice()),
    };
    if whole.is_empty() || !whole.iter().chain(fraction).all(u8::is_ascii_digit) { return None; }
    let whole = &whole[whole.iter().position(|b| *b != b'0').unwrap_or(whole.len().saturating_sub(1))..];
    let fraction = &fraction[..fraction.iter().rposition(|b| *b != b'0').map_or(0, |i| i + 1)];
    if negative && (whole != b"0" || !fraction.is_empty()) && !write_bytes(out, n, b"-") { return None; }
    if !write_bytes(out, n, whole) { return None; }
    if !fraction.is_empty() && !(write_bytes(out, n, b".") && write_bytes(out, n, fraction)) { return None; }
    Some(())
}

fn coordinate(row: &[u8], out: &mut [u8; 512]) -> Option<usize> {
    let attrs = json_object_member(row, b"attributes")?;
    let mut n = 0;
    match json_object_member(attrs, b"volume") {
        None | Some(b"null" | b"\"\"") => { if !write_bytes(out, &mut n, b"n") { return None; } }
        Some(_) => decimal(string(attrs, b"volume")?, out, &mut n)?,
    }
    if !write_bytes(out, &mut n, b"/") { return None; }
    decimal(string(attrs, b"chapter")?, out, &mut n)?;
    Some(n)
}

pub struct Positions {
    pub total: usize,
    pub seen: usize,
    anchor: usize,
    key: [u8; 512],
    key_len: usize,
}

impl Positions {
    pub fn new(total: usize) -> Result<Self, &'static str> {
        if total as u64 > 9_007_199_254_740_991 { return Err("chapter total exceeds safe positions"); }
        Ok(Self { total, seen: 0, anchor: 0, key: [0; 512], key_len: 0 })
    }

    pub fn begin_page(&self, offset: usize, total: usize) -> Result<(), &'static str> {
        if self.total != total || self.seen != offset { return Err("chapter position traversal drift"); }
        Ok(())
    }

    pub fn observe(&mut self, row: &[u8]) -> Result<u64, &'static str> {
        if self.seen >= self.total { return Err("chapter position count exceeds total"); }
        let mut key = [0; 512];
        let len = coordinate(row, &mut key).unwrap_or(0);
        // Missing/unknown coordinates are distinct real catalogue entries.
        // Known equal coordinates retain the adopted alternative-edition slot.
        if len == 0 || self.key_len != len || self.key[..len] != key[..len] { self.anchor = self.seen; }
        self.key[..len].copy_from_slice(&key[..len]);
        self.key_len = len;
        self.seen += 1;
        Ok((self.total - 1 - self.anchor) as u64)
    }

    #[allow(dead_code)] // English pagination; Chinese callers aggregate internally.
    pub fn read_cursor<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Option<Self> {
        fn number(s: &str) -> Option<usize> {
            if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) { return None; }
            s.parse().ok()
        }
        let mut result = Self::new(number(fields.next()?)?).ok()?;
        result.seen = number(fields.next()?)?;
        result.anchor = number(fields.next()?)?;
        let key = fields.next()?.as_bytes();
        if key.len() > result.key.len() || result.seen == 0 || result.seen >= result.total || result.anchor >= result.seen { return None; }
        if !key.is_empty() {
            let slash = key.iter().position(|c| *c == b'/')?;
            let mut canonical = [0; 512]; let mut n = 0;
            if &key[..slash] == b"n" { write_bytes(&mut canonical, &mut n, b"n"); }
            else { decimal(&key[..slash], &mut canonical, &mut n)?; }
            if !write_bytes(&mut canonical, &mut n, b"/") { return None; }
            decimal(&key[slash + 1..], &mut canonical, &mut n)?;
            if canonical[..n] != *key { return None; }
        }
        result.key[..key.len()].copy_from_slice(key); result.key_len = key.len();
        Some(result)
    }

    #[allow(dead_code)] // English pagination; Chinese callers aggregate internally.
    pub fn append_cursor(&self, out: &mut [u8], n: &mut usize) -> bool {
        write_bytes(out, n, b":") && write_usize(out, n, self.total)
            && write_bytes(out, n, b":") && write_usize(out, n, self.seen)
            && write_bytes(out, n, b":") && write_usize(out, n, self.anchor)
            && write_bytes(out, n, b":") && write_bytes(out, n, &self.key[..self.key_len])
    }
}

fn place<'a>(row: &'a [u8], manga: &[u8], group: &'a mut [u8; 296], position: u64) -> Option<ChapterPlacement<'a>> {
    let attrs = json_object_member(row, b"attributes")?;
    let language = string(attrs, b"translatedLanguage")?;
    let mut relationships = JsonArrayIter::from_raw_array(json_object_member(row, b"relationships")?)?;
    let mut groups: [&[u8]; 8] = [b""; 8];
    let mut count = 0;
    let mut uploader = None;
    let mut owned = false;
    while let Some(relation) = relationships.next_object() {
        match string(relation, b"type")? {
            b"manga" => owned |= string(relation, b"id") == Some(manga),
            b"scanlation_group" => {
                let id = string(relation, b"id").filter(|value| uuid(value))?;
                if count == groups.len() || groups[..count].contains(&id) { return None; }
                groups[count] = id; count += 1;
            }
            b"user" => uploader = string(relation, b"id").filter(|value| uuid(value)),
            _ => {}
        }
    }
    if !owned { return None; }
    if count == 0 {
        group[0] = b'u'; group[1..37].copy_from_slice(uploader?);
        return Some(ChapterPlacement::Variant { group: &group[..37], position, sequence: language });
    }
    groups[..count].sort_unstable(); group[0] = b'g'; let mut end = 1;
    for id in &groups[..count] { group[end..end + 36].copy_from_slice(id); end += 36; }
    Some(ChapterPlacement::Variant { group: &group[..end], position, sequence: language })
}

pub fn placement<'a>(row: &'a [u8], manga: &[u8], group: &'a mut [u8; 296], position: u64) -> ChapterPlacement<'a> {
    place(row, manga, group, position).unwrap_or(ChapterPlacement::ManualOnly)
}
