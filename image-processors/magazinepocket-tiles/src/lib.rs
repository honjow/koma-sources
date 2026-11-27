#![no_std]

const MAX_ROW_BYTES: usize = 128 * 1024;
const PIXEL_BYTES: usize = 4;

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

fn order() -> Option<[i32; 16]> {
    let mut data = [0; 16];
    if unsafe { config_len() } != 16 || unsafe { config_read(data.as_mut_ptr(), 16) } != 16 {
        return None;
    }
    let mut order = [0; 16];
    let mut seen = 0u16;
    for (tile, hex) in order.iter_mut().zip(data) {
        *tile = match hex {
            b'0'..=b'9' => i32::from(hex - b'0'),
            b'a'..=b'f' => i32::from(hex - b'a' + 10),
            _ => return None,
        };
        let bit = 1 << *tile;
        if seen & bit != 0 {
            return None;
        }
        seen |= bit;
    }
    Some(order)
}

fn read(x: i32, y: i32, w: i32, output: &mut [u8]) -> Option<()> {
    let bytes = output.len() as i32;
    (unsafe { read_region(x, y, w, 1, output.as_mut_ptr(), bytes) } == bytes).then_some(())
}

fn restore() -> Option<()> {
    let order = order()?;
    let (w, h) = unsafe { (width(), height()) };
    let stride = usize::try_from(w).ok()?.checked_mul(PIXEL_BYTES)?;
    if h <= 0 || stride == 0 || stride > MAX_ROW_BYTES || unsafe { output_create(w, h) } != 0 {
        return None;
    }
    let (block_w, block_h) = (w / 32 * 8, h / 32 * 8);
    let block_bytes = block_w as usize * PIXEL_BYTES;
    let mut row = [0; MAX_ROW_BYTES];
    let output = &mut row[..stride];
    for y in 0..h {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        if block_w == 0 || block_h == 0 || y >= block_h * 4 {
            read(0, y, w, output)?;
        } else {
            // The reference permutes the 4×4 grid of blocks aligned to
            // eight pixels. The incomplete right and bottom edges stay put.
            let dest = (y / block_h * 4) as usize;
            for (column, tile) in order[dest..dest + 4].iter().enumerate() {
                let start = column * block_bytes;
                let src = &mut output[start..start + block_bytes];
                read(
                    tile % 4 * block_w,
                    tile / 4 * block_h + y % block_h,
                    block_w,
                    src,
                )?;
            }
            if w > block_w * 4 {
                read(
                    block_w * 4,
                    y,
                    w - block_w * 4,
                    &mut output[block_bytes * 4..],
                )?;
            }
        }
        if unsafe { output_write(0, y, w, 1, output.as_ptr(), stride as i32) } != stride as i32 {
            return None;
        }
    }
    Some(())
}
