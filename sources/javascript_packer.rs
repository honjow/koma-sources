const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub(crate) fn expand(html: &str) -> core::result::Result<alloc::string::String, &'static str> {
    use alloc::{string::String, vec::Vec};
    // The reader script uses Dean Edwards Packer's token table. Expand only
    // its data string, without executing the script or evaluating expressions.
    let packed = html
        .split_once("eval(function(p,a,c,k,e,d)")
        .ok_or("parse_error")?
        .1;
    let data = packed.split_once("}('").ok_or("parse_error")?.1;
    let mut escaped = false;
    let end = data
        .char_indices()
        .find_map(|(index, ch)| {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                return Some(index);
            }
            None
        })
        .ok_or("parse_error")?;
    let program = &data[..end];
    let parameters = data[end..].strip_prefix("',").ok_or("parse_error")?;
    let (radix, table) = parameters.split_once(",'").ok_or("parse_error")?;
    let radix = radix
        .split_once(',')
        .ok_or("parse_error")?
        .0
        .parse::<usize>()
        .map_err(|_| "parse_error")?;
    if !(2..=62).contains(&radix) {
        return Err("parse_error");
    }
    let table: Vec<_> = table
        .split_once("'.split('|')")
        .ok_or("parse_error")?
        .0
        .split('|')
        .collect();
    let mut program_expanded = String::new();
    let token_char = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    let mut remaining = program;
    while !remaining.is_empty() {
        let first = remaining.chars().next().ok_or("parse_error")?;
        if !token_char(first) {
            program_expanded.push(first);
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        let end = remaining
            .find(|c| !token_char(c))
            .unwrap_or(remaining.len());
        let (token, rest) = remaining.split_at(end);
        let number = token.bytes().try_fold(0usize, |value, ch| {
            let digit = ALPHABET[..radix].iter().position(|&digit| digit == ch)?;
            value.checked_mul(radix)?.checked_add(digit)
        });
        let word = number
            .and_then(|i| table.get(i))
            .filter(|word| !word.is_empty())
            .copied()
            .unwrap_or(token);
        program_expanded.push_str(word);
        if program_expanded.len() > 1024 * 1024 {
            return Err("result_too_large");
        }
        remaining = rest;
    }
    Ok(program_expanded)
}
