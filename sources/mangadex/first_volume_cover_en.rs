//! English MangaDex first-volume cover selection based on Keiyoushi949dea.
//! Optional public metadata only: failures keep original covers.
use koma_source_sdk::json_utils::{json_object_member as member, JsonArrayIter, write_bytes, write_url_encoded, write_usize};
const MAX: usize = 100;
const BATCH: usize = 20;
#[derive(Clone, Copy)]
struct Entry { id: [u8;36], locale: [u8;64], locale_len: usize, file: [u8;256], file_len: usize, matched: bool }
const EMPTY: Entry = Entry { id:[0;36],locale:[0;64],locale_len:0,file:[0;256],file_len:0,matched:false };
pub struct Covers { entries: [Entry;MAX], len: usize }
fn string<'a>(v:&'a [u8], k:&[u8])->Option<&'a [u8]> { member(v,k)?.strip_prefix(b"\"")?.strip_suffix(b"\"") }
fn uuid(v:&[u8])->bool { v.len()==36 && v.iter().enumerate().all(|(i,c)| if matches!(i,8|13|18|23){*c==b'-'}else{c.is_ascii_hexdigit()}) }
fn filename(v:&[u8])->bool { !v.is_empty() && v.len()<=256 && v!=b"." && v!=b".." && v.iter().all(|c|c.is_ascii_alphanumeric() || matches!(c,b'.'|b'_'|b'-')) }
impl Covers {
    fn empty()->Self { Self{entries:[EMPTY;MAX],len:0} }
    fn add(&mut self, obj:&[u8]) {
        let Some(id)=string(obj,b"id").filter(|v|uuid(v)) else{return};
        let Some(locale)=member(obj,b"attributes").and_then(|a|string(a,b"originalLanguage")).filter(|v|!v.is_empty()&&v.len()<=64&&v.iter().all(|c|c.is_ascii_alphanumeric()||*c==b'-')) else{return};
        // LinkedHashMap associate semantics: last duplicate attributes, original order.
        let pos=self.entries[..self.len].iter().position(|e|e.id==id).unwrap_or(self.len);
        if pos==MAX{return};
        self.entries[pos].id.copy_from_slice(id);
        self.entries[pos].locale[..locale.len()].copy_from_slice(locale);
        self.entries[pos].locale_len=locale.len();
        if pos==self.len{self.len+=1;}
    }
    pub fn file<'a>(&'a self,id:&[u8])->Option<&'a [u8]> {
        self.entries[..self.len].iter().find(|e|e.id==id&&e.file_len>0).map(|e|&e.file[..e.file_len])
    }
}
pub fn load(req:&[u8],key:&[u8],json:&[u8],fetch:impl FnMut(&[u8])->Option<&'static [u8]>)->Covers {
    load_selected(req,key,json,None,fetch)
}
pub fn load_selected(req:&[u8],key:&[u8],json:&[u8],selected:Option<&[[u8;36]]>,mut fetch:impl FnMut(&[u8])->Option<&'static [u8]>)->Covers {
    let mut result=Covers::empty();
    if !matches!(member(req,b"settings").and_then(|s|member(s,key)),Some(b"true"|b"\"true\"")){return result;}
    let Some(data)=member(json,b"data") else{return result};
    if data.first()==Some(&b'{'){result.add(data)}else if let Some(mut it)=JsonArrayIter::from_raw_array(data){
        let mut count=0;
        while let Some(obj)=it.next_object(){
            if let Some(ids)=selected { if !string(obj,b"id").map_or(false,|id|ids.iter().any(|v|v.as_slice()==id)){continue;} }
            if count==MAX{break;}result.add(obj);count+=1;
        }
    }
    if result.len==0{return result;}
    // Keep the original <=20 request unchanged. Larger latest pages use the
    // same bounded query per batch so every item gets the same cover rule.
    for batch in result.entries[..result.len].chunks_mut(BATCH) {
        let mut url=[0;8192];let mut n=0;let mut locales=0;
        let mut ok=write_bytes(&mut url,&mut n,b"https://api.mangadex.org/cover?order%5Bvolume%5D=asc");
        for e in batch.iter(){ok=ok&&write_bytes(&mut url,&mut n,b"&manga%5B%5D=")&&write_bytes(&mut url,&mut n,&e.id);}
        for (i,e) in batch.iter().enumerate(){
            let locale=&e.locale[..e.locale_len];
            if batch[..i].iter().any(|p|&p.locale[..p.locale_len]==locale){continue;}
            locales+=1;ok=ok&&write_bytes(&mut url,&mut n,b"&locales%5B%5D=")&&write_url_encoded(&mut url,&mut n,locale);
        }
        ok=ok&&write_bytes(&mut url,&mut n,b"&limit=")&&write_usize(&mut url,&mut n,(batch.len()*locales).min(100))&&write_bytes(&mut url,&mut n,b"&offset=0");
        if !ok{continue;}
        let Some(body)=fetch(&url[..n]) else{continue};
        if member(body,b"result")!=Some(b"\"ok\""){continue;}
        let Some(mut it)=member(body,b"data").and_then(JsonArrayIter::from_raw_array) else{continue};
        while let Some(cover)=it.next_object(){
            let Some(a)=member(cover,b"attributes") else{continue};
            let Some(locale)=string(a,b"locale") else{continue};
            let Some(mut rels)=member(cover,b"relationships").and_then(JsonArrayIter::from_raw_array) else{continue};
            let mut owner=None;
            while let Some(rel)=rels.next_object(){if string(rel,b"type")==Some(b"manga"){owner=string(rel,b"id");break;}}
            let Some(e)=batch.iter_mut().find(|e|Some(e.id.as_slice())==owner) else{continue};
            if e.matched || locale!=&e.locale[..e.locale_len]{continue;}
            // Reference find() chooses the first locale match. An invalid first
            // filename falls back to the original, not a later volume.
            e.matched=true;
            if let Some(file)=string(a,b"fileName").filter(|v|filename(v)){e.file[..file.len()].copy_from_slice(file);e.file_len=file.len();}
        }
    }
    result
}
