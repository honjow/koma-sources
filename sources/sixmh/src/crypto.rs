use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
// Public template metadata key from the fixed-primary Crypto.kt. Not user credentials.
const METADATA_KEY: &[u8; 16] = b"9S8$vJnU2ANeSRoF";
pub fn decode_metadata(data: &mut [u8]) -> Result<&[u8], &'static str> {
    if data.len() < 32 || data.len() > 512 * 1024 || (data.len() - 16) % 16 != 0 {
        return Err("invalid metadata cipher bounds");
    }
    let (iv, ciphertext) = data.split_at_mut(16);
    decrypt(METADATA_KEY, iv, ciphertext)
}
pub fn decrypt<'a>(key: &[u8], iv: &[u8], data: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    if key.len() != 16 || iv.len() != 16 || data.is_empty() || data.len() % 16 != 0 {
        return Err("invalid AES-CBC bounds");
    }
    let cipher = cbc::Decryptor::<aes::Aes128>::new_from_slices(key, iv)
        .map_err(|_| "invalid metadata key")?;
    cipher
        .decrypt_padded_mut::<Pkcs7>(data)
        .map_err(|_| "invalid metadata padding")
}
