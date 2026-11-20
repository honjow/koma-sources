//! Bounded JSON encoding shared by host SDKs; no allocator required.
use crate::json_utils::{append_json_escaped, write_bytes, write_usize};

pub(crate) struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}
impl<'a> Writer<'a> {
    pub(crate) fn new(out: &'a mut [u8]) -> Self {
        Self { out, at: 0 }
    }
    pub(crate) fn raw(&mut self, value: &[u8]) -> Result<(), &'static str> {
        if write_bytes(self.out, &mut self.at, value) {
            Ok(())
        } else {
            Err("capacity_exceeded")
        }
    }
    pub(crate) fn string(&mut self, value: &[u8]) -> Result<(), &'static str> {
        self.raw(b"\"")?;
        if !append_json_escaped(self.out, &mut self.at, value) {
            return Err("capacity_exceeded");
        }
        self.raw(b"\"")
    }
    pub(crate) fn number(&mut self, value: usize) -> Result<(), &'static str> {
        if write_usize(self.out, &mut self.at, value) {
            Ok(())
        } else {
            Err("capacity_exceeded")
        }
    }
    pub(crate) fn len(&self) -> usize {
        self.at
    }
    pub(crate) fn base64(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        self.raw(b"\"")?;
        for chunk in bytes.chunks(3) {
            let n = ((chunk[0] as u32) << 16)
                | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
                | chunk.get(2).copied().unwrap_or(0) as u32;
            self.raw(&[
                ABC[(n >> 18) as usize],
                ABC[((n >> 12) & 63) as usize],
                if chunk.len() > 1 {
                    ABC[((n >> 6) & 63) as usize]
                } else {
                    b'='
                },
                if chunk.len() > 2 {
                    ABC[(n & 63) as usize]
                } else {
                    b'='
                },
            ])?;
        }
        self.raw(b"\"")
    }
}

pub(crate) fn decode_base64(encoded: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let (mut written, mut bits, mut count, mut padding) = (0, 0u32, 0, 0);
    for byte in encoded {
        if *byte == b'=' {
            padding += 1;
            if padding > 2 {
                return Err("invalid_response");
            }
            continue;
        }
        if padding > 0 {
            return Err("invalid_response");
        }
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("invalid_response"),
        };
        bits = (bits << 6) | digit as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            *out.get_mut(written).ok_or("capacity_exceeded")? = (bits >> count) as u8;
            written += 1;
        }
    }
    if encoded.len() % 4 != 0 || count != padding * 2 || (bits & ((1 << count) - 1)) != 0 {
        return Err("invalid_response");
    }
    Ok(written)
}
