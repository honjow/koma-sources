// PUBLUS configuration and page filename formats, adapted from Keiyoushi
// extensions-source lib/publus/Publus.kt at 8947c372cd87f546a0b80cb0e4b0a5a8add5ef88 (Apache-2.0).
use alloc::{format, string::String, vec::Vec};
use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    Engine,
};
use serde_json::Value;

pub(super) type PageKeys = [[u8; 32]; 3];
const KEY_BYTES: usize = 96;
const HEADER_LENGTH: usize = 128;
const CONFIG_NAME: &[u8] = b"configuration_pack.json";

pub(super) fn unpack(value: Value) -> Option<(Value, Option<PageKeys>)> {
    match value.get("data").and_then(Value::as_str) {
        Some(data) => {
            let (plain, keys) = decode(data)?;
            Some((serde_json::from_slice(&plain).ok()?, Some(keys)))
        }
        None => Some((value, None)),
    }
}

fn decode(data: &str) -> Option<(Vec<u8>, PageKeys)> {
    let bytes = data.as_bytes();
    let header = bytes.get(..HEADER_LENGTH)?;
    let engine = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
    let mut key_bytes = [0; KEY_BYTES];
    if engine.decode_slice(header, &mut key_bytes).ok()? != KEY_BYTES {
        return None;
    }
    let mut keys = [[0; 32]; 3];
    for (key, chunk) in keys.iter_mut().zip(key_bytes.chunks_exact(32)) {
        key.copy_from_slice(chunk);
    }
    let mut body = bytes.get(HEADER_LENGTH..)?.to_vec();
    body.retain(|byte| !byte.is_ascii_whitespace());
    let mut payload = engine.decode(body).ok()?;
    if payload.is_empty() {
        return None;
    }

    mutate(&mut payload, &keys);
    let mask = Rc4::new(&keys[1], CONFIG_NAME, &keys[2]);
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask.boxes[index % 256];
    }
    let odd = payload.len() % 2;
    Rc4::new(CONFIG_NAME, &keys[0], &keys[1]).apply(payload.iter_mut().rev().skip(odd).step_by(2));
    Rc4::new(&keys[2], CONFIG_NAME, &keys[0])
        .apply(payload.iter_mut().rev().skip(1 - odd).step_by(2));
    mix(&mut payload, &mut keys);
    Rc4::new(&keys[1], &keys[0], CONFIG_NAME).apply(&mut keys[2]);
    Rc4::new(&keys[0], CONFIG_NAME, &keys[2]).apply(&mut keys[1]);
    Rc4::new(CONFIG_NAME, &keys[2], &keys[1]).apply(&mut keys[0]);

    for index in (0..3).rev() {
        let other_keys = [keys[(index + 1) % 3], keys[(index + 2) % 3]];
        mutate(&mut keys[index], &other_keys);
    }
    Rc4::new(&keys[2], &keys[1], CONFIG_NAME).apply(&mut payload);
    Some((payload, keys))
}

fn mutate(data: &mut [u8], keys: &[[u8; 32]]) {
    let mut sum = 0u8;
    let mut xor = 0u8;
    for &byte in keys.iter().flatten() {
        sum = sum.wrapping_add(byte);
        xor ^= byte;
    }
    let shift = u32::from(xor >> 5);
    let mut buffer = [0u8; 32];
    for block in data.chunks_mut(32) {
        let mut block_sum = sum;
        let mut block_xor = xor;
        for (out, &input) in buffer.iter_mut().zip(block.iter()) {
            let mut value = input;
            if sum & 2 == 0 {
                value = ((value & 0x55) << 1) | ((value >> 1) & 0x55);
            }
            if sum & 4 == 0 {
                value = ((value & 0x33) << 2) | ((value >> 2) & 0x33);
            }
            if sum & 8 == 0 {
                value = value.rotate_left(4);
            }
            *out = value;
            block_sum = block_sum.wrapping_add(value);
            block_xor ^= value;
        }

        let len = block.len();
        for end in 1..=len {
            let mut group = 2;
            while end % group == 0 {
                if usize::from(block_sum) & group == 0 {
                    let half = group / 2;
                    for offset in end - group..end - half {
                        buffer.swap(offset, offset + half);
                    }
                }
                group *= 2;
            }
        }

        let rotate = usize::from(block_xor >> 3) % len;
        for (index, value) in block.iter_mut().enumerate() {
            let from = (index + len - rotate) % len;
            *value = buffer[from] >> shift;
            if shift != 0 {
                *value |= buffer[(from + len - 1) % len] << (8 - shift);
            }
        }
    }
}

fn mix(payload: &mut [u8], keys: &mut PageKeys) {
    for (index, byte) in payload.iter_mut().take(32).enumerate() {
        let mut values = [keys[0][index], keys[1][index], keys[2][index], *byte];
        let xor = values.iter().fold(0, |sum, item| sum ^ item);
        values.swap(usize::from(xor & 3), usize::from((xor >> 2) & 3));
        values.swap(usize::from((xor >> 4) & 3), usize::from(xor >> 6));
        [keys[0][index], keys[1][index], keys[2][index], *byte] = values;
    }
}

struct Rc4 {
    boxes: [u8; 256],
    row: u8,
    col: u8,
}

impl Rc4 {
    fn new(first: &[u8], second: &[u8], third: &[u8]) -> Self {
        let mut boxes = core::array::from_fn(|index| index as u8);
        let key = [first, second, third].concat();
        let mut partner = 0u8;
        for index in 0..boxes.len() {
            partner = partner
                .wrapping_add(boxes[index])
                .wrapping_add(key[index % key.len()]);
            boxes.swap(index, usize::from(partner));
        }
        Self {
            boxes,
            row: 0,
            col: 0,
        }
    }

    fn apply<'a>(&mut self, bytes: impl IntoIterator<Item = &'a mut u8>) {
        for byte in bytes {
            self.row = self.row.wrapping_add(1);
            self.col = self.col.wrapping_add(self.boxes[usize::from(self.row)]);
            self.boxes
                .swap(usize::from(self.row), usize::from(self.col));
            let index =
                self.boxes[usize::from(self.row)].wrapping_add(self.boxes[usize::from(self.col)]);
            *byte ^= self.boxes[usize::from(index)];
        }
    }
}

pub(super) fn filename(file: &str, no: i32, keys: Option<&PageKeys>) -> String {
    let number = format!("{no}");
    let Some(keys) = keys else {
        return format!("{file}/{number}.jpeg");
    };
    let mask: [u8; 32] =
        core::array::from_fn(|index| keys[0][index] ^ keys[1][index] ^ keys[2][index]);
    let folder = format!("{file}/");
    let mut path = Vec::from([0, 59]);
    for character in folder.encode_utf16().chain(number.encode_utf16()) {
        path.extend_from_slice(&character.to_be_bytes());
    }
    let rounds = 3 + 256usize
        .saturating_sub(number.len() * 2 + 2 * path.len())
        .div_ceil(path.len());
    // The reference stores this 64-bit hash in limbs of 22, 21 and 21 bits.
    let mut hash = (1_670_739u64 << 43) | (1_282_576u64 << 22) | 2_237_221;
    let mut key = mask.iter().copied().cycle();
    let page_offset = (1 + folder.encode_utf16().count()) * 2;
    for byte in path[page_offset..]
        .iter()
        .chain(path.iter().cycle().take((rounds - 1) * path.len()))
    {
        hash ^= u64::from(*byte ^ key.next().unwrap_or_default());
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let digest: [u8; 8] = core::array::from_fn(|index| hash.to_be_bytes()[index] ^ mask[index]);
    let mut file = format!("{folder}10");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        file.push(char::from(HEX[usize::from(byte >> 4)]));
        file.push(char::from(HEX[usize::from(byte & 15)]));
    }
    file.push_str(".jpeg");
    file
}
