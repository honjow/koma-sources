# DM5 package assets

- `icon.png` is a high-resolution redraw generated using the built-in imagegen
  tool after the user explicitly allowed redrawing. Its references were the
  204 × 50 DM5 wordmark from the official site and the 192 × 192 square icon
  used by the Keiyoushi DM5 extension. It is not an official high-resolution
  original. The reference extension icon has no documented original provenance.
- Official site reference: `https://css99tel.cdndm5.com/v202609180933/dm5/images/logo-big.png`
  (204 × 50; SHA-256 `53fe2c3851f05a23fa26a787ffcb186182b5ac3e1f7bdb2a48c328c590341c26`).
- The generated 1254 × 1254 image was uniformly resized to 1024 × 1024 for the
  package icon size limit, preserving transparency; no crop was applied.
  Packaged PNG SHA-256: `b717d62038cb4902cd2f4cffb738de221013f420fb27887e2051b4b2092db32f`.
- Package identity only; do not use as a manga cover or another source's icon.

## 0.1.28 官网图案裁切候选

图标从上列官网 204×50 原始字标中提取连通的红色“5 + 龙”图案，去除旁边的其他字样，等比放大并置于 960×960 透明画布。原图低分辨率，放大不等于获得高清原图；不是重绘。原图与候选及浅深背景检查保留在 Koma `.hvigor/outputs/source-icon-audit-20261112/`，最终包已取出图标亲审；237 浅色书源管理列表已核对该候选及版本。深色设备列表和浏览首页未验。SHA-256：`45929d5e2785d54f74c01036f9ff56cb4694f4b03c5a1a1e9d58f11d58ec9ae6`。
