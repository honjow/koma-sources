//! Stream the provider's authenticated AES-GCM response through scoped bodies.
//! Verify the tag before creating an output: failed responses never reach cache.
use aes::{cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit}, Aes256};
use ghash::{universal_hash::UniversalHash, GHash};
use koma_source_sdk::{body::BodyRef, host};
use subtle::ConstantTimeEq;
use crate::port::Result;

const NONCE: usize = 12;
const TAG: usize = 16;
const CHUNK: usize = 32 * 1024;

fn read_exact(input: &BodyRef, offset: usize, out: &mut [u8]) -> Result<()> {
    if input.read(offset, out, crate::http_out())? != out.len() { return Err("invalid_response"); }
    Ok(())
}

pub(crate) fn decrypt(input: &BodyRef, key: &[u8; 32], aad: &[u8]) -> Result<BodyRef> {
    let size = input.len().checked_sub(NONCE + TAG).filter(|v| *v > 0).ok_or("parse_error")?;
    let cipher = Aes256::new_from_slice(key).map_err(|_| "internal_error")?;
    let mut counter = [0u8; 16];
    read_exact(input, 0, &mut counter[..NONCE])?;
    counter[15] = 1;
    let mut h = GenericArray::default();
    cipher.encrypt_block(&mut h);
    let mut auth = GHash::new(&h);
    auth.update_padded(aad);
    let mut chunk = [0u8; CHUNK];
    for offset in (0..size).step_by(CHUNK) {
        if host::check_cancel() { return Err("cancelled"); }
        let n = (size - offset).min(CHUNK);
        read_exact(input, NONCE + offset, &mut chunk[..n])?;
        auth.update_padded(&chunk[..n]);
    }
    let mut bits = [0u8; 16];
    bits[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    bits[8..].copy_from_slice(&((size as u64) * 8).to_be_bytes());
    auth.update(&[bits.into()]);
    let mut tag = auth.finalize();
    let mut mask = counter.into();
    cipher.encrypt_block(&mut mask);
    for (value, byte) in tag.iter_mut().zip(mask) { *value ^= byte; }
    let mut expected = [0u8; TAG];
    read_exact(input, input.len() - TAG, &mut expected)?;
    if !bool::from(tag.as_slice().ct_eq(&expected)) { return Err("image_decode_failed"); }
    let mut output = BodyRef::create()?;
    let result = (|| -> Result<()> {
        let mut counter_value = 1u32;
        for offset in (0..size).step_by(CHUNK) {
            if host::check_cancel() { return Err("cancelled"); }
            let n = (size - offset).min(CHUNK);
            read_exact(input, NONCE + offset, &mut chunk[..n])?;
            for block in chunk[..n].chunks_mut(16) {
                counter_value = counter_value.checked_add(1).ok_or("image_decode_failed")?;
                counter[NONCE..].copy_from_slice(&counter_value.to_be_bytes());
                let mut mask = counter.into();
                cipher.encrypt_block(&mut mask);
                for (value, byte) in block.iter_mut().zip(mask) { *value ^= byte; }
            }
            output.write(offset, &chunk[..n], crate::http_out())?;
        }
        Ok(())
    })();
    if let Err(error) = result { let _ = output.close(); return Err(error); }
    Ok(output)
}
