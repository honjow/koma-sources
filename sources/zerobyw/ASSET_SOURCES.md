# Zerobyw asset source

## `icon.png`

- **Purpose:** Package-owned identity icon for the Zerobyw source only. It is
  not a manga cover, a generic category icon, or reusable product branding.
- **Official source:** `https://www.zerobyw33.com/favicon.ico`
- **Provenance:** The source's configured `https://www.zerobyw.com/` referer
  currently redirects to the first-party Zerobyw33 site, whose page displays
  the matching Zerobyw brand mark.
- **Retrieved:** 2026-08-14.
- **Processing:** Extracted the official 128×128 ICO frame as an RGBA PNG with
  Pillow; no artwork was redrawn or altered. The hostname may rotate, so this
  package retains the verified snapshot rather than dynamically loading it.
- **Packaged SHA-256:**
  `f69deab961f505281718665556930358c9b88c01ee77f07f26e95ea4f93048e0`
- **Rights note:** The mark remains the property of its owner and is included
  solely to identify this user-selected source.

2026-10-26显示名核对：源自身既有Referer https://www.zerobyw.com/ 普通打开重定向https://www.zerobyw33.com/pc/pc/，站点标题为“zero搬运网”。0.1.1仅补该官方简繁显示名/保英文Zerobyw、ID、原请求域名和可配置baseUrl、图标及权限，不把主页重定向改成源请求端点。


## 0.1.2 navigation and settings boundary (2026-10-27)

The official ordinary frontend is `https://www.zerobyw.com/` (currently redirects
to `https://www.zerobyw33.com/pc/pc/`). This declared navigation URL is independent
of the existing default request backend and does not grant Web-session origins.

The old WASM imported `koma_host.get_setting`, which exists in the development
runner but is absent from Koma's production Host ABI. The source now reads its
existing `baseUrl` value from the per-request `settings` object, like other
production sources. The default request backend, chapter identities and order
are retained. A repaired build/import contract is not full live-site acceptance.
