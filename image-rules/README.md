# 旧版图片规则参考

本目录记录旧版 schema 1 图片规则的格式与生成方式，旧资源作为兼容资料保留。当前图片处理插件以独立 WASM 提供，请使用[图片处理插件作者指南](plugins/README.md)中的订阅地址和开发方式。

## 新增或修改规则

1. 将参考图放入 `templates/`，保留原始素材及来源说明。在 `definitions/` 新建一个 JSON 文件作为规则包，参照 [`edge-banners.json`](definitions/edge-banners.json)。每个包有独立 `id`、正整数 `version`、`meta`（`name`、`description`、HTTPS `website`）和 `rules`；每条规则有独立 `id`、`name`、`description`。
2. 当前支持 `kind: edge_signature_v1`，用于识别页面顶部或底部的已知图案。`signatures` 的 `image` 相对于定义文件，读取图像尺寸并计算逐行签名；`top` 和 `bottom` 按顺序引用签名，`height` 可限制底部匹配高度。`crossSource` 表示允许跨源识别。可选 `scale` 包含 `minWidthExclusive`、`maxWidthExclusive`、`referenceWidth`；先缩放完整签名，再按目标宽度计算底部高度。`legacy` 可声明旧处理类型及模式，`sourceIds` 仅限定旧缓存声明的探测范围，不限制自动识别或已声明类型的匹配。
3. 用 Pillow 运行生成器并检查输出：

   ```bash
   python3 image-rules/generate.py
   ```

   可使用 `--definitions`、`--catalog`、`--output` 指向其他目录或目录元数据。生成时校验引用、类型及尺寸；输出为 `index.json` 和以 SHA-256 命名的规则包。实际用含目标横幅、无横幅和缩放或重压缩的页面样本检查识别与裁切，再提交规则。更改已有包内容时递增该包的 `version`。目录名称、简介及主页在 `catalog.json` 中维护。
4. 旧规则的生成流程保留供兼容维护；新增图片处理算法请按插件作者指南提供独立 WASM。

签名从水平 20%–80% 范围取 16 列，每行分别对 RGB 求平均并向下取整。当前模板由 Koma 的 `SourceBaoziBannerTemplates.ets` 原样提取，来源为 Keiyoushi 固定提交 `949dea252eb8e867b7b40834a26e13088764ff55`，基于 stevenyomi/baozibanner。三张 JPEG 的 SHA-256 分别为 `876cb19330763cb8aaeb3b09a5d563b5ddd537715e4a2a7b273814024b2b00fe`、`8d4706b611ab8c3799782698be3fe4e759d66418522504620d041e890cfc60d7`、`d900935cae37df0fed07d74b5f51d36ac1e491c67a56f9c24ac2d46f0395cb79`。生成器使用 Pillow JPEG 解码，与 Harmony ImageKit 解码逐值一致性尚未确认。
