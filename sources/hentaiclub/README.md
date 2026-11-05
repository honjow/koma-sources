# 绅士会所

Independent Rust/WASM port of complete HentaiClub.kt/Filters.kt at Keiyoushi
de743aed846da53e00c45ab26ee0e77f7bb4355b (extension v1, lib1.6, Apache-2.0).
Name 绅士会所, zh language and original host www.hentaiclub.net are retained.
The reference is MIXED: per-work rating stays unknown; no individual comic is
classified from the website name. There is no account, preference, latest list,
related-work or URL-import implementation in that reference.

Popular/search/tag/category/details/one fixed chapter/all original data-src
positions and original image headers are implemented. Category pagination
deliberately repeats the original category URL; unsupported latest/URL import
remain unsupported. The first content image is the reference's actual cover.
Opaque CDN URLs are not rejected by guessed ownership. PageID and supplied
manga/chapter/URL owners are strictly checked before image request output.

Anonymous network operations require source_rate_limit_hentaiclub_v1 before
HTTP: exact main hostname www.hentaiclub.net shares 2/1000ms, unrelated CDNs do
not. Single fixed-chapter metadata and image-request descriptions need no HTTP.
Jsoup's unquoted substring attribute selector is quoted for the Host scraper;
its meaning is unchanged. Transport/refusal/malformed/known-empty cases remain
separate, with no inferred takedown or payment.

## 参考对照与验收

- `get_listings` / `get_manga_list` 对应 Mihon 热门，无独立首页及最新列表。
  `search` 先用非空关键词，再按标签、分类、热门的顺序选择。关键词和标签使用
  页码游标；分类沿原参考对每个游标请求同一分类地址。是否有下一页沿用 24 项阈值。
- `get_manga` 对应详情的标题、首张正文封面、首标签作者、标签顺序、浏览量
  与完结状态。`get_chapters` 对应固定的「章节 1」，独立调用无需 HTTP。
  Mihon 的 `ONLY_FETCH_ONCE` 是后台调度提示，Koma ABI 没有对应字段。
- `get_pages` 对应原页表的全部位置，保留重复图片。`get_image_request` 按原
  User-Agent/Referer 返回请求描述；Reader 用章节页 ID，封面用同一作品 ID。
  两者都校验附带的归属，无需请求正文页。CDN 路径本身不包含作品 ID。
- 原站网页实测 7 个列表共 144 个卡片位置、107 部不同作品，60 部逐一核对
  详情、封面、章节及 2696 个原图位置（含 11 个重复位置）。10 部不同 R-15
  作品各一张正文真实下载并解码。另一次 HTTPS 传输失败发生在收到 HTTP 响应前。
  Koma 1.2.0 提供该源所需的主站限流能力。
