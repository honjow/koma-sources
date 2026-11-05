//! Reference browse filters; kept independent from existing keyword search.
use super::{gql_body_buf, json_object_member, post_graphql, strict_json};
use koma_source_sdk::json_utils::{append_json_escaped, append_json_unescaped_then_escaped, extract_json_string, write_bytes, write_usize, JsonArrayIter};

const POPULAR: &[u8] = super::QUERY_POPULAR;
const LATEST: &[u8] = b"query recentUpdate($pagination: Pagination!) { comics: recentUpdate(pagination: $pagination) { id title description status imageUrl authors { id name } categories { id name } } }";
const FILTERED: &[u8] = b"query comicByCategories($categoryId: [ID!]!, $pagination: Pagination!) { comics: comicByCategories(categoryId: $categoryId, pagination: $pagination) { id title description status imageUrl authors { id name } categories { id name } } }";
const CATEGORIES: &[u8] = b"query { allCategory { id name } }";

pub fn feature(req: &[u8], name: &[u8]) -> bool {
    let Some(raw) = json_object_member(req, b"hostFeatures") else { return false };
    let raw = raw.trim_ascii();
    if raw.first() != Some(&b'[') || raw.last() != Some(&b']') { return false; }
    raw[1..raw.len()-1].split(|b| *b == b',').any(|item| {
        let item = item.trim_ascii();
        item.len() == name.len()+2 && item.first()==Some(&b'"') && item.last()==Some(&b'"') && &item[1..item.len()-1]==name
    })
}
fn scalar<'a>(obj: &'a [u8], key: &[u8]) -> Result<&'a [u8], ()> {
    let Some(v) = json_object_member(obj,key) else { return Ok(b"") };
    if v.len()<2 || v.first()!=Some(&b'"') || v.last()!=Some(&b'"') { return Err(()); }
    Ok(&v[1..v.len()-1])
}
fn category_array(raw: &[u8]) -> Result<usize, ()> {
    if !strict_json::valid(raw) || raw.first()!=Some(&b'[') || raw.last()!=Some(&b']') { return Err(()); }
    let inside=raw[1..raw.len()-1].trim_ascii();
    if inside.is_empty(){return Ok(0)}
    let mut count=0usize;
    for value in inside.split(|b|*b==b',') {
        let v=value.trim_ascii();
        if v.len()<3 || v.len()>18 || v.first()!=Some(&b'"') || v.last()!=Some(&b'"') || !v[1..v.len()-1].iter().all(u8::is_ascii_digit) || v[1..v.len()-1].iter().all(|b|*b==b'0') {return Err(())}
        count+=1;if count>128{return Err(())}
    }
    Ok(count)
}
// Requests always use provider PAGE_SIZE=30; App's limit=20 is a hint and
// cursor numbers refer to provider pages. Nonempty keyword search stays separate.
pub fn build_list(req:&[u8],page:usize)->Result<usize,()> {
    let args=json_object_member(req,b"args").unwrap_or(req);
    let listing=scalar(args,b"listingId")?;
    let latest=match listing {b""|b"popular"|b"listing:popular"=>false,b"latest"|b"listing:latest"=>true,_=>return Err(())};
    let filters=if feature(req,b"source_filter_scope_header_v1") {json_object_member(args,b"filters").unwrap_or(b"{}")}else{b"{}"};
    if !strict_json::valid(filters)||filters.first()!=Some(&b'{'){return Err(())}
    let categories=json_object_member(filters,b"categories").unwrap_or(b"[]");let cat_count=category_array(categories)?;
    let sort=scalar(filters,b"sort")?;
    if !matches!(sort,b""|b"DATE_UPDATED"|b"MONTH_VIEWS"|b"VIEWS"|b"FAVORITE_COUNT"){return Err(())}
    let status=scalar(filters,b"status")?;if !matches!(status,b""|b"ONGOING"|b"END"){return Err(())}
    let rating=scalar(filters,b"rating")?;if !matches!(rating,b""|b"0"|b"1"|b"2"|b"3"|b"4"|b"5"){return Err(())}
    let filtered=cat_count>0 || !sort.is_empty() || !status.is_empty() || !rating.is_empty();
    let query=if filtered{FILTERED}else if latest{LATEST}else{POPULAR};
    let order: &[u8]=if !sort.is_empty(){sort}else if filtered||latest{b"DATE_UPDATED"}else{b"MONTH_VIEWS"};
    let dst=gql_body_buf();let mut n=0;let offset=page.checked_sub(1).and_then(|v|v.checked_mul(30)).ok_or(())?;
    if !(write_bytes(dst,&mut n,br#"{"query":""#) && append_json_escaped(dst,&mut n,query)
      && write_bytes(dst,&mut n,br#"","variables":{"pagination":{"offset":"#) && write_usize(dst,&mut n,offset)
      && write_bytes(dst,&mut n,br#","orderBy":""#) && write_bytes(dst,&mut n,order)
      && write_bytes(dst,&mut n,br#"","status":""#) && write_bytes(dst,&mut n,status)
      && write_bytes(dst,&mut n,br#"","asc":false,"limit":30,"sexyLevel":"#)
      && write_bytes(dst,&mut n,if rating.is_empty(){b"null"}else{rating})
      && write_bytes(dst,&mut n,br#"},"categoryId":"#) && write_bytes(dst,&mut n,categories)
      && write_bytes(dst,&mut n,b"}}")) { return Err(()); }
    Ok(n)
}
fn schema(req:&[u8],response:Option<&[u8]>)->Result<usize,()> {
    let enabled=feature(req,b"source_filter_scope_header_v1") && feature(req,b"source_filter_fetching_v1");
    if !enabled {let out=super::payload_buf();out[..14].copy_from_slice(b"{\"filters\":[]}");return Ok(14)}
    let out=super::payload_buf();let mut n=0;
    if !write_bytes(out,&mut n,"{\"supportsFilterFetching\":true,\"filters\":[{\"type\":\"header\",\"name\":\"篩選條件（搜索關鍵字時無效）\",\"scope\":\"browse\"},{\"id\":\"categories\",\"name\":\"類型（同時包含全部所選類型）\",\"type\":\"multiselect\",\"scope\":\"browse\"".as_bytes()){return Err(())}
    if let Some(response)=response {
        let data=json_object_member(response,b"data").ok_or(())?;let raw=json_object_member(data,b"allCategory").ok_or(())?;let expected=strict_json::object_count(raw).ok_or(())?;
        if expected>512{return Err(())}let mut iter=JsonArrayIter::from_raw_array(raw).ok_or(())?;let mut count=0usize;
        if !write_bytes(out,&mut n,b",\"options\":["){return Err(())}
        while let Some(obj)=iter.next_object(){let id=extract_json_string(obj,b"id").ok_or(())?;let name=extract_json_string(obj,b"name").ok_or(())?;
            if id.is_empty()||id.len()>16||!id.iter().all(u8::is_ascii_digit)||name.is_empty(){return Err(())}
            if count>0&&!write_bytes(out,&mut n,b","){return Err(())}
            if !(write_bytes(out,&mut n,b"{\"value\":\"")&&write_bytes(out,&mut n,id)&&write_bytes(out,&mut n,b"\",\"label\":\"")&&append_json_unescaped_then_escaped(out,&mut n,name)&&write_bytes(out,&mut n,b"\"}")){return Err(())}count+=1;
        }
        if count!=expected||!write_bytes(out,&mut n,b"]"){return Err(())}
    }
    let rest="},{\"id\":\"sort\",\"name\":\"排序\",\"type\":\"select\",\"scope\":\"browse\",\"options\":[{\"value\":\"DATE_UPDATED\",\"label\":\"更新\"},{\"value\":\"MONTH_VIEWS\",\"label\":\"本月觀看數（不能篩選類型）\"},{\"value\":\"VIEWS\",\"label\":\"觀看數\"},{\"value\":\"FAVORITE_COUNT\",\"label\":\"喜愛數\"}]},{\"id\":\"status\",\"name\":\"狀態\",\"type\":\"select\",\"scope\":\"browse\",\"options\":[{\"value\":\"\",\"label\":\"全部\"},{\"value\":\"ONGOING\",\"label\":\"連載\"},{\"value\":\"END\",\"label\":\"完結\"}]},{\"id\":\"rating\",\"name\":\"色氣程度\",\"type\":\"select\",\"scope\":\"browse\",\"options\":[{\"value\":\"\",\"label\":\"全部\"},{\"value\":\"0\",\"label\":\"無\"},{\"value\":\"1\",\"label\":\"1\"},{\"value\":\"2\",\"label\":\"2\"},{\"value\":\"3\",\"label\":\"3\"},{\"value\":\"4\",\"label\":\"≥4\"},{\"value\":\"5\",\"label\":\"5\"}]}]}";
    if !write_bytes(out,&mut n,rest.as_bytes()){return Err(())}Ok(n)
}
pub fn run_filters(req:&[u8])->u32 {
    let enabled=feature(req,b"source_filter_scope_header_v1") && feature(req,b"source_filter_fetching_v1");
    let args=json_object_member(req,b"args").unwrap_or(req);
    let fetch=enabled && json_object_member(args,b"fetchFilterData")==Some(b"true".as_slice());
    let response=if fetch {
        let b=gql_body_buf();let mut n=0;
        if !(write_bytes(b,&mut n,br#"{"query":""#)&&append_json_escaped(b,&mut n,CATEGORIES)&&write_bytes(b,&mut n,br#"","variables":{}}"#)) {return super::write_error("get_filters","internal_error","filter request overflow")}
        match post_graphql(req,&b[..n]) {Ok(v)=>Some(v),Err(e)=>{let(c,m)=super::fetch_error_code(e);return super::write_error("get_filters",c,m)}}
    }else{None};
    match schema(req,response){Ok(n)=>super::write_success_payload("get_filters",n),Err(())=>super::write_error("get_filters","source_error","filter data unavailable")}
}
