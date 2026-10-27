//! Fixed949dea SOURCE_INCLUDE: filter only a completely validated catalogue.
//! The old empty/default branch and exact retained chapter restoration stay raw.
use alloc::{collections::BTreeMap,string::{String,ToString},vec::Vec};
use regex::Regex;
use koma_source_sdk::json_utils::{json_object_member,append_json_escaped};
use super::{E,unescape};
pub fn supported(req:&[u8])->bool {
    let Some(raw)=json_object_member(req,b"hostFeatures") else{return false};
    raw.starts_with(b"[")&&raw.ends_with(b"]")&&raw[1..raw.len()-1].split(|b|*b==b',').any(|v|v.trim_ascii()==br#""source_chapter_exclusions_v1""#)
}
fn trim(s:&str)->&str {
    s.trim_matches(|c:char|(c.is_whitespace()&&c!='\u{85}')||('\u{1c}'..='\u{1f}').contains(&c))
}
pub fn selection(req:&[u8])->Result<Vec<String>,E> {
    if !supported(req){return Ok(Vec::new());}
    let Some(raw)=json_object_member(req,b"settings").and_then(|v|json_object_member(v,b"SOURCE_INCLUDE")) else{return Ok(Vec::new())};
    let content=raw.strip_prefix(b"\"").and_then(|v|v.strip_suffix(b"\"")).ok_or("invalid edition selection")?;
    if content.len()>8192{return Err("edition selection capacity");}
    let mut value=alloc::vec![0;content.len()];let value=unescape(content,&mut value)?;
    let value=core::str::from_utf8(value).map_err(|_|"invalid edition selection UTF8")?;
    let mut selected=Vec::new();
    for label in value.split(','){let label=trim(label).to_lowercase();if !label.is_empty()&&!selected.contains(&label){selected.push(label);}}
    Ok(selected)
}
fn decode_once(value:&str)->Result<String,E> {
    if !value.contains('&'){return Ok(value.to_string());}
    let cap=value.len().checked_mul(6).and_then(|v|v.checked_add(8)).ok_or("edition label capacity")?;
    let mut raw=alloc::vec![0;cap];let mut n=0;
    if !append_json_escaped(&mut raw,&mut n,value.as_bytes()){return Err("edition label capacity");}
    let mut decoded=alloc::vec![0;cap];let mut len=0;
    if !koma_source_sdk::plain_text::append_entity_decoded_json_content(&mut decoded,&mut len,&raw[..n]){return Err("edition label entity decode");}
    let mut plain=alloc::vec![0;len];let size=unescape(&decoded[..len],&mut plain)?.len();plain.truncate(size);
    String::from_utf8(plain).map_err(|_|"invalid edition label UTF8")
}
pub struct Normalizer { spaces:Regex,cache:BTreeMap<(String,bool),String> }
impl Normalizer {
    pub fn new()->Result<Self,E> {
        // Java Regex("\\s+") is ASCII by default, unlike a Unicode \s class.
        Ok(Self{spaces:Regex::new(r"[\x09-\x0d ]+").map_err(|_|"edition whitespace pattern")?,cache:BTreeMap::new()})
    }
    fn once(&self,value:&str)->Result<String,E> {
        let decoded=decode_once(value)?;let collapsed=self.spaces.replace_all(&decoded," ");Ok(trim(&collapsed).to_string())
    }
    fn cached(&mut self,raw:&str,edition:bool)->Result<String,E> {
        let key=(raw.to_string(),edition);if let Some(v)=self.cache.get(&key){return Ok(v.clone());}
        let mut result=self.once(raw)?;
        if edition {
            // resolveTargetComics, toSChapter, then disambiguateEditionLabels.
            result=self.once(&result)?;
        } else if let Some(first)=result.chars().next(){
            // Kotlin replaceFirstChar sees a UTF-16 Char. A supplementary first
            // codepoint is a surrogate there and is deliberately left intact.
            if first.len_utf16()==1&&first.is_lowercase(){
                let mut titled=String::new();for c in unicode_titlecase::to_titlecase(first){if c!='\0'{titled.push(c);}}
                titled.push_str(&result[first.len_utf8()..]);result=titled;
            }
        }
        result=self.once(&result)?;self.cache.insert(key,result.clone());Ok(result)
    }
    pub fn edition(&mut self,raw:&[u8])->Result<String,E> {
        self.cached(core::str::from_utf8(raw).map_err(|_|"invalid edition label UTF8")?,true)
    }
    pub fn uploader(&mut self,raw_json_content:&[u8])->Result<String,E> {
        let mut plain=alloc::vec![0;raw_json_content.len()];let value=unescape(raw_json_content,&mut plain)?;
        self.cached(core::str::from_utf8(value).map_err(|_|"invalid edition label UTF8")?,false)
    }
}
pub fn matches(selected:&[String],label:&str)->bool {
    if label.is_empty(){return false;}
    let lower=label.to_lowercase();selected.iter().any(|base|lower==*base||lower.strip_prefix(base).is_some_and(|rest|rest.starts_with(" [")))
}
