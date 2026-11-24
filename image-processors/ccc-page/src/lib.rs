#![no_std]

extern crate alloc;

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256};
use base64::{engine::general_purpose::STANDARD, Engine};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const BLOCK: usize = 16;
const CHUNK: usize = 32 * 1024;

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
    let size = unsafe { config_len() };
    if !matches!(size, 65 | 81 | 97) {
        return None;
    }
    let mut config = [0; 97];
    if unsafe { config_read(config.as_mut_ptr(), size) } != size {
        return None;
    }
    let key_len = size as usize - 33;
    if config[key_len] != b':' {
        return None;
    }
    let mut key = [0; 32];
    let mut iv = [0; BLOCK];
    decode_hex(&config[..key_len], &mut key[..key_len / 2])?;
    decode_hex(&config[key_len + 1..size as usize], &mut iv)?;
    match key_len {
        32 => decrypt(&Aes128::new_from_slice(&key[..16]).ok()?, iv),
        48 => decrypt(&Aes192::new_from_slice(&key[..24]).ok()?, iv),
        64 => decrypt(&Aes256::new_from_slice(&key).ok()?, iv),
        _ => None,
    }
}

fn decode_hex(input: &[u8], output: &mut [u8]) -> Option<()> {
    for (pair, byte) in input.chunks_exact(2).zip(output) {
        *byte = (pair[0] as char).to_digit(16)? as u8 * 16 + (pair[1] as char).to_digit(16)? as u8;
    }
    Some(())
}

fn read(offset: usize, out: &mut [u8]) -> Option<()> {
    let size = out.len() as i32;
    (unsafe { bytes_read(offset as i32, out.as_mut_ptr(), size) } == size).then_some(())
}

fn blocks<C>(cipher: &C, chain: &mut [u8; BLOCK], bytes: &mut [u8])
where
    C: BlockDecrypt + aes::cipher::BlockSizeUser<BlockSize = aes::cipher::consts::U16>,
{
    for block in bytes.chunks_exact_mut(BLOCK) {
        let mut next = [0; BLOCK];
        next.copy_from_slice(block);
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
        for (byte, previous) in block.iter_mut().zip(*chain) {
            *byte ^= previous;
        }
        *chain = next;
    }
}

fn decrypt<C>(cipher: &C, iv: [u8; BLOCK]) -> Option<()>
where
    C: BlockDecrypt + aes::cipher::BlockSizeUser<BlockSize = aes::cipher::consts::U16>,
{
    let total = usize::try_from(unsafe { bytes_len() }).ok()?;
    if total < 2 * BLOCK || total % BLOCK != 0 {
        return None;
    }
    let head_len = total.min(64);
    let mut head = [0; 64];
    read(0, &mut head[..head_len])?;
    let mut chain = iv;
    blocks(cipher, &mut chain, &mut head[..head_len]);
    let header_end = head[..head_len].iter().position(|&byte| byte == b',')?;
    let media = head[..header_end]
        .strip_prefix(b"data:image/")?
        .strip_suffix(b";base64")?;
    if media.is_empty()
        || !media
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-'))
    {
        return None;
    }
    let payload_start = header_end + 1;

    let tail_offset = total - 2 * BLOCK;
    let mut tail = [0; 2 * BLOCK];
    chain = iv;
    if tail_offset != 0 {
        read(tail_offset - BLOCK, &mut chain)?;
    }
    read(tail_offset, &mut tail)?;
    blocks(cipher, &mut chain, &mut tail);
    let pad = tail[tail.len() - 1] as usize;
    if !(1..=BLOCK).contains(&pad)
        || !tail[tail.len() - pad..]
            .iter()
            .all(|&byte| byte as usize == pad)
    {
        return None;
    }
    let plain_end = total - pad;
    let payload_len = plain_end.checked_sub(payload_start)?;
    if payload_len == 0 || payload_len % 4 != 0 {
        return None;
    }
    let suffix = &tail[..tail.len() - pad];
    let base64_pad = if suffix.ends_with(b"==") {
        2
    } else if suffix.ends_with(b"=") {
        1
    } else {
        0
    };
    let output_len = payload_len / 4 * 3 - base64_pad;
    if unsafe { bytes_output_create(output_len as i32) } != 0 {
        return None;
    }

    let mut encoded = [0; CHUNK + 3];
    let mut decoded = [0; 3 * CHUNK / 4];
    let mut carry = 0;
    let mut written = 0;
    chain = iv;
    for offset in (0..total).step_by(CHUNK) {
        if unsafe { check_cancel() } != 0 {
            return None;
        }
        let size = (total - offset).min(CHUNK);
        let chunk = &mut encoded[carry..carry + size];
        read(offset, chunk)?;
        blocks(cipher, &mut chain, chunk);
        let start = payload_start.saturating_sub(offset);
        let end = carry + (plain_end - offset).min(size);
        let full_end = end - (end - start) % 4;
        if full_end > start {
            let length = STANDARD
                .decode_slice(&encoded[start..full_end], &mut decoded)
                .ok()?;
            if written + length > output_len
                || unsafe { bytes_output_write(decoded.as_ptr(), length as i32) } != length as i32
            {
                return None;
            }
            written += length;
        }
        carry = end - full_end;
        encoded.copy_within(full_end..end, 0);
    }
    (carry == 0 && written == output_len).then_some(())
}
