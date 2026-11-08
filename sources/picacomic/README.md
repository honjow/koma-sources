# 哔咔漫画 (Picacomic)

Independent no_std Rust/WASM implementation of complete Picacomic.kt, Dto.kt,
Filters.kt and ChannelDns.kt at Keiyoushi
de743aed846da53e00c45ab26ee0e77f7bb4355b (v14/lib1.6, Apache-2.0).
The adopted source name, NSFW/zh classification and manhuabika.com public links
are preserved. The latter is the public Web front end declared by the reference.
Credentials are sent only to the Picacomic API origin. Browse, detail, chapters
and pages require a Picacomic account. Koma 1.2.0 or later is required.

Authentication uses the approved credentials descriptor and explicit Native
private form. Username maps to wire email. No username/password/token setting,
request arg, Web Cookie, operation result or log supplies authentication.
get_authentication reads ready private state, or aborts an empty owner, with
zero HTTP. No-input browse returns auth_required before any login/API request,
including with public args/settings that imitate credentials.

An explicit login uses credential_input only in its granted authenticate call,
then claims an initial or replaceVersionTag lease. The previous account remains
published until the new sign-in and atomic commit both succeed. Rejection,
cancellation, failed commit and abort preserve the previous account. Logout
invalidates only this source/install generation. Normal 401 refreshes the stale
saved-token generation once; concurrent calls adopt the newly published token.
The original request's URL/method/body/time/nonce/signature are retained on retry,
with only Authorization replaced. 403 does not trigger login or hidden retry.
Credentials and token live only in the existing private Asset owner.

All original public preferences remain: BLOCK_GENRES, IMAGE_QUALITY
(original/low/medium/high), APP_CHANNEL (default 2, options 1/2) and APP_CHANNEL_URL.
The old Android channel3 effective migration is interpreted as 2; no Host public
setting is silently written. Use Account management in source settings to sign
in, sign out or replace an account through the private form.
The reference category/tag fallback and exact blocked-word matching are kept.

Popular pagination, reference Latest/random (always more), favourite precedence,
rank precedence, empty keyword/category API, advanced-search JSON body, public
comic/reader URL import, metadata, complete paginated eps and complete paginated
pages are implemented. Chapter order and original page*limit ordinals remain;
repeated image URLs retain separate positions. A failed later page never returns
partial content. Nullable ISO timestamp parsing retains milliseconds and timezone
for the verified fixtures. Related/deeplink registration is not exposed by the
current source ABI. Standard image descriptions retain the original site's
Referer, without leaking API/auth headers to an opaque fileServer URL.

Signatures retain the fixed public API key/client headers and exact content type.
HMAC-SHA256 uses RustCrypto SHA256 with RFC2104 padding and the reference's public
key. Clock and secure random come from the existing private RPC. Rejection
sampling generates 32 characters uniformly from the original alphanumeric set.
The legacy Host bodyBase64 field carries raw UTF-8 text, as the existing
SDK/CLI/App actually implement; JSON bodies are not base64 encoded.

Channel 2 requests the Host DNS plan through `source_dns_pica_channel_v1`.
The Host selects the committed init URL and performs public initialization. The
source takes the returned opaque proof into the original signed HTTPS API
request and same-domain reader image descriptor. The proof is never persisted
in credentials, settings, or diagnostic output. A missing capability, refused
initialization, or missing proof stops that request before API access. The Host retains the verified plan in process memory for the same installed
package. Cover requests use their manga ID, and follow the same public headers
and origin-scoped transport as the original thumbnail. Channel 1
and unrelated image CDNs keep their ordinary transport. Anonymous browsing and
empty explicit credentials make no init or API request.

Local verification runs the actual WASM and production private-session owner
under Node adapters. The 44 previous protocol and ownership cases and 22 channel
cases use offline provider fixtures. Signed requests are checked by independent
Node crypto against the Kotlin signing key. Actual SourceModels consumes the
favourite checkbox and chapter date. The actual Host plan reuse is covered by the Host RCP adapter test;
The account status and settings metadata make no HTTP requests. Sign-in and
reader paths after successful authentication were exercised with fixtures.
Successful sign-in and reading with a real account remain unverified.

Internal V2 review correction: work IDs are canonical safe URL segments (dot or encoded path traversal fails before RPC), and both ordinary and URL-imported details require response _id ownership. Actual frozen V1 response/HTTP counterexamples remain in owner-before-fix.json; the new cases keep the original provider/Host fixtures and do not alter expected ownership to fit the implementation.
