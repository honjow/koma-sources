//! Opt-in ordinary HTTPS IMAGE headers. No Host ABI or legacy default changes.
//! Ordinary UA reference: mihonapp/mihon c3bf7a78c074079a605ed7de86d8b7f58bea183b,
//! core/common/src/main/kotlin/eu/kanade/tachiyomi/network/NetworkPreferences.kt (default_user_agent).
//! IMAGE policy reference: keiyoushi/extensions-source 8947c372cd87f546a0b80cb0e4b0a5a8add5ef88,
//! core/src/main/kotlin/keiyoushi/network/BrowserHeaders.kt. This is a public HTTP client identity,
//! independent of a WebView grant's UA and never a credential grant.
use crate::{http::Header, wire::Writer};

pub const DEFAULT_HTTP_USER_AGENT: &[u8] = b"Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36";

/// The caller establishes this from actual origins and the provider's known site.
/// Do not approximate arbitrary registrable domains by taking the last two labels.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SiteRelation {
    SameOrigin,
    SameSite,
    CrossSite,
}
impl SiteRelation {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::SameOrigin => b"same-origin",
            Self::SameSite => b"same-site",
            Self::CrossSite => b"cross-site",
        }
    }
}

/// Allocator-free preparation for the shared ordinary Android/Chromium identity.
/// Caller must use HTTPS. Referer is copied in full (bare Fetch.IMAGE semantics),
/// language is source-provided, and no UI locale or Web grant is consulted.
/// Unknown transport protocol adds neither Priority nor compression advertising.
/// There is no Origin, Sec-Fetch-User or Upgrade-Insecure-Requests on IMAGE GET.
pub struct ImageHeaders<'a> {
    referer: &'a [u8],
    language: &'a [u8],
    brands: [u8; 192],
    brands_len: usize,
    accept: &'static [u8],
    storage_access: Option<&'static [u8]>,
}
pub struct HeaderList<'a> {
    headers: [Header<'a>; 11],
    len: usize,
}
impl<'a> HeaderList<'a> {
    pub fn as_slice(&self) -> &[Header<'a>] {
        &self.headers[..self.len]
    }
}

fn valid_value(value: &[u8], max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && core::str::from_utf8(value).is_ok()
        && !value.iter().any(|b| *b < 0x20 || *b == 0x7f)
}
fn chrome_major() -> Result<usize, &'static str> {
    let ua = DEFAULT_HTTP_USER_AGENT;
    let start = ua
        .windows(7)
        .position(|s| s == b"Chrome/")
        .ok_or("invalid_user_agent")?
        + 7;
    let mut version = 0usize;
    let mut digits = 0usize;
    for b in &ua[start..] {
        if !b.is_ascii_digit() {
            break;
        }
        version = version
            .checked_mul(10)
            .and_then(|v| v.checked_add((b - b'0') as usize))
            .ok_or("invalid_user_agent")?;
        digits += 1;
    }
    if digits == 0 {
        return Err("invalid_user_agent");
    }
    Ok(version)
}
impl<'a> ImageHeaders<'a> {
    pub fn new(referer: &'a [u8], language: &'a [u8]) -> Result<Self, &'static str> {
        if !valid_value(referer, 8192) || !valid_value(language, 512) {
            return Err("invalid_header");
        }
        let major = chrome_major()?;
        let mut brands = [0; 192];
        let mut w = Writer::new(&mut brands);
        const ORDERS: [[usize; 3]; 6] = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        // Exact pinned GREASE alphabet; kept explicit to avoid whitespace ambiguity.
        const GREASE: [u8; 11] = [
            b' ', b'(', b':', b'-', b'.', b'/', b')', b';', b'=', b'?', b'_',
        ];
        let order = ORDERS[major % 6];
        for slot in 0..3 {
            if slot > 0 {
                w.raw(b", ")?;
            }
            if slot == order[0] {
                w.raw(b"\"Not")?;
                w.raw(&[GREASE[major % 11]])?;
                w.raw(b"A")?;
                w.raw(&[GREASE[(major + 1) % 11]])?;
                w.raw(b"Brand\";v=\"")?;
                w.number([8, 99, 24][major % 3])?;
                w.raw(b"\"")?;
            } else {
                w.raw(if slot == order[1] {
                    b"\"Chromium\";v=\""
                } else {
                    b"\"Google Chrome\";v=\""
                })?;
                w.number(major)?;
                w.raw(b"\"")?;
            }
        }
        let brands_len = w.len();
        Ok(Self {
            referer,
            language,
            brands,
            brands_len,
            accept: if major >= 155 {
                b"image/jxl,image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8"
            } else {
                b"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8"
            },
            storage_access: if major >= 142 {
                Some(b"active")
            } else if major >= 133 {
                Some(b"none")
            } else {
                None
            },
        })
    }
    pub fn headers(&self, relation: SiteRelation) -> HeaderList<'_> {
        let storage = if relation == SiteRelation::CrossSite {
            self.storage_access
        } else {
            None
        };
        HeaderList {
            headers: [
                Header {
                    name: b"Referer",
                    value: self.referer,
                },
                Header {
                    name: b"User-Agent",
                    value: DEFAULT_HTTP_USER_AGENT,
                },
                Header {
                    name: b"Accept-Language",
                    value: self.language,
                },
                Header {
                    name: b"Accept",
                    value: self.accept,
                },
                Header {
                    name: b"Sec-CH-UA",
                    value: &self.brands[..self.brands_len],
                },
                Header {
                    name: b"Sec-CH-UA-Mobile",
                    value: b"?1",
                },
                Header {
                    name: b"Sec-CH-UA-Platform",
                    value: b"\"Android\"",
                },
                Header {
                    name: b"Sec-Fetch-Mode",
                    value: b"no-cors",
                },
                Header {
                    name: b"Sec-Fetch-Dest",
                    value: b"image",
                },
                Header {
                    name: b"Sec-Fetch-Site",
                    value: relation.bytes(),
                },
                Header {
                    name: b"Sec-Fetch-Storage-Access",
                    value: storage.unwrap_or(b""),
                },
            ],
            len: if storage.is_some() { 11 } else { 10 },
        }
    }
}
