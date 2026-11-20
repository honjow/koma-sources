//! Optional, source-side HTTP interceptors over the existing host wire format.
//! The app owns actual request admission. An interceptor only runs for a request
//! sent by this client within the current source operation.

use crate::{host, source_protocol};

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
        let encoded = source_protocol::raw_string(self.json, b"bodyBase64")?.ok_or("invalid_response")?;
        let mut written = 0;
        let mut bits = 0u32;
        let mut count = 0;
        let mut padding = 0;
        for byte in encoded {
            if *byte == b'=' {
                padding += 1;
                if padding > 2 { return Err("invalid_response"); }
                continue;
            }
            if padding > 0 { return Err("invalid_response"); }
            let digit = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62, b'/' => 63,
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
        let response = Response { json: br#"{"bodyBase64":"AP+JUE5HAIA="}"# };
        let mut out = [0; 8];
        assert_eq!(response.body_bytes_into(&mut out), Ok(8));
        assert_eq!(out, [0, 255, 137, 80, 78, 71, 0, 128]);
        assert_eq!(response.body_bytes_into(&mut [0; 7]), Err("capacity_exceeded"));
        for json in [
            br#"{"bodyBase64":"===="}"#.as_slice(),
            br#"{"bodyBase64":"AA="}"#.as_slice(),
            br#"{"bodyBase64":"A==="}"#.as_slice(),
            br#"{"bodyBase64":"AB=="}"#.as_slice(),
        ] {
            assert_eq!(Response { json }.body_bytes_into(&mut out), Err("invalid_response"));
        }
    }
}
