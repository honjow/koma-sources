//! Reconcile provider-deduplicated chapters with the complete validated feed.
//! A missing remote chapter is not an exclusion unless the complete feed proves it.
//! The host retains old chapters, and this provider rejected multiple GraphQL roots;
//! query each validated edition separately and publish only a complete difference.
use super::*;

pub fn exclusions(good: usize, count: usize, full_query: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, E> {
    let mut all = BTreeSet::new();
    for ei in 0..good {
        let edition = unsafe { EDITIONS[ei].id };
        let mut page = 1usize;
        let mut expected = None;
        let mut accumulated = 0usize;
        loop {
            if host::check_cancel() { return Err("cancelled"); }
            if page > 10000 { return Err("catalogue capacity exceeded"); }
            let mut vars = [0u8; 256]; let mut n = 0;
            bytes(&mut vars, &mut n, b"{\"select\":{\"comic_id\":")?;
            q(&mut vars, &mut n, edition.as_bytes())?;
            bytes(&mut vars, &mut n, b",\"page\":")?; wn(&mut vars, &mut n, page)?;
            bytes(&mut vars, &mut n, b",\"size\":100,\"sortby\":\"chapter_desc\"}}")?;
            let data = query(full_query, &vars[..n])?;
            let list = required(data, b"get_comic_chapterList_fullList")?;
            let paging = required(list, b"paging")?;
            let total = integer(required(paging, b"total")?)?;
            let next = integer(required(paging, b"next")?)?;
            if total > 8192 || expected.map(|x| x != total).unwrap_or(false) { return Err("inconsistent catalogue total"); }
            expected = Some(total);
            if next != 0 && next != page + 1 { return Err("invalid chapter pagination"); }
            let mut rows = Array::new(required(list, b"items")?)?; let mut returned = 0;
            while let Some(item) = rows.next()? {
                if all.len() >= 8192 || returned >= 100 { return Err("catalogue capacity exceeded"); }
                let data = required(item, b"data")?;
                let chapter = Identity::new(text(data, b"id")?)?;
                if text(item, b"id")? != chapter.as_bytes() { return Err("chapter identity mismatch"); }
                if optional_text(data, b"dbStatus")? != Some(b"normal".as_slice()) { return Err("chapter not public normal"); }
                if !all.insert((edition.as_bytes().to_vec(), chapter.as_bytes().to_vec())) { return Err("duplicate chapter identity"); }
                returned += 1;
            }
            accumulated += returned;
            if accumulated > total || (next != 0 && (returned != 100 || accumulated >= total)) || (next == 0 && accumulated != total) {
                return Err("incomplete chapter pagination");
            }
            if next == 0 { break; } page = next;
        }
    }
    let mut kept = BTreeSet::new();
    for ch in unsafe { &CHS[..count] } {
        let key = (ch.edition.as_bytes().to_vec(), ch.id.as_bytes().to_vec());
        if !all.contains(&key) { return Err("deduplicated chapter missing from complete feed"); }
        kept.insert(key);
    }
    Ok(all.difference(&kept).cloned().collect())
}
