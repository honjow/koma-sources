//! Build a chapter response by declaring how the catalog can be read.

use crate::json_utils::{append_json_escaped, write_bytes, write_usize};
use crate::result::ResultBuffer;
use crate::source::SourceError;
use crate::source_protocol;

pub type ChapterResult<'a, E = SourceError> = Result<ChapterResponse<'a>, E>;

#[derive(Clone, Copy)]
pub struct Chapter<'a> {
    pub id: &'a [u8],
    pub manga_id: &'a [u8],
    pub title: &'a [u8],
    pub chapter_number: Option<ChapterNumber<'a>>,
    pub index: Option<usize>,
    pub sequence_index: Option<usize>,
    pub locked: Option<bool>,
    pub volume_number: Option<ChapterNumber<'a>>,
    pub language: Option<&'a [u8]>,
    pub scanlator: Option<&'a [u8]>,
    pub published_at: Option<&'a [u8]>,
    pub updated_at: Option<&'a [u8]>,
    pub updated_at_millis: Option<u64>,
    pub page_count: Option<usize>,
}

impl<'a> Chapter<'a> {
    pub const fn new(id: &'a [u8], manga_id: &'a [u8], title: &'a [u8]) -> Self {
        Self {
            id,
            manga_id,
            title,
            chapter_number: None,
            index: None,
            sequence_index: None,
            locked: None,
            volume_number: None,
            language: None,
            scanlator: None,
            published_at: None,
            updated_at: None,
            updated_at_millis: None,
            page_count: None,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ChapterNumber<'a> {
    Text(&'a [u8]),
    Integer(usize),
    Decimal(DecimalChapterNumber<'a>),
}

/// A decimal JSON number with its provider precision preserved.
#[derive(Clone, Copy)]
pub struct DecimalChapterNumber<'a>(&'a [u8]);

impl<'a> DecimalChapterNumber<'a> {
    pub fn new(value: &'a [u8]) -> Option<Self> {
        let value_without_sign = value.strip_prefix(b"-").unwrap_or(value);
        let mut exponent = value_without_sign.split(|byte| matches!(byte, b'e' | b'E'));
        let number = exponent.next()?;
        if let Some(power) = exponent.next() {
            let power = power
                .strip_prefix(b"+")
                .or_else(|| power.strip_prefix(b"-"))
                .unwrap_or(power);
            if power.is_empty()
                || !power.iter().all(u8::is_ascii_digit)
                || exponent.next().is_some()
            {
                return None;
            }
        }
        let mut parts = number.split(|byte| *byte == b'.');
        let integer = parts.next()?;
        if integer.is_empty()
            || !integer.iter().all(u8::is_ascii_digit)
            || (integer.len() > 1 && integer[0] == b'0')
        {
            return None;
        }
        if let Some(fraction) = parts.next() {
            if fraction.is_empty()
                || !fraction.iter().all(u8::is_ascii_digit)
                || parts.next().is_some()
            {
                return None;
            }
        }
        Some(Self(value))
    }
}

#[derive(Clone, Copy)]
pub enum ChapterOrder {
    OldestFirst,
    NewestFirst,
}

pub enum ChapterPlacement<'a> {
    /// A stable ordinal relative to the whole group, including across pages.
    /// The host accepts nonnegative integers up to JavaScript's safe integer limit.
    InGroup { group: &'a [u8], position: u64 },
    /// The source has no verified continuation position for this entry.
    ManualOnly,
}

pub enum ChapterPage<'a> {
    Complete,
    NextCursor(&'a [u8]),
}

/// A successful chapter payload produced by the SDK's catalog builders.
pub struct ChapterResponse<'a>(&'a [u8]);

impl<'a> ChapterResponse<'a> {
    pub fn single(buffer: &'a mut [u8], chapter: Chapter<'_>) -> ChapterResult<'a> {
        let mut writer = CatalogWriter::new(buffer)?;
        writer.push(chapter, None)?;
        writer.finish(ChapterPage::Complete)
    }

    /// The full catalog is in verified order. The array's presentation order
    /// is preserved, and the positions are counted from the oldest chapter.
    pub fn ordered(
        buffer: &'a mut [u8],
        request: &[u8],
        order: ChapterOrder,
        chapters: &[Chapter<'_>],
    ) -> ChapterResult<'a> {
        let mut writer = CatalogWriter::new(buffer)?;
        let enabled = reading_order_supported(request);
        for (index, chapter) in chapters.iter().enumerate() {
            let position = match order {
                ChapterOrder::OldestFirst => index,
                ChapterOrder::NewestFirst => chapters.len() - index - 1,
            };
            writer.push(
                *chapter,
                enabled.then_some((b"source-catalog", position as u64)),
            )?;
        }
        writer.finish(ChapterPage::Complete)
    }

    #[doc(hidden)]
    pub fn write<const N: usize>(self, buffer: &mut ResultBuffer<N>) -> u32 {
        if ResultBuffer::<N>::success_payload_len("get_chapters", self.0.len()) > 1024 * 1024 {
            return buffer.write_error(
                "get_chapters",
                "source_error",
                "chapters cannot fit current response capacity",
            );
        }
        let ptr = buffer.write_success("get_chapters", self.0);
        if ptr == 0 {
            return buffer.write_error("get_chapters", "internal_error", "response overflow");
        }
        ptr
    }
}

/// Grouped or paginated catalogs require a placement decision for every entry.
///
/// ```compile_fail
/// use koma_source_sdk::chapters::{Chapter, GroupedChapters};
/// let mut bytes = [0; 512];
/// let mut catalog = GroupedChapters::new(&mut bytes, b"{}").unwrap_or_else(|_| panic!());
/// catalog.push(Chapter::new(b"c1", b"m1", b"Chapter 1"));
/// ```
pub struct GroupedChapters<'a> {
    writer: CatalogWriter<'a>,
    reading_order: bool,
}

impl<'a> GroupedChapters<'a> {
    pub fn new(buffer: &'a mut [u8], request: &[u8]) -> Result<Self, SourceError> {
        Ok(Self {
            writer: CatalogWriter::new(buffer)?,
            reading_order: reading_order_supported(request),
        })
    }

    pub fn push(
        &mut self,
        chapter: Chapter<'_>,
        placement: ChapterPlacement<'_>,
    ) -> Result<(), SourceError> {
        let position = match placement {
            ChapterPlacement::InGroup { group, position } => {
                if group.trim_ascii().is_empty() {
                    return Err(SourceError::source_error("empty chapter reading group"));
                }
                if position > 9_007_199_254_740_991 {
                    return Err(SourceError::source_error(
                        "chapter reading position exceeds safe integer range",
                    ));
                }
                self.reading_order.then_some((group, position))
            }
            // Preserve legacy omission for unverified entries. Grouped sources
            // must leave manifest chapterOrder unset so these remain manual.
            ChapterPlacement::ManualOnly => None,
        };
        self.writer.push(chapter, position)
    }

    pub fn finish(self, page: ChapterPage<'_>) -> ChapterResult<'a> {
        self.writer.finish(page)
    }

    /// Append the existing scanlator-name observation and optional exclusions.
    /// The response can still only be constructed from typed chapters.
    pub fn finish_with_scanlators<'b>(
        self,
        page: ChapterPage<'_>,
        setting_id: &[u8],
        scanlators: impl IntoIterator<Item = &'b [u8]>,
        exclusions: Option<impl IntoIterator<Item = &'b [u8]>>,
    ) -> ChapterResult<'a> {
        let mut writer = self.writer;
        writer.page(page)?;
        writer.bytes(br#","settingObservations":{"#)?;
        writer.string(Some(setting_id))?;
        writer.bytes(b":")?;
        writer.string_array(scanlators)?;
        writer.bytes(b"}")?;
        if let Some(exclusions) = exclusions {
            writer.bytes(br#","excludedChapterIds":"#)?;
            writer.string_array(exclusions)?;
        }
        writer.complete()
    }

    /// Start a response with the source-owned IDs that a visibility setting removed.
    pub fn with_excluded_ids(
        self,
        page: ChapterPage<'_>,
    ) -> Result<ChapterExclusions<'a>, SourceError> {
        let mut writer = self.writer;
        writer.page(page)?;
        writer.bytes(br#","excludedChapterIds":["#)?;
        Ok(ChapterExclusions {
            writer,
            has_ids: false,
        })
    }
}

pub struct ChapterExclusions<'a> {
    writer: CatalogWriter<'a>,
    has_ids: bool,
}

impl<'a> ChapterExclusions<'a> {
    pub fn push(&mut self, id: &[u8]) -> Result<(), SourceError> {
        let start = self.writer.cursor;
        let result = (|| {
            if self.has_ids {
                self.writer.bytes(b",")?;
            }
            self.writer.string(Some(id))
        })();
        if result.is_err() {
            self.writer.cursor = start;
        } else {
            self.has_ids = true;
        }
        result
    }

    pub fn finish(mut self) -> ChapterResult<'a> {
        self.writer.bytes(b"]")?;
        self.writer.complete()
    }
}

pub fn reading_order_supported(request: &[u8]) -> bool {
    source_protocol::feature(request, b"chapter_reading_order_v1")
}

struct CatalogWriter<'a> {
    buffer: &'a mut [u8],
    cursor: usize,
}

impl<'a> CatalogWriter<'a> {
    const PREFIX: &'static [u8] = br#"{"items":["#;

    fn new(buffer: &'a mut [u8]) -> Result<Self, SourceError> {
        let mut writer = Self { buffer, cursor: 0 };
        writer.bytes(Self::PREFIX)?;
        Ok(writer)
    }

    fn push(
        &mut self,
        chapter: Chapter<'_>,
        position: Option<(&[u8], u64)>,
    ) -> Result<(), SourceError> {
        let start = self.cursor;
        if let Err(error) = self.write_chapter(chapter, position) {
            self.cursor = start;
            return Err(error);
        }
        Ok(())
    }

    fn write_chapter(
        &mut self,
        chapter: Chapter<'_>,
        position: Option<(&[u8], u64)>,
    ) -> Result<(), SourceError> {
        if self.cursor > Self::PREFIX.len() {
            self.bytes(b",")?;
        }
        self.bytes(br#"{"id":"#)?;
        self.string(Some(chapter.id))?;
        self.bytes(br#","mangaId":"#)?;
        self.string(Some(chapter.manga_id))?;
        self.bytes(br#","title":"#)?;
        self.string(Some(chapter.title))?;
        self.bytes(br#","chapterNumber":"#)?;
        self.chapter_number(chapter.chapter_number)?;
        if let Some(index) = chapter.index {
            self.bytes(br#","index":"#)?;
            self.number(index)?;
        }
        if let Some(index) = chapter.sequence_index {
            self.bytes(br#","sequenceIndex":"#)?;
            self.number(index)?;
        }
        if let Some(locked) = chapter.locked {
            self.bytes(br#","locked":"#)?;
            self.bytes(if locked { b"true" } else { b"false" })?;
        }
        self.bytes(br#","volumeNumber":"#)?;
        self.chapter_number(chapter.volume_number)?;
        if let Some(language) = chapter.language {
            self.bytes(br#","language":"#)?;
            self.string(Some(language))?;
        }
        if let Some(scanlator) = chapter.scanlator {
            self.bytes(br#","scanlator":"#)?;
            self.string(Some(scanlator))?;
        }
        self.bytes(br#","publishedAt":"#)?;
        self.string(chapter.published_at)?;
        self.bytes(br#","updatedAt":"#)?;
        if let Some(value) = chapter.updated_at_millis {
            self.number64(value)?;
        } else {
            self.string(chapter.updated_at)?;
        }
        self.bytes(br#","pageCount":"#)?;
        match chapter.page_count {
            Some(value) => self.number(value)?,
            None => self.bytes(b"null")?,
        }
        if let Some((group, ordinal)) = position {
            self.bytes(br#","readingGroup":"#)?;
            self.string(Some(group))?;
            self.bytes(br#","continuationEligible":true,"readingOrder":"#)?;
            self.number64(ordinal)?;
        }
        self.bytes(b"}")
    }

    fn finish(mut self, page: ChapterPage<'_>) -> ChapterResult<'a> {
        self.page(page)?;
        self.complete()
    }

    fn page(&mut self, page: ChapterPage<'_>) -> Result<(), SourceError> {
        self.bytes(br#"],"page":{"nextCursor":"#)?;
        match page {
            ChapterPage::Complete => self.bytes(br#"null,"hasMore":false}"#)?,
            ChapterPage::NextCursor(cursor) => {
                if cursor.is_empty() {
                    return Err(SourceError::source_error("empty chapter cursor"));
                }
                self.string(Some(cursor))?;
                self.bytes(br#","hasMore":true}"#)?;
            }
        }
        Ok(())
    }

    fn complete(mut self) -> ChapterResult<'a> {
        self.bytes(b"}")?;
        Ok(ChapterResponse(&self.buffer[..self.cursor]))
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), SourceError> {
        check_capacity(write_bytes(self.buffer, &mut self.cursor, value))
    }

    fn string_array<'b>(
        &mut self,
        values: impl IntoIterator<Item = &'b [u8]>,
    ) -> Result<(), SourceError> {
        self.bytes(b"[")?;
        for (index, value) in values.into_iter().enumerate() {
            if index > 0 {
                self.bytes(b",")?;
            }
            self.string(Some(value))?;
        }
        self.bytes(b"]")
    }

    fn string(&mut self, value: Option<&[u8]>) -> Result<(), SourceError> {
        match value {
            Some(text) => {
                self.bytes(b"\"")?;
                check_capacity(append_json_escaped(self.buffer, &mut self.cursor, text))?;
                self.bytes(b"\"")
            }
            None => self.bytes(b"null"),
        }
    }

    fn chapter_number(&mut self, value: Option<ChapterNumber<'_>>) -> Result<(), SourceError> {
        match value {
            Some(ChapterNumber::Text(value)) => self.string(Some(value)),
            Some(ChapterNumber::Integer(value)) => self.number(value),
            Some(ChapterNumber::Decimal(value)) => self.bytes(value.0),
            None => self.bytes(b"null"),
        }
    }

    fn number64(&mut self, mut value: u64) -> Result<(), SourceError> {
        let mut digits = [0; 20];
        let mut start = digits.len();
        loop {
            start -= 1;
            digits[start] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.bytes(&digits[start..])
    }

    fn number(&mut self, value: usize) -> Result<(), SourceError> {
        check_capacity(write_usize(self.buffer, &mut self.cursor, value))
    }
}

fn check_capacity(fits: bool) -> Result<(), SourceError> {
    if fits {
        Ok(())
    } else {
        Err(SourceError::internal_error("payload overflow"))
    }
}
