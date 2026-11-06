use super::{chapter_id, chapter_route, detail, provider_id, string, Result};
use alloc::{format, vec::Vec};
use serde_json::{json, Value};

fn number(text: &str, start: usize, end: usize) -> Option<i64> {
    let text = text.get(start..end)?;
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

fn date(text: &str) -> Option<i64> {
    let date = text.as_bytes();
    if date.get(4) != Some(&b'-')
        || date.get(7) != Some(&b'-')
        || date.get(10) != Some(&b'T')
        || date.get(13) != Some(&b':')
        || date.get(16) != Some(&b':')
    {
        return None;
    }
    let year = number(text, 0, 4)?;
    let month = number(text, 5, 7)?;
    let day = number(text, 8, 10)?;
    let hour = number(text, 11, 13)?;
    let minute = number(text, 14, 16)?;
    let second = number(text, 17, 19)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let calendar = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || !(1..=calendar[(month - 1) as usize]).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let mut rest = text.get(19..)?;
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=9).contains(&digits) {
            return None;
        }
        let used = digits.min(3);
        millis = number(fraction, 0, used)? * 10_i64.pow((3 - used) as u32);
        rest = fraction.get(digits..)?;
    }
    let offset = match rest.as_bytes() {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), _, _, b':', _, _] => {
            let h = number(rest, 1, 3)?;
            let m = number(rest, 4, 6)?;
            if h > 23 || m > 59 {
                return None;
            }
            (h * 3600 + m * 60) * if *sign == b'+' { 1 } else { -1 }
        }
        _ => return None,
    };
    let adjusted = year - i64::from(month <= 2);
    let era = adjusted.div_euclid(400);
    let within = adjusted - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = within * 365 + within / 4 - within / 100 + day_of_year;
    let days = era * 146097 + day_of_era - 719468;
    Some((days * 86400 + hour * 3600 + minute * 60 + second - offset) * 1000 + millis)
}

pub fn catalogue(mid: u64) -> Result<Value> {
    let data = detail(mid)?;
    let chapters = data["chapters"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .rev()
        .enumerate()
        .map(|(index, item)| {
            let cid = provider_id(item, "id")?;
            let mut chapter = json!({"id":chapter_id(mid,cid),"mangaId":format!("utifa:{mid}"),
                "title":string(item,"title")?,"index":index,"url":chapter_route(cid)});
            if let Some(timestamp) = item["updateTime"].as_str().and_then(date) {
                chapter["dateUpload"] = json!(timestamp);
            }
            Ok(chapter)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"chapters":chapters}))
}
