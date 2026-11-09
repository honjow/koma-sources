//! Match known watermarks independently in both bottom corners.
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
const RIGHT_PATTERNS: &[Pattern] = &[
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

const LEFT_CLOUD: Pattern = Pattern(include_bytes!("patterns/left-cloud.bin"));
const LEFT_DIAGONAL: Pattern = Pattern(include_bytes!("patterns/left-dm5-diagonal.bin"));
const LEFT_VERTICAL: Pattern = Pattern(include_bytes!("patterns/left-dm5-vertical.bin"));
const LEFT_DARK_DIAGONAL: Pattern = Pattern(include_bytes!("patterns/left-dark-dm5-diagonal.bin"));
const LEFT_DARK_VERTICAL: Pattern = Pattern(include_bytes!("patterns/left-dark-dm5-vertical.bin"));

const LEFT_PATTERNS: &[Pattern] = &[
    Pattern(include_bytes!("patterns/left-cloud-dm5-diagonal.bin")),
    Pattern(include_bytes!("patterns/left-cloud-dm5-vertical.bin")),
    LEFT_CLOUD,
    LEFT_DIAGONAL,
    LEFT_VERTICAL,
    LEFT_DARK_DIAGONAL,
    LEFT_DARK_VERTICAL,
];

// DM5 searches use their distinct strokes, avoiding the shared small copy mark.
const RIGHT_ARTWORK: &[Pair] = &[
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
];

const COPY_ARTWORK: Pair = Pair {
    light: COPY,
    dark: DARK_COPY,
    signature_width: 69,
    signature_height: 26,
};

const LEFT_ARTWORK: &[Pair] = &[
    Pair {
        light: LEFT_DIAGONAL,
        dark: LEFT_DARK_DIAGONAL,
        signature_width: 87,
        signature_height: 93,
    },
    Pair {
        light: LEFT_VERTICAL,
        dark: LEFT_DARK_VERTICAL,
        signature_width: 36,
        signature_height: 111,
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
        top_padding: usize,
    ) -> Option<(Rect, u8)> {
        let mut rect = self.rectangle();
        rect.height -= top_padding;
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

        for (row, expected_row) in self
            .pixels()
            .chunks_exact(rect.width)
            .skip(top_padding)
            .enumerate()
        {
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
        Some((rect, ((border_sum + clean_border / 2) / clean_border) as u8))
    }

    fn removable_top_padding(&self) -> usize {
        // Discard only template rows consisting entirely of background,
        // keeping a border row between the mark and adjacent panel.
        self.pixels()
            .chunks_exact(self.rectangle().width)
            .take(9)
            .take_while(|row| row.iter().all(|p| p.abs_diff(self.0[10]) <= 8))
            .count()
            .saturating_sub(1)
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
    let left = prepare_corner(width, height, Side::Left)?;
    let right = prepare_corner(width, height, Side::Right)?;
    if left.is_none() && right.is_none() {
        return Ok(0);
    }
    if let (Some(left), Some(right)) = (&left, &right) {
        // The independent matches must never overwrite each other's pixels.
        if left.width + right.width > width {
            return Err(());
        }
    }
    write_output(width, height, [left.as_ref(), right.as_ref()])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

struct Corner {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Corner {
    fn reverse_rows(&mut self) {
        for row in self.pixels.chunks_exact_mut(self.width * BYTES_PER_PIXEL) {
            let (left, right) = row.split_at_mut(self.width / 2 * BYTES_PER_PIXEL);
            for (a, b) in left
                .chunks_exact_mut(BYTES_PER_PIXEL)
                .zip(right.chunks_exact_mut(BYTES_PER_PIXEL).rev())
            {
                a.swap_with_slice(b);
            }
        }
    }

    fn fill(&mut self, rect: Rect, fill: u8) {
        let (x, y) = rect.origin(self.width, self.height);
        for row in y..y + rect.height {
            let start = (row * self.width + x) * BYTES_PER_PIXEL;
            for pixel in self.pixels[start..start + rect.width * BYTES_PER_PIXEL]
                .chunks_exact_mut(BYTES_PER_PIXEL)
            {
                pixel[..3].fill(fill);
            }
        }
    }
}

fn prepare_corner(width: usize, height: usize, side: Side) -> Result<Option<Corner>, ()> {
    let (patterns, templates) = match side {
        Side::Left => (
            LEFT_PATTERNS,
            artwork::Templates {
                pairs: LEFT_ARTWORK,
                copy: None,
                cloud: LEFT_CLOUD,
                prefer_fine_lines: true,
            },
        ),
        Side::Right => (
            RIGHT_PATTERNS,
            artwork::Templates {
                pairs: RIGHT_ARTWORK,
                copy: Some(&COPY_ARTWORK),
                cloud: CLOUD,
                prefer_fine_lines: false,
            },
        ),
    };
    let candidates = patterns.iter().filter(|pattern| {
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
        return Ok(None);
    }
    if host::cancelled() {
        return Err(());
    }
    // Keep a few clean pixels beyond the signatures for directional recovery.
    let corner_width = (corner_width + 18).min(width);
    let corner_height = (corner_height + 2).min(height);
    let mut corner = Corner {
        x: if side == Side::Left {
            0
        } else {
            width - corner_width
        },
        y: height - corner_height,
        width: corner_width,
        height: corner_height,
        pixels: vec![0; corner_width * corner_height * BYTES_PER_PIXEL],
    };
    host::region(
        corner.x as i32,
        corner.y as i32,
        corner_width as i32,
        corner_height as i32,
        &mut corner.pixels,
    )?;
    // Left patterns come from real left marks. Reversing the coordinates
    // lets them use the same matching and repair code as the right marks.
    if side == Side::Left {
        corner.reverse_rows();
    }
    let mut changed = fill_matching(&mut corner, candidates.clone(), false)?;
    if !changed {
        changed = artwork::restore(
            &mut corner.pixels,
            corner_width,
            corner_height,
            width,
            templates,
        )?;
    }
    if !changed {
        // A nearby panel can lie inside the outer margin of the template.
        // Retry with a tighter guard only after the established paths miss.
        changed = fill_matching(&mut corner, candidates, true)?;
    }
    if !changed {
        return Ok(None);
    }
    if side == Side::Left {
        corner.reverse_rows();
    }
    Ok(Some(corner))
}

fn fill_matching<'a>(
    corner: &mut Corner,
    candidates: impl Iterator<Item = &'a Pattern>,
    tighten_top: bool,
) -> Result<bool, ()> {
    for pattern in candidates {
        if host::cancelled() {
            return Err(());
        }
        let padding = if tighten_top {
            pattern.removable_top_padding()
        } else {
            0
        };
        if tighten_top && padding == 0 {
            continue;
        }
        if let Some((rect, fill)) =
            pattern.fill_if_matching(&corner.pixels, corner.width, corner.height, padding)
        {
            corner.fill(rect, fill);
            return Ok(true);
        }
    }
    Ok(false)
}

fn write_output(width: usize, height: usize, corners: [Option<&Corner>; 2]) -> Result<i32, ()> {
    const ROWS: usize = 32;
    let stride = width * BYTES_PER_PIXEL;
    let mut strip = vec![0; ROWS * stride];
    host::create(width as i32, height as i32)?;
    for top in (0..height).step_by(ROWS) {
        if host::cancelled() {
            return Err(());
        }
        let rows = ROWS.min(height - top);
        let bytes = &mut strip[..rows * stride];
        host::region(0, top as i32, width as i32, rows as i32, bytes)?;
        for corner in corners.into_iter().flatten() {
            let corner_stride = corner.width * BYTES_PER_PIXEL;
            for row in top.max(corner.y)..top + rows {
                let src = (row - corner.y) * corner_stride;
                let dst = (row - top) * stride + corner.x * BYTES_PER_PIXEL;
                bytes[dst..dst + corner_stride]
                    .copy_from_slice(&corner.pixels[src..src + corner_stride]);
            }
        }
        host::write(top as i32, width as i32, rows as i32, bytes)?;
    }
    Ok(1)
}
