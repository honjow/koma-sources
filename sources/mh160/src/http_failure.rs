// Only the Host's standardized top-level access signal identifies a challenge.
// A refused HTTP response or body containing a marker alone is insufficient.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HttpFailure {
    pub status: u16,
    challenge: bool,
}
impl HttpFailure {
    pub fn from_host(status: u16, access_signal: Option<&[u8]>) -> Self {
        Self { status, challenge: matches!(status, 401 | 403 | 503)
            && access_signal == Some(b"\"challenge_required\"".as_slice()) }
    }
    pub fn code(self) -> &'static str {
        if self.challenge { return "challenge_required"; }
        match self.status {
            401 | 403 => "permission_denied", 404 => "not_found", 429 => "rate_limit",
            400..=499 => "client_error", 500..=599 => "server_error", _ => "source_error",
        }
    }
    pub fn message(self) -> &'static str {
        if self.challenge { return "website verification required"; }
        match self.status {
            401 | 403 => "public request rejected", 404 => "not found", 429 => "rate limited",
            400..=499 => "client failure", 500..=599 => "server failure", _ => "unexpected HTTP status",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_challenge_retains_each_refused_status() {
        for status in [401, 403, 503] {
            let failure = HttpFailure::from_host(status, Some(b"\"challenge_required\""));
            assert_eq!(failure.status, status);
            assert_eq!(failure.code(), "challenge_required");
        }
    }
    #[test]
    fn refusal_and_lookalike_signals_never_infer_challenge() {
        for signal in [None, Some(b"challenge_required".as_slice()), Some(b"\"auth_required\""),
            Some(b"\"challenge_required_extra\""), Some(b"<title>Just a moment</title>")] {
            let failure = HttpFailure::from_host(403, signal);
            assert_eq!(failure.code(), "permission_denied"); assert_eq!(failure.status, 403);
        }
    }
    #[test]
    fn nonchallenge_statuses_keep_their_own_cause_and_status() {
        for (status, expected) in [(404,"not_found"),(429,"rate_limit"),(400,"client_error"),(500,"server_error")] {
            let failure = HttpFailure::from_host(status, Some(b"\"challenge_required\""));
            assert_eq!(failure.code(), expected); assert_eq!(failure.status, status);
        }
    }
}
