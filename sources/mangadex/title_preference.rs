//! Fixed949dea createBasicManga explicit false branch; original true/default
//! selection and encoder remain in their existing consumers.
use koma_source_sdk::json_utils::{json_object_member, JsonObjectStringIter, JsonArrayIter};
pub fn prefer(req: &[u8], key: &[u8]) -> bool {
    !json_object_member(req,b"settings").and_then(|s|json_object_member(s,key))
        .is_some_and(|v|v==b"false" || v==br#""false""#)
}
fn raw_string(raw: &[u8]) -> Option<&[u8]> {
    raw.strip_prefix(b"\"")?.strip_suffix(b"\"")
}
fn alternative<'a>(raw: &'a [u8], lang: &[u8]) -> Result<Option<&'a [u8]>, ()> {
    let Some(mut items)=JsonArrayIter::from_raw_array(raw) else {return Err(())};
    while let Some(item)=items.next_object() {
        if json_object_member(item,lang).is_some() {
            let mut values=JsonObjectStringIter::new(item).ok_or(())?;
            let first=values.next_string();
            // Fixed findTitleByLang uses first matching map, then singleOrNull;
            // a malformed multi-language map does not fall through to later maps.
            return Ok(if values.next_string().is_none(){first}else{None});
        }
    }
    Ok(None)
}
fn primary_first<'a>(attributes: &'a [u8], lang: &[u8]) -> Result<&'a [u8], ()> {
    let title=json_object_member(attributes,b"title").ok_or(())?;
    if let Some(v)=json_object_member(title,lang) {return raw_string(v).ok_or(())}
    let mut primary=JsonObjectStringIter::new(title).ok_or(())?;
    if let Some(v)=primary.next_string(){return Ok(v)}
    if let Some(alts)=json_object_member(attributes,b"altTitles") {
        if let Some(v)=alternative(alts,lang)? {return Ok(v)}
        if let Some(v)=alternative(alts,b"en")? {return Ok(v)}
    }
    Ok(b"")
}
pub fn append_primary_first(out: &mut [u8], n: &mut usize, attributes: &[u8], lang: &[u8]) -> bool {
    let Ok(raw)=primary_first(attributes,lang) else{return false};
    koma_source_sdk::plain_text::append_entity_decoded_json_content(out,n,raw)
}
