// SPDX-License-Identifier: Apache-2.0
use alloc::{format, string::String};

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "unix_time_ms"]
    fn host_unix_time_ms(out: *mut u8, cap: u32) -> i32;
}

pub(crate) fn now() -> Option<i64> {
    let mut stamp = [0u8; 8];
    if unsafe { host_unix_time_ms(stamp.as_mut_ptr(), stamp.len() as u32) } != 8 {
        return None;
    }
    i64::try_from(u64::from_le_bytes(stamp) / 1000).ok()
}

const SECONDS_PER_DAY: i64 = 86400;

fn days_in_month(year: i64, month: i64) -> Option<i64> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return None,
    })
}

fn iso(year: i64, month: i64, day: i64, seconds: i64) -> Option<String> {
    if !(1..=9999).contains(&year) || !(1..=days_in_month(year, month)?).contains(&day) {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    ))
}

fn days_before_year(year: i64) -> i64 {
    let prior = year - 1;
    prior * 365 + prior / 4 - prior / 100 + prior / 400
}

fn date_at(epoch: i64) -> Option<(i64, i64, i64)> {
    let ordinal = epoch.div_euclid(SECONDS_PER_DAY) + days_before_year(1970);
    if !(0..days_before_year(10000)).contains(&ordinal) {
        return None;
    }
    let (mut low, mut high) = (1, 10000);
    while high - low > 1 {
        let mid = (high + low) / 2;
        if days_before_year(mid) <= ordinal {
            low = mid;
        } else {
            high = mid;
        }
    }
    let mut day = ordinal - days_before_year(low) + 1;
    for month in 1..=12 {
        let length = days_in_month(low, month)?;
        if day <= length {
            return Some((low, month, day));
        }
        day -= length;
    }
    None
}

fn at_epoch(epoch: i64) -> Option<String> {
    let (year, month, day) = date_at(epoch)?;
    iso(year, month, day, epoch.rem_euclid(SECONDS_PER_DAY))
}

fn starts_with(text: &str, options: &[&str]) -> bool {
    options.iter().any(|word| text.starts_with(word))
}

fn word_in(text: &str, options: &[&str]) -> bool {
    options.iter().any(|word| {
        text.match_indices(word).any(|(position, _)| {
            !text[..position]
                .chars()
                .next_back()
                .is_some_and(char::is_alphabetic)
        })
    })
}

pub(crate) fn parse(text: &str, now: Option<i64>) -> Option<String> {
    if let Some((year, remainder)) = text.split_once('年') {
        if let Some((month, remainder)) = remainder.split_once('月') {
            if let Some(day) = remainder.trim().strip_suffix('日') {
                return iso(
                    year.trim().parse().ok()?,
                    month.trim().parse().ok()?,
                    day.trim().parse().ok()?,
                    0,
                );
            }
        }
    }

    let now = now?;
    let mut text = text.to_lowercase();
    for (word, replacement) in [
        ("ثانيتين", "2 ثوان"),
        ("دقيقتين", "2 دقائق"),
        ("ساعتين", "2 ساعات"),
        ("يومين", "2 أيام"),
        ("أسبوعين", "2 أسابيع"),
        ("شهرين", "2 أشهر"),
        ("سنتين", "2 سنوات"),
    ] {
        if text.contains(word) {
            text = text.replace(word, replacement);
        }
    }
    let midnight = now - now.rem_euclid(SECONDS_PER_DAY);
    if starts_with(&text, &["today", "hoje", "hoy"]) {
        return at_epoch(midnight);
    }
    if starts_with(&text, &["yesterday", "ontem", "ayer", "يوم واحد"]) {
        return at_epoch(midnight - SECONDS_PER_DAY);
    }
    let quantity = text
        .split(|ch: char| !ch.is_ascii_digit())
        .find(|word| !word.is_empty())?
        .parse::<i64>()
        .ok()?;
    let years = word_in(&text, &["year", "año", "ano", "năm", "yıl", "سنة", "سنوات"]);
    let months = word_in(
        &text,
        &["month", "mes", "tháng", "ay", "شهر", "أشهر", "شهور"],
    );
    if years || months {
        let (year, month, day) = date_at(now)?;
        let months = if years {
            quantity.checked_mul(12)?
        } else {
            quantity
        };
        let month_number = (year * 12 + month - 1).checked_sub(months)?;
        let year = month_number.div_euclid(12);
        let month = month_number.rem_euclid(12) + 1;
        return iso(
            year,
            month,
            day.min(days_in_month(year, month)?),
            now.rem_euclid(SECONDS_PER_DAY),
        );
    }

    for (seconds, words) in [
        (
            7 * SECONDS_PER_DAY,
            &["week", "semana", "tuần", "hafta", "أسبوع", "أسابيع"][..],
        ),
        (
            SECONDS_PER_DAY,
            &[
                "day", "día", "dia", "jour", "hari", "gün", "ngày", "giorni", "أيام", "天",
            ][..],
        ),
        (
            3600,
            &[
                "hour",
                "hora",
                "heure",
                "jam",
                "saat",
                "giờ",
                "ساعة",
                "ساعات",
                "小时",
            ][..],
        ),
        (
            60,
            &[
                "minute",
                "minuto",
                "min",
                "menit",
                "dakika",
                "phút",
                "دقيقة",
                "دقائق",
            ][..],
        ),
        (
            1,
            &["second", "segundo", "sec", "detik", "giây", "ثانية", "ثوان"][..],
        ),
    ] {
        if word_in(&text, words) {
            return at_epoch(now.checked_sub(quantity.checked_mul(seconds)?)?);
        }
    }
    None
}
