# 动漫屋 / DM5 0.1.20

源 ID `com.dm5.koma`，语言 `zh`，NSFW 标记保持。普通网站及生产请求主站为 `https://www.dm5.cn`；中文显示名为动漫屋，英文身份仍为 DM5。图标身份与原字节见 [ASSET_SOURCES.md](ASSET_SOURCES.md)，不因设置接入更换素材或 App 样式。

支持搜索、热门/最新完整分页、详情、目录、原图、图片请求描述及首页各 12 项预览。首页复用真实列表请求与原卡片解析，保留完整列表目的地；请求或完整性失败整次返回错误，不伪装空成功。

章节身份继续使用原 `manhua` 标识、网站 CID 和原 pageCount；身份/章卷/续读资格仍由既有 legacy 标题字段派生，完整展示标题独立处理。保留这一边界是为了不破坏已有书架、下载和阅读位置；不能因设置、标题展示或重构改写身份，也不从源语言推断章节正文语言。卡片封面保原父容器关联，正文签名、图片顺序及失败传播保原链路。

## 设置合同

- `sortChapter`：默认 `false`，原网站序列不变。仅用户显式开启时按网站章节 URL 数字由新到旧排列，同号保原相对顺序；ID、pageCount、标题与序列元数据不变。
- `chapterComments`：默认 `false`。仅请求顶层 `hostFeatures` 含 `text_page_image_v1` 时显示并消费，旧 App 即使留有 `true` 也只接收原 URL 页。

吐槽开启后，从原 viewer 保存 `DM5_PAGEPCOUNT` 和 `DM5_TIEBATOPICID`，原图完整取得后按网站 `pagerdata.ashx` 的 pageindex/pagesize/tid/cid/t=9 合同获取评论。每个响应数组转成 `Poster: PostContent` 段落；空数组使用「没有吐槽」。追加的描述为 `image.kind=text`、`image.text.template=dm5-chapter-comments`，保原图身份和排列。

JSON Unicode 与 HTML 实体由源侧解码，保正文标签字面量和 CR；复用既有 HTML5 Host 表并关闭 descriptor，不增加依赖库或新 Host ABI。损坏、缺必要元数据、容量或网络失败整次拒绝。Host 只把普通 Unicode 按固定本地模板绘制成 PNG，不解释站点 HTML、不外发文字。设置对之后加载的章节生效。

Android CookieManager 的清 Cookie 动作不是普通可保存设置，本源没有伪造该开关；不增加账户、Cookie、UA 或网络权限。
