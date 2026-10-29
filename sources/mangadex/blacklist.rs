//! MangaDex UUID blacklist is honored only with validated host settings.
//! Empty values preserve the previous requests. Exact chapter recovery must
//! bypass these filters so a previously opened chapter remains accessible.
use koma_source_sdk::json_utils::{json_object_member, extract_json_string, write_bytes};

fn feature(req: &[u8], name: &[u8]) -> bool {
    let Some(v) = json_object_member(req, b"hostFeatures") else { return false; };
    v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c == b',').any(|s| s.trim_ascii().strip_prefix(b"\"").and_then(|x|x.strip_suffix(b"\""))==Some(name))
}
pub fn supported(req: &[u8]) -> bool {
    feature(req,b"source_setting_uuid_list_v1") && feature(req,b"source_chapter_exclusions_v1")
}

fn valid(v: &[u8]) -> bool {
    v.len() == 36 && v.iter().enumerate().all(|(i,c)| match i {
        8|13|18|23 => *c == b'-', 14 => (b'1'..=b'5').contains(c),
        19 => matches!(*c,b'8'|b'9'|b'a'|b'b'),
        _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
    })
}
fn values<'a>(req: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    json_object_member(req,b"settings").and_then(|s| extract_json_string(s,key))
}
// Match JavaScript String.trim used by the validating host. JSON strings must
// be decoded before splitting, including escaped tabs and Unicode whitespace.
fn js_space(c:char)->bool { matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}') }
fn hex(v:u8)->Option<u32>{match v {b'0'..=b'9'=>Some((v-b'0') as u32),b'a'..=b'f'=>Some((v-b'a'+10) as u32),b'A'..=b'F'=>Some((v-b'A'+10) as u32),_=>None}}
fn code(s:&[u8])->Option<u32>{if s.len()!=4{return None} let mut n=0;for c in s {n=n*16+hex(*c)?}Some(n)}
fn decoded<'a>(raw:&[u8],out:&'a mut [u8])->Option<&'a str>{
 let mut i=0;let mut n=0;
 while i<raw.len() {
  let c=raw[i];i+=1;
  if c!=b'\\' { if c<32||c==b'"'||!write_bytes(out,&mut n,&[c]) {return None} continue; }
  let escape=*raw.get(i)?;i+=1;
  let cp=match escape {b'"'=>34,b'\\'=>92,b'/'=>47,b'b'=>8,b'f'=>12,b'n'=>10,b'r'=>13,b't'=>9,b'u'=>{
   let mut v=code(raw.get(i..i+4)?)?;i+=4;
   if (0xd800..=0xdbff).contains(&v) {if raw.get(i..i+2)!=Some(b"\\u"){return None} i+=2;let low=code(raw.get(i..i+4)?)?;i+=4;if !(0xdc00..=0xdfff).contains(&low){return None}v=0x10000+((v-0xd800)<<10)+(low-0xdc00)} else if (0xdc00..=0xdfff).contains(&v){return None} v
  },_=>return None};
  let ch=char::from_u32(cp)?;let mut tmp=[0;4];if !write_bytes(out,&mut n,ch.encode_utf8(&mut tmp).as_bytes()){return None}
 }
 core::str::from_utf8(&out[..n]).ok()
}
pub fn active(req: &[u8], groups: &[u8], uploaders: &[u8]) -> bool {
    if !supported(req) {return false}
    [groups,uploaders].iter().any(|k| values(req,k).map(|v| { let mut tmp=[0u8;32768]; decoded(v,&mut tmp).map(|s|!s.trim_matches(js_space).is_empty()).unwrap_or(true) }).unwrap_or(false))
}
pub fn append(out: &mut [u8], pos: &mut usize, req: &[u8], groups: &[u8], uploaders: &[u8]) -> bool {
    if !supported(req) { return true; }
    for (key,param) in [(groups,b"&excludedGroups%5B%5D=".as_slice()),(uploaders,b"&excludedUploaders%5B%5D=".as_slice())] {
        let Some(raw) = values(req,key) else { continue; };
        let mut tmp=[0u8;32768];
        let Some(value)=decoded(raw,&mut tmp) else{return false};
        if value.trim_matches(js_space).is_empty() { continue; }
        let mut ids: [&[u8]; 512] = [&[]; 512]; let mut count=0;
        for item in value.split(',') {
            let id=item.trim_matches(js_space).as_bytes();
            if !valid(id) { return false; }
            if ids[..count].contains(&id) { continue; }
            if count == ids.len() { return false; }
            ids[count]=id; count+=1;
        }
        ids[..count].sort_unstable();
        for i in 0..count {
            if i>0 && ids[i]==ids[i-1] { continue; }
            if !write_bytes(out,pos,param) || !write_bytes(out,pos,ids[i]) { return false; }
        }
    }
    true
}

pub fn contains(req:&[u8], key:&[u8], id:&[u8])->bool {
    let Some(raw)=values(req,key) else {return false}; let mut tmp=[0u8;32768];
    decoded(raw,&mut tmp).map(|s|s.split(',').any(|v|v.trim_matches(js_space).as_bytes()==id)).unwrap_or(false)
}
