// PUBLUS pixel layout adapted from Keiyoushi extensions-source
// lib/publus/Publus.kt at 8947c372cd87f546a0b80cb0e4b0a5a8add5ef88 (Apache-2.0).
use crate::Page;
use alloc::vec::Vec;

pub(super) const MAX_MOVES: usize = 16_384;

pub(super) struct Move {
    pub(super) read_x: usize,
    pub(super) read_y: usize,
    pub(super) write_x: usize,
    pub(super) write_y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl Move {
    pub(super) fn new(read: (usize, usize), write: (usize, usize), size: (usize, usize)) -> Self {
        Self {
            read_x: read.0,
            read_y: read.1,
            write_x: write.0,
            write_y: write.1,
            width: size.0,
            height: size.1,
        }
    }
}

pub(super) struct Grid {
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) block_width: usize,
    pub(super) block_height: usize,
    pub(super) rest_width: usize,
    pub(super) rest_height: usize,
}

impl Grid {
    pub(super) fn new(
        width: usize,
        height: usize,
        block_width: usize,
        block_height: usize,
    ) -> Option<Self> {
        let cols = width.checked_div(block_width)?;
        let rows = height.checked_div(block_height)?;
        if (cols + 1).checked_mul(rows + 1)? > MAX_MOVES {
            return None;
        }
        Some(Self {
            cols,
            rows,
            block_width,
            block_height,
            rest_width: width % block_width,
            rest_height: height % block_height,
        })
    }

    pub(super) fn col(&self, col: usize, gap: usize) -> usize {
        col * self.block_width + if col >= gap { self.rest_width } else { 0 }
    }

    pub(super) fn row(&self, row: usize, gap: usize) -> usize {
        row * self.block_height + if row >= gap { self.rest_height } else { 0 }
    }

    pub(super) fn push(
        &self,
        moves: &mut Vec<Move>,
        read: (usize, usize),
        write: (usize, usize),
        size: (usize, usize),
    ) {
        if size.0 > 0 && size.1 > 0 {
            moves.push(Move::new(read, write, size));
        }
    }
}

fn rest_position(count: usize, pattern: usize, prime: usize) -> Option<usize> {
    if count == 0 {
        return Some(0);
    }
    let position = count - prime * pattern % count;
    if position < count {
        return Some(position);
    }
    let fallback = (count as isize - 4) % count as isize;
    if fallback == 0 {
        Some(count - 1)
    } else {
        usize::try_from(fallback).ok()
    }
}

// A remainder strip may be placed on either side of the intersection.
fn row_gap(col: usize, grid: &Grid, center: (usize, usize), pattern: usize) -> usize {
    let (x, y) = center;
    let first = (col < x) == (pattern % 2 == 1);
    let (size, offset) = if first { (y, 0) } else { (grid.rows - y, y) };
    if size == 0 {
        0
    } else {
        (col + 53 * pattern + 59 * y) % size + offset
    }
}

fn col_gap(row: usize, grid: &Grid, center: (usize, usize), pattern: usize) -> usize {
    let (x, y) = center;
    let second = (row < y) == (pattern % 2 == 1);
    let (size, offset) = if second { (grid.cols - x, x) } else { (x, 0) };
    if size == 0 {
        0
    } else {
        (row + 67 * pattern + x + 71) % size + offset
    }
}

pub(super) fn without_keys(width: usize, height: usize, page: &Page) -> Option<Vec<Move>> {
    let grid = Grid::new(width, height, 64, 64)?;
    let (cols, rows) = (grid.cols, grid.rows);
    let pattern = format_args_sum(&page.file, page.no);
    let x = rest_position(cols, pattern, 43)?;
    let y = rest_position(rows, pattern, 47)?;
    let mut moves = Vec::new();
    let center = (x * 64, y * 64);

    grid.push(
        &mut moves,
        center,
        center,
        (grid.rest_width, grid.rest_height),
    );
    if grid.rest_height > 0 {
        for col in 0..cols {
            let other = (col + 61 * pattern) % cols;
            grid.push(
                &mut moves,
                (
                    grid.col(other, x),
                    64 * row_gap(other, &grid, (x, y), pattern),
                ),
                (grid.col(col, x), 64 * y),
                (64, grid.rest_height),
            );
        }
    }
    if grid.rest_width > 0 {
        for row in 0..rows {
            let other = (row + 73 * pattern) % rows;
            grid.push(
                &mut moves,
                (
                    64 * col_gap(other, &grid, (x, y), pattern),
                    grid.row(other, y),
                ),
                (64 * x, grid.row(row, y)),
                (grid.rest_width, 64),
            );
        }
    }
    for col in 0..cols {
        for row in 0..rows {
            let other_col = (col + 29 * pattern + 31 * row) % cols;
            let other_row = (row + 37 * pattern + 41 * other_col) % rows;
            grid.push(
                &mut moves,
                (
                    grid.col(other_col, col_gap(other_row, &grid, (x, y), pattern)),
                    grid.row(other_row, row_gap(other_col, &grid, (x, y), pattern)),
                ),
                (grid.col(col, x), grid.row(row, y)),
                (64, 64),
            );
        }
    }
    Some(moves)
}

fn format_args_sum(file: &str, no: i32) -> usize {
    (file.encode_utf16().map(usize::from).sum::<usize>()
        + usize::from(b'/')
        + alloc::format!("{no}")
            .bytes()
            .map(usize::from)
            .sum::<usize>())
        % 4
        + 1
}
