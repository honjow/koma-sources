//! Typed chapter output shared by both XCOMIC languages.
//! Display labels and sorting never determine the provider reading position.
use super::*;
use koma_source_sdk::chapters::{Chapter as OutputChapter, ChapterNumber, ChapterPlacement,
    DecimalChapterNumber, GroupedChapters};

fn decoded(raw: &[u8], key: &[u8]) -> Result<Vec<u8>, E> {
    let value = text(raw, key)?;
    let mut out = alloc::vec![0; value.len()];
    let len = unescape(value, &mut out)?.len();out.truncate(len);Ok(out)
}
fn number<'a>(raw: &'a [u8], key: &[u8]) -> Result<Option<ChapterNumber<'a>>, E> {
    match member(raw, key)? {
        None | Some(b"null") => Ok(None),
        Some(value) => DecimalChapterNumber::new(value).map(|v|Some(ChapterNumber::Decimal(v)))
            .ok_or("invalid chapter number"),
    }
}
pub fn push(catalog: &mut GroupedChapters<'_>, raw: &[u8], label: Option<&str>,
    placement: ChapterPlacement<'_>) -> Result<(), E> {
    let id=decoded(raw,b"id")?;let work=decoded(raw,b"mangaId")?;
    let title=decoded(raw,b"title")?;let language=decoded(raw,b"language")?;
    let scanlator=if member(raw,b"scanlator")?.is_some(){Some(decoded(raw,b"scanlator")?)}else{None};
    let mut chapter=OutputChapter::new(&id,&work,&title);
    chapter.chapter_number=number(raw,b"chapterNumber")?;
    chapter.volume_number=number(raw,b"volumeNumber")?;
    chapter.language=Some(&language);
    chapter.scanlator=label.map(str::as_bytes).or(scanlator.as_deref());
    catalog.push(chapter,placement).map_err(|_| "payload overflow")
}
