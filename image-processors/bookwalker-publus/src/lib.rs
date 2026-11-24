#![no_std]
extern crate alloc;

use alloc::{string::String, vec};
use serde::Deserialize;

mod keyed;
mod moves;

type PageKeys = [[u8; 32]; 3];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    file: String,
    no: i32,
    width: usize,
    height: usize,
    block_width: usize,
    block_height: usize,
    ns: i64,
    ps: i64,
    rs: i64,
    keys: Option<PageKeys>,
}

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

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

fn restore() -> Option<()> {
    let len = usize::try_from(unsafe { config_len() }).ok()?;
    if len == 0 || len > 64 * 1024 {
        return None;
    }
    let mut config = vec![0u8; len];
    if unsafe { config_read(config.as_mut_ptr(), len as i32) } != len as i32 {
        return None;
    }
    let page: Page = serde_json::from_slice(&config).ok()?;
    let w = usize::try_from(unsafe { width() }).ok()?;
    let h = usize::try_from(unsafe { height() }).ok()?;
    if w == 0 || h == 0 || page.file.is_empty() || page.file.len() > 4096 || page.no < 0 {
        return None;
    }
    let mut moves = match &page.keys {
        Some(keys) if page.block_width != 0 => {
            if page.block_height == 0
                || page.block_width > i32::MAX as usize
                || page.block_height > i32::MAX as usize
            {
                return None;
            }
            keyed::with_keys(w, h, &page, keys)?
        }
        _ => moves::without_keys(w, h, &page)?,
    };

    let (out_w, out_h) = if page.width > 0 && page.height > 0 {
        (page.width, page.height)
    } else {
        (w, h)
    };
    if out_w > w || out_h > h || moves.is_empty() {
        return None;
    }
    for tile in &moves {
        if tile.read_x.checked_add(tile.width)? > w
            || tile.read_y.checked_add(tile.height)? > h
            || tile.write_x.checked_add(tile.width)? > w
            || tile.write_y.checked_add(tile.height)? > h
        {
            return None;
        }
    }
    moves.sort_by_key(|tile| tile.write_y);
    render(out_w, out_h, &moves)
}

fn render(width: usize, height: usize, moves: &[moves::Move]) -> Option<()> {
    const PIXEL: usize = 4;
    let row_bytes = width.checked_mul(PIXEL)?;
    if row_bytes == 0 || row_bytes > 1024 * 1024 {
        return None;
    }
    let strip_height = (256 * 1024 / row_bytes).clamp(1, 64);
    let max_tile_height = moves.iter().map(|tile| tile.height).max()?;
    let mut input = vec![0u8; row_bytes * strip_height];
    let mut output = vec![0u8; row_bytes * strip_height];
    if unsafe { output_create(width as i32, height as i32) } != 0 {
        return None;
    }
    for y in (0..height).step_by(strip_height) {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let rows = strip_height.min(height - y);
        let bytes = rows * row_bytes;
        output[..bytes].fill(0);
        let first = moves.partition_point(|tile| tile.write_y + max_tile_height <= y);
        for tile in moves[first..]
            .iter()
            .take_while(|tile| tile.write_y < y + rows)
        {
            if tile.write_x >= width {
                continue;
            }
            let from = y.max(tile.write_y);
            let to = (y + rows).min(tile.write_y + tile.height);
            if from >= to {
                continue;
            }
            let copy_width = tile.width.min(width - tile.write_x);
            let copy_rows = to - from;
            let tile_stride = copy_width * PIXEL;
            let count = tile_stride * copy_rows;
            let read = unsafe {
                read_region(
                    tile.read_x as i32,
                    (tile.read_y + from - tile.write_y) as i32,
                    copy_width as i32,
                    copy_rows as i32,
                    input.as_mut_ptr(),
                    count as i32,
                )
            };
            if read != count as i32 {
                return None;
            }
            for (line, data) in input[..count].chunks_exact(tile_stride).enumerate() {
                let offset = (from - y + line) * row_bytes + tile.write_x * PIXEL;
                output[offset..offset + tile_stride].copy_from_slice(data);
            }
        }
        let written = unsafe {
            output_write(
                0,
                y as i32,
                width as i32,
                rows as i32,
                output.as_ptr(),
                bytes as i32,
            )
        };
        if written != bytes as i32 {
            return None;
        }
    }
    Some(())
}
