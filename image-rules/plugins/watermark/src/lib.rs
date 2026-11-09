//! Match known bottom-right watermarks in an otherwise flat margin.
use koma_image_sdk as host;

mod artwork;
use artwork::Pair;

const BYTES_PER_PIXEL: usize = 4;

const CLOUD: Pattern = Pattern(include_bytes!("patterns/cloud.bin"));
const DIAGONAL: Pattern = Pattern(include_bytes!("patterns/dm5-diagonal.bin"));
const HORIZONTAL: Pattern = Pattern(include_bytes!("patterns/dm5-horizontal.bin"));
const VERTICAL: Pattern = Pattern(include_bytes!("patterns/dm5-vertical.bin"));
const COPY: Pattern = Pattern(include_bytes!("patterns/copy.bin"));
const DARK_DIAGONAL: Pattern = Pattern(include_bytes!("patterns/dark-dm5-diagonal.bin"));
const DARK_HORIZONTAL: Pattern = Pattern(include_bytes!("patterns/dark-dm5-horizontal.bin"));
const DARK_VERTICAL: Pattern = Pattern(include_bytes!("patterns/dark-dm5-vertical.bin"));
const DARK_COPY: Pattern = Pattern(include_bytes!("patterns/dark-copy.bin"));

// Check complete composites before smaller overlapping marks.
const PATTERNS: &[Pattern] = &[
    Pattern(include_bytes!("patterns/cloud-dm5-diagonal.bin")),
    Pattern(include_bytes!("patterns/cloud-dm5-horizontal.bin")),
    Pattern(include_bytes!("patterns/cloud-dm5-vertical.bin")),
    CLOUD,
    DIAGONAL,
    HORIZONTAL,
    VERTICAL,
    COPY,
    DARK_DIAGONAL,
    DARK_HORIZONTAL,
    DARK_VERTICAL,
    DARK_COPY,
];

// DM5 searches use their distinct strokes, avoiding the shared small copy mark.
const ARTWORK_PAIRS: &[Pair] = &[
    Pair {
        light: DIAGONAL,
        dark: DARK_DIAGONAL,
        signature_width: 100,
        signature_height: 67,
    },
    Pair {
        light: HORIZONTAL,
        dark: DARK_HORIZONTAL,
        signature_width: 51,
        signature_height: 42,
    },
    Pair {
        light: VERTICAL,
        dark: DARK_VERTICAL,
        signature_width: 70,
        signature_height: 86,
    },
    Pair {
        light: COPY,
        dark: DARK_COPY,
        signature_width: 69,
        signature_height: 26,
    },
];

#[derive(Clone, Copy)]
struct Pattern(&'static [u8]);

impl Pattern {
    fn number(&self, offset: usize) -> usize {
        usize::from(u16::from_le_bytes([self.0[offset], self.0[offset + 1]]))
    }

    fn page_width(&self) -> usize {
        self.number(0)
    }

    fn rectangle(&self) -> Rect {
        Rect {
            right: self.number(2),
            bottom: self.number(4),
            width: self.number(6),
            height: self.number(8),
        }
    }

    fn pixels(&self) -> &'static [u8] {
        &self.0[11..]
    }

    fn fill_if_matching(
        &self,
        corner: &[u8],
        corner_width: usize,
        corner_height: usize,
    ) -> Option<u8> {
        let rect = self.rectangle();
        let background = self.0[10];
        let (x, y) = rect.origin(corner_width, corner_height);
        let stride = corner_width * 4;
        let mut deviation = 0usize;
        let mut outliers = 0usize;
        let mut signature_points = 0usize;
        let mut signature_hits = 0usize;
        let mut border_points = 0usize;
        let mut clean_border = 0usize;
        let mut border_sum = 0usize;

        for (row, expected_row) in self.pixels().chunks_exact(rect.width).enumerate() {
            let start = (y + row) * stride + x * 4;
            let input_row = &corner[start..start + rect.width * 4];
            for (col, (pixel, expected)) in input_row.chunks_exact(4).zip(expected_row).enumerate()
            {
                let [r, g, b, alpha] = [pixel[0], pixel[1], pixel[2], pixel[3]];
                let spread = r.max(g).max(b) - r.min(g).min(b);
                // Color, transparency and strong unexplained luminance can be artwork.
                if alpha != 255 || spread > 30 {
                    return None;
                }
                let luma = luma(pixel);
                let error = luma.abs_diff(*expected);
                if error > 60 {
                    return None;
                }
                deviation += usize::from(error);
                outliers += usize::from(error > 22);
                if expected.abs_diff(background) >= 14 {
                    signature_points += 1;
                    signature_hits += usize::from(luma.abs_diff(background) >= 10 && error <= 35);
                }
                if row == 0 || row + 1 == rect.height || col == 0 || col + 1 == rect.width {
                    let distance = luma.abs_diff(background);
                    if distance > 20 {
                        return None;
                    }
                    border_points += 1;
                    if distance <= 8 {
                        clean_border += 1;
                        border_sum += usize::from(luma);
                    }
                }
            }
        }

        let pixels = rect.width * rect.height;
        // A high signature match alone is insufficient: the entire rectangle
        // must agree with the mark on a flat margin before replacing pixels.
        if deviation > 7 * pixels
            || outliers * 100 > 4 * pixels
            || signature_hits * 100 < 90 * signature_points
            || clean_border * 100 < 97 * border_points
        {
            return None;
        }
        Some(((border_sum + clean_border / 2) / clean_border) as u8)
    }
}

#[derive(Clone, Copy)]
struct Rect {
    right: usize,
    bottom: usize,
    width: usize,
    height: usize,
}

impl Rect {
    fn origin(self, width: usize, height: usize) -> (usize, usize) {
        (
            width - self.right - self.width,
            height - self.bottom - self.height,
        )
    }
}

fn luma(pixel: &[u8]) -> u8 {
    ((77 * u16::from(pixel[0]) + 150 * u16::from(pixel[1]) + 29 * u16::from(pixel[2]) + 128) >> 8)
        as u8
}

#[no_mangle]
pub extern "C" fn koma_image_processor_abi() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn koma_image_process() -> i32 {
    process().unwrap_or(-1)
}

fn process() -> Result<i32, ()> {
    let width = usize::try_from(unsafe { host::width() }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { host::height() }).map_err(|_| ())?;
    let candidates = PATTERNS.iter().filter(|pattern| {
        let rect = pattern.rectangle();
        pattern.page_width() == width && rect.bottom + rect.height <= height
    });
    let (corner_width, corner_height) = candidates.clone().fold((0, 0), |(w, h), pattern| {
        let rect = pattern.rectangle();
        (
            w.max(rect.right + rect.width),
            h.max(rect.bottom + rect.height),
        )
    });
    if corner_width == 0 {
        return Ok(0);
    }
    if host::cancelled() {
        return Err(());
    }
    // Keep a few clean pixels beyond the signatures for directional recovery.
    let corner_width = (corner_width + 18).min(width);
    let corner_height = (corner_height + 2).min(height);
    let mut corner = vec![0; corner_width * corner_height * 4];
    host::region(
        (width - corner_width) as i32,
        (height - corner_height) as i32,
        corner_width as i32,
        corner_height as i32,
        &mut corner,
    )?;
    for pattern in candidates {
        if host::cancelled() {
            return Err(());
        }
        if let Some(fill) = pattern.fill_if_matching(&corner, corner_width, corner_height) {
            let rect = pattern.rectangle();
            let (x, y) = rect.origin(corner_width, corner_height);
            for row in y..y + rect.height {
                let start = (row * corner_width + x) * BYTES_PER_PIXEL;
                for pixel in corner[start..start + rect.width * BYTES_PER_PIXEL]
                    .chunks_exact_mut(BYTES_PER_PIXEL)
                {
                    pixel[..3].fill(fill);
                }
            }
            return write_output(width, height, corner_width, corner_height, &corner);
        }
    }
    if artwork::restore(
        &mut corner,
        corner_width,
        corner_height,
        width,
        ARTWORK_PAIRS,
        CLOUD,
    )? {
        return write_output(width, height, corner_width, corner_height, &corner);
    }
    Ok(0)
}

fn write_output(
    width: usize,
    height: usize,
    corner_width: usize,
    corner_height: usize,
    corner: &[u8],
) -> Result<i32, ()> {
    const ROWS: usize = 32;
    let stride = width * BYTES_PER_PIXEL;
    let corner_stride = corner_width * BYTES_PER_PIXEL;
    let x = width - corner_width;
    let y = height - corner_height;
    let mut strip = vec![0; ROWS * stride];
    host::create(width as i32, height as i32)?;
    for top in (0..height).step_by(ROWS) {
        if host::cancelled() {
            return Err(());
        }
        let rows = ROWS.min(height - top);
        let bytes = &mut strip[..rows * stride];
        host::region(0, top as i32, width as i32, rows as i32, bytes)?;
        for row in top.max(y)..top + rows {
            let src = (row - y) * corner_stride;
            let dst = (row - top) * stride + x * BYTES_PER_PIXEL;
            bytes[dst..dst + corner_stride].copy_from_slice(&corner[src..src + corner_stride]);
        }
        host::write(top as i32, width as i32, rows as i32, bytes)?;
    }
    Ok(1)
}
