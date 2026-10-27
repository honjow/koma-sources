//! Fixed Keiyoushi949dea content-rating preference for the independent Chinese
//! sources. Defaults retain their prior safe/suggestive public catalogue.
use koma_source_sdk::json_utils::{json_object_member,write_bytes};
const VALUES: [&[u8];4] = [b"safe",b"suggestive",b"erotica",b"pornographic"];
fn mask(req:&[u8], key:&[u8])->u8 {
    let Some(raw)=json_object_member(req,b"settings").and_then(|s|json_object_member(s,key)) else{return 3};
    let Some(inner)=raw.trim_ascii().strip_prefix(b"[").and_then(|v|v.strip_suffix(b"]")) else{return 3};
    if inner.trim_ascii().is_empty(){return 0;}
    let mut result=0;
    for item in inner.split(|b|*b==b','){
        let Some(value)=item.trim_ascii().strip_prefix(b"\"").and_then(|v|v.strip_suffix(b"\"")) else{return 3};
        let Some(index)=VALUES.iter().position(|v|*v==value) else{return 3};
        result|=1<<index;
    }
    result
}
pub fn append_query(out:&mut [u8],cursor:&mut usize,req:&[u8],key:&[u8])->bool {
    let selection=mask(req,key);
    for (i,value) in VALUES.iter().enumerate(){
        if selection&(1<<i)!=0 && !(write_bytes(out,cursor,b"&contentRating%5B%5D=")&&write_bytes(out,cursor,value)){return false;}
    }
    true
}
pub fn accepts(req:&[u8],key:&[u8],rating:&[u8])->bool {
    let selection=mask(req,key);
    // An explicitly empty reference set omits the API filter. Keep known API
    // ratings in that case; unknown/missing data never broadens the old guard.
    VALUES.iter().enumerate().any(|(i,value)|*value==rating && (selection==0||selection&(1<<i)!=0))
}
