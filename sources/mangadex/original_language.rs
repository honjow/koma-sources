//! Fixed Keiyoushi949dea original-language preference. This is unrelated to
//! the translated chapter language; only public browse/search queries consume it.
use koma_source_sdk::json_utils::json_object_member;
fn mask(req: &[u8], key: &[u8]) -> u8 {
    let Some(raw) = json_object_member(req, b"settings").and_then(|s|json_object_member(s,key)) else { return 0; };
    let Some(inner) = raw.trim_ascii().strip_prefix(b"[").and_then(|v|v.strip_suffix(b"]")) else { return 0; };
    if inner.trim_ascii().is_empty() { return 0; }
    let mut result = 0;
    for item in inner.split(|b| *b==b',') {
        let item=item.trim_ascii();
        // Values are the three fixed ASCII option IDs. Never interpolate raw
        // preference content into URLs, or mistake a substring for a selection.
        result |= match item { b"\"ja\""=>1,b"\"zh\""=>2,b"\"ko\""=>4,_=>0 };
    }
    result
}
pub fn query(req: &[u8], key: &[u8], search: bool) -> &'static [u8] {
    // The reference search filter writes zh-hk immediately before zh; the
    // preference-backed popular/latest set adds zh-hk after its chosen values.
    match (mask(req,key), search) {
        (1,_)=>b"&originalLanguage%5B%5D=ja",
        (2,false)=>b"&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=zh-hk",
        (2,true)=>b"&originalLanguage%5B%5D=zh-hk&originalLanguage%5B%5D=zh",
        (3,false)=>b"&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=zh-hk",
        (3,true)=>b"&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=zh-hk&originalLanguage%5B%5D=zh",
        (4,_)=>b"&originalLanguage%5B%5D=ko",
        (5,_)=>b"&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=ko",
        (6,false)=>b"&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=ko&originalLanguage%5B%5D=zh-hk",
        (6,true)=>b"&originalLanguage%5B%5D=zh-hk&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=ko",
        (7,false)=>b"&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=ko&originalLanguage%5B%5D=zh-hk",
        (7,true)=>b"&originalLanguage%5B%5D=ja&originalLanguage%5B%5D=zh-hk&originalLanguage%5B%5D=zh&originalLanguage%5B%5D=ko",
        _=>b"",
    }
}
