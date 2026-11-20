//! Existing private-session transactions, not another credential store.
//! Version tags and initializer leases belong to the installed source. Preserve
//! the exact returned tag when invalidating an observed generation.
use crate::{body::rpc, source_protocol as p, wire::Writer};

#[derive(Clone, Copy)]
pub enum SessionRef {
    BrowserGrant,
    SourceCookies,
}
pub enum Transaction<'a> {
    Ready {
        value_json: &'a [u8],
        version_tag: &'a [u8],
    },
    Owner {
        lease: &'a [u8],
    },
}
pub struct PrivateSession;
impl PrivateSession {
    pub fn begin<'a>(
        replace_version: Option<&[u8]>,
        request: &mut [u8],
        out: &'a mut [u8],
    ) -> Result<Transaction<'a>, &'static str> {
        let mut w = Writer::new(request);
        w.raw(br#"{"type":"private_session","version":1,"op":"begin""#)?;
        if let Some(version) = replace_version {
            w.raw(b",\"replaceVersionTag\":")?;
            w.string(version)?;
        }
        w.raw(b"}")?;
        let n = w.len();
        let response = rpc(&request[..n], out)?;
        match p::raw_string(response, b"state")? {
            Some(b"ready") => Ok(Transaction::Ready {
                value_json: p::direct(response, b"value")?.ok_or("invalid_response")?,
                version_tag: p::raw_string(response, b"versionTag")?.ok_or("invalid_response")?,
            }),
            Some(b"owner") => Ok(Transaction::Owner {
                lease: p::raw_string(response, b"lease")?.ok_or("invalid_response")?,
            }),
            _ => Err("invalid_response"),
        }
    }
    pub fn commit(
        lease: &[u8],
        value: &[u8],
        request: &mut [u8],
        out: &mut [u8],
    ) -> Result<(), &'static str> {
        Self::change(b"commit", b"lease", lease, Some(value), request, out)
    }
    pub fn abort(lease: &[u8], request: &mut [u8], out: &mut [u8]) -> Result<(), &'static str> {
        Self::change(b"abort", b"lease", lease, None, request, out)
    }
    pub fn invalidate(
        version: &[u8],
        request: &mut [u8],
        out: &mut [u8],
    ) -> Result<(), &'static str> {
        Self::change(b"invalidate", b"versionTag", version, None, request, out)
    }
    fn change(
        op: &[u8],
        key: &[u8],
        token: &[u8],
        value: Option<&[u8]>,
        request: &mut [u8],
        out: &mut [u8],
    ) -> Result<(), &'static str> {
        let mut w = Writer::new(request);
        w.raw(b"{\"type\":\"private_session\",\"version\":1,\"op\":")?;
        w.string(op)?;
        w.raw(b",")?;
        w.string(key)?;
        w.raw(b":")?;
        w.string(token)?;
        if let Some(value) = value {
            w.raw(b",\"value\":")?;
            w.string(value)?;
        }
        w.raw(b"}")?;
        let n = w.len();
        rpc(&request[..n], out)?;
        Ok(())
    }
}
