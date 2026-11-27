# Internet Archive

互联网档案馆的公开可读扫描件，覆盖公版绘本、历史杂志、老漫画和艺术画册。图标记录见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

原站接口参考 [Item Metadata API](https://archive.org/developers/md-read.html) 与 [BookReader](https://github.com/internetarchive/bookreader)。

| 准入层 | Koma 操作和源内入口 |
|---|---|
| 发现 | `search` 和 `get_manga_list` 调用 `advancedsearch.php`。保留栏目范围、搜索词及 `args.page.cursor`。候选可能需要在打开时拒绝。公版栏目以公版元数据或已核验样本为入口。 |
| 元数据 | `get_manga`、`get_chapters`、`get_pages` 共同读取 `metadata/{identifier}`。校验条目身份，拦截限制及加密标记，仅接受非私有 processed JP2/JPEG ZIP。纯 PDF 条目不进入读图。 |
| 完整可读卷 | `reader::Book::open` 使用 ZIP 对应的实际 `subPrefix` 和条目数据节点读取 `BookReaderJSIA.php`。必需的借阅和图像保护标志必须明确为 false。校验子书、ZIP、所有展开页以及 `viewable`，任一受限页拒绝整卷。 |
| 页图 | 采用已与条目、节点和扫描 ZIP 验证归属的 HTTPS `BookReaderImages.php` URI。Koma 直接加载图片。页 ID 是 item、子书和 leaf，页序按 BookReader spread 展开。 |

每个可读扫描卷是独立章节。Koma 的页表入口一次消费完整卷，源对 JSIA 和原始 WASM JSON 分别设预算，超限时返回错误并保留完整性。公开可读条件和公版分类各自判断。
