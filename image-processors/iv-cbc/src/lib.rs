#![no_std]

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::{Aes128, Aes256};

const BLOCK_SIZE: usize = 16;
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

fn restore() -> Option<()> {
    let encoded_len = usize::try_from(unsafe { config_len() }).ok()?;
    if !matches!(encoded_len, 32 | 64) {
        return None;
    }
    let mut encoded = [0u8; 64];
    if unsafe { config_read(encoded.as_mut_ptr(), encoded_len as i32) } != encoded_len as i32 {
        return None;
    }
    let mut key = [0u8; 32];
    for (n, bytes) in encoded[..encoded_len].chunks_exact(2).enumerate() {
        key[n] = hex(bytes[0])? << 4 | hex(bytes[1])?;
    }
    if encoded_len == 32 {
        let cipher = Aes128::new_from_slice(&key[..16]).ok()?;
        decrypt(&cipher)
    } else {
        let cipher = Aes256::new_from_slice(&key).ok()?;
        decrypt(&cipher)
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn read(offset: usize, output: &mut [u8]) -> Option<()> {
    let size = i32::try_from(output.len()).ok()?;
    let offset = i32::try_from(offset).ok()?;
    (unsafe { bytes_read(offset, output.as_mut_ptr(), size) } == size).then_some(())
}

fn decrypt<C>(cipher: &C) -> Option<()>
where
    C: BlockDecrypt + aes::cipher::BlockSizeUser<BlockSize = aes::cipher::consts::U16>,
{
    let total = usize::try_from(unsafe { bytes_len() }).ok()?;
    if total < 2 * BLOCK_SIZE || total % BLOCK_SIZE != 0 {
        return None;
    }
    // CBC only needs the previous block to validate the last block first.
    // This determines the exact output length before opening the host writer.
    let mut tail = [0u8; 2 * BLOCK_SIZE];
    read(total - tail.len(), &mut tail)?;
    let mut last = [0u8; BLOCK_SIZE];
    last.copy_from_slice(&tail[BLOCK_SIZE..]);
    cipher.decrypt_block(GenericArray::from_mut_slice(&mut last));
    for (byte, chain) in last.iter_mut().zip(tail) {
        *byte ^= chain;
    }
    let pad = usize::from(last[BLOCK_SIZE - 1]);
    if !(1..=BLOCK_SIZE).contains(&pad)
        || !last[BLOCK_SIZE - pad..].iter().all(|&b| b as usize == pad)
    {
        return None;
    }
    let output_len = total - BLOCK_SIZE - pad;
    if output_len == 0 || unsafe { bytes_output_create(output_len as i32) } != 0 {
        return None;
    }

    let mut chain = [0u8; BLOCK_SIZE];
    read(0, &mut chain)?;
    let mut chunk = [0u8; CHUNK_SIZE];
    let mut offset = BLOCK_SIZE;
    while offset < total {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let size = (total - offset).min(chunk.len());
        read(offset, &mut chunk[..size])?;
        for block in chunk[..size].chunks_exact_mut(BLOCK_SIZE) {
            let mut next = [0u8; BLOCK_SIZE];
            next.copy_from_slice(block);
            cipher.decrypt_block(GenericArray::from_mut_slice(block));
            for (byte, previous) in block.iter_mut().zip(chain) {
                *byte ^= previous;
            }
            chain = next;
        }
        offset += size;
        let write_size = if offset == total { size - pad } else { size } as i32;
        if write_size > 0 && unsafe { bytes_output_write(chunk.as_ptr(), write_size) } != write_size
        {
            return None;
        }
    }
    Some(())
}
