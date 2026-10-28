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

2026-10-28：参考扩展 949dea252eb8e867b7b40834a26e13088764ff55 已采用 `/pc/pc/`、`/pc/details/?kuid=` 和 `/pc/view/index.php?zjid=`。新目录使用 `manga:pc-<数字>`，既有 `manga:<旧标识>` 及其章节继续走原 `/comic/` 路由，不能凭数字相同自动迁移用户进度。0.1.5 将未配置时的默认地址改为官网跳转及参考扩展一致的 `http://www.zerobyw33.com`，并走新站协议；已保存的非空地址优先，显式旧地址仍走旧协议。0.1.4 的原默认地址本次验证无法连接；不静默覆盖已保存的地址。参考扩展另有失败时远端域名发现并写入偏好的机制，本源暂未实现。参考地址的目录、详情、章节与图片已按具名实站样本验证；页面变化和旧站内容可用性仍受站点实际状态约束。
