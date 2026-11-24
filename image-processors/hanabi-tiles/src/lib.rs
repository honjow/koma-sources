#![no_std]
extern crate alloc;

use aes_gcm::{aead::AeadInPlace, Aes256Gcm, KeyInit, Nonce, Tag};
use alloc::{vec, vec::Vec};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// This ticket belongs to the flower site's public reader protocol.
const TICKET_KEY: [u8; 32] = [
    0xbb, 0xbd, 0x03, 0x65, 0xd8, 0x1d, 0x8d, 0xaf, 0xda, 0x24, 0xf7, 0xf8, 0xd0, 0xc1, 0x69, 0x74,
    0xea, 0xad, 0x26, 0xfa, 0xf3, 0x61, 0x01, 0x52, 0xaf, 0x92, 0xf4, 0x57, 0x1f, 0x6d, 0xcf, 0x45,
];
const MAX_TILE_COUNT: usize = 16_384;
const MAX_TICKET_SIZE: usize = 4096;
const BYTES_PER_PIXEL: usize = 4;

#[link(wasm_import_module = "koma_image")]
extern "C" {
    fn width() -> i32;
    fn height() -> i32;
    fn config_len() -> i32;
    fn config_read(out: *mut u8, capacity: i32) -> i32;
    fn check_cancel() -> i32;
    fn read_region(x: i32, y: i32, w: i32, h: i32, out: *mut u8, capacity: i32) -> i32;
    fn output_create(w: i32, h: i32) -> i32;
    fn output_write(x: i32, y: i32, w: i32, h: i32, input: *const u8, length: i32) -> i32;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

#[no_mangle]
pub extern "C" fn koma_image_processor_abi() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn koma_image_process() -> i32 {
    if restore().is_some() {
        1
    } else {
        -1
    }
}

struct Grid {
    width: usize,
    height: usize,
    cols: usize,
    rows: usize,
}

struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Grid {
    fn tile(&self, index: usize) -> Rect {
        let col = index % self.cols;
        let row = index / self.cols;
        let x = col * self.width / self.cols;
        let y = row * self.height / self.rows;
        Rect {
            x,
            y,
            w: (col + 1) * self.width / self.cols - x,
            h: (row + 1) * self.height / self.rows - y,
        }
    }
}

fn decrypt_ticket(ticket: &str, nonce: &str) -> Option<[u8; 32]> {
    let mut plain = [0u8; MAX_TICKET_SIZE];
    let len = STANDARD.decode_slice(ticket, &mut plain).ok()?;
    let mut iv = [0u8; 12];
    if len <= 16 || STANDARD.decode_slice(nonce, &mut iv).ok()? != iv.len() {
        return None;
    }
    let (plain, tag) = plain[..len].split_at_mut(len - 16);
    Aes256Gcm::new_from_slice(&TICKET_KEY)
        .ok()?
        .decrypt_in_place_detached(Nonce::from_slice(&iv), b"", plain, Tag::from_slice(tag))
        .ok()?;
    if plain.is_empty() || !plain.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(Sha256::digest(plain).into())
}

fn permutation(seed: [u8; 32], count: usize) -> Vec<usize> {
    let mut rng = ChaCha8Rng::from_seed(seed);
    let mut source_to_destination = (0..count).collect::<Vec<_>>();
    for len in (2..=count).rev() {
        let other = rng.next_u32() as usize % len;
        source_to_destination.swap(len - 1, other);
    }
    let mut destination_to_source = vec![0; count];
    for (source, destination) in source_to_destination.into_iter().enumerate() {
        destination_to_source[destination] = source;
    }
    destination_to_source
}

fn restore() -> Option<()> {
    let len = usize::try_from(unsafe { config_len() }).ok()?;
    if len == 0 || len > 6144 {
        return None;
    }
    let mut config = vec![0u8; len];
    if unsafe { config_read(config.as_mut_ptr(), len as i32) } != len as i32 {
        return None;
    }
    let mut fields = core::str::from_utf8(&config).ok()?.split('|');
    let seed = decrypt_ticket(fields.next()?, fields.next()?)?;
    let grid = Grid {
        width: usize::try_from(unsafe { width() }).ok()?,
        height: usize::try_from(unsafe { height() }).ok()?,
        cols: fields.next()?.parse().ok()?,
        rows: fields.next()?.parse().ok()?,
    };
    let count = grid.cols.checked_mul(grid.rows)?;
    if fields.next().is_some()
        || count == 0
        || count > MAX_TILE_COUNT
        || grid.cols > grid.width
        || grid.rows > grid.height
    {
        return None;
    }
    render(&grid, &permutation(seed, count))
}

fn render(grid: &Grid, source_at: &[usize]) -> Option<()> {
    let row_len = grid.width.checked_mul(BYTES_PER_PIXEL)?;
    if row_len == 0 || row_len > 1024 * 1024 {
        return None;
    }
    let strip_height = (256 * 1024 / row_len).clamp(1, 64);
    let mut output = vec![0; strip_height * row_len];
    let mut input = vec![0; strip_height * row_len];
    if unsafe { output_create(grid.width as i32, grid.height as i32) } != 0 {
        return None;
    }
    for row in 0..grid.rows {
        let band = grid.tile(row * grid.cols);
        for start in (0..band.h).step_by(strip_height) {
            if unsafe { check_cancel() } != 0 {
                return None;
            }
            let h = strip_height.min(band.h - start);
            let output_len = h * row_len;
            output[..output_len].fill(0);
            for col in 0..grid.cols {
                let destination = row * grid.cols + col;
                let dest = grid.tile(destination);
                let src = grid.tile(source_at[destination]);
                let copy_w = dest.w.min(src.w);
                let copy_h = h.min(dest.h.min(src.h).saturating_sub(start));
                if copy_h == 0 || copy_w == 0 {
                    continue;
                }
                let copy_row_len = copy_w * BYTES_PER_PIXEL;
                let read_len = copy_row_len * copy_h;
                let read = unsafe {
                    read_region(
                        src.x as i32,
                        (src.y + start) as i32,
                        copy_w as i32,
                        copy_h as i32,
                        input.as_mut_ptr(),
                        read_len as i32,
                    )
                };
                if read != read_len as i32 {
                    return None;
                }
                for (line_index, line) in input[..read_len].chunks_exact(copy_row_len).enumerate() {
                    let offset = line_index * row_len + dest.x * BYTES_PER_PIXEL;
                    output[offset..offset + copy_row_len].copy_from_slice(line);
                }
            }
            let written = unsafe {
                output_write(
                    0,
                    (band.y + start) as i32,
                    grid.width as i32,
                    h as i32,
                    output.as_ptr(),
                    output_len as i32,
                )
            };
            if written != output_len as i32 {
                return None;
            }
        }
    }
    Some(())
}
