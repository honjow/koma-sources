//! Optional, source-side HTTP interceptors over the existing host wire format.
//! The app owns actual request admission. An interceptor only runs for a request
//! sent by this client within the current source operation.

use crate::{body::BodyRef, session::SessionRef, wire::Writer};
use crate::{host, source_protocol};

#[derive(Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}
impl Method {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Get => b"GET",
            Self::Post => b"POST",
            Self::Put => b"PUT",
            Self::Patch => b"PATCH",
            Self::Delete => b"DELETE",
            Self::Head => b"HEAD",
            Self::Options => b"OPTIONS",
        }
    }
}
pub enum Body<'a> {
    Empty,
    Text(&'a [u8]),
    Bytes(&'a [u8]),
    Reference(&'a BodyRef),
}
pub enum ResponseKind {
    Text,
    Bytes,
    BodyRef,
}
pub struct Header<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

/// Fully bounded request encoding. The encoded Request still traverses the same
/// interceptors, including source-defined signing and response/retry policies.
pub struct RequestSpec<'a> {
    pub url: &'a [u8],
    pub method: Method,
    pub headers: &'a [Header<'a>],
    pub body: Body<'a>,
    pub response: ResponseKind,
    pub session: SessionRef,
    pub timeout_ms: u32,
    pub final_url: bool,
    pub response_headers: bool,
}
impl<'a> RequestSpec<'a> {
    pub fn get(url: &'a [u8]) -> Self {
        Self {
            url,
            method: Method::Get,
            headers: &[],
            body: Body::Empty,
            response: ResponseKind::Text,
            session: SessionRef::BrowserGrant,
            timeout_ms: 15000,
            final_url: false,
            response_headers: false,
        }
    }
    pub fn encode<'out>(&self, out: &'out mut [u8]) -> Result<Request<'out>, &'static str> {
        let mut w = Writer::new(out);
        w.raw(b"{\"url\":")?;
        w.string(self.url)?;
        w.raw(b",\"method\":")?;
        w.string(self.method.bytes())?;
        w.raw(b",\"headers\":{")?;
        for (index, header) in self.headers.iter().enumerate() {
            if header.name.is_empty()
                || header
                    .name
                    .iter()
                    .any(|c| !c.is_ascii_graphic() || *c == b':')
                || header
                    .value
                    .iter()
                    .any(|c| *c == b'\r' || *c == b'\n' || *c == 0)
            {
                return Err("invalid_header");
            }
            if index > 0 {
                w.raw(b",")?;
            }
            w.string(header.name)?;
            w.raw(b":")?;
            w.string(header.value)?;
        }
        w.raw(b"},\"timeoutMs\":")?;
        w.number(self.timeout_ms as usize)?;
        match self.body {
            Body::Empty => (),
            Body::Text(value) => {
                w.raw(b",\"body\":")?;
                w.string(value)?;
            }
            Body::Bytes(value) => {
                w.raw(b",\"requestBytes\":")?;
                w.base64(value)?;
            }
            Body::Reference(value) => {
                w.raw(b",\"requestBodyRef\":")?;
                w.string(value.id())?;
            }
        }
        match self.response {
            ResponseKind::Text => (),
            ResponseKind::Bytes => w.raw(br#","responseKind":"bodyBytes""#)?,
            ResponseKind::BodyRef => w.raw(br#","responseKind":"bodyRef""#)?,
        }
        if matches!(self.session, SessionRef::SourceCookies) {
            w.raw(br#","session":"source""#)?;
        }
        if self.final_url || self.response_headers {
            w.raw(b",\"responseMetadata\":{")?;
            if self.final_url {
                w.raw(b"\"finalUrl\":true")?;
            }
            if self.response_headers {
                if self.final_url {
                    w.raw(b",")?;
                }
                w.raw(b"\"headers\":true")?;
            }
            w.raw(b"}")?;
        }
        w.raw(b"}")?;
        let length = w.len();
        Ok(Request::from_json(&out[..length]))
    }
}

#[derive(Clone, Copy)]
pub struct Request<'a> {
    json: &'a [u8],
}

impl<'a> Request<'a> {
    pub const fn from_json(json: &'a [u8]) -> Self {
        Self { json }
    }

    pub const fn as_json(self) -> &'a [u8] {
        self.json
    }
}

pub struct Response<'a> {
    json: &'a [u8],
}

impl<'a> Response<'a> {
    /// Header values remain a JSON array to preserve duplicate fields. Header
    /// names are lower-case; Set-Cookie is deliberately kept in the host jar.
    pub fn header_values_json(&self, lower_name: &[u8]) -> Result<Option<&[u8]>, &'static str> {
        let Some(headers) = source_protocol::direct(self.json, b"headers")? else {
            return Ok(None);
        };
        source_protocol::direct(headers, lower_name)
    }
    pub fn body_ref(&self) -> Result<BodyRef, &'static str> {
        BodyRef::parse(
            source_protocol::direct(self.json, b"bodyRef")?.ok_or("body_ref_unavailable")?,
        )
    }
    pub fn as_json(&self) -> &'a [u8] {
        self.json
    }

    /// HTTP failures have a status too. A transport failure envelope may omit it.
    pub fn status(&self) -> Option<u16> {
        let status = source_protocol::direct(self.json, b"status").ok()??;
        core::str::from_utf8(status).ok()?.parse().ok()
    }

    /// Decode a binary HTTP response without converting its payload to UTF-8.
    pub fn body_bytes_into(&self, out: &mut [u8]) -> Result<usize, &'static str> {
        let encoded =
            source_protocol::raw_string(self.json, b"bodyBase64")?.ok_or("invalid_response")?;
        let mut written = 0;
        let mut bits = 0u32;
        let mut count = 0;
        let mut padding = 0;
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

    /// Effective URL of a response when requested from a compatible host.
    pub fn final_url<'out>(&self, out: &'out mut [u8]) -> Result<Option<&'out [u8]>, &'static str> {
        source_protocol::text(self.json, b"responseUrl", out)
    }

    /// Decodes the text body for any HTTP status. The caller interprets the status.
    pub fn body_text<'out>(&self, out: &'out mut [u8]) -> Result<Option<&'out [u8]>, &'static str> {
        source_protocol::text(self.json, b"bodyText", out)
    }

    /// Decodes the text body using the existing source protocol body decoder.
    /// Install RequireSuccess to apply the standard HTTP status policy first.
    pub fn decode_body_into(&self, out: &mut [u8]) -> Result<usize, &'static str> {
        source_protocol::decode_http_body(self.json, out)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CallError {
    Transport(i32),
    InvalidResponseLength,
    Rejected(&'static str),
}

/// Applies the standard source protocol HTTP error mapping. Sources that need
/// to inspect a non-success response can leave this out or put their policy
/// between this interceptor and the transport.
pub struct RequireSuccess;

impl<E: From<CallError>> Interceptor<E> for RequireSuccess {
    fn intercept<'out>(
        &self,
        request: Request<'_>,
        next: Chain<'_, E>,
        out: &'out mut [u8],
    ) -> Result<Response<'out>, E> {
        let response = next.proceed(request, out)?;
        source_protocol::require_http_success(response.as_json()).map_err(CallError::Rejected)?;
        Ok(response)
    }
}

pub trait Interceptor<E = CallError> {
    fn intercept<'out>(
        &self,
        request: Request<'_>,
        next: Chain<'_, E>,
        out: &'out mut [u8],
    ) -> Result<Response<'out>, E>;
}

type Transport = fn(&[u8], &mut [u8]) -> Result<usize, i32>;

pub struct Chain<'a, E = CallError> {
    interceptors: &'a [&'a dyn Interceptor<E>],
    transport: Transport,
}

impl<E: From<CallError>> Chain<'_, E> {
    /// Runs the rest of the chain. A retry must drop the old response before
    /// reusing the output buffer. Every send still goes through host admission.
    pub fn proceed<'out>(
        &self,
        request: Request<'_>,
        out: &'out mut [u8],
    ) -> Result<Response<'out>, E> {
        if let Some((interceptor, rest)) = self.interceptors.split_first() {
            let next = Self {
                interceptors: rest,
                transport: self.transport,
            };
            return interceptor.intercept(request, next, out);
        }
        let written = (self.transport)(request.as_json(), out).map_err(CallError::Transport)?;
        let json = out.get(..written).ok_or(CallError::InvalidResponseLength)?;
        Ok(Response { json })
    }
}

pub struct Client<'a, E = CallError> {
    chain: Chain<'a, E>,
}

impl<'a, E: From<CallError>> Client<'a, E> {
    pub const fn new(interceptors: &'a [&'a dyn Interceptor<E>]) -> Self {
        Self {
            chain: Chain {
                interceptors,
                transport: host::http_request,
            },
        }
    }

    pub fn send<'out>(
        &self,
        request: Request<'_>,
        out: &'out mut [u8],
    ) -> Result<Response<'out>, E> {
        self.chain.proceed(request, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[test]
    fn typed_request_preserves_binary_upload_and_metadata_through_transport() {
        let mut spec = RequestSpec::get(b"https://fixture.test/data?q=\"quoted\"");
        spec.method = Method::Post;
        spec.body = Body::Bytes(&[0, 255, 128]);
        spec.response = ResponseKind::BodyRef;
        spec.session = SessionRef::SourceCookies;
        spec.response_headers = true;
        let mut encoded = [0; 1024];
        let request = spec.encode(&mut encoded).unwrap();
        let client: Client<'_> = Client {
            chain: Chain {
                interceptors: &[],
                transport: |request, out| {
                    assert_eq!(
                        source_protocol::raw_string(request, b"method").unwrap(),
                        Some(b"POST".as_slice())
                    );
                    let mut decoded = [0; 3];
                    crate::wire::decode_base64(
                        source_protocol::raw_string(request, b"requestBytes")
                            .unwrap()
                            .unwrap(),
                        &mut decoded,
                    )
                    .unwrap();
                    assert_eq!(decoded, [0, 255, 128]);
                    let json = br#"{"ok":true,"status":206,"headers":{"content-type":["image/png"]},"bodyRef":{"id":"abc-123","length":2097153}}"#;
                    out[..json.len()].copy_from_slice(json);
                    Ok(json.len())
                },
            },
        };
        let mut out = [0; 512];
        let response = client.send(request, &mut out).unwrap();
        assert_eq!(response.status(), Some(206));
        assert_eq!(response.body_ref().unwrap().len(), 2097153);
        assert_eq!(
            response.header_values_json(b"content-type").unwrap(),
            Some(br#"["image/png"]"#.as_slice())
        );
    }

    fn transport(request: &[u8], out: &mut [u8]) -> Result<usize, i32> {
        assert_eq!(request, br#"{"url":"https://fixture.test/rewritten"}"#);
        let result = br#"{"ok":true,"status":404,"bodyText":"missing"}"#;
        out[..result.len()].copy_from_slice(result);
        Ok(result.len())
    }

    struct Layer<'a> {
        name: &'static str,
        trace: &'a RefCell<Vec<(&'static str, bool)>>,
    }

    impl Interceptor for Layer<'_> {
        fn intercept<'out>(
            &self,
            request: Request<'_>,
            next: Chain<'_>,
            out: &'out mut [u8],
        ) -> Result<Response<'out>, CallError> {
            self.trace.borrow_mut().push((self.name, true));
            let request = if self.name == "inner" {
                Request::from_json(br#"{"url":"https://fixture.test/rewritten"}"#)
            } else {
                request
            };
            let response = next.proceed(request, out)?;
            assert_eq!(response.status(), Some(404));
            self.trace.borrow_mut().push((self.name, false));
            Ok(response)
        }
    }

    #[test]
    fn traverses_requests_forward_and_responses_in_reverse() {
        let trace = RefCell::new(Vec::new());
        let outer = Layer {
            name: "outer",
            trace: &trace,
        };
        let inner = Layer {
            name: "inner",
            trace: &trace,
        };
        let client = Client {
            chain: Chain {
                interceptors: &[&outer, &inner],
                transport,
            },
        };
        let mut out = [0; 128];
        let response = client
            .send(
                Request::from_json(br#"{"url":"https://fixture.test/original"}"#),
                &mut out,
            )
            .unwrap();
        assert_eq!(
            *trace.borrow(),
            [
                ("outer", true),
                ("inner", true),
                ("inner", false),
                ("outer", false)
            ]
        );
        let mut text = [0; 16];
        assert_eq!(
            response.body_text(&mut text).unwrap(),
            Some(b"missing".as_slice())
        );
    }

    #[test]
    fn host_bridge_errors_propagate_without_replacing_the_response() {
        let client: Client<'_> = Client {
            chain: Chain {
                interceptors: &[],
                transport: |_, _| Err(-3),
            },
        };
        assert!(matches!(
            client.send(Request::from_json(b"{}"), &mut [0; 8]),
            Err(CallError::Transport(-3))
        ));
    }

    #[test]
    fn binary_body_preserves_bytes_and_rejects_invalid_or_oversized_payloads() {
        let response = Response {
            json: br#"{"bodyBase64":"AP+JUE5HAIA="}"#,
        };
        let mut out = [0; 8];
        assert_eq!(response.body_bytes_into(&mut out), Ok(8));
        assert_eq!(out, [0, 255, 137, 80, 78, 71, 0, 128]);
        assert_eq!(
            response.body_bytes_into(&mut [0; 7]),
            Err("capacity_exceeded")
        );
        for json in [
            br#"{"bodyBase64":"===="}"#.as_slice(),
            br#"{"bodyBase64":"AA="}"#.as_slice(),
            br#"{"bodyBase64":"A==="}"#.as_slice(),
            br#"{"bodyBase64":"AB=="}"#.as_slice(),
        ] {
            assert_eq!(
                Response { json }.body_bytes_into(&mut out),
                Err("invalid_response")
            );
        }
    }
}
