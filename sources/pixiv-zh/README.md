# pixiv 0.1.14

支持章节阅读序能力的宿主使用系列已校验的 content_order 续读；单幅作品和旧宿主行为保持原样。

Pixiv anonymous All ages manga search; separate artwork and complete public series catalogues. Service zh is UI language, not a body-language claim; usage unknown. No user/login or home.
Raw page cursor carries exact query, cumulative unique artwork IDs and emitted series IDs. Complete public catalogue before success; no fabricated book total or chapter numbers. Chapter/volume and content language null. Original images remain the default with the existing Pixiv Referer. Explicit quality preferences use only the same API page's real URL variants.
A series may expose fewer visible works than its total. Only uniquely identified visible works bound to the same series and owner are returned, after the public content endpoint returns an empty page and the top-level display_series_content_count matches. Pagination retains the endpoint's cumulative entry-offset last_order; content_order remains the real per-work reading order and may contain gaps. Missing works and their reasons are never inferred. Embedded display counts may use the reported total or public visible count; they do not control catalogue completeness. Unknown structures, contradictory counts, duplicates and capacity overflow fail the whole operation.
Metadata must be complete UTF8/JSON below1MiB; bounded4096 members/pages. Overflow, within-page duplicates, drift, access and cancellation fail the whole operation. Legal cross-page artwork overlap is deduplicated after validating its complete card; a nonempty page with no new artwork fails explicitly. Missing/null optional artwork comments omit description; malformed nonstring comments remain rejected.

The image-quality contract follows fixed Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 pref_image_quality select: thumb_mini / small / regular / original, default original. A missing chosen variant falls forward only through the remaining larger variants, matching upstream; no filename or URL is guessed. Unknown values retain original. Every page still requires its valid original identity, exact count and original-URL deduplication before any output; All ages, private/login rejection and i.pximg.net URL validation stay unchanged. Page IDs/index/order and default URLs are unchanged; no credentials, cookies, mirror or new host grants. Official pixiv Help Center uses lowercase pixiv; display names follow that spelling. The existing official icon, homepage, identity and runtime behavior are unchanged.

Search filters support the reference Posted before and Posted after fields when the host supports scoped filters. Nonempty values are sent as ecd and scd; the cursor retains the dates so a changed filter cannot reuse an earlier result page. Empty values preserve the existing request and cursor. The source continues to accept only anonymous All ages manga; the remaining reference filters are not implemented.


## 匿名热门与最新候选

固定参考 949dea252eb8e867b7b40834a26e13088764ff55：热门调用漫画日榜再批量取作品详情；最新调用漫画更新接口并略过广告。参考按作品和系列身份去重、每次缓冲50项，非空结果后允许继续直到空结果。Koma get_listings/get_manga_list 的 popular/latest 分别消费这些接口；opaque cursor 独立保存源页、页内位置、未消费的公开页快照与已见作品和系列，拒绝跨栏目复用；快照避免数据刷新后重取同页造成跳项。保留本源原有匿名、全年龄、非私有、非仅登录的访问边界；不可访问项不被输出，结构损坏或榜单和详情身份不一致不冒部分成功。

本地候选已用匿名中文接口真实榜单及详情四页、最新五页回放前50及续页，真实数据后的空页是合成终态；有界容量、用户路径及真实末页仍需独立验证。候选新增匿名标签、部分/完整匹配和日期排序；空词按参考以标签或「漫画」为词请求，含关键词时在本站结果中匹配标签。官网真实搜索、两种标签模式、游标变化及空批续页已回放。含关键词标签筛选按参考先取36项估算窗口、最多1000项，公开页快照保存未消费余项；真实三页回放部分匹配窗口66项/结果50项、完整匹配窗口69项/结果49项，零命中上限与第二页失败恢复为受控合成验证，真实终点及普通 Native 尚未验。用户查找需登录，其他选项包含超出当前匿名全年龄范围的能力，不能据新增栏目声称全面功能对齐。
