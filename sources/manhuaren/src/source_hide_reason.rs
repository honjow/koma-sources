// Source-local classification, not a recovery or access fallback. The provider's
// exact hide reason belongs to this manga only; malformed/ambiguous data must not
// become a no_chapters claim. Keep edition arrays and existing ordering intact.
use koma_source_sdk::plain_text::decode_json_content;

pub struct DetailReason<'a> {
    pub raw: &'a [u8],
    pub confirmed_blocked_empty: bool,
}

fn ws(s: &[u8], mut p: usize) -> usize {
    while matches!(s.get(p), Some(b' ' | b'\n' | b'\r' | b'\t')) { p += 1; }
    p
}
fn hex4(s: &[u8]) -> Option<u16> {
    if s.len() != 4 { return None; }
    let mut value = 0u16;
    for &c in s {
        value = value * 16 + match c {
            b'0'..=b'9' => (c-b'0') as u16,
            b'a'..=b'f' => (c-b'a'+10) as u16,
            b'A'..=b'F' => (c-b'A'+10) as u16,
            _ => return None,
        };
    }
    Some(value)
}
fn string_end(s: &[u8], p: usize) -> Option<usize> {
    if s.get(p) != Some(&b'"') { return None; }
    let mut i = p+1;
    while i < s.len() {
        match s[i] {
            b'"' => return Some(i+1),
            0..=31 => return None,
            b'\\' => {
                i += 1;
                match *s.get(i)? {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {},
                    b'u' => {
                        let code = hex4(s.get(i+1..i+5)?)?;
                        i += 4;
                        if (0xd800..=0xdbff).contains(&code) {
                            if s.get(i+1..i+3) != Some(b"\\u") { return None; }
                            let low = hex4(s.get(i+3..i+7)?)?;
                            if !(0xdc00..=0xdfff).contains(&low) { return None; }
                            i += 6;
                        } else if (0xdc00..=0xdfff).contains(&code) { return None; }
                    },
                    _ => return None,
                }
            },
            _ => {},
        }
        i += 1;
    }
    None
}
fn value_end(s: &[u8], p: usize, depth: usize) -> Option<usize> {
    if depth > 32 { return None; }
    let p = ws(s,p);
    match *s.get(p)? {
        b'"' => string_end(s,p),
        b'{' | b'[' => {
            let object = s[p] == b'{';
            let close = if object { b'}' } else { b']' };
            let mut i = ws(s,p+1);
            if s.get(i) == Some(&close) { return Some(i+1); }
            loop {
                if object {
                    i = ws(s,string_end(s,i)?);
                    if s.get(i) != Some(&b':') { return None; }
                    i = ws(s,i+1);
                }
                i = ws(s,value_end(s,i,depth+1)?);
                if s.get(i) == Some(&close) { return Some(i+1); }
                if s.get(i) != Some(&b',') { return None; }
                i = ws(s,i+1);
            }
        },
        b't' => (s.get(p..p+4) == Some(b"true")).then_some(p+4),
        b'f' => (s.get(p..p+5) == Some(b"false")).then_some(p+5),
        b'n' => (s.get(p..p+4) == Some(b"null")).then_some(p+4),
        b'-' | b'0'..=b'9' => {
            let mut i = p;
            if s[i] == b'-' { i += 1; }
            if s.get(i) == Some(&b'0') { i += 1; }
            else {
                if !matches!(s.get(i),Some(b'1'..=b'9')) { return None; }
                while s.get(i).map_or(false,u8::is_ascii_digit) { i += 1; }
            }
            if s.get(i) == Some(&b'.') {
                i += 1; let start = i;
                while s.get(i).map_or(false,u8::is_ascii_digit) { i += 1; }
                if i == start { return None; }
            }
            if matches!(s.get(i),Some(b'e' | b'E')) {
                i += 1;
                if matches!(s.get(i),Some(b'+' | b'-')) { i += 1; }
                let start = i;
                while s.get(i).map_or(false,u8::is_ascii_digit) { i += 1; }
                if i == start { return None; }
            }
            Some(i)
        },
        _ => None,
    }
}
// The whole document is validated before this accessor. Decode member names so
// an escaped duplicate cannot contradict the plain mangaId/reason/array member.
fn member_once<'a>(s: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, ()> {
    let start = ws(s,0);
    if s.get(start) != Some(&b'{') { return Err(()); }
    let mut i = ws(s,start+1);
    let mut found = None;
    let mut decoded_key = [0u8;128];
    while s.get(i) != Some(&b'}') {
        let end = string_end(s,i).ok_or(())?;
        let matches = decode_json_content(&s[i+1..end-1],&mut decoded_key).map_err(|_|())? == key;
        i = ws(s,end);
        if s.get(i) != Some(&b':') { return Err(()); }
        i = ws(s,i+1);
        let end = value_end(s,i,0).ok_or(())?;
        if matches {
            // Missing is Ok(None); duplicate/ambiguous is an error, including
            // optional errorResponse. Never turn an error into "no error".
            if found.is_some() { return Err(()); }
            found = Some(&s[i..end]);
        }
        i = ws(s,end);
        if s.get(i) == Some(&b'}') { break; }
        if s.get(i) != Some(&b',') { return Err(()); }
        i = ws(s,i+1);
    }
    Ok(found)
}
fn empty_array(value: &[u8]) -> bool {
    value.first() == Some(&b'[') && value.last() == Some(&b']') && ws(value,1) == value.len()-1
}

pub fn detail<'a>(api_json: &'a [u8], manga_id: &[u8]) -> Option<DetailReason<'a>> {
    core::str::from_utf8(api_json).ok()?;
    if value_end(api_json,0,0).map(|end|ws(api_json,end)) != Some(api_json.len()) { return None; }
    if member_once(api_json,b"errorResponse").ok()?.is_some() { return None; }
    let response = member_once(api_json,b"response").ok()??;
    if member_once(response,b"mangaId").ok()?? != manga_id { return None; }
    let quoted = member_once(response,b"mangaHideReason").ok()??;
    if quoted.first() != Some(&b'"') || quoted.last() != Some(&b'"') { return None; }
    let raw = &quoted[1..quoted.len()-1];
    let mut decoded = [0u8;1024];
    let reason = decode_json_content(raw,&mut decoded).ok()?;
    if reason.is_empty() { return None; }
    // This classification is grounded in the five live public getDetail
    // responses. Other reasons remain visible text without guessed semantics.
    let blocked = reason == "抱歉,该漫画已经被屏蔽".as_bytes();
    let all_empty = [b"mangaEpisode".as_slice(),b"mangaWords",b"mangaRolls"].iter()
        .all(|key|member_once(response,key).ok().flatten().map_or(false,empty_array));
    Some(DetailReason { raw, confirmed_blocked_empty:blocked && all_empty })
}
