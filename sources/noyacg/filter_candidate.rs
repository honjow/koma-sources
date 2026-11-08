//! Local opt-in candidate: sentinel IDs preserve empty upstream form values.
use super::*;
const SCHEMA: &str = r#"{"filters":[{"id":"mode","name":"類型","kind":"select","default":"default","options":[{"value":"default","label":"預設"},{"value":"tag","label":"標籤"},{"value":"author","label":"作者"}]},{"id":"sort","name":"排序方式","kind":"select","default":"time","options":[{"value":"time","label":"時間"},{"value":"views","label":"閲讀"},{"value":"favorites","label":"收藏"},{"value":"rating","label":"評分"}]},{"id":"finished","name":"完結狀態","kind":"select","default":"all","options":[{"value":"all","label":"全部"},{"value":"false","label":"連載中"},{"value":"true","label":"已完結"}]}]}"#;
pub fn write_schema() -> u32 { let b=SCHEMA.as_bytes();payload_buf()[..b.len()].copy_from_slice(b);write_success_payload("get_filters",b.len()) }
fn read<'a>(filters:Option<&'a [u8]>,key:&[u8])->Option<Option<&'a [u8]>> {
 match filters.and_then(|f|json_object_member(f,key)) {None=>Some(None),Some(v)=>Some(Some(v.strip_prefix(b"\"")?.strip_suffix(b"\"")?))}
}
pub fn selected<'a>(req:&'a [u8], legacy_mode:&'a [u8])->Option<(&'a [u8], &'a [u8], &'a [u8])> {
 let filters=json_object_member(req,b"args").and_then(|a|json_object_member(a,b"filters"));
 if filters.is_some_and(|f|!f.starts_with(b"{") || !f.ends_with(b"}")){return None}
 let mode=read(filters,b"mode")?.unwrap_or(legacy_mode);let sort=read(filters,b"sort")?.unwrap_or(b"time");let finished=read(filters,b"finished")?.unwrap_or(b"all");
 if !matches!(mode,b"default"|b"tag"|b"author") || !matches!(sort,b"time"|b"views"|b"favorites"|b"rating") || !matches!(finished,b"all"|b"false"|b"true") {return None}
 Some((mode,if sort==b"time" {b""} else {sort},if finished==b"all" {b""} else {finished}))
}
