# MANGA Plus Creators 英文 0.1.1

独立 `com.mangapluscreators.en.koma`；语言 en。正式订阅 SAFE1；单作品分级未知，实际源用量未知。只有提供方有限热门列表、关键词搜索、详情、完整分页目录、有序页描述符和 imageRequest。无最新/筛选/首页/登录/重试/JS/转换。搜索返回实际提供方结果而不是精确标题过滤。

保留的匿名真实元数据：30 张热门卡片；30 张 Sense Life 查询卡片包括具名作品第六话；17 个唯一章节；自有 Public/EN 第一话 72 个唯一连续 pc 描述符。未抓取美术/封面二进制，无中文内容声称，无 native/正文/解码器/下载/持久化验收。其他标题和真实多页目录保持 OPEN。未知/畸形/含糊/非自有/分页/重复/容量失败整个操作，无部分目录成功。

稳定作品 ID `mpc:en:<实际标题 contentsID>`；章节 `mpcch:en:<标题>:<集数>`；页 `mpcpage:en:<标题>:<集数>:<原0基索引>`。图像使用实际原源返回描述符带独立规范 mangaUrl 和严格宿主/作品/集数路径校验；imageRequest 加头且零元数据 HTTP。无持久缓存/临时 CDN 变通。完整提供方成员资格和有序 URL 校验保留在 get_pages。伪造的相互一致的调用方描述符/上下文不由 imageRequest 独立提供方证明。叶图像请求移除 Origin/使用集数 Referer。封面计划校验精确详情封面/当前稳定作品所有者，使用标题 Referer。

SDK 保留字节相同；源本地内存/UTF8/严格 JSON/描述符 RAII 门禁。Native 1MiB 体和 1000 项/页容量；大型或不受支持的未来源模板明确失败。发布的热门/搜索在主站有限，意外 next 标记失败而不是声明终端。完整目录只跟随显式同标题 next，从不从计数猜 next。观察到的仅查询 `?page=N` 只接受为已校验作品的精确下一页；隐藏的 `javascript:void(0);` 箭头只在同一分页部件把当前页标记为活动时结束目录。未知、冲突或跨作品链接仍失败。单个真实摘要段落可超过 8 KiB；只有摘要文本就地规范化到既有 32 KiB 描述预算内，其他选择器限制和溢出失败保持不变。

历史官方 SVG 渲染保留品牌/宽高比/alpha；它被下面 0.1.3 的图标交付取代。minAppVersion 0.1.1 保守对齐当前宿主路由/描述符合同。宿主回放只证明从保留字节产生的实际生产 WASM 输出，不是当前在线验收。

R2 只为 App registry 选择的 get_image_request 修正实际图像封套 sourceId 空兼容性；非空外部 sourceId 仍被拒绝。缺失 mangaUrl 失败关闭。Native 验收保持 OPEN。

0.1.1 仅源候选接受两种观察到的集数图像路径形态：直接 JPG 文件名（#003）和原正数数字目录/JPG 文件名（#001）。宿主、集数、文件名、单 `h` 签名和独立标题路由检查保留。生产 0.1.0 在 HTTP 前拒绝了全部 35 个真实 #003 图像计划；那个失败被保留。不声称设备或修复路径验收。

## 规范 0.1.2 集成

更早章节记录验收前的调查状态。既有 0.1.1 的 Sense Life 003 P1/P2/P3 普通阅读、35/35 下载和 P3 冷恢复已在 197 上按原有限 native 回执中的限制验收。当前集成保留那个完整提供方，包括两种观察到的图像路径形态、源本地 ABI 门禁和精确元数据。只有版本变到 0.1.2；提供方 2xx 门禁后使用当前 SDK 状态解码器。官方 SVG 原件和 1024 渲染保留；App 呈现保持。当前构建/回放和普通 237 更新必须单独记录。

## 0.1.3 浏览器身份

使用官方域默认 favicon 的蓝盘/白 C 身份；原 16/32px ICO 和全词标 SVG 保留。头部声明/单独 logo 使用批准未发现。初始无损 32px PNG 在 237 高密度管理/Browse 清晰度上失败并通过普通导入回滚到 012。其实际失败和恢复证据保留在 Koma 既有审计分区的 `category-membership/mpc013-icon-awake237-001`、`mpc013-rollback012237-001` 和 `mpc013-icon-restored237-001`。

替换的 512px 候选使用 Root 审查过的理想圆/弧/圆头 2D 几何和原蓝/白/深色身份，以 `browser-identity-reconstruction.svg` 作为可维护输入。它是近似重建，不是官方高分辨率原件；无像素描摹/新底板/阴影/遮罩或全词标拼接。SourceInfo/导出信息/Cargo/lock 步进 012→013；业务和元数据不变。离线构建/实际 WASM 信息 013 通过。普通 237 012→013 导入完成。Root 审查了完整管理和 Browse 图像：完整 C/盘/内外边缘清晰，无初始 32px 模糊、生成斑点或额外底板/阴影。58 个 sourceId/启用对保持相同顺序；原 Browse miaoqu 组、10000ms/AWAKE 和焦点 koma0/window208 恢复。只移除/读回缺失本次运行的传输包；租约 e015ad5d 释放。证据：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/mpc013-geometric-awake237-001/{manager-mpc013-geometric.png,browse-mpc013-geometric-full.png,browse-original-group-restored.png}` 和 `mpc013-geometric-restored237-001/run-metadata.json`。只验收这些图标消费者；无源内容/阅读声称。首个几何 Browse 截图落在顶部模糊区域，不是验收的全行图像。

## 2026-11-10 显示名精简

按用户要求去掉单源语言后缀，当前显示名为 `MANGA Plus Creators`；源 ID、内容、图标和阅读行为不变。历史名称仅保留在旧版本记录中。
