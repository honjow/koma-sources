//! Use clean portions of a monochrome panel frame to constrain restoration.
use super::{difference, rgb, Composite, Pair, Plane, BYTES_PER_PIXEL};
use crate::host;

const PROFILE_WIDTH: usize = 100;
const PROFILE_HEIGHT: usize = 64;
const FRAME_SPAN: usize = 25;

pub(super) fn may_have_frame(plane: &Plane<'_>) -> bool {
    if plane.width < 240 || plane.height < 120 {
        return false;
    }
    let stride = plane.width;
    let span = stride - 120;
    // A full-width light-to-dark change alone is insufficient. This cheap
    // test merely avoids searching signatures where no bottom stroke is visible.
    (plane.height - PROFILE_HEIGHT + 3..plane.height - 3)
        .rev()
        .any(|row| {
            let mut anchors = 0;
            for column in (0..span).step_by(12) {
                if plane.luminance[(row - 3) * stride + column] < 50
                    && plane.luminance[(row + 3) * stride + column] > 230
                {
                    anchors += 1;
                }
            }
            anchors * 12 * 100 >= 75 * span
        })
}

pub(super) fn repair(plane: &Plane<'_>, pixels: &mut [u8], pair: &Pair) -> Result<bool, ()> {
    let rect = pair.light.rectangle();
    let (x, y) = rect.origin(plane.width, plane.height);
    let mask = plane.mask(pair, x, y, super::HALO);
    let Some(frame) = Frame::find(plane, &mask, x, y) else {
        return Ok(false);
    };
    let mut affected = 0;
    let mut replacements = Vec::new();
    for (i, &marked) in mask.iter().enumerate() {
        if i % (16 * plane.width) == 0 && host::cancelled() {
            return Err(());
        }
        if !marked {
            continue;
        }
        affected += 1;
        let start = i * BYTES_PER_PIXEL;
        let pixel = rgb(pixels, i);
        if pixels[start + 3] != 255 || difference([pixel[0]; 3], pixel) > 12 {
            return Ok(false);
        }
        let col = i % plane.width;
        let row = i / plane.width;
        let actual = plane.luminance[i];
        let model = Composite::at(pair, col as isize - x as isize, row as isize - y as isize);
        let mut expected = frame.shade(col, row);
        if expected.is_none()
            && error(&model, actual, 255) <= 49
            && (model.transmission < 56 || error(&model, actual, 255) <= error(&model, actual, 0))
            && white_on_two_axes(plane, pixels, &mask, col, row)
        {
            expected = Some(255);
        }
        let shade = expected
            .filter(|shade| error(&model, actual, *shade) <= 49)
            .or_else(|| inverse_gray(&model, actual));
        if let Some(shade) = shade {
            replacements.push((start, shade));
        }
    }
    if affected == 0 || replacements.len() * 100 < affected * 85 {
        return Ok(false);
    }
    for (start, shade) in replacements {
        pixels[start..start + 3].fill(shade);
    }
    Ok(true)
}

fn error(model: &Composite, actual: u8, shade: u8) -> u8 {
    model.over([shade; 3])[0].abs_diff(actual)
}

fn inverse_gray(model: &Composite, actual: u8) -> Option<u8> {
    if model.transmission < 115 {
        return None;
    }
    let value = 1 + (i32::from(actual) - model.additive) * 254 / i32::from(model.transmission);
    match value {
        -40..=22 => Some(0),
        233..=295 => Some(255),
        -14..=269 if model.transmission >= 178 => Some(value.clamp(0, 255) as u8),
        _ => None,
    }
}

fn white_on_two_axes(plane: &Plane<'_>, pixels: &[u8], mask: &[bool], x: usize, y: usize) -> bool {
    let mut agreements = 0;
    for (dx, dy) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
        let white = |direction| {
            plane
                .neighbor(pixels, mask, x, y, direction)
                .is_some_and(|(distance, color)| {
                    distance <= 28 && *color.iter().min().unwrap() >= 243
                })
        };
        if white((dx, dy)) && white((-dx, -dy)) {
            agreements += 1;
            if agreements == 2 {
                return true;
            }
        }
    }
    false
}

struct Frame {
    vertical: Profile,
    horizontal: Profile,
}

impl Frame {
    fn find(plane: &Plane<'_>, mask: &[bool], x: usize, y: usize) -> Option<Self> {
        if plane.width < 240 || plane.height < 120 || y < 16 {
            return None;
        }
        let top = y.min(62);
        let inner_width = x.saturating_sub(8).max(140);
        let vertical = profile(
            plane,
            mask,
            plane.width - PROFILE_WIDTH,
            PROFILE_WIDTH,
            top,
            true,
        )?;
        let horizontal = profile(
            plane,
            mask,
            plane.height - PROFILE_HEIGHT,
            PROFILE_HEIGHT,
            inner_width,
            false,
        )?;
        Some(Self {
            vertical,
            horizontal,
        })
    }

    fn shade(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.vertical.edge || y >= self.horizontal.edge {
            let v = self.vertical.outside(x).unwrap_or(0);
            let h = self.horizontal.outside(y).unwrap_or(0);
            Some(v.max(h))
        } else {
            self.vertical.ink(x).or_else(|| self.horizontal.ink(y))
        }
    }
}

struct Profile {
    start: usize,
    edge: usize,
    values: Vec<u8>,
}

impl Profile {
    fn outside(&self, index: usize) -> Option<u8> {
        (index >= self.edge).then(|| self.values[index - self.start])
    }

    fn ink(&self, index: usize) -> Option<u8> {
        if index >= self.start && index + FRAME_SPAN >= self.edge {
            let shade = self.values[index - self.start];
            if shade <= 80 {
                return Some(shade);
            }
        }
        None
    }
}

fn profile(
    plane: &Plane<'_>,
    mask: &[bool],
    start: usize,
    count: usize,
    length: usize,
    vertical: bool,
) -> Option<Profile> {
    let sample = |axis: usize, along: usize| {
        let index = if vertical {
            along * plane.width + start + axis
        } else {
            (start + axis) * plane.width + along
        };
        (!mask[index]).then_some(plane.luminance[index])
    };
    let mut values = Vec::with_capacity(count);
    for axis in 0..count {
        let mut histogram = [0u16; 256];
        let mut samples = 0u16;
        for along in 0..length {
            if let Some(pixel) = sample(axis, along) {
                histogram[pixel as usize] += 1;
                samples += 1;
            }
        }
        if usize::from(samples) * 2 < length {
            return None;
        }
        let mut total = 0;
        let median = histogram.iter().position(|n| {
            total += *n;
            total > samples / 2
        })?;
        values.push(median as u8);
    }
    // Require a long dark-to-white boundary with an independently visible
    // stroke on one side and a light margin on the other.
    for edge in (5..count - 5).rev() {
        if values[edge] < 128
            || values[edge - 5..edge - 1].iter().any(|&v| v > 45)
            || values[edge + 1..edge + 6].iter().any(|&v| v < 233)
        {
            continue;
        }
        let agrees = |axes: std::ops::Range<usize>, dark: bool| {
            let mut good = 0;
            for axis in axes {
                for along in 0..length {
                    if sample(axis, along).is_some_and(|p| if dark { p < 50 } else { p > 230 }) {
                        good += 1;
                    }
                }
            }
            good * 100 >= 79 * length * 3
        };
        if agrees(edge - 4..edge - 1, true) && agrees(edge + 2..edge + 5, false) {
            return Some(Profile {
                start,
                edge: start + edge,
                values,
            });
        }
    }
    None
}
