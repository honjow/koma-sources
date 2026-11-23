//! Source-owned UI strings. No allocation, global language, host imports, or UI permissions.
//! The 2026-11-23 environment/presentation contract keeps source_info zero-argument
//! and all operation result fields plain strings. Old hosts use the author's default,
//! then the field literal; they never acquire a guessed English/content locale.
use crate::{runtime::Context, source_protocol as p};

pub const MAX_LOCALES: usize = 16;
const MAX_TAG: usize = 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidMetadata,
    BufferTooSmall,
    InvalidTemplate,
    MissingArgument,
}

/// ASCII case-folded BCP-47 subset, matching the existing localizedNames grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    bytes: [u8; MAX_TAG],
    len: usize,
}
impl Locale {
    const EMPTY: Self = Self {
        bytes: [0; MAX_TAG],
        len: 0,
    };
    pub fn parse(value: &str) -> Option<Self> {
        if value.is_empty() || value.len() > MAX_TAG {
            return None;
        }
        let mut parts = value.split('-');
        let language = parts.next()?;
        if !(2..=8).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_alphabetic())
        {
            return None;
        }
        let mut count = 1;
        for part in parts {
            count += 1;
            if count > 6
                || !(1..=8).contains(&part.len())
                || !part.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                return None;
            }
        }
        let mut tag = Self::EMPTY;
        tag.len = value.len();
        for (target, source) in tag.bytes.iter_mut().zip(value.bytes()) {
            *target = source.to_ascii_lowercase();
        }
        Some(tag)
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
    fn from_json(value: &[u8]) -> Option<Self> {
        let mut buffer = [0; MAX_TAG];
        let raw = unquote(value)?;
        let decoded = crate::plain_text::decode_json_content(raw, &mut buffer).ok()?;
        Self::parse(core::str::from_utf8(decoded).ok()?)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Environment {
    ui_locale: Locale,
    fallbacks: [Locale; MAX_LOCALES],
    len: usize,
}
impl Environment {
    pub(crate) fn from_request(request: &[u8]) -> Option<Self> {
        let env = p::direct(request, b"environment").ok()??;
        if p::direct(env, b"version").ok()?? != b"1" {
            return None;
        }
        let ui_locale = Locale::from_json(p::direct(env, b"uiLocale").ok()??)?;
        let raw = p::direct(env, b"localeFallbacks").ok()??;
        if raw.first() != Some(&b'[') {
            return None;
        }
        let mut result = Self {
            ui_locale,
            fallbacks: [Locale::EMPTY; MAX_LOCALES],
            len: 0,
        };
        let mut count = 0;
        for (_, value) in Entries::array(raw) {
            count += 1;
            if count > MAX_LOCALES {
                return None;
            }
            let tag = Locale::from_json(value)?;
            if !result.fallbacks[..result.len].contains(&tag) {
                result.fallbacks[result.len] = tag;
                result.len += 1;
            }
        }
        let expected = Candidates::new(ui_locale);
        if result.len != expected.len
            || !result.fallbacks[..result.len]
                .iter()
                .zip(expected)
                .all(|(actual, expected)| *actual == expected)
        {
            return None;
        }
        Some(result)
    }
    pub fn ui_locale(&self) -> &str {
        self.ui_locale.as_str()
    }
    pub fn locale_fallbacks(&self) -> impl Iterator<Item = &str> {
        self.fallbacks[..self.len].iter().map(Locale::as_str)
    }
}

/// Borrows the embedded metadata. Missing/unknown optional Catalogs are empty,
/// while structurally malformed JSON is an explicit error.
#[derive(Clone, Copy)]
pub struct Catalog<'a> {
    messages: Option<&'a [u8]>,
    default: Option<Locale>,
}
impl<'a> Catalog<'a> {
    pub fn from_metadata(metadata: &'a [u8]) -> Result<Self, Error> {
        p::complete(metadata).map_err(|_| Error::InvalidMetadata)?;
        let empty = Self {
            messages: None,
            default: None,
        };
        let Some(localization) =
            p::direct(metadata, b"localization").map_err(|_| Error::InvalidMetadata)?
        else {
            return Ok(empty);
        };
        if p::direct(localization, b"version").ok().flatten() != Some(b"1".as_slice()) {
            return Ok(empty);
        }
        let default = p::direct(localization, b"defaultLocale")
            .ok()
            .flatten()
            .and_then(Locale::from_json);
        let messages = p::direct(localization, b"messages")
            .ok()
            .flatten()
            .filter(|v| v.first() == Some(&b'{'));
        Ok(Self { messages, default })
    }
    /// Returned UTF-8 is in `out`. Capacity errors never expose truncated text.
    pub fn text<'o>(
        &self,
        context: &Context<'_>,
        key: &str,
        literal: &str,
        out: &'o mut [u8],
    ) -> Result<&'o str, Error> {
        let selected = self.select(context, key);
        let required = selected
            .map(string_size)
            .unwrap_or(Some(literal.len()))
            .ok_or(Error::InvalidMetadata)?;
        if required > out.len() {
            return Err(Error::BufferTooSmall);
        }
        if let Some(raw) = selected {
            let bytes = crate::plain_text::decode_json_content(raw, out)
                .map_err(|_| Error::InvalidMetadata)?;
            core::str::from_utf8(bytes).map_err(|_| Error::InvalidMetadata)
        } else {
            out[..literal.len()].copy_from_slice(literal.as_bytes());
            core::str::from_utf8(&out[..literal.len()]).map_err(|_| Error::InvalidMetadata)
        }
    }
    /// Formatting is opt-in. Two caller buffers avoid hidden allocation and make
    /// the template/output budgets explicit; braces escape as {{ and }}.
    pub fn format<'o>(
        &self,
        context: &Context<'_>,
        key: &str,
        literal: &str,
        args: &[Argument<'_>],
        template: &mut [u8],
        out: &'o mut [u8],
    ) -> Result<&'o str, Error> {
        let value = self.text(context, key, literal, template)?;
        let required = render(value, args, None)?;
        if required > out.len() {
            return Err(Error::BufferTooSmall);
        }
        render(value, args, Some(out))?;
        core::str::from_utf8(&out[..required]).map_err(|_| Error::InvalidTemplate)
    }
    fn select(&self, context: &Context<'_>, key: &str) -> Option<&'a [u8]> {
        let values = p::direct(self.messages?, key.as_bytes()).ok()??;
        if values.first() != Some(&b'{') {
            return None;
        }
        // Count raw members BEFORE canonicalization/deduplication. Invalid optional
        // entries never replace a usable later candidate or the field literal.
        let mut seen = [Locale::EMPTY; MAX_LOCALES];
        let mut used = 0;
        for (index, (tag, _)) in Entries::object(values).enumerate() {
            if index >= MAX_LOCALES {
                return None;
            }
            if let Some(tag) = tag.and_then(Locale::from_json) {
                if seen[..used].contains(&tag) {
                    return None;
                }
                seen[used] = tag;
                used += 1;
            }
        }
        if let Some(env) = context.environment() {
            for locale in Candidates::new(env.ui_locale) {
                if let Some(raw) = translation(values, locale, key) {
                    return Some(raw);
                }
            }
            // The host list is a snapshot, not permission to cross language/script.
            // Only the normative candidates derived above may be selected.
        }
        if let Some(default) = self.default {
            for locale in Candidates::new(default) {
                if let Some(raw) = translation(values, locale, key) {
                    return Some(raw);
                }
            }
        }
        None
    }
}

pub struct Argument<'a> {
    pub name: &'a str,
    pub value: Value<'a>,
}
pub enum Value<'a> {
    Text(&'a str),
    Unsigned(u64),
    Signed(i64),
}
fn render(
    template: &str,
    args: &[Argument<'_>],
    mut out: Option<&mut [u8]>,
) -> Result<usize, Error> {
    let mut written = 0usize;
    let mut emit = |bytes: &[u8]| -> Result<(), Error> {
        let end = written
            .checked_add(bytes.len())
            .ok_or(Error::BufferTooSmall)?;
        if let Some(buffer) = out.as_deref_mut() {
            if end > buffer.len() {
                return Err(Error::BufferTooSmall);
            }
            buffer[written..end].copy_from_slice(bytes);
        }
        written = end;
        Ok(())
    };
    let bytes = template.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'{' || bytes[at] == b'}' {
            let brace = bytes[at];
            if bytes.get(at + 1) == Some(&brace) {
                emit(&[brace])?;
                at += 2;
                continue;
            }
            if brace == b'}' {
                return Err(Error::InvalidTemplate);
            }
            let end = bytes[at + 1..]
                .iter()
                .position(|b| *b == b'}')
                .map(|n| at + 1 + n)
                .ok_or(Error::InvalidTemplate)?;
            let name = &template[at + 1..end];
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(Error::InvalidTemplate);
            }
            let mut matched = args.iter().filter(|arg| arg.name == name);
            let arg = matched.next().ok_or(Error::MissingArgument)?;
            if matched.next().is_some() {
                return Err(Error::InvalidTemplate);
            }
            match arg.value {
                Value::Text(text) => emit(text.as_bytes())?,
                Value::Unsigned(n) => {
                    let mut buf = [0; 20];
                    emit(decimal(n, &mut buf))?;
                }
                Value::Signed(n) => {
                    if n < 0 {
                        emit(b"-")?;
                    }
                    let mut buf = [0; 20];
                    emit(decimal(n.unsigned_abs(), &mut buf))?;
                }
            }
            at = end + 1;
        } else {
            let end = bytes[at..]
                .iter()
                .position(|b| *b == b'{' || *b == b'}')
                .map(|n| at + n)
                .unwrap_or(bytes.len());
            emit(&bytes[at..end])?;
            at = end;
        }
    }
    Ok(written)
}
fn decimal(mut n: u64, buffer: &mut [u8; 20]) -> &[u8] {
    let mut at = buffer.len();
    loop {
        at -= 1;
        buffer[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            return &buffer[at..];
        }
    }
}

// At most exact, inferred-script+region, script, language. No candidate borrows
// an opposite Chinese script; an author default is a separate explicit stage.
struct Candidates {
    locales: [Locale; 4],
    len: usize,
    at: usize,
}
impl Candidates {
    fn new(locale: Locale) -> Self {
        let mut result = Self {
            locales: [Locale::EMPTY; 4],
            len: 0,
            at: 0,
        };
        result.push(locale);
        let mut parts = locale.as_str().split('-');
        let language = parts.next().unwrap_or("");
        let mut script = "";
        let mut region = "";
        for part in parts {
            if script.is_empty() && part.len() == 4 && part.bytes().all(|b| b.is_ascii_alphabetic())
            {
                script = part;
            } else if region.is_empty()
                && ((part.len() == 2 && part.bytes().all(|b| b.is_ascii_alphabetic()))
                    || (part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit())))
            {
                region = part;
            }
        }
        if language == "zh" && script.is_empty() {
            script = match region {
                "hk" | "tw" | "mo" => "hant",
                "cn" | "sg" => "hans",
                _ => "",
            };
        }
        if !script.is_empty() {
            if !region.is_empty() {
                result.push_parts(language, script, region);
            }
            result.push_parts(language, script, "");
        }
        if script.is_empty() && !region.is_empty() {
            result.push_parts(language, "", region);
        }
        result.push(Locale::parse(language).unwrap_or(Locale::EMPTY));
        result
    }
    fn push(&mut self, locale: Locale) {
        if locale.len > 0 && !self.locales[..self.len].contains(&locale) {
            self.locales[self.len] = locale;
            self.len += 1;
        }
    }
    fn push_parts(&mut self, language: &str, script: &str, region: &str) {
        let mut bytes = [0; MAX_TAG];
        let mut at = 0;
        for part in [language, script, region] {
            if part.is_empty() {
                continue;
            }
            if at > 0 {
                bytes[at] = b'-';
                at += 1;
            }
            bytes[at..at + part.len()].copy_from_slice(part.as_bytes());
            at += part.len();
        }
        if let Ok(text) = core::str::from_utf8(&bytes[..at]) {
            if let Some(tag) = Locale::parse(text) {
                self.push(tag);
            }
        }
    }
}
impl Iterator for Candidates {
    type Item = Locale;
    fn next(&mut self) -> Option<Locale> {
        if self.at == self.len {
            None
        } else {
            let tag = self.locales[self.at];
            self.at += 1;
            Some(tag)
        }
    }
}

fn translation<'a>(values: &'a [u8], locale: Locale, key: &str) -> Option<&'a [u8]> {
    let mut result = None;
    let mut duplicate = false;
    for (tag, value) in Entries::object(values) {
        if tag.and_then(Locale::from_json) != Some(locale) {
            continue;
        }
        if result.is_some() || duplicate {
            duplicate = true;
            result = None;
            continue;
        }
        let raw = unquote(value)?;
        let mut chars = JsonChars::new(raw);
        let mut count = 0;
        let mut first = None;
        let mut last = None;
        while let Some(ch) = chars.next() {
            let ch = ch?;
            let single_line = key == "source.name"
                || key == "authentication.usernameLabel"
                || key == "authentication.passwordLabel";
            if ch == '\u{7f}'
                || (ch < '\u{20}' && (single_line || !matches!(ch, '\n' | '\r' | '\t')))
            {
                return None;
            }
            count += ch.len_utf16();
            if first.is_none() {
                first = Some(ch);
            }
            last = Some(ch);
        }
        let single_line = key == "source.name"
            || key == "authentication.usernameLabel"
            || key == "authentication.passwordLabel";
        if count == 0
            || (single_line && (first.is_some_and(trim_char) || last.is_some_and(trim_char)))
            || JsonChars::new(raw).all(|ch| ch.is_some_and(trim_char))
        {
            return None;
        }
        if (key == "source.name"
            || key == "authentication.usernameLabel"
            || key == "authentication.passwordLabel")
            && count > 128
        {
            return None;
        }
        result = Some(raw);
    }
    if duplicate {
        None
    } else {
        result
    }
}
fn trim_char(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
fn string_size(raw: &[u8]) -> Option<usize> {
    let mut n = 0usize;
    for ch in JsonChars::new(raw) {
        n = n.checked_add(ch?.len_utf8())?;
    }
    Some(n)
}
fn unquote(value: &[u8]) -> Option<&[u8]> {
    if value.len() >= 2 && value[0] == b'"' && value[value.len() - 1] == b'"' {
        Some(&value[1..value.len() - 1])
    } else {
        None
    }
}

/// Tiny raw-value iterator used only after the existing strict JSON validator.
/// It does not search nested namesakes or allocate a parsed metadata tree.
struct Entries<'a> {
    raw: &'a [u8],
    at: usize,
    object: bool,
}
impl<'a> Entries<'a> {
    fn object(raw: &'a [u8]) -> Self {
        Self {
            raw,
            at: 1,
            object: true,
        }
    }
    fn array(raw: &'a [u8]) -> Self {
        Self {
            raw,
            at: 1,
            object: false,
        }
    }
    fn skip_ws(&mut self) {
        while self.raw.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn value(&mut self) -> Option<&'a [u8]> {
        self.skip_ws();
        let start = self.at;
        let mut depth = 0;
        let mut string = false;
        while let Some(&byte) = self.raw.get(self.at) {
            if string {
                if byte == b'\\' {
                    self.at += 2;
                    continue;
                }
                if byte == b'"' {
                    string = false;
                    self.at += 1;
                    if depth == 0 {
                        return Some(&self.raw[start..self.at]);
                    }
                    continue;
                }
            } else {
                match byte {
                    b'"' => string = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                        if depth == 0 {
                            self.at += 1;
                            return Some(&self.raw[start..self.at]);
                        }
                    }
                    b',' | b':' | b' ' | b'\n' | b'\r' | b'\t' if depth == 0 => break,
                    _ => {}
                }
            }
            self.at += 1;
        }
        (self.at > start).then_some(&self.raw[start..self.at])
    }
}
impl<'a> Iterator for Entries<'a> {
    type Item = (Option<&'a [u8]>, &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_ws();
        if self.raw.get(self.at) == Some(&b',') {
            self.at += 1;
            self.skip_ws();
        }
        if matches!(self.raw.get(self.at), None | Some(b'}' | b']')) {
            return None;
        }
        let key = if self.object {
            let key = self.value()?;
            self.skip_ws();
            if self.raw.get(self.at) != Some(&b':') {
                self.at = self.raw.len();
                return None;
            }
            self.at += 1;
            Some(key)
        } else {
            None
        };
        Some((key, self.value()?))
    }
}
struct JsonChars<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> JsonChars<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn hex(&mut self) -> Option<u32> {
        let raw = self.bytes.get(self.at..self.at + 4)?;
        let mut n = 0;
        for b in raw {
            n = n * 16 + (*b as char).to_digit(16)?;
        }
        self.at += 4;
        Some(n)
    }
    fn character(&mut self) -> Option<char> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        if byte == b'\\' {
            let escape = *self.bytes.get(self.at)?;
            self.at += 1;
            return match escape {
                b'"' => Some('"'),
                b'\\' => Some('\\'),
                b'/' => Some('/'),
                b'n' => Some('\n'),
                b'r' => Some('\r'),
                b't' => Some('\t'),
                b'b' => Some('\u{8}'),
                b'f' => Some('\u{c}'),
                b'u' => {
                    let mut n = self.hex()?;
                    if (0xd800..=0xdbff).contains(&n) {
                        if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
                            return None;
                        }
                        self.at += 2;
                        let low = self.hex()?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return None;
                        }
                        n = 0x10000 + ((n - 0xd800) << 10) + (low - 0xdc00);
                    }
                    char::from_u32(n)
                }
                _ => None,
            };
        }
        if byte < 32 || byte == b'"' {
            return None;
        }
        let length = if byte < 0x80 {
            1
        } else if byte < 0xe0 {
            2
        } else if byte < 0xf0 {
            3
        } else {
            4
        };
        let start = self.at - 1;
        let ch = core::str::from_utf8(self.bytes.get(start..start + length)?)
            .ok()?
            .chars()
            .next()?;
        self.at = start + length;
        Some(ch)
    }
}
impl Iterator for JsonChars<'_> {
    type Item = Option<char>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.bytes.len() {
            None
        } else {
            let value = self.character();
            if value.is_none() {
                self.at = self.bytes.len();
            }
            Some(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const METADATA: &[u8]=br#"{"localization":{"version":1,"defaultLocale":"zh-Hant","messages":{"name":{"zh-Hant":"Traditional","zh-Hans":"Simplified","en":"English"},"region":{"zh-HK":"Exact","zh-Hant-HK":"Regional","zh-Hant":"Traditional","zh":"Chinese"},"invalid":{"en":"\ud800","zh-Hant":"\u9810\u8a2d"},"format":{"zh-Hant":"{who}: {count} {{chapters}}"}}}}"#;
    fn request(locale: &str) -> std::string::String {
        let fallbacks = match locale {
            "zh-Hans-CN" => r#""zh-Hans-CN","zh-Hans","zh""#,
            "zh-HK" => r#""zh-HK","zh-Hant-HK","zh-Hant","zh""#,
            "ZH-hant-HK" => r#""ZH-hant-HK","zh-Hant","zh""#,
            "zh-MO" => r#""zh-MO","zh-Hant-MO","zh-Hant","zh""#,
            "zh-CN" => r#""zh-CN","zh-Hans-CN","zh-Hans","zh""#,
            "zh-Hans-HK" => r#""zh-Hans-HK","zh-Hans","zh""#,
            _ => r#""en""#,
        };
        std::format!(
            r#"{{"hostFeatures":["source_localization_v1"],"environment":{{"version":1,"uiLocale":"{locale}","localeFallbacks":[{fallbacks}]}}}}"#
        )
    }
    #[test]
    fn locale_environment_and_legacy_fallback_are_explicit() {
        let c = Catalog::from_metadata(METADATA).unwrap();
        let mut out = [0; 128];
        let req = request("zh-Hans-CN");
        let ctx = Context::from_request(req.as_bytes());
        assert_eq!(ctx.environment().unwrap().ui_locale(), "zh-hans-cn");
        assert_eq!(c.text(&ctx, "name", "literal", &mut out), Ok("Simplified"));
        for r in [b"{}".as_slice(), br#"{"environment":{"version":1,"uiLocale":"en","localeFallbacks":["en"]}}"#,
            br#"{"hostFeatures":["source_localization_v1"],"environment":{"version":2,"uiLocale":"en","localeFallbacks":["en"]}}"#,
            br#"{"hostFeatures":["source_localization_v1"],"environment":{"version":1,"uiLocale":"bad_tag","localeFallbacks":[]}}"#] {
            let ctx=Context::from_request(r); assert!(ctx.environment().is_none());
            assert_eq!(c.text(&ctx,"name","literal",&mut out),Ok("Traditional"));
        }
        for metadata in [br#"{}"#.as_slice(), br#"{"localization":{"version":2}}"#] {
            assert_eq!(
                Catalog::from_metadata(metadata)
                    .unwrap()
                    .text(&ctx, "name", "literal", &mut out),
                Ok("literal")
            );
        }
    }
    #[test]
    fn script_and_region_candidates_do_not_cross_chinese_scripts() {
        let c = Catalog::from_metadata(METADATA).unwrap();
        let mut out = [0; 128];
        for (locale, key, expected) in [
            ("zh-HK", "region", "Exact"),
            ("ZH-hant-HK", "region", "Regional"),
            ("zh-MO", "name", "Traditional"),
            ("zh-CN", "name", "Simplified"),
            ("zh-Hans-HK", "name", "Simplified"),
        ] {
            let req = request(locale);
            assert_eq!(
                c.text(
                    &Context::from_request(req.as_bytes()),
                    key,
                    "literal",
                    &mut out
                ),
                Ok(expected)
            );
        }
        let c=Catalog::from_metadata(br#"{"localization":{"version":1,"defaultLocale":"en","messages":{"x":{"zh-Hans":"Wrong script","en":"Default"}}}}"#).unwrap();
        let req = request("zh-HK");
        assert_eq!(
            c.text(
                &Context::from_request(req.as_bytes()),
                "x",
                "literal",
                &mut out
            ),
            Ok("Default")
        );
    }
    #[test]
    fn malformed_optional_values_and_duplicate_aliases_fall_back() {
        let req = request("en");
        let ctx = Context::from_request(req.as_bytes());
        let mut out = [0; 128];
        let c = Catalog::from_metadata(METADATA).unwrap();
        assert_eq!(c.text(&ctx, "invalid", "literal", &mut out), Ok("預設"));
        assert_eq!(c.text(&ctx, "absent", "literal", &mut out), Ok("literal"));
        let c=Catalog::from_metadata(br#"{"localization":{"version":1,"defaultLocale":"zh-Hant","messages":{"x":{"en":"A","EN":"B","zh-Hant":"Default"},"y":{"en":"A","en":"B","zh-Hant":"Default"}}}}"#).unwrap();
        for key in ["x", "y"] {
            assert_eq!(c.text(&ctx, key, "literal", &mut out), Ok("literal"));
        }
        let over = std::format!(
            r#"{{"hostFeatures":["source_localization_v1"],"environment":{{"version":1,"uiLocale":"en","localeFallbacks":[{}]}}}}"#,
            std::vec!["\"en\"";17].join(",")
        );
        assert!(Context::from_request(over.as_bytes())
            .environment()
            .is_none());
        let many = std::format!(
            r#"{{"localization":{{"version":1,"defaultLocale":"en","messages":{{"x":{{{}}}}}}}}}"#,
            std::vec!["\"en\":\"x\"";17].join(",")
        );
        assert_eq!(
            Catalog::from_metadata(many.as_bytes())
                .unwrap()
                .text(&ctx, "x", "literal", &mut out),
            Ok("literal")
        );
    }
    #[test]
    fn buffers_and_formatting_preserve_complete_utf8_and_parameters() {
        let c = Catalog::from_metadata(METADATA).unwrap();
        let ctx = Context::from_request(b"{}");
        let mut template = [0; 128];
        let mut out = [0; 128];
        let args = [
            Argument {
                name: "who",
                value: Value::Text("讀者🙂"),
            },
            Argument {
                name: "count",
                value: Value::Signed(-12),
            },
        ];
        assert_eq!(
            c.format(&ctx, "format", "literal", &args, &mut template, &mut out),
            Ok("讀者🙂: -12 {chapters}")
        );
        assert_eq!(
            c.format(&ctx, "format", "literal", &[], &mut template, &mut out),
            Err(Error::MissingArgument)
        );
        let mut small = [b'!'; 2];
        assert_eq!(
            c.text(&ctx, "invalid", "🙂", &mut small),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(&small, b"!!");
        assert_eq!(
            c.format(&ctx, "format", "literal", &args, &mut template, &mut small),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(&small, b"!!");
    }
}
