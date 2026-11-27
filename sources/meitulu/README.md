# 美图录

按模特、写真机构与主题整理的高清真人写真图库，收录内地、港台及日韩套图。完整图集作为一个章节，源与图集均标记 NSFW。版本 0.1.0，最低 Koma 1.5.5；源自编栏目、筛选、章节标题、说明与错误提供五语。

## 移植前的源码对照

固定检查 Keiyoushi extensions-source `c09b174459c037c62292e4e7a929a0cd59430bc4` 的完整目录树：此版本不存在 Meitulu 适配器，因此没有冒认旧域或其他站点为本源上游。实际对照同版本 [Junmeitu](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/junmeitu)、本仓 [junmeitu](../junmeitu)、[jjcos](../jjcos)、[misskon](../misskon) 与 [photo_gallery.rs](../photo_gallery.rs)；Meitulu 路由和解析按 2026-11-28 的 `www.meitulu.me` 公开 HTML 独立核实。宿主请求沿 SDK → `request.args` → 开发宿主核对，未访问 Koma 仓库。

| 行为 | 参考与当前官网 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | 官网“热门美女”是模特入口，套图为 `/xihuan/` 精选和首页“最新发布”区块，无独立图集热门榜。 | `get_manga_list → browse`；稳定 listing ID `popular` 显示五语“精选图集”；`latest` 仅取最新区块，拒绝混入精选／随机图集。默认两项无更多页，不伪造独立首页。 |
| 分类 | `/xihuan/`、`/guochan/`、`/gangtai/`、`/rihan/`。后三项 `index_2.html` 续页。 | `get_filters → category`，`browse` 消费；`args.page.cursor` 为从 1 起的页码，真实“下一页”决定结束。筛选归档有分页，默认首屏结束。 |
| 搜索 | 实站表单 GET `/search?q=`，现网结果没有下一页。 | `search → browse`；非空关键词优先，不把分类／标签混作组合搜索；继续游标无效。 |
| 标签、模特、机构 | 三者在当前站点统一为 `/t/<code>/` 归档，分页为 `index_N.html`。 | 三个五语文本筛选只允许填写一项，优先于分类；留空关键词才能生效。代码保留原大小写，不猜测机构／模特。 |
| 详情、单章 | `h5.top-title`、`section.mb-3` 的机构、模特、数量及同站原图。 | `detail` 输出写真标记和原站标签；typed `photo_gallery::run_get_chapters` 生成一个完整图集章节。 |
| 全部分页 | `/item/<id>.html` 和 `<id>_N.html`，`.my-pagination` 有末页／当前页，`section.mb-3` 标注总图片数。参考 Junmeitu 的全分页失败语义，但不沿用其不同 URL、选择器与图片 CDN。 | `get_pages` 顺序抓取所有页面；逐页核对标题、当前页、分页同套身份、总页数及总图片数，重复 URL 或缺中间页明确失败，不返回部分成功。最终正文张数必须等于站方标注数。最多 1000 个 HTML 页面、6000 张图片，超预算明确失败。 |
| 正文图片与过滤 | 正文为 `.container-inner-fix-m > img` 的同站 `/p/<id>-<hex>.jpg`；`/static/mlogo.jpg` 是 UI，`/poster/*.jpg` 是列表／推荐缩图。 | 正文容器与同套 ID 两重约束，保持站方原序，静态图和推荐图不可能进入页表。页图附图集页 Referer；`get_image_request` 处理封面和原图并附官网 Referer。 |
| 登录、设置 | 公开 GET；首版不接登录、下载计数或外站广告。 | `settings` 为空、`home` 未实现；普通 HTTP 使用公开 UA，不设置验证页 UA，不读取响应头。 |

完整图集、稳定数字作品 ID、同套图片边界、NSFW 和分页失败不返回半套，是本源兼容边界。不得为减少请求而截断长图集，也不得用解析整个页面所有 img 的方式混入 UI／推荐图。更改需真实站点反例或用户明确允许。公共约定沿 [制作规范](../../docs/WRITING_A_SOURCE.md#移植前的源码对照)，不为本源重写宿主合同。

## 验证与素材

包内最终 `source.wasm` 的列表、搜索、四类归档、标签／模特／机构、五语、详情、单章、全部分页和完整套图逐张下载／解码结果记录于 `dist/wave-11/verification.json`。独立 HTML 对照确认最终 URL 顺序、正文张数与原站完全一致。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本任务禁止触碰 Koma，App 书源列表及真机普通阅读路径未验，主机验证不替代 App 验收。
