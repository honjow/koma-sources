#![no_std]

const PIXEL: usize = 4;
const MAX_ROW: usize = 128 * 1024;
const BAKU: &[u8] = br#"{"mode":"baku"}"#;

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

fn read(x: i32, y: i32, w: i32, output: &mut [u8]) -> Option<()> {
    let bytes = output.len() as i32;
    (unsafe { read_region(x, y, w, 1, output.as_mut_ptr(), bytes) } == bytes).then_some(())
}

fn restore() -> Option<()> {
    let mut config = [0; BAKU.len()];
    let len = config.len() as i32;
    if unsafe { config_len() } != len
        || unsafe { config_read(config.as_mut_ptr(), len) } != len
        || config != BAKU
    {
        return None;
    }
    let (w, h) = unsafe { (width(), height()) };
    let stride = usize::try_from(w).ok()?.checked_mul(PIXEL)?;
    if h <= 0 || stride == 0 || stride > MAX_ROW || unsafe { output_create(w, h) } != 0 {
        return None;
    }
    let (tile_w, tile_h) = (w / 32 * 8, h / 32 * 8);
    let mut row = [0; MAX_ROW];
    let output = &mut row[..stride];

    for y in 0..h {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        if tile_w == 0 || tile_h == 0 || y >= 4 * tile_h {
            read(0, y, w, output)?;
        } else {
            // GigaViewer's baku transposes the four-by-four block coordinates.
            // Pixel positions within each block and the incomplete edges stay fixed.
            let dest_row = y / tile_h;
            let inner_y = y % tile_h;
            let tile_bytes = tile_w as usize * PIXEL;
            for dest_col in 0..4 {
                let start = dest_col as usize * tile_bytes;
                read(
                    dest_row * tile_w,
                    dest_col * tile_h + inner_y,
                    tile_w,
                    &mut output[start..start + tile_bytes],
                )?;
            }
            let edge = w - 4 * tile_w;
            if edge > 0 {
                read(4 * tile_w, y, edge, &mut output[4 * tile_bytes..])?;
            }
        }
        if unsafe { output_write(0, y, w, 1, output.as_ptr(), stride as i32) } != stride as i32 {
            return None;
        }
    }
    Some(())
}
