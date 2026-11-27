# Twi4（星海社四格）

星海社「最前線」的日文四格漫画栏目，刊载《トモちゃんは女の子！》等原创四格连载。主页为 <https://sai-zen-sen.jp/comics/twi4/>。源 ID 为 `jp.saizensen.twi4.ja.koma`，最低 Koma 版本声明为 1.4.1。

## 上游与 Koma 调用对照

固定 Keiyoushi `extensions-source` 提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69`，目录 `src/ja/twi4/`，参考 `Twi4.kt` 与 Gradle 入口。宿主消费位置包括 BrowseViewModel、SourceMangaDetailRequest、SourceChapterListResolver 和 SourceChapterPageHydrator。

| 上游行为 | Koma 消费与实现 |
|---|---|
| 热门合并近期更新和全量目录；排除座谈会及不属于作品的条目，站内无独立搜索 API | `get_listings` 提供一个目录；`get_manga_list` 用同一组选择器并按 slug 去重保留首见顺序。 `search` 在该目录按 Unicode 不区分大小写的标题子串过滤 |
| 详情从 `title` 和 `#introduction` 得到标题、封面、简介与人员；完结标记仅在首页 | `get_manga` 读取作品与首页。原作优先填入作者，漫画优先填入画师，通用作者署名可补足两者；保留作品原站链接 |
| `#backnumbers` 提供章链接，按话号倒序，找不到话号时保留原标题 | `get_chapters` 保存章路径作为身份，去重后按解析话号倒序，同号及无号链接保持站点顺序，通过 typed SDK 生成阅读位置 |
| `article.comic` 下的一张图是整话 | `get_pages` 读取 `article.comic div > div > p > img`，解析为绝对图片 URL，返回一个页描述符 |

Mihon source ID 为 `493846804748918744`。作品迁移以 `/comics/twi4/<slug>/` 为准，章迁移以同一个作品目录内的原站路径为准。日期沿上游留空。当前支持目录内标准作品和上述 HTML 阅读模板。联网和真机需重点覆盖连载与完结作品、人员署名、长目录、单图解码与连读，并检查书源列表图标。

## 本地验证

从最终包解出 WASM 回放了实站目录与标题搜索。另用构造响应核对完结状态回填、作者与画师、重复章折叠和单图页表。
