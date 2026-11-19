//! Recognize a scaled known logo on a verified uniform margin.
use super::{host, luma, Corner, Pattern, Side, BYTES_PER_PIXEL, LEFT_CLOUD};

pub(super) fn prepare(width: usize, height: usize, side: Side) -> Result<Option<Corner>, ()> {
    if !(500..=2400).contains(&width) {
        return Ok(None);
    }
    let pattern = LEFT_CLOUD;
    let rect = pattern.rectangle();
    let ratio = width as f64 / pattern.page_width() as f64;
    let extra = (20.0 * ratio).ceil() as usize + 2;
    let max_w = (rect.width as f64 * ratio * 1.025).ceil() as usize;
    let max_h = (rect.height as f64 * ratio * 1.025).ceil() as usize;
    let cw = (max_w + extra).min(width);
    let ch = (max_h + extra).min(height);
    if cw < max_w || ch < max_h {
        return Ok(None);
    }
    let mut corner = Corner {
        x: if side == Side::Left { 0 } else { width - cw },
        y: height - ch,
        width: cw,
        height: ch,
        pixels: vec![0; cw * ch * BYTES_PER_PIXEL],
    };
    host::region(
        corner.x as i32,
        corner.y as i32,
        cw as i32,
        ch as i32,
        &mut corner.pixels,
    )?;
    // Search only margins that plausibly contain the pale logo. This makes
    // ordinary white pages and colorful artwork inexpensive to reject.
    let (mut samples, mut light, mut ink) = (0usize, 0usize, 0usize);
    for row in (0..ch).step_by(4) {
        for col in (0..cw).step_by(4) {
            let i = (row * cw + col) * 4;
            let v = luma(&corner.pixels[i..i + 4]);
            samples += 1;
            light += usize::from(v >= 240);
            ink += usize::from(v < 230);
        }
    }
    if light * 100 < samples * 65 || ink < 20.max(samples / 100) {
        return Ok(None);
    }
    let mut best: Option<(f64, usize, usize, usize, usize, Vec<u8>)> = None;
    for k in [-2_i32, -1, 0, 1, 2] {
        if host::cancelled() {
            return Err(());
        }
        let scale = ratio * (1.0 + k as f64 * 0.01);
        let tw = (rect.width as f64 * scale).round() as usize;
        let th = (rect.height as f64 * scale).round() as usize;
        if tw < 64 || th < 24 || tw > cw || th > ch {
            continue;
        }
        let expected = resize(pattern, tw, th);
        let max_edge = ((14.0 * ratio).ceil() as usize).min(cw - tw);
        let max_bottom = ((14.0 * ratio).ceil() as usize).min(ch - th);
        for inset in 0..=max_edge {
            let x = if side == Side::Left {
                inset
            } else {
                cw - inset - tw
            };
            for bottom in (0..=max_bottom).step_by(2) {
                let y = ch - bottom - th;
                let score = correlation(&corner, &expected, x, y, tw, th);
                if score >= 0.84 && best.as_ref().is_none_or(|v| score > v.0) {
                    best = Some((score, x, y, tw, th, expected.clone()));
                }
            }
        }
    }
    let Some((_score, x, y, tw, th, expected)) = best else {
        return Ok(None);
    };
    // Correlation identifies the logo; every pixel and its surrounding border
    // must still agree with a neutral, flat margin before any fill is allowed.
    let mut error = 0usize;
    let mut outliers = 0usize;
    let mut border = 0usize;
    let mut clean = 0usize;
    let mut sum = 0usize;
    for row in 0..th {
        for col in 0..tw {
            let i = (y + row) * cw + x + col;
            let pixel = &corner.pixels[i * 4..i * 4 + 4];
            if pixel[3] != 255
                || pixel[0].max(pixel[1]).max(pixel[2]) - pixel[0].min(pixel[1]).min(pixel[2]) > 40
            {
                return Ok(None);
            }
            let v = luma(pixel);
            let diff = v.abs_diff(expected[row * tw + col]);
            if diff > 72 {
                return Ok(None);
            }
            error += usize::from(diff);
            outliers += usize::from(diff > 24);
            if row == 0 || row + 1 == th || col == 0 || col + 1 == tw {
                border += 1;
                if v.abs_diff(255) > 20 {
                    return Ok(None);
                }
                if v >= 247 {
                    clean += 1;
                    sum += usize::from(v);
                }
            }
        }
    }
    let n = tw * th;
    if error > 9 * n || outliers * 100 > 8 * n || clean * 100 < 97 * border {
        return Ok(None);
    }
    let fill = ((sum + clean / 2) / clean) as u8;
    for row in y..y + th {
        for col in x..x + tw {
            let i = (row * cw + col) * 4;
            corner.pixels[i..i + 3].fill(fill);
        }
    }
    Ok(Some(corner))
}
fn resize(p: Pattern, w: usize, h: usize) -> Vec<u8> {
    let r = p.rectangle();
    let src = p.pixels();
    let mut out = vec![0; w * h];
    for y in 0..h {
        let fy = (((y as f64 + 0.5) * r.height as f64 / h as f64) - 0.5)
            .clamp(0.0, (r.height - 1) as f64);
        let y0 = fy.floor() as usize;
        let y1 = (y0 + 1).min(r.height - 1);
        let dy = fy - y0 as f64;
        for x in 0..w {
            let fx = (((x as f64 + 0.5) * r.width as f64 / w as f64) - 0.5)
                .clamp(0.0, (r.width - 1) as f64);
            let x0 = fx.floor() as usize;
            let x1 = (x0 + 1).min(r.width - 1);
            let dx = fx - x0 as f64;
            let a = src[y0 * r.width + (r.width - 1 - x0)] as f64;
            let b = src[y0 * r.width + (r.width - 1 - x1)] as f64;
            let c = src[y1 * r.width + (r.width - 1 - x0)] as f64;
            let d = src[y1 * r.width + (r.width - 1 - x1)] as f64;
            out[y * w + x] =
                ((a + (b - a) * dx) * (1.0 - dy) + (c + (d - c) * dx) * dy).round() as u8;
        }
    }
    out
}
fn correlation(c: &Corner, t: &[u8], x: usize, y: usize, w: usize, h: usize) -> f64 {
    let (mut n, mut a, mut b, mut aa, mut bb, mut ab) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    for row in (1..h - 1).step_by(6) {
        for col in (1..w - 1).step_by(6) {
            let u = t[row * w + col] as f64;
            let i = ((y + row) * c.width + x + col) * 4;
            let v = luma(&c.pixels[i..i + 4]) as f64;
            n += 1.0;
            a += u;
            b += v;
            aa += u * u;
            bb += v * v;
            ab += u * v;
        }
    }
    let va = n * aa - a * a;
    let vb = n * bb - b * b;
    let cov = n * ab - a * b;
    if va <= 0.0 || vb <= 0.0 || cov <= 0.0 {
        0.0
    } else {
        cov / (va * vb).sqrt()
    }
}
