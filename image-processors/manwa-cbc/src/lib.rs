#![no_std]

use aes::{
    cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit},
    Aes128,
};

// This is the fixed public reader key and IV used by manwa.me's image format.
const READER_KEY: [u8; 16] = *b"my2ecret782ecret";
const BLOCK_SIZE: usize = READER_KEY.len();
const CHUNK_SIZE: usize = 32 * 1024;

#[link(wasm_import_module = "koma_image")]
extern "C" {
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
    let length = usize::try_from(unsafe { bytes_len() }).ok()?;
    if length == 0 || length % BLOCK_SIZE != 0 {
        return None;
    }
    if unsafe { check_cancel() } != 0 || unsafe { bytes_output_create(length as i32) } != 0 {
        return None;
    }
    let cipher = Aes128::new_from_slice(&READER_KEY).ok()?;
    let mut previous = READER_KEY;
    let mut chunk = [0; CHUNK_SIZE];
    for offset in (0..length).step_by(CHUNK_SIZE) {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let size = (length - offset).min(chunk.len());
        let size_i32 = size as i32;
        if unsafe { bytes_read(offset as i32, chunk.as_mut_ptr(), size_i32) } != size_i32 {
            return None;
        }
        for block in chunk[..size].chunks_exact_mut(BLOCK_SIZE) {
            let mut current = [0; BLOCK_SIZE];
            current.copy_from_slice(block);
            cipher.decrypt_block(GenericArray::from_mut_slice(block));
            for (byte, chain) in block.iter_mut().zip(previous) {
                *byte ^= chain;
            }
            previous = current;
        }
        // The upstream uses NoPadding and returns every plaintext byte.
        if unsafe { bytes_output_write(chunk.as_ptr(), size_i32) } != size_i32 {
            return None;
        }
    }
    Some(())
}
