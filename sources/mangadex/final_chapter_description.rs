//! Fixed949dea MangaDexHelper final chapter description, shared by three languages.
//! Keep Koma's existing absent/default description; append only on explicit opt-in.
use koma_source_sdk::json_utils::{json_object_member, write_bytes};
fn value<'a>(attributes: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, ()> {
    let Some(raw) = json_object_member(attributes, key) else { return Ok(None) };
    if raw == b"null" { return Ok(None) }
    let text = raw.strip_prefix(b"\"").and_then(|v|v.strip_suffix(b"\"")).ok_or(())?;
    Ok(if text.is_empty() { None } else { Some(text) })
}
pub fn enabled(req: &[u8], key: &[u8]) -> bool {
    json_object_member(req, b"settings").and_then(|s|json_object_member(s,key))
        .is_some_and(|v|v==b"true" || v==br#""true""#)
}
pub fn has_values(attributes: &[u8]) -> bool {
    // Invalid non-string fields must reach append's error, rather than vanish.
    [b"lastVolume".as_slice(),b"lastChapter"].iter().any(|key| !matches!(value(attributes,key),Ok(None)))
}
pub fn append(out: &mut [u8], n: &mut usize, attributes: &[u8], has_description: bool) -> bool {
    let (Ok(volume),Ok(chapter))=(value(attributes,b"lastVolume"),value(attributes,b"lastChapter")) else { return false };
    if volume.is_none() && chapter.is_none() { return true }
    if has_description && !write_bytes(out,n,b"\\n\\n") { return false }
    if !write_bytes(out,n,b"Final chapter:\\n") { return false }
    if let Some(v)=volume {
        if !write_bytes(out,n,b"Vol.") || !koma_source_sdk::plain_text::append_entity_decoded_json_content(out,n,v) { return false }
    }
    if let Some(c)=chapter {
        if volume.is_some() && !write_bytes(out,n,b" ") { return false }
        if !write_bytes(out,n,b"Ch.") || !koma_source_sdk::plain_text::append_entity_decoded_json_content(out,n,c) { return false }
    }
    true
}
