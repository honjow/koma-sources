# Dragon Ball Multiverse 中文 0.1.2

沿官网中文入口和原稳定身份消费作品、章节及页链接。章节列表不提供可证的每章翻译可用性，因此省略固定 language 字段表示未知，不能从 cn wrapper 或缩略图参数推断正文语言。正文含气泡时，支持 `source_page_text_overlay_v1` 的宿主按页面结构合成文字；旧宿主未声明能力或气泡坐标无法完整解析时，整章返回 `unsupported_chapter_overlay`，避免无字原图。页 ID 与原图片 URL 保持不变。

原首章22页计划（含cover0和8/20双页）与116章ID/章号/顺序保持；不补9/21。116首个真实页面含英文HTML气泡；是否为中文以实际页面为准，不推断翻译。其它章正文语言与全源可用性未验。

生产入口为根build.sh --source dbmultiverse-zh；使用当前SDK及既有标准flags，必需icon.png保原bytes，metadata.json仅已知localizedNames进入manifest。不引入旧owner写死010的临时package_candidate.py。标准编译WASM不保证与旧owner直接cargo全bytes相同；成员/完整生产输出须单独核对。

历史010/011 owner及证据：/Users/honjow/git/Koma-sources-dbmultiverse-zh-20261011；/private/tmp/koma-dbm011-chapter-language-unknown-20261017。011仅Host语言候选已Root亲审，canonical集成/Native须另审。原22页实体只保有限目检，不冒全章或全部作品验收。

## 中文栏目显示候选

固定参考只有单页作品列表，未提供最新、筛选或有效搜索。候选将栏目名「Popular」及源介绍本地化为中文，作品标题仍使用官网原文；2026-11-06 官网中文列表 HTTP 200，实际新旧 WASM 对同一原体均输出5项且无下一页，数据逐字段一致。正文和气泡呈现未在本次重新验收。

## 2026-11-10 显示名精简

按用户要求去掉单源语言后缀，当前显示名为 `DB Multiverse`；源 ID、内容、图标和阅读行为不变。历史名称仅保留在旧版本记录中。
