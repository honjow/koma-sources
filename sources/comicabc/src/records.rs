//! Fixed public literal-record schema. No JavaScript is executed.
//! Field positions come from the observed ps/image references, not variable names.
#[derive(Clone, Copy)]
struct Field<'a> {
    name: &'a str,
    offset: usize,
    size: usize,
}
#[derive(Clone, Copy)]
struct Constant<'a> {
    name: &'a str,
    value: usize,
}
struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }
    fn ws(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .map_or(false, u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn take(&mut self, token: &str) -> bool {
        self.ws();
        if self.text[self.pos..].starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Option<&'a str> {
        self.ws();
        let start = self.pos;
        let b = self.text.as_bytes();
        if !b
            .get(self.pos)
            .map_or(false, |x| x.is_ascii_alphabetic() || *x == b'_')
        {
            return None;
        }
        self.pos += 1;
        while b
            .get(self.pos)
            .map_or(false, |x| x.is_ascii_alphanumeric() || *x == b'_')
        {
            self.pos += 1;
        }
        Some(&self.text[start..self.pos])
    }
    fn number(&mut self) -> Option<usize> {
        self.ws();
        let start = self.pos;
        let mut n = 0usize;
        while let Some(c) = self
            .text
            .as_bytes()
            .get(self.pos)
            .filter(|x| x.is_ascii_digit())
        {
            n = n.checked_mul(10)?.checked_add((c - b'0') as usize)?;
            self.pos += 1;
            if self.pos - start > 8 {
                return None;
            }
        }
        if self.pos == start {
            None
        } else {
            Some(n)
        }
    }
    fn literal(&mut self) -> Option<&'a str> {
        self.ws();
        let q = *self.text.as_bytes().get(self.pos)?;
        if q != b'\'' && q != b'"' {
            return None;
        }
        self.pos += 1;
        let start = self.pos;
        while let Some(&c) = self.text.as_bytes().get(self.pos) {
            if c == q {
                let s = &self.text[start..self.pos];
                self.pos += 1;
                return Some(s);
            }
            if !c.is_ascii_alphanumeric() {
                return None;
            }
            self.pos += 1;
        }
        None
    }
}
fn find_field<'a>(fields: &[Field<'a>], name: &str) -> Option<Field<'a>> {
    fields.iter().copied().find(|f| f.name == name)
}
fn lc(s: &[u8]) -> Option<usize> {
    if s.len() != 2 {
        return None;
    }
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let a = alphabet.iter().position(|x| *x == s[0])?;
    let b = alphabet.iter().position(|x| *x == s[1])?;
    Some(if s[0] == b'Z' { 8000 + b } else { a * 52 + b })
}
/// Selected chapter metadata; URL formatting is the same bounded schema.
#[derive(Debug)]
pub struct Record {
    pub page_count: usize,
    pub server: usize,
    pub part: u8,
    pub codes: [u8; 40],
}
pub fn decode(
    html: &[u8],
    comic: usize,
    chapter: usize,
    requested_part: Option<u8>,
) -> Option<Record> {
    if comic == 0 || comic > 999999999 || chapter > 8051 {
        return None;
    }
    let html = core::str::from_utf8(html).ok()?;
    let marker = "$(\"#comics-pics\").html(xx)";
    let mark = html.find(marker)?;
    if html[mark + marker.len()..].contains(marker) {
        return None;
    }
    let start = html[..mark].rfind("<script")?;
    let open = html[start..mark].find('>')? + start + 1;
    let script = &html[open..mark];
    if script.len() > 131072 {
        return None;
    }
    let empty = Constant { name: "", value: 0 };
    let mut constants = [empty; 32];
    let mut nc = 0;
    let mut table_name = "";
    let mut data = "";
    for (i, _) in script.match_indices("var ") {
        let mut p = Cursor::new(&script[i + 4..]);
        let name = match p.ident() {
            Some(x) => x,
            None => continue,
        };
        if !p.take("=") {
            continue;
        }
        let saved = p.pos;
        if let Some(n) = p.number() {
            if p.take(";") {
                if nc == constants.len() {
                    return None;
                }
                constants[nc] = Constant { name, value: n };
                nc += 1;
            }
        } else {
            p.pos = saved;
            if let Some(s) = p.literal() {
                if s.len() > 200 && p.take(";") {
                    if !data.is_empty() {
                        return None;
                    }
                    data = s;
                    table_name = name;
                }
            }
        }
    }
    if data.is_empty()
        || data.len() > 131072
        || constants[..nc]
            .iter()
            .filter(|x| x.name == "ti" && x.value == comic)
            .count()
            != 1
    {
        return None;
    }
    let loopstart = script.find("for (var i=0;i<")?;
    let mut loopcursor = Cursor::new(&script[loopstart..]);
    if !loopcursor.take("for (var i=0;i<") {
        return None;
    }
    let count = loopcursor.number()?;
    if count == 0 || count > 8192 || !loopcursor.take(";i++)") {
        return None;
    }
    let bytes = count.checked_mul(47)?;
    if data.len() != bytes + 83
        || data.get(bytes..bytes + 36)? != "546865496d67696d67636f6d69632e6a7067"
    {
        return None;
    }
    let recordloop = &script[loopstart..];
    let blank = Field {
        name: "",
        offset: 0,
        size: 0,
    };
    let mut fields = [blank; 5];
    let mut nf = 0;
    let mut helper = "";
    for (i, _) in recordloop.match_indices("var ") {
        let mut p = Cursor::new(&recordloop[i + 4..]);
        let name = match p.ident() {
            Some(x) => x,
            None => continue,
        };
        if !p.take("=lc(") {
            continue;
        }
        if nf == fields.len() {
            return None;
        }
        let h = p.ident()?;
        if helper.is_empty() {
            helper = h;
        } else if helper != h {
            return None;
        }
        if !p.take("(") || p.ident()? != table_name || !p.take(",i*(") {
            return None;
        }
        let constant = p.ident()?;
        let value = constants[..nc].iter().find(|c| c.name == constant)?.value;
        let plus = p.take("+");
        if !plus && !p.take("-") {
            return None;
        }
        let delta = p.number()?;
        let width = if plus {
            value.checked_add(delta)?
        } else {
            value.checked_sub(delta)?
        };
        if width != 47 || !p.take(")+") {
            return None;
        }
        let offset = p.number()?;
        let size = if p.take(",") { p.number()? } else { 40 };
        if !p.take("));") {
            return None;
        }
        // A record is exactly 47 bytes. Accept only the observed field widths,
        // wholly inside that record, with no overlap; roles are checked below.
        let end = offset.checked_add(size)?;
        if !matches!(size, 1 | 2 | 40)
            || end > 47
            || fields[..nf].iter().any(|f| offset < f.offset + f.size && f.offset < end)
        {
            return None;
        }
        fields[nf] = Field { name, offset, size };
        nf += 1;
    }
    if nf != 5 {
        return None;
    }
    let ps = recordloop.find("ps=")?;
    let mut p = Cursor::new(&recordloop[ps..]);
    if !p.take("ps=") {
        return None;
    }
    let page_name = p.ident()?;
    if !p.take(";if(") {
        return None;
    }
    let chapter_name = p.ident()?;
    if !p.take("==ch") {
        return None;
    }
    let render = script.find("for(var j=1;j<=ps;j++)")?;
    let rendering = &script[render..];
    let server_end = rendering.find(", 0, 1)")?;
    let server_start = rendering[..server_end].rfind('(')?;
    let server_name = rendering[server_start + 1..server_end].trim();
    let before = &rendering[..server_start];
    if !before.ends_with(helper) {
        return None;
    }
    let code_end = rendering.find(",mm(j),3)")?;
    let code_start = rendering[..code_end].rfind('(')?;
    let code_name = rendering[code_start + 1..code_end].trim();
    if !rendering[..code_start].ends_with(helper) {
        return None;
    }
    let part_end = rendering.find("==\"0\"?\"\":")?;
    let part_start = rendering[..part_end].rfind('(')?;
    let part_name = &rendering[part_start + 1..part_end];
    let mut q = Cursor::new(&rendering[part_end + 9..]);
    if q.ident()? != part_name {
        return None;
    }
    let page = find_field(&fields, page_name)?;
    let server = find_field(&fields, server_name)?;
    let chapter_field = find_field(&fields, chapter_name)?;
    let code = find_field(&fields, code_name)?;
    let part_field = find_field(&fields, part_name)?;
    // Obfuscation permutes fields, while the actual chapter/ps/image references
    // identify their roles. Five distinct roles with 2+40+2+2+1 bytes, bounded
    // and non-overlapping above, must cover the complete 47-byte record.
    let roles = [chapter_field, code, page, server, part_field];
    if roles.iter().enumerate().any(|(i, field)| {
        roles[..i].iter().any(|other| other.offset == field.offset)
    })
        || chapter_field.size != 2
        || code.size != 40
        || part_field.offset != 46
        || part_field.size != 1
        || page.size != 2
        || server.size != 2
    {
        return None;
    }
    let mut selected = None;
    for r in data.as_bytes()[..bytes].chunks_exact(47) {
        if lc(&r[chapter_field.offset..chapter_field.offset + 2])? != chapter {
            continue;
        }
        let part = r[46];
        if part != b'0' && !part.is_ascii_lowercase() {
            return None;
        }
        if let Some(wanted) = requested_part {
            if part != wanted {
                continue;
            }
        }
        // A catalogue ID without suffix uses the first matching record, as on the public page.
        if selected.is_none() {
            let page_count = lc(&r[page.offset..page.offset + 2])?;
            let server = lc(&r[server.offset..server.offset + 2])?;
            if page_count == 0 || !(10..=99).contains(&server) {
                return None;
            }
            let mut codes = [0; 40];
            codes.copy_from_slice(&r[code.offset..code.offset + 40]);
            selected = Some(Record {
                page_count,
                server,
                part,
                codes,
            });
        }
    }
    selected
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_identifiers_and_field_positions() {
        let a = include_bytes!(
            "../../../.hermes-artifacts/comicabc-20261010/reader-data-script-653-1.txt"
        );
        let b = include_bytes!(
            "../../../.hermes-artifacts/comicabc-20261010/reader-data-script-15059-1.txt"
        );
        // Fixture text is script only, while the production decoder requires a script boundary.
        for (raw, comic, pages, server) in [(&a[..], 653, 90, 74), (&b[..], 15059, 44, 94)] {
            let mut h = std::vec::Vec::from(b"<script language=\"javascript\">" as &[u8]);
            h.extend_from_slice(raw);
            h.extend_from_slice(b"</script>");
            let r = decode(&h, comic, 1, None).expect("named public literal schema");
            assert_eq!(r.page_count, pages);
            assert_eq!(r.server, server);
            assert!(decode(&h, comic + 1, 1, None).is_none());
            assert!(decode(&h, comic, 8052, None).is_none());
        }
    }
}
