// Ported from Keiyoushi BiliManga SearchCaptcha.kt, Apache-2.0.
// Upstream 1aba299dd03549a98d6f4461f6022d1e9adf04c0.
use super::R;

const MAX_WIDTH: usize = 256;
const MAX_HEIGHT: usize = 128;
const MAX_PIXELS: usize = MAX_WIDTH * MAX_HEIGHT;
static mut PIXELS: [u32; MAX_PIXELS] = [0; MAX_PIXELS];
static mut FILTERED: [u8; MAX_PIXELS * 4 + MAX_HEIGHT] = [0; MAX_PIXELS * 4 + MAX_HEIGHT];
static mut SCANLINES: [u8; MAX_PIXELS * 4] = [0; MAX_PIXELS * 4];

const DIGITS: [&[&[u8]]; 10] = [
    &[
        b"...##...",
        b"..####..",
        b".##..##.",
        b"##....##",
        b"##....##",
        b"##....##",
        b"##....##",
        b".##..##.",
        b"..####..",
        b"...##...",
    ],
    &[
        b"..##..", b".###..", b"####..", b"..##..", b"..##..", b"..##..", b"..##..", b"..##..",
        b"..##..", b"######",
    ],
    &[
        b"..####..",
        b".##..##.",
        b"##....##",
        b"......##",
        b".....##.",
        b"....##..",
        b"...##...",
        b"..##....",
        b".##.....",
        b"########",
    ],
    &[
        b".#####..",
        b"##...##.",
        b"......##",
        b".....##.",
        b"...###..",
        b".....##.",
        b"......##",
        b"......##",
        b"##...##.",
        b".#####..",
    ],
    &[
        b".....##.",
        b"....###.",
        b"...####.",
        b"..##.##.",
        b".##..##.",
        b"##...##.",
        b"########",
        b".....##.",
        b".....##.",
        b".....##.",
    ],
    &[
        b"#######.",
        b"##......",
        b"##......",
        b"##.###..",
        b"###..##.",
        b"......##",
        b"......##",
        b"##....##",
        b".##..##.",
        b"..####..",
    ],
    &[
        b"..####..",
        b".##..##.",
        b"##....#.",
        b"##......",
        b"##.###..",
        b"###..##.",
        b"##....##",
        b"##....##",
        b".##..##.",
        b"..####..",
    ],
    &[
        b"########",
        b"......##",
        b"......##",
        b".....##.",
        b"....##..",
        b"...##...",
        b"..##....",
        b".##.....",
        b"##......",
        b"##......",
    ],
    &[
        b"..####..",
        b".##..##.",
        b"##....##",
        b".##..##.",
        b"..####..",
        b".##..##.",
        b"##....##",
        b"##....##",
        b".##..##.",
        b"..####..",
    ],
    &[
        b"..####..",
        b".##..##.",
        b"##....##",
        b"##....##",
        b".##..###",
        b"..###.##",
        b"......##",
        b".#....##",
        b".##..##.",
        b"..####..",
    ],
];

fn number(bytes: &[u8]) -> R<usize> {
    Ok(u32::from_be_bytes(bytes.try_into().map_err(|_| "parse_error")?) as usize)
}

fn crc(bytes: &[u8]) -> u32 {
    let mut result = !0u32;
    for byte in bytes {
        result ^= *byte as u32;
        for _ in 0..8 {
            result = (result >> 1) ^ (0xedb88320u32.wrapping_mul(result & 1));
        }
    }
    !result
}

struct Image {
    width: usize,
    height: usize,
    pixels: &'static [u32],
}

// Decode bounded, non-interlaced 8-bit PNGs. Invalid or unsupported images
// leave the existing manual verification path available.
fn png(bytes: &[u8]) -> R<Image> {
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return Err("parse_error");
    }
    let mut offset = 8;
    let mut width = 0;
    let mut height = 0;
    let mut channels = 0;
    let mut indexed = false;
    let mut palette = [0xff000000u32; 256];
    let mut palette_len = 0;
    let mut ended = false;
    while offset + 12 <= bytes.len() {
        let size = number(&bytes[offset..offset + 4])?;
        let end = size.checked_add(12)
            .and_then(|size| offset.checked_add(size))
            .filter(|end| *end <= bytes.len())
            .ok_or("parse_error")?;
        let kind = &bytes[offset + 4..offset + 8];
        let data = &bytes[offset + 8..end - 4];
        if crc(&bytes[offset + 4..end - 4]) != number(&bytes[end - 4..end])? as u32 {
            return Err("parse_error");
        }
        match kind {
            b"IHDR" => {
                if width != 0
                    || offset != 8
                    || data.len() != 13
                    || data[8] != 8
                    || data[10..] != [0, 0, 0]
                {
                    return Err("parse_error");
                }
                width = number(&data[..4])?;
                height = number(&data[4..8])?;
                if width == 0 || height == 0 || width > MAX_WIDTH || height > MAX_HEIGHT {
                    return Err("capacity_exceeded");
                }
                channels = match data[9] {
                    0 | 3 => 1,
                    2 => 3,
                    4 => 2,
                    6 => 4,
                    _ => return Err("parse_error"),
                };
                indexed = data[9] == 3;
            }
            b"PLTE" => {
                if data.len() % 3 != 0 || data.len() > 768 {
                    return Err("parse_error");
                }
                palette_len = data.len() / 3;
                for (i, color) in data.chunks_exact(3).enumerate() {
                    palette[i] |=
                        (color[0] as u32) << 16 | (color[1] as u32) << 8 | color[2] as u32;
                }
            }
            b"IEND" => {
                ended = data.is_empty();
                offset = end;
                break;
            }
            b"IDAT" | b"tRNS" => (),
            _ if kind[0].is_ascii_uppercase() => return Err("parse_error"),
            _ => (),
        }
        offset = end;
    }
    if !ended || channels == 0 || (indexed && palette_len == 0) {
        return Err("parse_error");
    }
    let stride = width * channels;
    let expected = (stride + 1) * height;
    let filtered = unsafe { &mut *core::ptr::addr_of_mut!(FILTERED) };
    let mut at = 8;
    let parts = core::iter::from_fn(|| {
        while at + 12 <= offset {
            let size = number(&bytes[at..at + 4]).ok()?;
            let start = at;
            at += size + 12;
            if &bytes[start + 4..start + 8] == b"IDAT" {
                return Some(&bytes[start + 8..at - 4]);
            }
        }
        None
    });
    let written = miniz_oxide::inflate::decompress_slice_iter_to_slice(
        &mut filtered[..expected],
        parts,
        true,
        false,
    )
    .map_err(|_| "parse_error")?;
    if written != expected {
        return Err("parse_error");
    }
    let scanlines = unsafe { &mut *core::ptr::addr_of_mut!(SCANLINES) };
    for y in 0..height {
        let filter = filtered[y * (stride + 1)];
        for x in 0..stride {
            let left = if x >= channels {
                scanlines[y * stride + x - channels]
            } else {
                0
            };
            let up = if y > 0 {
                scanlines[(y - 1) * stride + x]
            } else {
                0
            };
            let corner = if y > 0 && x >= channels {
                scanlines[(y - 1) * stride + x - channels]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => {
                    let p = left as i32 + up as i32 - corner as i32;
                    let a = (p - left as i32).abs();
                    let b = (p - up as i32).abs();
                    let c = (p - corner as i32).abs();
                    if a <= b && a <= c {
                        left
                    } else if b <= c {
                        up
                    } else {
                        corner
                    }
                }
                _ => return Err("parse_error"),
            };
            scanlines[y * stride + x] = filtered[y * (stride + 1) + 1 + x].wrapping_add(predictor);
        }
    }
    let pixels = unsafe { &mut *core::ptr::addr_of_mut!(PIXELS) };
    for i in 0..width * height {
        let sample = &scanlines[i * channels..(i + 1) * channels];
        pixels[i] = if indexed {
            if sample[0] as usize >= palette_len {
                return Err("parse_error");
            }
            palette[sample[0] as usize]
        } else if channels <= 2 {
            0xff000000 | (sample[0] as u32) * 0x010101
        } else {
            0xff000000 | (sample[0] as u32) << 16 | (sample[1] as u32) << 8 | sample[2] as u32
        };
    }
    Ok(Image {
        width,
        height,
        pixels: &pixels[..width * height],
    })
}

#[derive(Clone, Copy)]
struct Colour {
    value: u32,
    count: usize,
    x: usize,
    y: usize,
}
#[derive(Clone, Copy)]
struct Glyph {
    score: f32,
    x: i32,
    digit: u8,
}

pub fn solve(bytes: &[u8]) -> R<[u8; 4]> {
    let image = png(bytes)?;
    let mut background = [0u32; MAX_WIDTH];
    for x in 0..image.width {
        let mut best = 0;
        for y in 0..image.height {
            let color = image.pixels[y * image.width + x];
            let count = (0..image.height)
                .filter(|row| image.pixels[row * image.width + x] == color)
                .count();
            if count > best {
                best = count;
                background[x] = color;
            }
        }
    }
    let empty = Colour {
        value: 0,
        count: 0,
        x: MAX_WIDTH,
        y: MAX_HEIGHT,
    };
    let mut colours = [empty; 256];
    let mut count = 0;
    for (index, color) in image.pixels.iter().enumerate() {
        let x = index % image.width;
        let y = index / image.width;
        if *color == background[x] {
            continue;
        }
        let group = if let Some(i) = colours[..count]
            .iter()
            .position(|group| group.value == *color)
        {
            i
        } else {
            if count == colours.len() {
                return Err("parse_error");
            }
            colours[count].value = *color;
            count += 1;
            count - 1
        };
        colours[group].count += 1;
        colours[group].x = colours[group].x.min(x);
        colours[group].y = colours[group].y.min(y);
    }
    let member = |color: u32, x: i32, y: i32| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < image.width
            && (y as usize) < image.height
            && color != background[x as usize]
            && image.pixels[y as usize * image.width + x as usize] == color
    };
    let mut best = [Glyph {
        score: f32::NEG_INFINITY,
        x: 0,
        digit: 0,
    }; 4];
    for colour in colours[..count].iter().filter(|group| group.count >= 12) {
        let shadow = colours[..count]
            .iter()
            .filter(|other| other.count >= 12 && other.value != colour.value)
            .any(|other| {
                image
                    .pixels
                    .iter()
                    .enumerate()
                    .filter(|(i, pixel)| {
                        **pixel == colour.value
                            && **pixel != background[i % image.width]
                            && member(
                                other.value,
                                (i % image.width) as i32 - 1,
                                (i / image.width) as i32 - 1,
                            )
                    })
                    .count()
                    * 2
                    > colour.count
            });
        if shadow {
            continue;
        }
        let mut glyph = Glyph {
            score: f32::NEG_INFINITY,
            x: colour.x as i32,
            digit: 0,
        };
        for (digit, rows) in DIGITS.iter().enumerate() {
            let size = rows
                .iter()
                .map(|row| row.iter().filter(|c| **c == b'#').count())
                .sum::<usize>() as f32;
            for x in colour.x as i32 - 2..=colour.x as i32 + 2 {
                for y in colour.y as i32 - 2..=colour.y as i32 + 2 {
                    let mut hits = 0;
                    let mut extra = 0;
                    for (dy, row) in rows.iter().enumerate() {
                        for (dx, pixel) in row.iter().enumerate() {
                            if member(colour.value, x + dx as i32, y + dy as i32) {
                                if *pixel == b'#' {
                                    hits += 1;
                                } else {
                                    extra += 1;
                                }
                            }
                        }
                    }
                    let score = (2 * hits - extra) as f32 / size - 1.0;
                    if score > glyph.score {
                        glyph = Glyph {
                            score,
                            x,
                            digit: b'0' + digit as u8,
                        };
                    }
                }
            }
        }
        if glyph.score > best[3].score {
            best[3] = glyph;
            best.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        }
    }
    if !best[3].score.is_finite() {
        return Err("parse_error");
    }
    best.sort_unstable_by_key(|glyph| glyph.x);
    Ok(best.map(|glyph| glyph.digit))
}
