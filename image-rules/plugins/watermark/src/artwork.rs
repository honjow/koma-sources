//! Restore a narrow mark where the surrounding artwork predicts the hidden pixels.
use super::{luma, Pattern, Rect, BYTES_PER_PIXEL};

const NEIGHBOR_LIMIT: usize = 56;
const HALO: usize = 3;
const VERTICAL: (isize, isize) = (0, 1);
const HORIZONTAL: (isize, isize) = (1, 0);
const DIRECTIONS: [(isize, isize); 10] = [
    VERTICAL,
    HORIZONTAL,
    (1, 1),
    (1, -1),
    (1, 2),
    (1, -2),
    (2, 1),
    (2, -1),
    (3, 1),
    (3, -1),
];

pub(super) struct Pair {
    pub light: Pattern,
    pub dark: Pattern,
    // Use the distinctive part of each logo, outside the other overlaid mark.
    pub signature_width: usize,
    pub signature_height: usize,
}

pub(super) struct Templates<'a> {
    pub pairs: &'a [Pair],
    pub copy: Option<&'a Pair>,
    pub cloud: Pattern,
    // Newly calibrated narrow left marks can cross a faint horizontal stroke.
    // Keep the existing right-side reconstruction order for v1 matches.
    pub prefer_fine_lines: bool,
}

pub(super) fn restore(
    corner: &mut [u8],
    width: usize,
    height: usize,
    page_width: usize,
    templates: Templates<'_>,
) -> Result<bool, ()> {
    let luminance: Vec<u8> = corner.chunks_exact(BYTES_PER_PIXEL).map(luma).collect();
    let plane = Plane {
        luminance: &luminance,
        width,
        height,
    };
    let mut best = None;
    let mut highest = 0.0;
    let mut next = 0.0;
    for pair in templates.pairs {
        if super::host::cancelled() {
            return Err(());
        }
        let score = plane.pair_signature(pair, page_width);
        if score > highest {
            next = highest;
            highest = score;
            best = Some(pair);
        } else if score > next {
            next = score;
        }
    }
    let pair = match best {
        Some(pair) if highest >= 0.73 && highest - next >= 0.22 => pair,
        // A copy-only match cannot account for an overlapping DM5 mark.
        _ if highest < 0.28 => {
            let Some(copy) = templates
                .copy
                .filter(|p| plane.pair_signature(p, page_width) >= 0.73)
            else {
                return Ok(false);
            };
            copy
        }
        _ => return Ok(false),
    };
    let cloud = templates.cloud;
    let cloud_rect = cloud.rectangle();
    if cloud.page_width() == page_width
        && cloud_rect.height + cloud_rect.bottom <= height
        && plane.signature(cloud, None, 227, cloud_rect.height) >= 0.23
    {
        return Ok(false);
    }
    plane.reconstruct(corner, pair, templates.prefer_fine_lines)
}

struct Plane<'a> {
    luminance: &'a [u8],
    width: usize,
    height: usize,
}

impl Plane<'_> {
    fn pair_signature(&self, pair: &Pair, page_width: usize) -> f64 {
        let rect = pair.light.rectangle();
        if pair.light.page_width() != page_width || rect.height + rect.bottom > self.height {
            return 0.0;
        }
        self.signature(
            pair.light,
            Some(pair.dark),
            pair.signature_width,
            pair.signature_height,
        )
    }

    fn signature(&self, light: Pattern, dark: Option<Pattern>, columns: usize, rows: usize) -> f64 {
        let rect = light.rectangle();
        let (x, y) = rect.origin(self.width, self.height);
        let mut correlation = Correlation::default();
        for row in 2..rows.min(rect.height - 2) {
            for col in 2..columns.min(rect.width - 2) {
                let offset = row * rect.width + col;
                let white_detail = detail(light.pixels(), rect.width, offset);
                let black_detail = dark.map_or(0, |p| detail(p.pixels(), rect.width, offset));
                if white_detail.abs() + black_detail.abs() <= 24 {
                    continue;
                }
                let level = i32::from(self.local_level(x + col, y + row));
                let expected = (level * white_detail + (255 - level) * black_detail) / 255;
                let actual = detail(self.luminance, self.width, (y + row) * self.width + x + col);
                correlation.push(expected, actual);
            }
        }
        correlation.value()
    }

    fn local_level(&self, x: usize, y: usize) -> u8 {
        let columns = [x.saturating_sub(8), x, (x + 8).min(self.width - 1)];
        let rows = [y.saturating_sub(8), y, (y + 8).min(self.height - 1)];
        let mut values = [0; 9];
        for (i, row) in rows.into_iter().enumerate() {
            for (j, column) in columns.into_iter().enumerate() {
                values[i * 3 + j] = self.luminance[row * self.width + column];
            }
        }
        values.sort_unstable();
        values[4]
    }

    fn reconstruct(
        &self,
        corner: &mut [u8],
        pair: &Pair,
        prefer_fine_lines: bool,
    ) -> Result<bool, ()> {
        let rect = pair.light.rectangle();
        let (x, y) = rect.origin(self.width, self.height);
        let mask = self.mask(pair, x, y, HALO);
        let Some(direction) = self.continuity(&mask, rect, x, y, prefer_fine_lines) else {
            return Ok(false);
        };
        let directions: &[(isize, isize)] = match direction {
            Some(VERTICAL) => &[VERTICAL],
            Some(HORIZONTAL) => &[HORIZONTAL],
            _ => &DIRECTIONS,
        };
        let smooth = direction.is_none();
        if smooth && self.has_unaligned_lines(corner, &mask, &self.mask(pair, x, y, HALO + 5)) {
            return Ok(false);
        }
        let mut recovered = corner.to_vec();
        for (row, mask_row) in mask.chunks_exact(self.width).enumerate() {
            if row % 16 == 0 && super::host::cancelled() {
                return Err(());
            }
            for (column, &marked) in mask_row.iter().enumerate() {
                if !marked {
                    continue;
                }
                let offset = row * self.width + column;
                if corner[offset * BYTES_PER_PIXEL + 3] != 255 {
                    return Ok(false);
                }
                let model = Composite::at(
                    pair,
                    column as isize - x as isize,
                    row as isize - y as isize,
                );
                let actual = rgb(corner, offset);
                let mut best = None;
                for &axis in directions {
                    let Some((a, before)) =
                        self.neighbor(corner, &mask, column, row, (-axis.0, -axis.1))
                    else {
                        continue;
                    };
                    let Some((b, after)) = self.neighbor(corner, &mask, column, row, axis) else {
                        continue;
                    };
                    let spread = difference(before, after);
                    if spread > 75 {
                        continue;
                    }
                    let guess = std::array::from_fn(|c| {
                        ((usize::from(before[c]) * b + usize::from(after[c]) * a + (a + b) / 2)
                            / (a + b)) as u8
                    });
                    let projected = model.over(guess);
                    let error = difference(projected, actual);
                    let mut score = spread * 80
                        + (a + b) * 75
                        + error.min(35) * usize::from(model.transmission.saturating_sub(140)) / 2;
                    if smooth {
                        // The neutral watermark attenuates the channel differences.
                        // Prefer a continuation that agrees with the surviving color.
                        score += color_difference(actual, projected) * 500;
                    }
                    if best.is_none_or(|(old_score, _)| score < old_score) {
                        best = Some((score, guess));
                    }
                }
                let Some((_, guess)) = best else {
                    // Abandon the whole restoration if a stroke cannot be continued.
                    return Ok(false);
                };
                let start = offset * BYTES_PER_PIXEL;
                recovered[start..start + 3].copy_from_slice(&model.recover(actual, guess));
            }
        }
        if smooth {
            // Suppress isolated color speckles caused by compressed watermark edges.
            // Directional line continuation keeps its original one-pixel detail.
            self.smooth_replacements(corner, &recovered, &mask);
        } else {
            corner.copy_from_slice(&recovered);
        }
        Ok(true)
    }

    fn smooth_replacements(&self, corner: &mut [u8], recovered: &[u8], mask: &[bool]) {
        for (i, _) in mask.iter().enumerate().filter(|(_, marked)| **marked) {
            let x = i % self.width;
            let y = i / self.width;
            let columns = [x.saturating_sub(1), x, (x + 1).min(self.width - 1)];
            let rows = [y.saturating_sub(1), y, (y + 1).min(self.height - 1)];
            for channel in 0..3 {
                let mut values = [0; 9];
                for (j, row) in rows.into_iter().enumerate() {
                    for (k, col) in columns.into_iter().enumerate() {
                        values[j * 3 + k] =
                            recovered[(row * self.width + col) * BYTES_PER_PIXEL + channel];
                    }
                }
                values.sort_unstable();
                corner[i * BYTES_PER_PIXEL + channel] = values[4];
            }
        }
    }

    fn mask(&self, pair: &Pair, x: usize, y: usize, radius: usize) -> Vec<bool> {
        let rect = pair.light.rectangle();
        let mut mask = vec![false; self.luminance.len()];
        for (i, (&light, &dark)) in pair
            .light
            .pixels()
            .iter()
            .zip(pair.dark.pixels())
            .enumerate()
        {
            if (255 - light).max(dark.abs_diff(1)) <= 2 {
                continue;
            }
            let col = x + i % rect.width;
            let row = y + i / rect.width;
            let left = col.saturating_sub(radius);
            let right = (col + radius + 1).min(self.width);
            for yy in row.saturating_sub(radius)..(row + radius + 1).min(self.height) {
                mask[yy * self.width + left..yy * self.width + right].fill(true);
            }
        }
        mask
    }

    fn has_unaligned_lines(&self, corner: &[u8], mask: &[bool], nearby: &[bool]) -> bool {
        let mut x_edges = 0;
        let mut y_edges = 0;
        let mut affected = 0;
        let edge = |a: usize, b: usize| {
            nearby[b]
                && !mask[b]
                && self.luminance[a].abs_diff(self.luminance[b]) >= 12
                && color_difference(rgb(corner, a), rgb(corner, b)) <= 4
        };
        for i in 0..mask.len() {
            if mask[i] {
                affected += 1;
            } else if nearby[i] {
                if i % self.width + 2 < self.width && edge(i, i + 2) {
                    x_edges += 1;
                }
                if i / self.width + 2 < self.height && edge(i, i + 2 * self.width) {
                    y_edges += 1;
                }
            }
        }
        // An average gradient misses narrow, sparse ink strokes. If the
        // bordering pixels indicate a line without a verified continuation
        // direction, leave it intact instead of smoothing across the stroke.
        x_edges.max(y_edges) * 40 > affected && x_edges.max(y_edges) > 2 * x_edges.min(y_edges)
    }

    fn continuity(
        &self,
        mask: &[bool],
        rect: Rect,
        x: usize,
        y: usize,
        prefer_fine_lines: bool,
    ) -> Option<Option<(isize, isize)>> {
        let mut horizontal = Gradient::default();
        let mut vertical = Gradient::default();
        for row in y..y + rect.height {
            for col in x..x + rect.width {
                let i = row * self.width + col;
                if mask[i] {
                    continue;
                }
                if col + 1 < x + rect.width && !mask[i + 1] {
                    horizontal.push(self.luminance[i].abs_diff(self.luminance[i + 1]));
                }
                if row + 1 < y + rect.height && !mask[i + self.width] {
                    vertical.push(self.luminance[i].abs_diff(self.luminance[i + self.width]));
                }
            }
        }
        let gx = horizontal.mean()?;
        let gy = vertical.mean()?;
        let smooth = gx <= 3.4 && gy <= 3.4;
        if smooth && !prefer_fine_lines {
            return Some(None);
        }
        // Even a smooth region may contain one long ink stroke. Give
        // a strong directional signal priority over isotropic smoothing.
        if gx >= 1.0 && gy <= 2.0 && gx > 3.0 * gy {
            Some(Some(VERTICAL))
        } else if gy >= 1.0 && gx <= 2.0 && gy > 3.0 * gx {
            Some(Some(HORIZONTAL))
        } else if smooth {
            Some(None)
        } else {
            None
        }
    }

    fn neighbor(
        &self,
        corner: &[u8],
        mask: &[bool],
        x: usize,
        y: usize,
        (dx, dy): (isize, isize),
    ) -> Option<(usize, [u8; 3])> {
        for distance in 1..=NEIGHBOR_LIMIT {
            let col = x.checked_add_signed(dx * distance as isize)?;
            let row = y.checked_add_signed(dy * distance as isize)?;
            if col >= self.width || row >= self.height {
                return None;
            }
            let i = row * self.width + col;
            if !mask[i] && corner[i * BYTES_PER_PIXEL + 3] == 255 {
                return Some((distance, rgb(corner, i)));
            }
        }
        None
    }
}

struct Composite {
    transmission: u8,
    additive: i32,
}

impl Composite {
    fn at(pair: &Pair, x: isize, y: isize) -> Self {
        let rect = pair.light.rectangle();
        if x < 0 || y < 0 || x as usize >= rect.width || y as usize >= rect.height {
            return Self {
                transmission: 254,
                additive: 1,
            };
        }
        let i = y as usize * rect.width + x as usize;
        let dark = pair.dark.pixels()[i];
        Self {
            transmission: pair.light.pixels()[i].saturating_sub(dark).min(254),
            additive: i32::from(dark),
        }
    }

    fn over(&self, rgb: [u8; 3]) -> [u8; 3] {
        rgb.map(|c| {
            (self.additive + (i32::from(self.transmission) * (i32::from(c) - 1) + 127) / 254)
                .clamp(0, 255) as u8
        })
    }

    fn recover(&self, actual: [u8; 3], guess: [u8; 3]) -> [u8; 3] {
        // Recover detail directly from the translucent part when it agrees
        // with the surrounding artwork. Near-opaque pixels use continuation.
        if self.transmission < 182 {
            return guess;
        }
        let inverse = actual.map(|c| {
            (1 + (i32::from(c) - self.additive) * 254 / i32::from(self.transmission)).clamp(0, 255)
                as u8
        });
        if difference(inverse, guess) > 7 {
            return guess;
        }
        std::array::from_fn(|c| ((2 * u16::from(guess[c]) + u16::from(inverse[c]) + 1) / 3) as u8)
    }
}

fn detail(pixels: &[u8], stride: usize, i: usize) -> i32 {
    4 * i32::from(pixels[i])
        - i32::from(pixels[i - 2])
        - i32::from(pixels[i + 2])
        - i32::from(pixels[i - 2 * stride])
        - i32::from(pixels[i + 2 * stride])
}

fn rgb(corner: &[u8], i: usize) -> [u8; 3] {
    let start = i * BYTES_PER_PIXEL;
    [corner[start], corner[start + 1], corner[start + 2]]
}

fn difference(a: [u8; 3], b: [u8; 3]) -> usize {
    usize::from(
        a[0].abs_diff(b[0])
            .max(a[1].abs_diff(b[1]))
            .max(a[2].abs_diff(b[2])),
    )
}

fn color_difference(a: [u8; 3], b: [u8; 3]) -> usize {
    let distance = |channel: usize| {
        let first = i32::from(a[channel]) - i32::from(a[0]);
        let second = i32::from(b[channel]) - i32::from(b[0]);
        first.abs_diff(second) as usize
    };
    distance(1).max(distance(2))
}

#[derive(Default)]
struct Gradient {
    sum: usize,
    count: usize,
}

impl Gradient {
    fn push(&mut self, magnitude: u8) {
        self.count += 1;
        self.sum += usize::from(magnitude);
    }

    fn mean(&self) -> Option<f64> {
        (self.count >= 32).then(|| self.sum as f64 / self.count as f64)
    }
}

#[derive(Default)]
struct Correlation {
    count: i64,
    x: i64,
    y: i64,
    xx: i64,
    yy: i64,
    xy: i64,
}

impl Correlation {
    fn push(&mut self, x: i32, y: i32) {
        let (x, y) = (i64::from(x), i64::from(y));
        self.count += 1;
        self.x += x;
        self.y += y;
        self.xx += x * x;
        self.yy += y * y;
        self.xy += x * y;
    }

    fn value(&self) -> f64 {
        let vx = self.count * self.xx - self.x * self.x;
        let vy = self.count * self.yy - self.y * self.y;
        if self.count < 64 || vx <= 0 || vy <= 0 {
            return 0.0;
        }
        let covariance = self.count * self.xy - self.x * self.y;
        covariance as f64 / ((vx as f64) * (vy as f64)).sqrt()
    }
}
