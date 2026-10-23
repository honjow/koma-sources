## 0.1.11：未知章节编号保持未知

`get_chapters` 不再把 API 的空编号、缺失编号或 `null` 填成 `"0"`，输出为 `chapterNumber: null`；真实 `"0"`、普通编号和小数编号保持。非字符串编号仍不作为有效编号，不猜测标题。章节 ID、标题、目录原序、请求参数和请求次数均保持；`publishedAt` / `updatedAt` 仍按原合同输出 `null`，本次不引入时间语义变更。

本次用原编译 WASM 与新编译 WASM 回放同一份明确标注的合成 API 输入（并非 Super Lovers 的真实 feed）：`"1"`、`"0"`、`null`、字段缺失、空串、空白串、`"1.5"`、数值型字段共八项；只允许未知编号变成 `null`，其余响应深比较一致，HTTP 请求内容及单次调用次数一致。生产入口 `bash build.sh --source mangadex` 生成 `mangadex-0.1.11.koma`。产物和回放明细：`/tmp/koma-mangadex-metadata-20261023/`。尚待 App 普通源更新、现有详情刷新及书架首次阅读验收，不能将本项称为实机首章已通过。

# MangaDex 0.1.10 current standard source

完整继承已审018设置/精确章节恢复、019分页/JSON结构/Home职责与0110完整external-only终态。源身份org.mangadex.koma、章节ch:身份、原图标/ASSET_SOURCES不变；不跟随MangaUP等外部正文。current canonical既有exact-chapter分支在此完整后继保留，读取直接data/attributes/relationships，不覆盖其它源WIP或SDK。

0110跨页cursor mdxc1携带known/hosted/external/other事实，仅完整遍历终页全external且无hosted/other才unsupported；真实total0正常空，mixed保留本站章。旧数字非首页历史未知不能冒断all-external。原页数据省流/原settings/filter与全部正文消费者保持原后继。

可信owner及原Root Host证据：/private/tmp/koma-sono-zero-reason-design-20261016/{candidate,delivery-binding.json,ROOT_REVIEW.json,production-replay}。真实原体只有Sono首100及LoveBullet21章；后页/终态/混合/错误是明确受控回放。0.1.10已在237旧App8b8151f1具名显示不支持章节（原m39/m41）；最新保存注册记录仍0110，不是本轮实时设备结论。较细220外链说明/全部语言未验。

生产入口 `bash build.sh --source mangadex` 使用当前canonical SDK/必需icon合同，新增manifest.icon元数据但保旧字段；新标准WASM与旧冻结WASM可能不同，不能据旧Native冒本轮新编已验。原crate-local Cargo.lock为历史非workspace构建输入保留；实际workspace Cargo.lock本crate同步0.1.10。本源根Cargo及build registry原已登记，不添加其它源。图标来自既有官方PWA声明，字节保留；App显示裁剪独立保留。无新网络/下载/部署/样式变更。
