# WEBTOON（繁体中文）0.1.6

独立源 ID `com.webtoons.zh-hant.koma`、内容语言 `zh-Hant`；英文 WEBTOON 与简体咚漫是另外的源，不别名迁移或共用历史。普通网站为 `https://www.webtoons.com`，图标与身份资源保持，素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。上游 SAFE 是源级标记，不是逐作品分类或使用验收。

公开列表由 trending/popular/originals/canvas 四个有限榜单组成，热门列表 cursor 1..4 对应四桶。搜索只支持可证明结束的首个 All 页面；上游若显示下一页则明确不支持，不伪造终止游标。详情使用原桌面页，目录使用手机 JSON，普通 viewer 使用 `#_imageList` 原图。

Manga 身份保语言、类型、真实 title_no 及 list 路径；章节身份保真实 viewerLink/query，页面身份保 title/episode/index。目录必须有明确终止 cursor 且不超过 2048 项；JSON、UTF8、URL、容量、未知/受门控结构明确拒绝，不返回截断成功。保原目录反转与原相对序列，不从 index 捏造默认章号。

繁中 Canvas API 的真实 viewer 可为同官网 `/en/canvas/.../viewer`，但必须带唯一且精确的 `readingLanguageCode=zh-hant`；此形式保网站原 ID/query，不改写成中文路径。缺语言、其它语言或重复参数不放行；既有 `/zh-hant/` 作品保持原合同。

## 设置合同

- `useMaxQuality`：默认 `false`，原 URL 不变。仅显式开启时移除完整的 `type=q90` 查询对，保其它参数、转义、ID 与顺序。
- `useSequentialNumbering`：默认 `false`，保中文标题明确数字/未知 null。开启时按原 episodeList 的 index+1 对应反转后的 `chapterNumber=count-outputIndex`，不改章名、ID、index 或返回顺序。
- `showAuthorsNotes`：默认 `false`，仅顶层 `hostFeatures` 含 `text_page_image_v1` 时显示并消费；旧 App 即使留有 true 也只接收原 URL 页。

备注从同次 viewer 的 `div.creator_note p.author_text` 和 `div.creator_note .author_name span` 取真实文字，非空才在原图末尾追加 `image.kind=text`、template=`webtoon-author-notes`、heading=`Author's Notes from {creator}`、paragraphs=[note]。DOM 已完成实体解码；Host 只把普通 Unicode 用固定模板本地绘为 PNG，不外发、不当作 URL。无备注不造空页，原图身份、顺序保持；设置对之后加载的章节生效。

不提供 MotionToon、登录/付费/账户、年龄或 GDPR Cookie 预同意、自动重试、挑战或镜像切换。图片传输仅把原 URI 中字面 ASCII 空格序列化为 %20，保其它 UTF8/escape；原公开 Firefox121/Referer/Origin 请求边界保持。
