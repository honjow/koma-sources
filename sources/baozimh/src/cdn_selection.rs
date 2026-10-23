// Finite candidates declared by the saved twmanga page_runtime_v5 script.
// The host ABI has no timing/concurrency/KV: this is bounded sequential HEAD
// fallback in descending site level, retaining declaration order within a level.
const DEFAULT_SUFFIX: &[u8] = b".bzcdn.net";
const CANDIDATES: [&[u8]; 5] = [
    b"-ogsm1-uspho.bzcdn.net", b"-rsa1-usla.bzcdn.net", b"-mha1-nlams.bzcdn.net",
    b".baozimh.com", b"-dpa1-cnhk.bzcdn.net",
];

#[derive(Clone, Copy)]
struct Url<'a> {
    raw: &'a [u8],
    shard_end: usize,
    authority_end: usize,
    directory_end: usize,
}
impl Url<'_> {
    fn authority(&self) -> &[u8] { &self.raw[8..self.authority_end] }
    fn suffix(&self) -> &[u8] { &self.raw[self.shard_end..self.authority_end] }
    fn directory(&self) -> &[u8] { &self.raw[self.authority_end..self.directory_end] }
}
fn parse(raw: &[u8]) -> Option<Url<'_>> {
    let rest = raw.strip_prefix(b"https://s")?;
    let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || raw.iter().any(|c| *c <= 32 || *c == 127 || b"\\\"<>".contains(c)) { return None; }
    let shard_end = 9 + digits;
    let authority_end = raw[8..].iter().position(|c| *c == b'/')? + 8;
    if shard_end > authority_end { return None; }
    let suffix = &raw[shard_end..authority_end];
    if suffix != DEFAULT_SUFFIX && !CANDIDATES.contains(&suffix) { return None; }
    let path_end = raw[authority_end..].iter().position(|c| *c == b'?' || *c == b'#')
        .map(|n| n + authority_end).unwrap_or(raw.len());
    let directory_end = raw[authority_end..path_end].iter().rposition(|c| *c == b'/')? + authority_end + 1;
    if directory_end >= path_end { return None; }
    Some(Url { raw, shard_end, authority_end, directory_end })
}

pub struct Selection<'a> { original: Url<'a>, suffix: &'static [u8] }
pub struct Rewritten<'a> { pub prefix: &'a [u8], pub suffix: &'static [u8], pub tail: &'a [u8] }
impl Selection<'_> {
    pub fn rewrite<'a>(&self, raw: &'a [u8]) -> Option<Rewritten<'a>> {
        let url = parse(raw)?;
        // Keep other image hosts/shards and other chapters untouched.
        if url.authority() != self.original.authority() || url.directory() != self.original.directory() { return None; }
        Some(Rewritten { prefix: &raw[..url.shard_end], suffix: self.suffix, tail: &raw[url.authority_end..] })
    }
}

pub fn choose<'a, E>(first: &'a [u8], slug: &[u8], mut probe: impl FnMut(&[&[u8]]) -> Result<bool, E>)
    -> Result<Option<Selection<'a>>, E> {
    let original = match parse(first) { Some(value) => value, None => return Ok(None) };
    let owner = match original.directory().strip_prefix(b"/scomic/") { Some(value) => value, None => return Ok(None) };
    if !owner.starts_with(slug) || owner.get(slug.len()) != Some(&b'/') { return Ok(None); }
    if probe(&[first])? { return Ok(None); }
    for suffix in CANDIDATES {
        if suffix == original.suffix() { continue; }
        if probe(&[&first[..original.shard_end], suffix, &first[original.authority_end..]])? {
            return Ok(Some(Selection { original, suffix }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIRST: &[u8] = b"https://s1.bzcdn.net/scomic/book/0/2-tag/1.jpg?q=keep";
    #[test]
    fn default_success_does_not_probe_candidates() {
        let mut count=0;
        assert!(choose::<()>(FIRST,b"book", |_| {count+=1;Ok(true)}).unwrap().is_none());
        assert_eq!(count,1);
    }
    #[test]
    fn fallback_stops_at_first_success_and_keeps_owner_and_uri_parts() {
        let mut requests=Vec::new();
        let selected=choose::<()>(FIRST,b"book",|parts| {requests.push(parts.concat());Ok(requests.len()==3)}).unwrap().unwrap();
        assert_eq!(requests.len(),3);
        assert!(requests[1].starts_with(b"https://s1-ogsm1-uspho.bzcdn.net/"));
        assert!(requests[2].starts_with(b"https://s1-rsa1-usla.bzcdn.net/"));
        let rewritten=selected.rewrite(FIRST).unwrap();
        assert_eq!([rewritten.prefix,rewritten.suffix,rewritten.tail].concat(),requests[2]);
        for other in [b"https://s1.bzcdn.net/scomic/book/0/3-tag/1.jpg".as_slice(),b"https://s2.bzcdn.net/scomic/book/0/2-tag/1.jpg",b"https://s1-rsa1-usla.bzcdn.net/scomic/book/0/2-tag/1.jpg"] {
            assert!(selected.rewrite(other).is_none());
        }
    }
    #[test]
    fn all_failure_bounded_and_unknown_urls_never_probed() {
        let mut count=0;assert!(choose::<()>(FIRST,b"book",|_| {count+=1;Ok(false)}).unwrap().is_none());assert_eq!(count,6);
        for url in [b"http://s1.bzcdn.net/scomic/book/0/2-tag/1.jpg".as_slice(),b"https://s1.bzcdn.net.evil/scomic/book/0/2-tag/1.jpg",b"https://user@s1.bzcdn.net/scomic/book/0/2-tag/1.jpg",b"https://s1.bzcdn.net:443/scomic/book/0/2-tag/1.jpg",b"https://s1.bzcdn.net/scomic/other/0/2-tag/1.jpg"] {
            assert!(choose::<()>(url,b"book",|_| panic!("unexpected probe")).unwrap().is_none());
        }
    }
    #[test]
    fn cancellation_is_not_converted_to_preserving_success() {
        assert!(choose(FIRST,b"book",|_| Err::<bool,_>("cancelled")).is_err());
    }
}
