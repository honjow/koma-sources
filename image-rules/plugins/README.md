# 图片处理插件 SDK（候选）

插件用 `wasm32-unknown-unknown` 编译，导出线性 `memory`、`koma_image_processor_abi() -> i32`（返回 1）与 `koma_image_process() -> i32`。后者返回 0 保留原图、1 表示完整输出、负数表示失败。示例 Rust SDK 位于 `sdk/`；`banner/` 是横幅处理器，`invert/` 是独立反色示例，不加入默认目录。

Host 只提供 `koma_image` imports：尺寸、配置长度与读取、紧密、非预乘的 RGBA8888 区域读取（保留 alpha）、一次输出创建、按完整行顺序写入和取消检查。读取及写入成功返回字节数，失败返回 -1；创建成功返回 0。`check_cancel` 非零即停止。返回 1 前必须覆盖输出每一行；0、错误及 trap 均丢弃输出。配置为 `{parameters,settings,context:{application,sourceHint?}}`，最长 4 MiB。宿主可能在 5 秒后取消；模块上限 2 MiB、线性内存上限 8 MiB，输出上限 256 MiB。

在此独立 workspace 内使用已安装 wasm target 构建（不改书源 workspace 锁）：

```sh
RUSTFLAGS='-C link-arg=--max-memory=8388608' cargo build --manifest-path image-rules/plugins/Cargo.toml --target wasm32-unknown-unknown --release --locked
python3 image-rules/generate_plugins.py --output /tmp/koma-image-plugin-candidate
```

`plugin-definitions/*.json` 各描述一个插件，提供 id、version、meta、processor、parameters 与 application；`parametersFromRule` 可从现有签名定义生成参数。processor.wasm 相对定义文件，或用 `--processor id=path` 覆盖；`--definitions`、`--catalog` 和 `--output` 可指定其他位置。生成 schema 2 `image_plugins` 索引、内容寻址 JSON 包及 WASM。用户通过明确添加插件订阅获得处理器；旧 schema 1 资源不删除。默认目录提供“去除包子漫画横幅”插件；独立反色示例不在默认目录中。源码作者须在真实样本、空处理、取消及无关页面上验证输出；不要把构建成功当成设备验收。

目录元数据由 `image-rules/plugin-catalog.json` 提供，目录名称为“Koma 图片处理插件”，与旧图片规则目录分离。插件定义可选 `output`，例如 `{"format":"image/jpeg","quality":90}`；格式支持 JPEG、PNG 和 WebP，声明 output 时 quality 为 0–100 整数。未声明时宿主用 PNG 保留透明度。多个处理器依次生效时，由宿主按最后一个有效处理器的输出格式打包结果。`application.sourceHints[].sourceIds` 可记录旧缓存探测来源，不限制新图片或转载图片的匹配。

插件定义可选 `settings` 数组，供宿主以共享表单呈现配置。每项使用 `id`、`label` 和 `kind`，`defaultValue` 可选；可选 `description`，数字范围必须提供有限的 `minValue`、`maxValue`、`step`（最大值大于最小值，step 大于 0）；select 和 multiselect 必须提供 `options`（每项含 id 和 label）。支持的 kind 为 `string`、`boolean`、`select`、`multiselect`、`range`；具体表单入口和保存行为由宿主提供。当前包子插件没有额外配置字段。

发布工作流 `.github/workflows/image-plugins.yml` 手动启动，使用固定的非 latest `image-plugins-v1` Release；先上传内容寻址的 WASM 和包清单，再更新索引。旧 `image-rules-v1` 目录和资产独立保留。

默认插件订阅：`https://github.com/honjow/koma-sources/releases/download/image-plugins-v1/index.json`。反色开发示例独立订阅：`https://github.com/honjow/koma-sources/releases/download/image-plugins-v1/example-index.json`；其 `amount` 为 0–100，默认 100，0 保留原图，100 完全反色，中间值按比例混合，alpha 不变。
