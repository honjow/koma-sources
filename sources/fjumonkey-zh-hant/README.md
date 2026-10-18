# 輔大猴・四格漫畫

作者原部落格「[專欄]四格漫畫」：https://fjumonkey.pixnet.net/blog/categories/1708975 。僅這個個人漫畫分類，按實際next逐頁瀏覽，不掃6頁、不把頁數乘每頁數當總量。每post獨立作品、一個完整閱讀單元，原標題不改；chapterNumber/volumeNumber=null，readingGroup=該作品MID，continuationEligible=false。Holly Plus標題中的59等數字不作全源章序。作者自署「輔大猴部落格」，正式作品評級/連載狀態/usage unknown。

source ID `com.fjumonkey.pixnet.zh-hant.koma`，0.1.0，ABI0.2，zh-Hant。能力僅listings/mangaList/detail/chapters/pages/imageRequest；不提供search/home/filter/settings/credentials。short postID保原字符串，最多24位正數字，不經f64/usize；MID=`fjumonkey:zh-hant:post:<post>`、CID=`fjumonkeyread:zh-hant:post:<post>`、PID=`fjumonkeypage:zh-hant:post:<post>:<0-based ordinal>`，只有本地ordinal/頁計數轉整數。

目錄限定主`#article-area > #article-box > div.article[id]`，逐項原ID/data-article-link/h2 URL/標題/個人分類/直接img封面一致，按post去重。兩頁各12原身份已核，合併24；24作品只有14個不同cover URI，原49/48及47至39共11篇共享同一封面，合法保留所有作品。原next完整URI作cursor，驗可見當前頁、back/next連續；canonical/og只鎖原分類基礎URL（page2原站不含query）。末6僅原導航，後4頁/終態未核，不能假終態。

詳情主div.article無id、li.title無data-link，不探不存在屬性；核唯一主article、h2原URL/標題、canonical/og、meta author、PIXNET name/blog_id/post_id/AdultWarning=false。目錄原Person graph與詳情原BlogPosting分開驗；詳情核其author引用、url/@id/mainEntityOfPage/headline/articleSection/isPartOf/inLanguage及原datePublished，無合成graph。comscore/part與歸屬無關不鎖。共用成人模板不代表已啟用門檻，亦不推全站評級。

本次原詳情的`article-content-inner > img[src]`為直接img，沒有p包裝。圖片只收主正文全部img且原URI為作者HTTPS媒體合法形式，拒外域/額外query/子目錄/重複圖/嵌入播放器或未知圖片樹。首《夏日玩水須知》原5JPEG完整保序：400×308標題P1、四幅600×380正文；不刪題字、邊框、版權，不裁四格、不轉碼/縮放。正文附文「⊕ 夏日玩水須知」「更多輔大猴四格漫畫...」、原Tag及原舊href都放詳情description的原文附記/原文連結；只保存文字，舊wretch連結不跟隨，不作現今目錄。原HTML父樹另保；不聲稱Reader圖片頁已排印原HTML文字。JSON-LD image1不是全篇。

**圖片兩個角色刻意不同：**

- MID封面（App實際pageId=MID且獨立mangaUrl與post一致）：只接受精確`http://pic.pimg.tw/fjumonkey/<10digits>-<positive digits>.jpg`或`https://pimg.1px.tw/fjumonkey/<10digits>-<positive digits>.jpg`兩種原聲明scheme/host/作者路徑，無query/任意子目錄/憑據；原樣返回URL、標準Firefox UA與該post Referer，0 metadata GET。前者對應目錄封面，後者可對應原詳情封面；不互相轉換、不稱alias或原體相同。此角色信任先前catalog/detail元數據配對，**不獨立重新驗證post→cover映射**；另一篇同作者合法媒體URI在MID/owner一致時也能被接受。這是明確的封面邊界，不誤報為正文歸屬檢查或安全阻塞。共享封面合法。
- PID正文：post/ordinal/mangaUrl一致，每次1 GET重新核該post主DOM、頁數、完整bodyURI逐字及ordinal，未在原DOM的同作者URI也拒絕。原媒體檔名不帶post，不用basename/HTTP別名推正文身份，不能套MID封面的較寬邊界。無pageId或chapterID作pageId拒絕。

每個正常metadata GET為SDK現有標準UA/原文章或分類Referer。圖片計畫明確標準Firefox UA/原post Referer，沒有登入/憑據/重試或URL重寫。get_listings 0 HTTP、browse每頁1 GET、detail/chapters/pages各1 GET、MID封面0 GET、PID正文計畫各1 GET。實際圖片位元組由App圖片層另外讀。原目錄HTTP cover實體尚未GET；品牌圖一條301/302不推其他封面，NetworkKit的HTTP/redirect與圖片headers組合仍待Root具名Native驗收。

原header回首頁30×30 RGB JPEG角色小圖沿原301/302鏈取得，原bytes SHA `d67a48a9fc9d44c2f26ceb00fc4da95acbad8eb12036e2e01cc702e9b07e56dc` 保持，不冒名PNG/轉碼；沿既有optional無icon包。原head/Person通用平台cover不借作作者圖標。

只用凍結兩頁/首詳情做實際production Cpp HtmlHost+Lexbor/Wasmtime回放：兩頁24身份/next、共享封面未折疊、首詳情/一章/5頁、兩類封面與5正文計畫、有限非法參數與明示synthetic malformed HTML、真實ABI；各operation句柄末0只是資源關閉，並非0圖片。封面明示信任邊界也有一項獨立合成請求可接受的回放，不能把它改稱owner拒絕。無新live請求，不跑後頁/其它正文或舊源矩陣。

原68合同、page2補14、URI角色糾正5均保留；舊原REPORT的「首封面/正文完整URI相同」誤句已由獨立correction明確更正，不覆寫原凍結。源限制HTML<1MiB、每頁≤24項、每篇≤64圖、JSON深度≤64；結構不符明確拒絕，不猜URI回退。其餘23篇/後4頁/未來目錄/離線/Native unknown。原素材不隨包分發。

獨立clone `/private/tmp/Koma-sources-fjumonkey-zh-hant-20261015`，base `d5ee46d9821c40c2931161517d27027671e6f0c9`；交付證據 `/private/tmp/koma-fjumonkey-zh-hant-implementation-20261015`。保護全部舊凍結及主WIP，不改App/Kit/SDK/runner原件，不設備/commit/push/release。Root審完整diff/原合同後負責普通197驗收。
