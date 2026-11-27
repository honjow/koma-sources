#![no_std]

const BUFFER_BYTES: usize = 16 * 1024;
const MAX_HEX_BYTES: usize = 64;

#[link(wasm_import_module = "koma_image")]
extern "C" {
    fn config_len() -> i32;
    fn config_read(out: *mut u8, capacity: i32) -> i32;
    fn bytes_len() -> i32;
    fn bytes_read(offset: i32, out: *mut u8, length: i32) -> i32;
    fn bytes_output_create(length: i32) -> i32;
    fn bytes_output_write(input: *const u8, length: i32) -> i32;
    fn check_cancel() -> i32;
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
    if !(2..=MAX_HEX_BYTES).contains(&len) || len % 2 != 0 {
        return None;
    }
    let mut config = [0; MAX_HEX_BYTES];
    if unsafe { config_read(config.as_mut_ptr(), len as i32) } != len as i32 {
        return None;
    }
    let mut key = [0; MAX_HEX_BYTES / 2];
    for (byte, pair) in key.iter_mut().zip(config[..len].chunks_exact(2)) {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        *byte = (high * 16 + low) as u8;
    }
    let key = &key[..len / 2];
    let total = unsafe { bytes_len() };
    if total <= 0 || unsafe { bytes_output_create(total) } != 0 {
        return None;
    }

    // The provider repeats the key over the entire encoded file, including across host reads.
    let mut buffer = [0; BUFFER_BYTES];
    let mut offset = 0;
    while offset < total {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let length = (total - offset).min(BUFFER_BYTES as i32);
        if unsafe { bytes_read(offset, buffer.as_mut_ptr(), length) } != length {
            return None;
        }
        let mask = key.iter().cycle().skip(offset as usize % key.len());
        for (byte, mask) in buffer[..length as usize].iter_mut().zip(mask) {
            *byte ^= mask;
        }
        if unsafe { bytes_output_write(buffer.as_ptr(), length) } != length {
            return None;
        }
        offset += length;
    }
    Some(())
}
