//! Host carries this data only within a single live detail transaction. The
//! source still re-fetches the title and rejects any missing/reordered edition.
use super::*;
fn supported(r:&[u8])->bool {
    let Some(v)=json_object_member(r,b"hostFeatures") else{return false};
    v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c|*c==b',').any(|v|v.trim_ascii()==br#""source_detail_chapter_context_v1""#)
}
pub fn emit(out:&mut [u8],n:&mut usize,req:&[u8],title:&[u8],work:&[u8],good:usize)->Result<(),E>{
    if !supported(req){return Ok(())}
    bytes(out,n,br#", "chapterContext":{"version":1,"sourceVersion":""#)?;
    bytes(out,n,SOURCE_INFO.version.as_bytes())?;bytes(out,n,br#"","work":""#)?;bytes(out,n,work)?;bytes(out,n,br#"","editions":["#)?;
    let mut a=Array::new(required(title,b"comic_ids")?)?;let mut count=0;
    while let Some(v)=a.next()? {
        let id=Identity::new(quoted(v)?)?;
        if count>0{bytes(out,n,b",")?}bytes(out,n,br#"{"id":""#)?;bytes(out,n,id.as_bytes())?;
        let edition=unsafe{EDITIONS[..good].iter().find(|e|e.id.as_bytes()==id.as_bytes())};
        if let Some(e)=edition {bytes(out,n,br#"","available":true,"label":"#)?;q(out,n,&e.label[..e.label_len])?;}
        else {bytes(out,n,br#"","available":false"#)?;}
        bytes(out,n,b"}")?;count+=1;
    }
    bytes(out,n,b"]}")
}
pub fn load(req:&[u8],title:&[u8],work:&[u8])->Option<usize>{
    if !supported(req){return None}
    let args=args(req).ok()?;if member(args,b"chapterId").ok()?.is_some(){return None}
    let ctx=member(args,b"chapterContext").ok()??;
    if member(ctx,b"version").ok()??!=b"1" || text(ctx,b"sourceVersion").ok()?!=SOURCE_INFO.version.as_bytes() || text(ctx,b"work").ok()?!=work {return None}
    let mut originals=Array::new(required(title,b"comic_ids").ok()?).ok()?;
    let mut sent=Array::new(required(ctx,b"editions").ok()?).ok()?;let mut total=0;let mut good=0;
    loop {let a=originals.next().ok()?;let b=sent.next().ok()?;match (a,b){(None,None)=>break,(Some(a),Some(b))=>{
        let ident=Identity::new(quoted(a).ok()?).ok()?;
        if text(b,b"id").ok()?!=ident.as_bytes() || total>=512{return None}
        unsafe{if IDS[..total].iter().any(|x|x.as_bytes()==ident.as_bytes()){return None} IDS[total]=ident;}
        total+=1;
        match member(b,b"available").ok()?? {
            b"false"=>{if member(b,b"label").ok()?.is_some(){return None}},
            b"true"=>{let mut item=NO_EDITION;item.id=ident;let raw=text(b,b"label").ok()?;item.label_len=unescape(raw,&mut item.label).ok()?.len();unsafe{EDITIONS[good]=item};good+=1;},
            _=>return None,
        }
    },_=>return None}}
    unsafe{EDITIONS[..good].sort_unstable_by(|a,b|a.id.as_bytes().cmp(b.id.as_bytes()));}
    Some(good)
}
