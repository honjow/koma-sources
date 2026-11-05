// Public website literal data, parsed independently from the pinned Apache2
// CopyMangaHtml.kt contract. No eval, JS runtime, remote code or identifier call.
use super::*;
pub fn literal(input: &str) -> R<Value> {
    let mut p = Parser {
        chars: input.chars().collect(),
        i: 0,
    };
    let value = p.value(0)?;
    p.ws();
    if p.take(';') {
        p.ws()
    }
    if p.i != p.chars.len() || !value.is_array() {
        return Err("parse_error".into());
    }
    if value
        .as_array()
        .is_some_and(|a| a.iter().any(|v| !v.is_object()))
    {
        return Err("parse_error".into());
    }
    Ok(value)
}
struct Parser {
    chars: Vec<char>,
    i: usize,
}
impl Parser {
    fn ws(&mut self) {
        while self.chars.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1
        }
    }
    fn take(&mut self, c: char) -> bool {
        if self.chars.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, c: char) -> R<()> {
        self.ws();
        if self.take(c) {
            Ok(())
        } else {
            Err("parse_error".into())
        }
    }
    fn value(&mut self, depth: usize) -> R<Value> {
        if depth > 64 {
            return Err("parse_error".into());
        }
        self.ws();
        match self.chars.get(self.i).copied() {
            Some('[') => {
                self.i += 1;
                let mut out = Vec::new();
                self.ws();
                if self.take(']') {
                    return Ok(json!(out));
                }
                loop {
                    out.push(self.value(depth + 1)?);
                    self.ws();
                    if self.take(']') {
                        break;
                    }
                    self.expect(',')?
                }
                Ok(json!(out))
            }
            Some('{') => {
                self.i += 1;
                let mut out = serde_json::Map::new();
                self.ws();
                if self.take('}') {
                    return Ok(Value::Object(out));
                }
                loop {
                    self.ws();
                    let key = if matches!(self.chars.get(self.i), Some('\'') | Some('"')) {
                        self.string()?
                    } else {
                        self.identifier()?
                    };
                    self.expect(':')?;
                    out.insert(key, self.value(depth + 1)?);
                    self.ws();
                    if self.take('}') {
                        break;
                    }
                    self.expect(',')?
                }
                Ok(Value::Object(out))
            }
            Some('\'') | Some('"') => self.string().map(Value::String),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(_) => match self.identifier()?.as_str() {
                "null" | "None" | "undefined" => Ok(Value::Null),
                "true" | "True" => Ok(json!(true)),
                "false" | "False" => Ok(json!(false)),
                _ => Err("parse_error".into()),
            },
            None => Err("parse_error".into()),
        }
    }
    fn identifier(&mut self) -> R<String> {
        self.ws();
        let a = self.i;
        while self
            .chars
            .get(self.i)
            .is_some_and(|c| (*c as u32) <= 0xffff && (c.is_alphanumeric() || *c == '_'))
        {
            self.i += 1
        }
        if a == self.i {
            return Err("parse_error".into());
        }
        Ok(self.chars[a..self.i].iter().collect())
    }
    fn number(&mut self) -> R<Value> {
        let start = self.i;
        self.take('-');
        let digits = self.i;
        while self.chars.get(self.i).is_some_and(char::is_ascii_digit) {
            self.i += 1
        }
        if self.i == digits {
            return Err("parse_error".into());
        }
        if self.take('.') {
            while self.chars.get(self.i).is_some_and(char::is_ascii_digit) {
                self.i += 1
            }
        }
        let raw: String = self.chars[start..self.i].iter().collect();
        if let Ok(n) = raw.parse::<i32>() {
            Ok(json!(n))
        } else {
            let n = raw.parse::<f64>().map_err(|_| E::from("parse_error"))?;
            serde_json::Number::from_f64(n)
                .map(Value::Number)
                .ok_or_else(|| "parse_error".into())
        }
    }
    fn hex(&mut self, n: usize) -> R<u16> {
        let mut out = 0u16;
        for _ in 0..n {
            let c = *self
                .chars
                .get(self.i)
                .ok_or_else(|| E::from("parse_error"))?;
            self.i += 1;
            out = (out << 4) | c.to_digit(16).ok_or_else(|| E::from("parse_error"))? as u16
        }
        Ok(out)
    }
    fn string(&mut self) -> R<String> {
        let quote = *self
            .chars
            .get(self.i)
            .ok_or_else(|| E::from("parse_error"))?;
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = *self
                .chars
                .get(self.i)
                .ok_or_else(|| E::from("parse_error"))?;
            self.i += 1;
            if c == quote {
                return Ok(out);
            }
            if c != '\\' {
                out.push(c);
                continue;
            }
            let c = *self
                .chars
                .get(self.i)
                .ok_or_else(|| E::from("parse_error"))?;
            self.i += 1;
            match c {
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' | 'x' => {
                    let first = self.hex(if c == 'u' { 4 } else { 2 })?;
                    let scalar = if (0xd800..=0xdbff).contains(&first) {
                        if !self.take('\\') || !self.take('u') {
                            return Err("parse_error".into());
                        }
                        let low = self.hex(4)?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return Err("parse_error".into());
                        }
                        0x10000 + ((first as u32 - 0xd800) << 10) + (low as u32 - 0xdc00)
                    } else {
                        first as u32
                    };
                    out.push(char::from_u32(scalar).ok_or_else(|| E::from("parse_error"))?)
                }
                _ => out.push(c),
            }
        }
    }
}
