//! Recognize the centered Baozi brand at its observed scale, then locate the
//! surrounding white margin independently of the banner's text or height.
use super::host;
const MARK: &[u8; 3600] = include_bytes!("baozi-brand.rgb");

pub(super) fn crop(width: i32, height: i32, top: bool, base: i32) -> Result<i32, ()> {
    if !(320..=2400).contains(&width) || height < 100 {
        return Ok(0);
    }
    let scan = (width * 2 / 5).min(height - 1).min(700);
    let mut row = vec![0; width as usize * 4];
    let (mut first, mut last, mut left, mut right, mut total) = (-1, -1, width, -1, 0);
    let mut gaps = 0;
    for offset in 0..scan {
        if host::cancelled() {
            return Err(());
        }
        let y = if top { offset } else { height - 1 - offset };
        host::region(0, base + y, width, 1, &mut row)?;
        let mut count = 0;
        for x in width / 4..width * 3 / 5 {
            let p = &row[x as usize * 4..x as usize * 4 + 4];
            if p[3] == 255
                && p[0] > 180
                && p[1] > 140
                && p[2] < 110
                && p[0] as u16 > p[2] as u16 * 2
                && p[1] as u16 > p[2] as u16 * 2
            {
                count += 1;
                left = left.min(x);
                right = right.max(x);
            }
        }
        if count >= (width / 250).max(3) {
            if first < 0 {
                first = offset;
            }
            last = offset;
            total += count;
            gaps = 0;
        } else if first >= 0 {
            gaps += 1;
            if gaps >= 3 {
                break;
            }
        }
    }
    if first < 0 {
        return Ok(0);
    }
    let h = last - first + 1;
    let w = right - left + 1;
    if w < 16
        || h < 16
        || w > width / 6
        || h > width / 6
        || w * 10 < h * 7
        || w * 10 > h * 14
        || total < w * h / 5
    {
        return Ok(0);
    }
    let y = if top { first } else { height - 1 - last };
    let brand_width = (w * 175 + 28) / 56;
    if left < 0 || left + brand_width > width || (left * 2 + brand_width - width).abs() > w {
        return Ok(0);
    }
    let mut mark = vec![0; brand_width as usize * h as usize * 4];
    host::region(left, base + y, brand_width, h, &mut mark)?;
    let mut error = 0usize;
    let mut outliers = 0usize;
    for py in 0..20usize {
        for px in 0..60usize {
            let sx = ((2 * px + 1) * brand_width as usize / 120).min(brand_width as usize - 1);
            let sy = ((2 * py + 1) * h as usize / 40).min(h as usize - 1);
            let i = (sy * brand_width as usize + sx) * 4;
            let j = (py * 60 + px) * 3;
            let mut worst = 0u8;
            for c in 0..3 {
                let e = mark[i + c].abs_diff(MARK[j + c]);
                error += e as usize;
                worst = worst.max(e);
            }
            outliers += usize::from(worst > 80);
        }
    }
    if error > 30 * 3600 || outliers > 180 {
        return Ok(0);
    }
    // The brand name alone can occur in comic art. Require the separate wide
    // attribution line beneath it and neutral side margins before any crop.
    let mut line_first = -1;
    let mut line_last = -1;
    let mut line_left = width;
    let mut line_right = -1;
    let mut line_ink = 0;
    for py in (y + h)..(y + h * 3).min(height) {
        if host::cancelled() {
            return Err(());
        }
        host::region(0, base + py, width, 1, &mut row)?;
        let mut ink = 0;
        for x in width / 10..width * 9 / 10 {
            let p = &row[x as usize * 4..x as usize * 4 + 4];
            if p[3] == 255 && p[0] < 180 && p[1] < 180 && p[2] < 180 {
                ink += 1;
                line_left = line_left.min(x);
                line_right = line_right.max(x);
            }
        }
        if ink >= (w / 4).max(5) {
            if line_first < 0 {
                line_first = py;
            }
            line_last = py;
            line_ink += ink;
        }
    }
    if line_first < 0
        || line_first < y + h
        || line_last - line_first > h
        || line_right - line_left < w * 3
        || line_ink < w * 3
    {
        return Ok(0);
    }
    let (from, to) = if top { (0, line_last + 1) } else { (y, height) };
    for py in from..to {
        if host::cancelled() {
            return Err(());
        }
        host::region(0, base + py, width, 1, &mut row)?;
        let mut not_white = 0;
        for x in (0..width / 8).chain(width * 7 / 8..width) {
            let p = &row[x as usize * 4..x as usize * 4 + 4];
            if p[3] != 255 || p[0] < 240 || p[1] < 240 || p[2] < 240 {
                not_white += 1;
            }
        }
        if not_white > width / 80 {
            return Ok(0);
        }
    }
    // All white-margin checks inspect the whole image width. Stop at the
    // first uncertain row once a clean gap has been established; faint art is
    // preserved rather than treated as removable whitespace.
    let start = if top { line_last + 1 } else { y - 1 };
    let direction = if top { 1 } else { -1 };
    let mut py = start;
    let mut clean = 0;
    while py >= 0 && py < height && (py - start).abs() <= scan {
        if host::cancelled() {
            return Err(());
        }
        host::region(0, base + py, width, 1, &mut row)?;
        let mut nonwhite = 0;
        let mut dark = 0;
        for p in row.chunks_exact(4) {
            if p[3] != 255 || p[0] < 248 || p[1] < 248 || p[2] < 248 {
                nonwhite += 1;
            }
            if p[3] != 255 || p[0] < 230 || p[1] < 230 || p[2] < 230 {
                dark += 1;
            }
        }
        if nonwhite == 0 {
            clean += 1;
        } else if clean >= (w / 8).max(3) {
            let boundary = refine_boundary(width, height, py, base, top)?;
            let boundary = if boundary != py {
                trim_bleached_edge(width, height, boundary, base, top)?
            } else {
                boundary
            };
            return Ok(if top { boundary } else { height - 1 - boundary });
        } else if clean == 0 && (py - start).abs() < (h / 5).max(2) {
            // Antialiased brand edges can extend beyond the yellow bounds.
            // A clean gutter is still required before any crop is accepted.
        } else if dark > width / 20 || nonwhite > width / 4 {
            return Ok(0);
        }
        py += direction;
    }
    Ok(0)
}

// A hard full-width image edge may bleed into the white gutter when encoded
// as JPEG. Advance past that fringe only when its last row closely follows the
// next dark row; otherwise the first uncertain row remains part of the image.
fn refine_boundary(width: i32, height: i32, first: i32, base: i32, top: bool) -> Result<i32, ()> {
    let mut row = vec![0; width as usize * 4];
    let mut previous = Vec::<f64>::new();
    let mut last_dense = false;
    let mut previous_min = 255u8;
    let mut previous_nonwhite = 0usize;
    let limit = (width / 120).clamp(3, 12);
    for offset in 0..limit {
        let y = if top { first + offset } else { first - offset };
        if y < 0 || y >= height {
            break;
        }
        if host::cancelled() {
            return Err(());
        }
        host::region(0, base + y, width, 1, &mut row)?;
        let mut dark = 0usize;
        let mut nonwhite = 0usize;
        let mut sum = 0usize;
        let mut minimum = 255u8;
        let mut gray = Vec::with_capacity(width as usize);
        for p in row.chunks_exact(4) {
            if p[3] != 255 {
                return Ok(first);
            }
            let lo = p[0].min(p[1]).min(p[2]);
            minimum = minimum.min(lo);
            dark += usize::from(lo < 230);
            nonwhite += usize::from(lo < 248);
            let value = (p[0] as usize + p[1] as usize + p[2] as usize) / 3;
            sum += value;
            gray.push(value as f64);
        }
        let hard_edge = if top {
            dark * 5 >= width as usize * 4
        } else {
            dark * 5 >= width as usize * 3
        };
        if hard_edge {
            if previous.is_empty() || !last_dense {
                return Ok(first);
            }
            let n = width as f64;
            let (mut a, mut b, mut aa, mut bb, mut ab) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for (u, v) in previous.iter().zip(gray.iter()) {
                a += u;
                b += v;
                aa += u * u;
                bb += v * v;
                ab += u * v;
            }
            let va = n * aa - a * a;
            let vb = n * bb - b * b;
            let cov = n * ab - a * b;
            return Ok(
                if va > 0.0
                    && vb > 0.0
                    && (if top {
                        cov / (va * vb).sqrt() >= 0.70
                    } else {
                        (cov / (va * vb).sqrt()).abs() >= 0.45
                    })
                {
                    y
                } else {
                    first
                },
            );
        }
        if minimum < 215 || sum < 240 * width as usize || dark > width as usize / 8 {
            return Ok(first);
        }
        // A stray faint mark before the JPEG edge may be real artwork.
        // Do not consume it if the following rows become whiter again.
        if !previous.is_empty()
            && (minimum > previous_min.saturating_add(2)
                || nonwhite + width as usize / 250 < previous_nonwhite)
        {
            return Ok(first);
        }
        previous_min = minimum;
        previous_nonwhite = nonwhite;
        last_dense = nonwhite > width as usize / 2;
        previous = gray;
    }
    Ok(first)
}

// JPEG can blend the last content row almost entirely with the removed white
// banner. Remove that row only when it is a near-white, strongly correlated
// copy of its neighbour and fits a white-compositing model. Otherwise retain it.
fn trim_bleached_edge(
    width: i32,
    height: i32,
    boundary: i32,
    base: i32,
    top: bool,
) -> Result<i32, ()> {
    let next = if top { boundary + 1 } else { boundary - 1 };
    if next < 0 || next >= height {
        return Ok(boundary);
    }
    if host::cancelled() {
        return Err(());
    }
    let mut edge = vec![0; width as usize * 4];
    let mut inner = edge.clone();
    host::region(0, base + boundary, width, 1, &mut edge)?;
    host::region(0, base + next, width, 1, &mut inner)?;
    let n = width as f64;
    let (mut a, mut b, mut aa, mut bb, mut ab) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, q) in edge.chunks_exact(4).zip(inner.chunks_exact(4)) {
        if p[3] != 255 || q[3] != 255 {
            return Ok(boundary);
        }
        let u = (q[0] as f64 + q[1] as f64 + q[2] as f64) / 3.0;
        let v = (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0;
        a += u;
        b += v;
        aa += u * u;
        bb += v * v;
        ab += u * v;
    }
    let va = n * aa - a * a;
    let vb = n * bb - b * b;
    let cov = n * ab - a * b;
    if va <= 0.0 || vb <= 0.0 || cov <= 0.0 {
        return Ok(boundary);
    }
    let slope = cov / va;
    let intercept = (b - slope * a) / n;
    if b / n >= 220.0
        && b / n - a / n >= 50.0
        && (0.08..=0.50).contains(&slope)
        && (intercept - 255.0 * (1.0 - slope)).abs() <= 12.0
        && cov / (va * vb).sqrt() >= 0.90
    {
        // A JPEG overshoot can darken the following row too. Only discard it
        // when the shift is almost entirely in the same direction across the
        // page; otherwise it may be a genuine line in the artwork.
        let deeper = if top { next + 1 } else { next - 1 };
        if deeper < 0 || deeper >= height {
            return Ok(next);
        }
        let mut reference = vec![0; width as usize * 4];
        host::region(0, base + deeper, width, 1, &mut reference)?;
        let (mut darker, mut lighter, mut sum) = (0usize, 0usize, 0.0f64);
        for (p, q) in inner.chunks_exact(4).zip(reference.chunks_exact(4)) {
            if p[3] != 255 || q[3] != 255 {
                return Ok(next);
            }
            let delta =
                (p[0] as f64 + p[1] as f64 + p[2] as f64 - q[0] as f64 - q[1] as f64 - q[2] as f64)
                    / 3.0;
            darker += usize::from(delta < -4.0);
            lighter += usize::from(delta > 4.0);
            sum += delta;
        }
        if darker * 100 >= width as usize * 40
            && lighter * 100 < width as usize * 8
            && sum / n <= -4.0
            && sum / n >= -20.0
        {
            Ok(deeper)
        } else {
            Ok(next)
        }
    } else {
        Ok(boundary)
    }
}
