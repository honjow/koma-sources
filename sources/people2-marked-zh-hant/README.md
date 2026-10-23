# 人2 漫畫標記圖文 0.1.1

來源為人2作者部落格的「◎人2 x 生活」。只收錄原目錄 preview img 的 alt 或 title 中，以 Unicode 空白分隔、獨立 token 為 `#漫畫` 的作者明確標記圖文；不是整個生活分類，也不把未標記文章判為非漫畫。不按標題、圖數或硬編碼 post 名單選篇。評級、使用率及額外對外發布授權未知；本候選未發布。

- ID：`com.people2.pixnet.marked.zh-hant.koma`，語言 `zh-Hant`，版本 `0.1.1`。
- 原分類：`https://people2.pixnet.net/blog/categories/8002293952`。每次請求只取原一頁；cursor 為原 next 完整 URI，核作者、分類 canonical、`PIXNET.post_id=0`、當前頁與原前後頁。
- 已留第1頁12篇中4篇有標記；第2頁12篇0標記仍有真 next3，回傳 `items:[]`、`hasNextPage:true` 與原 cursor。沒有掃後頁、補篇、假終態或宣稱完整總量。App 在空頁上的普通接續仍未 Native 驗收。
- 字串 postID 不轉浮點或 wasm32 整數。MID/CID/PID 自含原 post 字串；實際 post URI由原平台已證固定 `/blog/posts/` 路徑及該ID組成。目錄逐篇核原 data-link/category/ID，重複ID拒絕，密碼保護項跳過且不解鎖。被選作品再核唯一原 post 標題、原 preview URI。
- 每篇是獨立完整閱讀單元；章號與卷號 null、readingGroup 為該 MID、continuationEligible=false。不從題名、日期或生肖推章號。
- 詳情/目錄/頁面/圖片計畫分別重新讀原 post，核 canonical/og/BlogPosting/作者/生活分類與明確漫畫文字。主 Body 的原 Flickr URI及順序不改；主文附記和連結文字保留在簡介。原5張首篇含單幅、漫畫情景及第5圖解，全部保留，不拆假章或裁圖。
- 「相關連結」的社交/作者博客按鈕按原 DOM 角色、85px尺寸及聲明src/href配對排除，不把固定前 N 圖當正文規則。原主文允許已核 p/div.paragraph-element 直接嵌圖或 photo anchor 兩形；仍逐項數量對齊，不支援混合未知層級時顯式錯誤。未知圖/媒體形狀不能靜默捨棄。不得改 `_z` 為猜測的原圖後綴或代換網域。
- 圖片計畫需 MID 封面或 PID 本文角色，獨立 owner/mangaUrl（若提供）須吻合。MID 只接受當前原 Body 首圖，PID 按當前 post 的完整原 URI/ordinal 嚴格復核；無跨呼叫快取或狀態依賴。各計畫1個 metadata GET，之後圖片下載由App按原URI與標準 UA/原 post Referer執行。
- 能力只有 listings/mangaList/detail/chapters/pages/imageRequest。無已核搜尋、home、filter、setting、credential合同，均不宣稱支援。
- 图标更新（0.1.1）：原作者正文相关链接明确绑定 https://www.facebook.com/people2 ，该账号准确 og:image 的完整720×720 JPEG原头像现用作源标识。保留原作者署名、角色与原背景，未裁切、重绘或新增遮罩、底板、阴影。来源读取于2026-10-20；设备呈现另验，不冒额外商业分发许可。

原始材料分別保存在 `/private/tmp/koma-people2-*-20261015` 的只讀 binding。實作使用未改現行 production C++ HtmlHost/Lexbor 和 Wasmtime ABI對保留原 HTTP 回放；不是 NetworkKit/Native 通過。僅首篇5個 JPEG 實體已解碼/目檢，其餘原圖實體未取；目前主頁和生活頁12篇均不是純漫畫保證。原 spotlight 未閉合 Instagram 錨在 Lexbor 首 h2重構空連結的首失敗和 scope probe均保留，真正標題限定原作者 post 域/path，不改原HTML/Host。

成本：get_listings 0 GET；每個 browse頁1 GET；detail/chapters/pages各1 GET；cover/body plan各1 GET；非法 host/ID/owner/cursor前置拒絕0 GET，合法語法但錯 ordinal需1 GET確認。構建期間除 Root明確批准的另外3篇原HTML各1次資格GET，沒有新增網路或圖片請求。

靜態能力邊界：原聖誕文另有 People2 movie script 嵌入，萬聖節文另有 YouTube iframe。只允許原已核 `https://ext.pixnet.tv/user/People2/embed/movie/<正整數>.js` 和 `https://www.youtube.com/embed/<11字元ID>` 作明確原地址文字引用；含這些嵌入的詳情開頭先說明僅提供靜態圖與可見附文，影音未承載。無執行、抓取、watch URL合成或新網路域；inline/未知 script/iframe、video/object/embed 仍拒絕。本源不聲稱完整消費原 post 所有媒介。另三篇靜態圖陣列為2/2/3，所有附文保留，這7張實體未GET/目檢。


## 237 普通空中间页分页观察（2026-10-23）

普通picker首次导入既有正式0.1.1，登记63→64只新增本源，原63条完整记录和相对顺序保持。现装App 0.1.2-pre/code3普通Browse首屏HTTP200，实际显示原4本及封面；一次正常滚到末尾后连续完成4次后继get_manga_list，均HTTP200/completed，最终仍4本并显示“没有更多了”。当前消费者没有在原留存的零匹配中间页处提前终止；源码严格校验next/back页号并仅按实际next输出hasMore，App按该标记保留后继。日志未导出逐页原JSON封套，不能把resultChars335/279当逐字段响应实读；本次仅接受具名普通分页消费，不宣称详情、阅读、下载或整源通过。

原件沿Koma既有237 category-membership：`people2011-empty237-{gate,imported,firstpage,nextpage,final-restored}-001`，nextpage内为连续日志及完整原图。最终普通Browse喵趣组已恢复，reader整份内容与进入前相同，原10000ms/AWAKE、本人传包及两scratch不存在均读回，租约释放、哈希关闭。仅保留正常新装People2；未改源/App代码、版本、图标、偏好、读态或其他源。
