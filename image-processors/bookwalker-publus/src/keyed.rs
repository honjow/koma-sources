// Keyed PUBLUS permutation, adapted from Keiyoushi extensions-source
// lib/publus/Publus.kt at 8947c372cd87f546a0b80cb0e4b0a5a8add5ef88 (Apache-2.0).
use crate::{
    moves::{Grid, Move},
    Page, PageKeys,
};
use alloc::{vec, vec::Vec};

const VARIANTS: u32 = 6;
const DEFAULT_SEED: u32 = 2_463_534_242;
const SHIFTS: [[u32; 3]; 81] = [
    [1, 3, 10],
    [1, 5, 16],
    [1, 5, 19],
    [1, 9, 29],
    [1, 11, 6],
    [1, 11, 16],
    [1, 19, 3],
    [1, 21, 20],
    [1, 27, 27],
    [2, 5, 15],
    [2, 5, 21],
    [2, 7, 7],
    [2, 7, 9],
    [2, 7, 25],
    [2, 9, 15],
    [2, 15, 17],
    [2, 15, 25],
    [2, 21, 9],
    [3, 1, 14],
    [3, 3, 26],
    [3, 3, 28],
    [3, 3, 29],
    [3, 5, 20],
    [3, 5, 22],
    [3, 5, 25],
    [3, 7, 29],
    [3, 13, 7],
    [3, 23, 25],
    [3, 25, 24],
    [3, 27, 11],
    [4, 3, 17],
    [4, 3, 27],
    [4, 5, 15],
    [5, 3, 21],
    [5, 7, 22],
    [5, 9, 7],
    [5, 9, 28],
    [5, 9, 31],
    [5, 13, 6],
    [5, 15, 17],
    [5, 17, 13],
    [5, 21, 12],
    [5, 27, 8],
    [5, 27, 21],
    [5, 27, 25],
    [5, 27, 28],
    [6, 1, 11],
    [6, 3, 17],
    [6, 17, 9],
    [6, 21, 7],
    [6, 21, 13],
    [7, 1, 9],
    [7, 1, 18],
    [7, 1, 25],
    [7, 13, 25],
    [7, 17, 21],
    [7, 25, 12],
    [7, 25, 20],
    [8, 7, 23],
    [8, 9, 23],
    [9, 5, 14],
    [9, 5, 25],
    [9, 11, 19],
    [9, 21, 16],
    [10, 9, 21],
    [10, 9, 25],
    [11, 7, 12],
    [11, 7, 16],
    [11, 17, 13],
    [11, 21, 13],
    [12, 9, 23],
    [13, 3, 17],
    [13, 3, 27],
    [13, 5, 19],
    [13, 17, 15],
    [14, 1, 15],
    [14, 13, 15],
    [15, 1, 29],
    [17, 15, 20],
    [17, 15, 23],
    [17, 15, 26],
];

struct Random {
    state: u32,
    shifts: [u32; 3],
    variant: u32,
}

impl Random {
    fn for_index(index: u32, seed: u32) -> Self {
        Self {
            state: if seed == 0 { DEFAULT_SEED } else { seed },
            shifts: SHIFTS[((index / VARIANTS) as usize) % SHIFTS.len()],
            variant: index % VARIANTS,
        }
    }

    fn reseed(&mut self, seed: u32) {
        self.state = if seed == 0 { DEFAULT_SEED } else { seed };
    }

    fn draw(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return 0;
        }
        let bound = bound as u32;
        let [a, b, c] = self.shifts;
        loop {
            let mut value = self.state;
            match self.variant {
                0 => {
                    value ^= value << a;
                    value ^= value >> b;
                    value ^= value << c;
                }
                1 => {
                    value ^= value << c;
                    value ^= value >> b;
                    value ^= value << a;
                }
                2 => {
                    value ^= value >> a;
                    value ^= value << b;
                    value ^= value >> c;
                }
                3 => {
                    value ^= value >> c;
                    value ^= value << b;
                    value ^= value >> a;
                }
                4 => {
                    value ^= value << a;
                    value ^= value << c;
                    value ^= value >> b;
                }
                _ => {
                    value ^= value >> a;
                    value ^= value >> c;
                    value ^= value << b;
                }
            }
            self.state = value;
            let previous = value.wrapping_sub(1);
            let remainder = previous % bound;
            if previous - remainder <= u32::MAX - bound {
                return remainder as usize;
            }
        }
    }
}

struct Positions {
    values: Vec<usize>,
    leading: usize,
    used: usize,
    remaining: usize,
}

impl Positions {
    fn new(count: usize, leading: usize) -> Self {
        Self {
            values: vec![0; count],
            leading,
            used: 0,
            remaining: count,
        }
    }

    fn insert(&mut self, other: &Self, selected: usize, rng: &mut Random) {
        let leading = selected < self.leading;
        let current = if leading {
            self.used
        } else {
            self.used + self.remaining
        };
        let precedes = |other_value| {
            if leading {
                current < other_value
            } else {
                current > other_value
            }
        };
        let mut left = other.used;
        while left > 0 && precedes(other.values[left - 1]) {
            left -= 1;
        }
        let mut right = other.used + other.remaining;
        while right < other.values.len() && precedes(other.values[right]) {
            right += 1;
        }
        let index = if leading { self.used } else { current - 1 };
        self.values[index] = left + rng.draw(right - left);
        if leading {
            self.used += 1;
            self.leading -= 1;
        }
        self.remaining -= 1;
    }
}

struct StripLayout {
    x: usize,
    y: usize,
    gap_by_col: Vec<usize>,
    gap_by_row: Vec<usize>,
}

impl StripLayout {
    fn new(grid: &Grid, x: usize, y: usize, rng: &mut Random) -> Self {
        let mut cols = Positions::new(grid.cols, x);
        let mut rows = Positions::new(grid.rows, y);
        while cols.remaining + rows.remaining > 0 {
            let index = rng.draw(cols.remaining + rows.remaining);
            if index < cols.remaining {
                cols.insert(&rows, index, rng);
            } else {
                rows.insert(&cols, index - cols.remaining, rng);
            }
        }
        Self {
            x,
            y,
            gap_by_col: cols.values,
            gap_by_row: rows.values,
        }
    }

    fn full(&self, grid: &Grid, x: usize, y: usize) -> (usize, usize) {
        (
            grid.col(x, self.gap_by_row[y]),
            grid.row(y, self.gap_by_col[x]),
        )
    }

    fn middle(&self, grid: &Grid) -> (usize, usize) {
        (self.x * grid.block_width, self.y * grid.block_height)
    }

    fn bottom(&self, grid: &Grid, col: usize) -> (usize, usize) {
        (
            grid.col(col, self.x),
            self.gap_by_col[col] * grid.block_height,
        )
    }

    fn right(&self, grid: &Grid, row: usize) -> (usize, usize) {
        (
            self.gap_by_row[row] * grid.block_width,
            grid.row(row, self.y),
        )
    }
}

fn permute(count: usize, rng: &mut Random) -> Vec<usize> {
    let mut slots = vec![0; count];
    for i in 0..count {
        let previous = rng.draw(i + 1);
        slots[i] = slots[previous];
        slots[previous] = i;
    }
    slots
}

fn internal_gap(count: usize, rng: &mut Random) -> usize {
    if count < 4 {
        rng.draw(count + 1)
    } else {
        rng.draw(count - 1) + 1
    }
}

fn other_gap(count: usize, original: usize, rng: &mut Random) -> usize {
    if count == 0 {
        return 0;
    }
    let index = rng.draw(count);
    index + usize::from(index >= original)
}

fn fold_key(key: &[u8; 32]) -> u32 {
    key.chunks_exact(4).fold(0, |word, bytes| {
        word ^ u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    })
}

pub(super) fn with_keys(
    width: usize,
    height: usize,
    page: &Page,
    keys: &PageKeys,
) -> Option<Vec<Move>> {
    let grid = Grid::new(width, height, page.block_width, page.block_height)?;
    let total = 47
        + page.file.encode_utf16().map(u32::from).sum::<u32>()
        + alloc::format!("{}", page.no)
            .bytes()
            .map(u32::from)
            .sum::<u32>()
        + keys.iter().flatten().copied().map(u32::from).sum::<u32>();
    let repeated = (total & 255) * 0x01010101;
    let seed_layout = total % ((SHIFTS.len() as u32) * VARIANTS);
    let x_seed = repeated ^ fold_key(&keys[0]) ^ page.ns as u32;
    let y_seed = repeated ^ fold_key(&keys[1]) ^ page.ps as u32;
    let rest_seed = repeated ^ fold_key(&keys[2]) ^ page.rs as u32;
    let cross_seed = x_seed ^ y_seed ^ rest_seed;
    let mut rng = Random::for_index(
        seed_layout ^ grid.cols as u32 ^ grid.rows as u32,
        cross_seed,
    );
    let salt = rng.draw(65_536) as u32 | ((rng.draw(65_536) as u32) << 16);
    let salt_high = rng.draw(512) as u32;
    // The reference packs the dimension and seed into exact 64-bit integers.
    // Its high 16-bit mix consequently depends only on these grid dimensions.
    let mut rng = Random::for_index(
        grid.cols as u32 ^ grid.rows as u32 ^ seed_layout,
        cross_seed,
    );
    let mask = rng.draw(65_536) as u32 | ((rng.draw(65_536) as u32) << 16);
    let variant = salt_high ^ seed_layout ^ rng.draw(512) as u32;

    let salt = salt ^ mask;
    let mut rng = Random::for_index(variant, salt ^ x_seed);
    let full = permute(grid.cols * grid.rows, &mut rng);
    rng.reseed(salt ^ y_seed);
    let x = internal_gap(grid.cols, &mut rng);
    let y = internal_gap(grid.rows, &mut rng);
    let read_x = other_gap(grid.cols, x, &mut rng);
    let read_y = other_gap(grid.rows, y, &mut rng);
    rng.reseed(salt ^ rest_seed);
    let write_layout = StripLayout::new(&grid, x, y, &mut rng);
    let bottom = permute(grid.cols, &mut rng);
    let right = permute(grid.rows, &mut rng);
    let read_layout = StripLayout::new(&grid, read_x, read_y, &mut rng);

    let mut moves = Vec::new();
    for col in 0..grid.cols {
        for row in 0..grid.rows {
            let other = full[col + row * grid.cols];
            grid.push(
                &mut moves,
                read_layout.full(&grid, other % grid.cols, other / grid.cols),
                write_layout.full(&grid, col, row),
                (grid.block_width, grid.block_height),
            );
        }
    }
    grid.push(
        &mut moves,
        read_layout.middle(&grid),
        write_layout.middle(&grid),
        (grid.rest_width, grid.rest_height),
    );
    for (col, other) in bottom.into_iter().enumerate() {
        grid.push(
            &mut moves,
            read_layout.bottom(&grid, other),
            write_layout.bottom(&grid, col),
            (grid.block_width, grid.rest_height),
        );
    }
    for (row, other) in right.into_iter().enumerate() {
        grid.push(
            &mut moves,
            read_layout.right(&grid, other),
            write_layout.right(&grid, row),
            (grid.rest_width, grid.block_height),
        );
    }
    Some(moves)
}
