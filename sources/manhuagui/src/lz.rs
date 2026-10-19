// Bounded LZ-string Base64 decompression. Reference: Keiyoushi LZString,
// fixed primary 10f06eb8, Apache-2.0; lz-string algorithm by pieroxy.
const CAP: usize = 32768;
static mut PREFIX: [u16; CAP] = [u16::MAX; CAP];
static mut CHAR: [u16; CAP] = [0; CAP];
static mut STACK: [u16; CAP] = [0; CAP];
fn value(c: u8) -> Result<u8, &'static str> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        b'=' => Ok(64),
        _ => Err("invalid LZ alphabet"),
    }
}
struct Bits<'a> {
    s: &'a [u8],
    i: usize,
    pos: u8,
    val: u8,
}
impl<'a> Bits<'a> {
    fn new(s: &'a [u8]) -> Result<Self, &'static str> {
        if s.is_empty() || s.len() > 1024 * 1024 || s.len() % 4 != 0 {
            return Err("invalid LZ input bounds");
        };
        let pad = s.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 3 || s[..s.len() - pad].contains(&b'=') {
            return Err("invalid LZ padding");
        }
        Ok(Self {
            s,
            i: 1,
            pos: 32,
            val: value(s[0])?,
        })
    }
    fn read(&mut self, n: usize) -> Result<usize, &'static str> {
        if n > 16 {
            return Err("LZ code width overflow");
        };
        let mut v = 0;
        for shift in 0..n {
            if self.val == 64 {
                return Err("premature LZ padding");
            };
            if self.val & self.pos != 0 {
                v |= 1 << shift
            };
            self.pos >>= 1;
            if self.pos == 0 {
                self.pos = 32;
                self.val = value(*self.s.get(self.i).ok_or("truncated LZ bits")?)?;
                self.i += 1;
            }
        }
        Ok(v)
    }
    fn finish(&self) -> Result<(), &'static str> {
        let mask = (self.pos as u16 * 2 - 1) as u8;
        if self.val & mask != 0 {
            return Err("nonzero LZ tail");
        };
        if self.s[self.i..].iter().any(|c| *c != b'=') {
            return Err("invalid LZ tail");
        };
        Ok(())
    }
}
fn expand(code: usize, dict: usize, out: &mut [u16], n: &mut usize) -> Result<u16, &'static str> {
    let pre = unsafe { &mut *core::ptr::addr_of_mut!(PREFIX) };
    let chars = unsafe { &mut *core::ptr::addr_of_mut!(CHAR) };
    let stack = unsafe { &mut *core::ptr::addr_of_mut!(STACK) };
    let mut c = code;
    let mut len = 0;
    loop {
        if c >= dict || len >= CAP {
            return Err("invalid LZ dictionary reference");
        };
        stack[len] = chars[c];
        len += 1;
        if pre[c] == u16::MAX {
            break;
        };
        let prev = pre[c] as usize;
        if prev >= c {
            return Err("cyclic LZ dictionary");
        };
        c = prev;
    }
    let first = stack[len - 1];
    if *n + len > out.len() {
        return Err("LZ output overflow");
    };
    for j in (0..len).rev() {
        out[*n] = stack[j];
        *n += 1
    }
    Ok(first)
}
fn add(dict: &mut usize, prefix: u16, ch: u16) -> Result<(), &'static str> {
    if *dict >= CAP {
        return Err("LZ dictionary overflow");
    };
    unsafe {
        PREFIX[*dict] = prefix;
        CHAR[*dict] = ch
    };
    *dict += 1;
    Ok(())
}
pub fn decompress(input: &[u8], out: &mut [u16]) -> Result<usize, &'static str> {
    let mut b = Bits::new(input)?;
    let ch = match b.read(2)? {
        0 => b.read(8)? as u16,
        1 => b.read(16)? as u16,
        _ => return Err("empty or invalid LZ stream"),
    };
    if out.is_empty() {
        return Err("LZ output overflow");
    };
    let mut dict = 3;
    add(&mut dict, u16::MAX, ch)?;
    let mut w = 3;
    let mut n = 0;
    expand(w, dict, out, &mut n)?;
    let mut enlarge = 4usize;
    let mut width = 3;
    loop {
        let mut code = b.read(width)?;
        match code {
            0 | 1 => {
                let ch = b.read(if code == 0 { 8 } else { 16 })? as u16;
                code = dict;
                add(&mut dict, u16::MAX, ch)?;
                enlarge = enlarge.checked_sub(1).ok_or("invalid LZ width")?
            }
            2 => {
                b.finish()?;
                return Ok(n);
            }
            _ => {}
        }
        if enlarge == 0 {
            enlarge = 1 << width;
            width += 1;
        }
        let first = if code < dict {
            expand(code, dict, out, &mut n)?
        } else if code == dict {
            let first = expand(w, dict, out, &mut n)?;
            if n >= out.len() {
                return Err("LZ output overflow");
            };
            out[n] = first;
            n += 1;
            first
        } else {
            return Err("invalid LZ dictionary code");
        };
        add(&mut dict, w as u16, first)?;
        w = code;
        enlarge = enlarge.checked_sub(1).ok_or("invalid LZ width")?;
        if enlarge == 0 {
            enlarge = 1 << width;
            width += 1;
        }
    }
}
pub fn utf8<'a>(src: &[u16], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    for result in core::char::decode_utf16(src.iter().copied()) {
        let ch = result.map_err(|_| "invalid LZ UTF16")?;
        let mut b = [0; 4];
        let v = ch.encode_utf8(&mut b).as_bytes();
        if n + v.len() > out.len() {
            return Err("LZ UTF8 output overflow");
        };
        out[n..n + v.len()].copy_from_slice(v);
        n += v.len();
    }
    Ok(&out[..n])
}
