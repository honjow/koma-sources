# Manhuagui 0.1.2

Source-only ordinary public HTML adapter for https://www.manhuagui.com, based on Keiyoushi extensions-source commit 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0). LZ-string algorithm attribution: Keiyoushi LZString (Apache-2.0), with an independent algorithm check against pieroxy/lz-string 1.5.0 (reference file licensed WTFPL v2); bounded source-local implementation.

Stable source ID com.manhuagui.koma; manga IDs remain comic/<digits>, canonical absolute chapter IDs and page:<chapterURL>:<index> remain unchanged. Catalogue uses actual section order with ul groups reversed within each section; unknown chapter numbers and dates stay null. Public work ratings are unknown; source-wide nsfw conservatively reflects upstream mixed content. Usage is unknown.

Popular/latest paging, text search, detail, public catalogue, fixed Base64-LZ/radix62 dictionary-decoded page metadata and ordinary image requests are implemented. Optional home/filters/settings/credentials/deeplink are not implemented. Fixed initial Chrome 140, Accept-Language, Referer and Origin match the reviewed parent contract. No cookies, hidden VIEWSTATE decoding, adult access, login-check/vote side effects, mirror/UA switching, JavaScript evaluation or image-byte decryption. Unknown packed forms, malformed/truncated data and capacity overflow fail explicitly.

The existing ABI cannot express upstream cross-call main-site 10 requests/10 seconds or image 4/second limits; no equivalent limiter is claimed. HTTP compression parity (br/gzip/zstd) is unproven. This candidate has local code/build gates only; no current source-site or physical-device chain has been accepted. Old tree connection/SSL timeouts are separate transport history, not HTTP rejection evidence.

Source version is 0.1.2; minimum App version remains 0.1.0 (registry fourth field). These are independent. The previous candidate incorrectly required App 0.1.1; root observed native install_blocked/app_version_unsupported on App0.1.0 at17:04:49 and17:05:52, with no package request. Root owns that physical failure evidence. No App check/version/SDK change is needed.

0.1.2 adds finite top-level failed-host-envelope classification only when no valid numeric HTTP status is present. Root observed 0.1.1 canonical popular timeout at17:14:58→17:15:13, status0 and no HTTP response/body. Network failures and native timeout now remain network_error/timeout with fixed privacy-safe messages. Existing numeric status/statusCode exchanges, including ok:false, retain HTTP2xx/403/429 behavior; no retry or request contract change.
