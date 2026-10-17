# Dragon Ball Multiverse 中文 0.1.1

沿已审010官网中文入口和原稳定身份消费当前作品/章节/页链接；没有新增源、overlay渲染或样式。章节列表不提供可证的每章翻译可用性，011省略固定language字段表示未知，不能从cn wrapper/缩略图参数推断中文正文。原source级zh、中文名称及正文overlay整次unsupported_chapter_overlay guard保持。

原首章22页计划（含cover0和8/20双页）与116章ID/章号/顺序保持；不补9/21。116首个真实页面含英文HTML气泡且仍拒绝整章，不称其中文可读。其它章正文语言与全源可用性未验。

生产入口为根build.sh --source dbmultiverse-zh；使用当前SDK及既有标准flags，必需icon.png保原bytes，metadata.json仅已知localizedNames进入manifest。不引入旧owner写死010的临时package_candidate.py。标准编译WASM不保证与旧owner直接cargo全bytes相同；成员/完整生产输出须单独核对。

历史010/011 owner及证据：/Users/honjow/git/Koma-sources-dbmultiverse-zh-20261011；/private/tmp/koma-dbm011-chapter-language-unknown-20261017。011仅Host语言候选已Root亲审，canonical集成/Native须另审。原22页实体只保有限目检，不冒全章或全部作品验收。
