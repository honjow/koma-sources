// Local RPC over the existing host import. Failed/invalid processing preserves
// the source title; the optional built-in cleaner still runs independently.
use super::*;

pub fn supported(req: &[u8]) -> bool {
    member(req,b"hostFeatures").ok().flatten().and_then(|v| Array::new(v).ok()).map_or(false,|mut a| {
        while let Ok(Some(v))=a.next() { if v==br#""source_java_regex_v1""# {return true;} } false
    })
}

pub fn pattern(req: &[u8]) -> Option<String> {
    if !supported(req) { return None; }
    let raw=optional_text(json_object_member(req,b"settings")?,b"REMOVE_TITLE_CUSTOM").ok()??;
    let mut out=alloc::vec![0;raw.len()];
    let text=core::str::from_utf8(unescape(raw,&mut out).ok()?).ok()?;
    if text.is_empty() {None} else {Some(text.to_string())}
}

pub fn remove(pattern: &str, titles: &[String]) -> Option<Vec<String>> {
    let mut req=alloc::vec![0u8;65536];let mut n=0;
    bytes(&mut req,&mut n,br#"{"type":"java_regex","version":1,"op":"remove","pattern":"#).ok()?;
    q(&mut req,&mut n,pattern.as_bytes()).ok()?;
    bytes(&mut req,&mut n,br#", "texts":["#).ok()?;
    for (i,title) in titles.iter().enumerate() {if i>0 {bytes(&mut req,&mut n,b",").ok()?;}q(&mut req,&mut n,title.as_bytes()).ok()?;}
    bytes(&mut req,&mut n,b"]}").ok()?;
    let size=host::http_request(&req[..n],http_out()).ok()?;
    let response=&http_out()[..size];
    if member(response,b"ok").ok()?? != b"true" {return None;}
    let data=member(response,b"data").ok()??;
    if member(data,b"valid").ok()?? != b"true" {return None;}
    let mut arr=Array::new(member(data,b"texts").ok()??).ok()?;let mut result=Vec::new();
    while let Some(value)=arr.next().ok()? {let raw=quoted(value).ok()?;let mut buf=alloc::vec![0;raw.len()];let value=core::str::from_utf8(unescape(raw,&mut buf).ok()?).ok()?;result.push(value.to_string());}
    if result.len()!=titles.len() {return None;} Some(result)
}

pub fn decode_title(title: &[u8]) -> Option<String> {
    let raw=quoted(title).ok()?;let mut out=alloc::vec![0;raw.len()];
    Some(core::str::from_utf8(unescape(raw,&mut out).ok()?).ok()?.to_string())
}
