# LINE WEBTOON（繁体中文）0.1.9

独立源 ID `com.webtoons.zh-hant.koma`、内容语言 `zh-Hant`；英文 WEBTOON 与简体咚漫是另外的源，不别名迁移或共用历史。普通网站为 `https://www.webtoons.com`，图标与身份资源保持，素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。上游 SAFE 是源级标记，不是逐作品分类或使用验收。

公开列表由 trending/popular/originals/canvas 四个有限榜单组成，热门列表 cursor 1..4 对应四桶。搜索保持原 All 首屏的结果与顺序，按其真实“查看全部”入口依次续取 Originals 和 Canvas 的原分页；仅依据原当前页和下一页链接推进，查询、分类或页码不匹配时失败。两类首页与 All 重叠时，保留完全相同的原作品 MID，由宿主按 sourceId + MID 合并去重，空批按已有宿主续取；不猜固定偏移。详情使用原桌面页，目录使用手机 JSON，普通 viewer 使用 `#_imageList` 原图。

Manga 身份保语言、类型、真实 title_no 及 list 路径；章节身份保真实 viewerLink/query，页面身份保 title/episode/index。目录必须有明确终止 cursor 且不超过 2048 项；JSON、UTF8、URL、容量、未知/受门控结构明确拒绝，不返回截断成功。保原目录反转与原相对序列，不从 index 捏造默认章号。

繁中 Canvas API 的真实 viewer 可为同官网 `/en/canvas/.../viewer`，但必须带唯一且精确的 `readingLanguageCode=zh-hant`；此形式保网站原 ID/query，不改写成中文路径。缺语言、其它语言或重复参数不放行；既有 `/zh-hant/` 作品保持原合同。

## 设置合同

- `useMaxQuality`：默认 `false`，原 URL 不变。仅显式开启时移除完整的 `type=q90` 查询对，保其它参数、转义、ID 与顺序。
- `useSequentialNumbering`：默认 `false`，保中文标题明确数字/未知 null。开启时按原 episodeList 的 index+1 对应反转后的 `chapterNumber=count-outputIndex`，不改章名、ID、index 或返回顺序。
- `showAuthorsNotes`：默认 `false`，仅顶层 `hostFeatures` 含 `text_page_image_v1` 时显示并消费；旧 App 即使留有 true 也只接收原 URL 页。

备注从同次 viewer 的 `div.creator_note p.author_text` 和 `div.creator_note .author_name span` 取真实文字，非空才在原图末尾追加 `image.kind=text`、template=`webtoon-author-notes`、heading=`Author's Notes from {creator}`、paragraphs=[note]。DOM 已完成实体解码；Host 只把普通 Unicode 用固定模板本地绘为 PNG，不外发、不当作 URL。无备注不造空页，原图身份、顺序保持；设置对之后加载的章节生效。

不提供 MotionToon、登录/付费/账户、年龄或 GDPR Cookie 预同意、自动重试、挑战或镜像切换。图片传输仅把原 URI 中字面 ASCII 空格序列化为 %20，保其它 UTF8/escape；原公开 Firefox121/Referer/Origin 请求边界保持。

## 官方中文站品牌

繁中官网 [原創頁](https://www.webtoons.com/zh-hant/originals) 与 [官方公告](https://www.webtoons.com/zh-hant/notice/list?page=1) 使用 `LINE WEBTOON`。本源使用该完整品牌并注明繁体中文；英文源独立使用自身官网 WEBTOON 品牌。

## 搜索续页（0.1.9）

原官网 All 查询“愛”返回22张卡（正式12、Canvas 10），两类均有“查看全部”；0.1.8误把该预览当作终点。0.1.9 保留22项原 MID、内容和顺序，下一 cursor 表示原分类和站点页；Originals 完结后进入 Canvas。支持站点数字分页与每十页块的原 next 按钮，校验原类别、查询和下一页，非法 cursor 在网络前拒绝；取消不请求站点，HTTP失败不生成成功游标。

同一原 HTML、生产 WASM 与原 C++ Lexbor Host 回放：首屏22与0.1.8 items逐字段一致，Originals第1/2页各30、Canvas第1页28，Canvas第10页“愛”11项及终点，另一个 `a` 查询第10页原按钮指向第11页；已核取消、非法 cursor 和503失败。桌面 runner 实时遍历“愛”经宿主相同 sourceId+MID 规则合并得411个不同可见作品、到原站终点。官网标示 Originals 145、Canvas 283，但本次实际可见 Canvas 卡266，不把站点标示总数当作已证明作品数。热门第一页与0.1.8在同一原 HTML 上 data逐字段一致。设备 Native、普通加载更多和远程发布另验。
