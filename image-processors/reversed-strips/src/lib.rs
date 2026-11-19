#![no_std]
extern crate alloc;

use alloc::vec;

const BAND_BYTES: usize = 64 * 1024;
const MAX_ROW_BYTES: usize = 1024 * 1024;

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
    let mut config = [0; 16];
    if !(3..=config.len()).contains(&len)
        || unsafe { config_read(config.as_mut_ptr(), len as i32) } != len as i32
    {
        return None;
    }
    let (axis, count) = core::str::from_utf8(&config[..len]).ok()?.split_once('|')?;
    let parts = count.parse::<usize>().ok()?;
    if parts == 0 {
        return None;
    }
    let w = usize::try_from(unsafe { width() }).ok()?;
    let h = usize::try_from(unsafe { height() }).ok()?;
    let stride = w.checked_mul(4)?;
    if stride == 0 || stride > MAX_ROW_BYTES || h == 0 {
        return None;
    }
    if unsafe { output_create(w as i32, h as i32) } != 0 {
        return None;
    }

    let rows_per_band = (BAND_BYTES / stride).max(1);
    let mut input = vec![0; rows_per_band * stride];
    match axis {
        "x" => {
            let mut output = vec![0; input.len()];
            let strip = w / parts;
            for y in (0..h).step_by(rows_per_band) {
                let rows = rows_per_band.min(h - y);
                let band = &mut output[..rows * stride];
                band.fill(0);
                // BoyLove copies only full-width strips in place at height >= 4000.
                // Its trailing unpainted pixels stay transparent.
                if h >= 4000 {
                    if strip > 0 {
                        copy_columns(y, rows, 0, 0, strip * parts, &mut input, band, stride)?;
                    }
                } else if strip == 0 {
                    copy_columns(y, rows, 0, 0, w, &mut input, band, stride)?;
                } else {
                    // The leftmost input strip carries the remainder.
                    for i in 0..parts - 1 {
                        copy_columns(
                            y,
                            rows,
                            w - strip * (i + 1),
                            strip * i,
                            strip,
                            &mut input,
                            band,
                            stride,
                        )?;
                    }
                    let first = w - strip * (parts - 1);
                    copy_columns(y, rows, 0, w - first, first, &mut input, band, stride)?;
                }
                write_band(y, w, rows, band)?;
            }
        }
        "y" => {
            let strip = h / parts;
            if strip == 0 {
                copy_rows(0, 0, h, w, rows_per_band, &mut input)?;
            } else {
                // Roumanwu's bottom input block carries the remainder.
                let bottom = h - strip * (parts - 1);
                copy_rows(h - bottom, 0, bottom, w, rows_per_band, &mut input)?;
                for i in 1..parts {
                    copy_rows(
                        strip * (parts - i - 1),
                        bottom + strip * (i - 1),
                        strip,
                        w,
                        rows_per_band,
                        &mut input,
                    )?;
                }
            }
        }
        _ => return None,
    }
    Some(())
}

fn read_band(x: usize, y: usize, w: usize, h: usize, bytes: &mut [u8]) -> Option<()> {
    if unsafe { check_cancel() } != 0 {
        return None;
    }
    let len = bytes.len() as i32;
    (unsafe {
        read_region(
            x as i32,
            y as i32,
            w as i32,
            h as i32,
            bytes.as_mut_ptr(),
            len,
        )
    } == len)
        .then_some(())
}

fn write_band(y: usize, w: usize, h: usize, bytes: &[u8]) -> Option<()> {
    let len = bytes.len() as i32;
    (unsafe { output_write(0, y as i32, w as i32, h as i32, bytes.as_ptr(), len) } == len)
        .then_some(())
}

fn copy_rows(
    source: usize,
    dest: usize,
    rows: usize,
    w: usize,
    step: usize,
    scratch: &mut [u8],
) -> Option<()> {
    for start in (0..rows).step_by(step) {
        let count = step.min(rows - start);
        let band = &mut scratch[..w * count * 4];
        read_band(0, source + start, w, count, band)?;
        write_band(dest + start, w, count, band)?;
    }
    Some(())
}

fn copy_columns(
    y: usize,
    rows: usize,
    source: usize,
    dest: usize,
    width: usize,
    scratch: &mut [u8],
    output: &mut [u8],
    stride: usize,
) -> Option<()> {
    let row_bytes = width * 4;
    let band = &mut scratch[..rows * row_bytes];
    read_band(source, y, width, rows, band)?;
    for (output_row, input_row) in output
        .chunks_exact_mut(stride)
        .zip(band.chunks_exact(row_bytes))
    {
        output_row[dest * 4..dest * 4 + row_bytes].copy_from_slice(input_row);
    }
    Some(())
}
