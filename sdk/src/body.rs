//! Host-owned data for one source operation. Never retain references in settings,
//! private state or interaction checkpoints. The host retires them at call end.
use crate::{
    host, source_protocol as p,
    wire::{decode_base64, Writer},
};

pub const CHUNK_BYTES: usize = 65536;
#[derive(Debug)]
pub struct BodyRef {
    id: [u8; 64],
    id_len: usize,
    length: usize,
}
impl BodyRef {
    pub(crate) fn parse(json: &[u8]) -> Result<Self, &'static str> {
        let mut id = [0; 64];
        let id_len = p::text(json, b"id", &mut id)?
            .ok_or("invalid_body_ref")?
            .len();
        if id_len == 0
            || !id[..id_len]
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || *c == b'-')
        {
            return Err("invalid_body_ref");
        }
        let length = core::str::from_utf8(p::direct(json, b"length")?.ok_or("invalid_body_ref")?)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or("invalid_body_ref")?;
        Ok(Self { id, id_len, length })
    }
    pub fn id(&self) -> &[u8] {
        &self.id[..self.id_len]
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    pub fn create() -> Result<Self, &'static str> {
        let mut response = [0; 256];
        let data = rpc(
            br#"{"type":"source_body","version":1,"op":"create"}"#,
            &mut response,
        )?;
        Self::parse(p::direct(data, b"bodyRef")?.ok_or("invalid_body_ref")?)
    }
    pub fn read(
        &self,
        offset: usize,
        out: &mut [u8],
        wire_buffer: &mut [u8],
    ) -> Result<usize, &'static str> {
        if out.len() > CHUNK_BYTES || offset > self.length {
            return Err("invalid_body_range");
        }
        let mut request = [0; 256];
        let mut w = Writer::new(&mut request);
        self.prefix(&mut w, b"read")?;
        w.raw(b",\"offset\":")?;
        w.number(offset)?;
        w.raw(b",\"length\":")?;
        w.number(out.len())?;
        w.raw(b"}")?;
        let n = w.len();
        let response = rpc(&request[..n], wire_buffer)?;
        decode_base64(
            p::raw_string(response, b"base64")?.ok_or("invalid_response")?,
            out,
        )
    }
    /// Random overwrite inside an existing body, or append at its end. No holes.
    pub fn write(
        &mut self,
        offset: usize,
        bytes: &[u8],
        wire_buffer: &mut [u8],
    ) -> Result<(), &'static str> {
        if bytes.len() > CHUNK_BYTES || offset > self.length {
            return Err("invalid_body_range");
        }
        let mut w = Writer::new(wire_buffer);
        self.prefix(&mut w, b"write")?;
        w.raw(b",\"offset\":")?;
        w.number(offset)?;
        w.raw(b",\"base64\":")?;
        w.base64(bytes)?;
        w.raw(b"}")?;
        let n = w.len();
        let mut response = [0; 256];
        rpc(&wire_buffer[..n], &mut response)?;
        self.length = self.length.max(
            offset
                .checked_add(bytes.len())
                .ok_or("invalid_body_range")?,
        );
        Ok(())
    }
    pub fn close(self) -> Result<(), &'static str> {
        let mut request = [0; 256];
        let mut w = Writer::new(&mut request);
        self.prefix(&mut w, b"close")?;
        w.raw(b"}")?;
        let n = w.len();
        rpc(&request[..n], &mut [0; 256])?;
        Ok(())
    }
    /// The source operation's data object. After successful return only the host
    /// owns this body; it becomes the same cached image for reading and download.
    pub fn image_result(self, out: &mut [u8]) -> Result<usize, &'static str> {
        if self.is_empty() {
            return Err("body_empty_image");
        }
        let mut w = Writer::new(out);
        w.raw(b"{\"image\":{\"kind\":\"body\",\"ref\":")?;
        w.string(self.id())?;
        w.raw(b"}}")?;
        Ok(w.len())
    }
    fn prefix(&self, w: &mut Writer<'_>, op: &[u8]) -> Result<(), &'static str> {
        w.raw(b"{\"type\":\"source_body\",\"version\":1,\"op\":")?;
        w.string(op)?;
        w.raw(b",\"id\":")?;
        w.string(self.id())
    }
}

pub(crate) fn rpc<'a>(request: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let n = host::http_request(request, out).map_err(|_| "host_call_failed")?;
    let response = &out[..n];
    if p::direct(response, b"ok")? != Some(b"true".as_slice()) {
        let code = p::direct(response, b"error")?
            .and_then(|error| p::raw_string(error, b"code").ok().flatten());
        return Err(match code {
            Some(b"body_limit_exceeded") => "body_limit_exceeded",
            Some(b"body_not_found") => "body_not_found",
            Some(b"invalid_body_range") => "invalid_body_range",
            Some(b"body_unavailable") => "body_unavailable",
            Some(b"source_request_cancelled") => "source_request_cancelled",
            Some(b"invalid_identity") => "invalid_identity",
            _ => "host_operation_failed",
        });
    }
    Ok(response)
}
