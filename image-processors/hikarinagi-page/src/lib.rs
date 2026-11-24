#![no_std]

use aes::{
    cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit},
    Aes256,
};
use ghash::{universal_hash::UniversalHash, GHash};
use subtle::ConstantTimeEq;

const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;
const CHUNK_SIZE: usize = 32 * 1024;

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

fn read(offset: usize, buffer: &mut [u8]) -> Option<()> {
    let len = i32::try_from(buffer.len()).ok()?;
    (unsafe { bytes_read(i32::try_from(offset).ok()?, buffer.as_mut_ptr(), len) } == len)
        .then_some(())
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn restore() -> Option<()> {
    let mut config = [0u8; 1024];
    let len = usize::try_from(unsafe { config_len() }).ok()?;
    if !(66..=config.len()).contains(&len)
        || unsafe { config_read(config.as_mut_ptr(), len as i32) } != len as i32
        || config[64] != b'|'
    {
        return None;
    }
    let aad = &config[65..len];
    let mut key = [0u8; 32];
    for (value, digits) in key.iter_mut().zip(config[..64].chunks_exact(2)) {
        *value = hex(digits[0])? << 4 | hex(digits[1])?;
    }
    let cipher = Aes256::new_from_slice(&key).ok()?;
    let total = usize::try_from(unsafe { bytes_len() }).ok()?;
    let ciphertext_len = total.checked_sub(NONCE_SIZE + TAG_SIZE)?;
    if ciphertext_len == 0 {
        return None;
    }

    let mut counter = [0u8; 16];
    read(0, &mut counter[..NONCE_SIZE])?;
    counter[15] = 1; // GCM initial counter J0 for a 96-bit IV.
    let mut h = GenericArray::default();
    cipher.encrypt_block(&mut h);
    let mut auth = GHash::new(&h);
    auth.update_padded(aad);

    // Verify the original GCM tag before opening the host output. Two passes
    // keep memory bounded even for a full-size source response.
    let mut chunk = [0u8; CHUNK_SIZE];
    for offset in (0..ciphertext_len).step_by(chunk.len()) {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let size = (ciphertext_len - offset).min(chunk.len());
        read(NONCE_SIZE + offset, &mut chunk[..size])?;
        auth.update_padded(&chunk[..size]);
    }
    let mut bits = [0u8; 16];
    bits[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    bits[8..].copy_from_slice(&((ciphertext_len as u64) * 8).to_be_bytes());
    auth.update(&[bits.into()]);
    let mut tag = auth.finalize();
    let mut mask = counter.into();
    cipher.encrypt_block(&mut mask);
    for (value, mask) in tag.iter_mut().zip(mask) {
        *value ^= mask;
    }
    let mut expected = [0u8; TAG_SIZE];
    read(total - TAG_SIZE, &mut expected)?;
    if !bool::from(tag.as_slice().ct_eq(&expected)) {
        return None;
    }
    if unsafe { bytes_output_create(ciphertext_len as i32) } != 0 {
        return None;
    }

    let mut counter_value = 1u32;
    for offset in (0..ciphertext_len).step_by(chunk.len()) {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let size = (ciphertext_len - offset).min(chunk.len());
        read(NONCE_SIZE + offset, &mut chunk[..size])?;
        for block in chunk[..size].chunks_mut(16) {
            counter_value = counter_value.checked_add(1)?;
            counter[NONCE_SIZE..].copy_from_slice(&counter_value.to_be_bytes());
            let mut mask = counter.into();
            cipher.encrypt_block(&mut mask);
            for (value, mask) in block.iter_mut().zip(mask) {
                *value ^= mask;
            }
        }
        if unsafe { bytes_output_write(chunk.as_ptr(), size as i32) } != size as i32 {
            return None;
        }
    }
    Some(())
}
