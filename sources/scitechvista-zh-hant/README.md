# 科技大觀園・科普漫畫大賽 0.1.0

來源為 `https://scitechvista.nat.gov.tw/Student/Article/C010002/new`。科技大觀園資源頁實際連至「科普漫畫大賽」；這是官方平台刊載學生與其他作者的參賽作品，不是宣稱政府創作全部漫畫。SourceInfo.author 的 Koma 指源開發者。逐作作者按原欄位保留；學校／組別與原刊登日期保留於作品說明，不推斷缺失作者、民國日期的時區或發佈時間。正式評級、使用率未知。

## 目錄與操作

現有真實6頁，5+5+5+5+5+3共28個不同作品UUID。源不硬編碼28標題、作者、UUID或正文URI：動態解析主要目錄每篇title/href/作者/學校或組別/日期/cover。以 `kf-paginator-date` 原筆數、當前頁、總頁核數量（5篇/頁，已核最多6頁、30作品容量）；實際下一頁連結必須存在。請求第2–6頁先從首頁驗該頁真實href，再讀該頁；首頁有實際1..5與末頁6連結。數量、總頁變更、身份重複或越界即拒絕，不猜終態或額外路由；page7、非法cursor於網路前拒絕。首次失敗探針已保留：全頁`h2.main_tit`也命中「推薦文章」，最終依原父樹收窄至 `.contents_wrap > .container-mx > h2.main_tit`，不改Host。

支援listings/manga_list/detail/chapters/pages/image_request；search/home/filters/settings/credentials無真實合同，均不支援。不以假home或額外本地搜尋截斷分頁。App的`args.page.cursor`首屏null、後續"2".."6"；輸出`data.hasNextPage`與`page.nextCursor/hasMore`。第6頁false/null，保留舊數字page輸入相容，拒絕零、前置零、URL游標。

每篇獨立作品／一個完整閱讀單元；chapterNumber/volumeNumber=null，readingGroup=該作品MID，continuationEligible=false。ID綁原UUID，例如 `scitechvista:zh-hant:comic:8b101398-009a-4e24-a860-1b692694179c`，章前綴`scitechvistaread:`，頁前綴`scitechvistapage:`加從0開始的ordinal。不按日期或陣列位置編章號，不跨作品自動相鄰，不裁四格成四頁。

## 正文、身份與圖片請求

從當前真目錄找到作品後，只GET原 `/Article/C010002/detail?ID=UUID`。詳情主要h2、Author、Division、原日期需與目錄相符，head og:title及JSON-LD mainEntityOfPage.@id須與本作品匹配。保留空學校／組別，不把「社會組」冒充學校，也不把作者併成平台名。日期只有原日精度，放在說明原字串，不填造epoch時間。

正文僅取 `#block > .DetailContent > span.templateLongText.kf-det-content` 唯一主要樣版；推薦文章、側邊欄、活動banner、footer與分享圖不在此樹。按主要樣版內所有真img[src]的DOM順序輸出1–64個完整原圖，chapter.pageCount取實際數量，不把首篇一張強推其餘27都一張。缺src、無圖、重複URI、超量、跨站、越目錄、query或不支持的圖片路徑均失敗。當前已核正文為 `/FileDownload/Article/數字.jpg`；同一目錄的數字檔名且副檔名jpg/JPG/jpeg/png/PNG/gif僅在真DOM聲明時可被動取出，不合成檔名或把其他27篇格式當已驗收。其餘結構變化顯式拒絕。

封面維持目錄完整 `/UploadPlugin?file=...` 原URI及percent字節，這是原頁公開圖片連結，不生成或破解file值；不與正文URI混用。get_image_request只接受正確作品MID封面身份或缺pageId的精確目錄cover；正文須正確作品UUID、ordinal與當次主要樣版圖完整URI逐一相等。mangaUrl/mangaId獨立核對，外部URL不作HTTP入口。輸出原圖URI及作品詳情Referer，不下載或將漫畫打包分發。

## 原圖標與巨大CMYK首圖

历史0.1.0的 `icon.png` 保持原詳情head `rel=icon` 指向 `https://scitechvista.nat.gov.tw/images/favicon.ico` 的542原字節，SHA256 `731674b17b7000fed26867e417318d4172cc73b923a38e43dd0bf219d7bfd0c5`。URL及MIME叫ICO，但實際簽名字節是PNG、16×16 RGBA，直接保存為包規定icon.png；沒有轉碼、縮放、裁切、改色、遮罩、加底或陰影。原V字favicon小槽清晰度未實機驗收。

首篇《幽冥六劍客： 蚊子讓你又癢又腫的秘密》，蕭楷諺／桃園市立桃園國民中學，107/12/25，原文明示2018全國科普漫畫大賽銀獎、學生組四格。唯一完整原JPEG `https://scitechvista.nat.gov.tw/FileDownload/Article/20181225165123000000637.jpg` 是3276×14174、CMYK、28,793,918字節，SHA256 `12fb3f0f5e39b35cdd7129bf9d8e665e8881da0417001062d7eb6febb8a33656`。未轉RGB、縮小或切圖，未新增網路重抓。約46.4MP的Native解碼、記憶體、顏色、下載成本仍未知；本機圖片查看不能替代197。原包不含漫畫圖片。

## 生產Host與成本邊界

原件 `/private/tmp/koma-scitechvista-zh-hant-public-contract-20261015` 的45份綁定保留10次一次匿名GET、完整原HTTP/頭/回執、9份原字節父樹；原匿名響應帶ASP.NET session cookie，但取證未保存cookie jar或發送cookie，未登入/執行JS/猜API/繞挑戰。實作與回放新增live GET為0。

沿既有工具鏈獨立複製當前未改App C++ HtmlHost/.h及1057份Lexbor，用Wasmtime與生產WASM對原6目錄及首詳情回放：28逐條title/author/division/date/cover/detail、6分頁旗標、28封面計畫、首詳情/章/原頁/圖片計畫、失敗封閉與完整ABI，每操作描述符歸零。合成畸形HTML／非法請求均明確分開；沒有合成成功正文冒充實站。XML/JS/圖片轉換均未加入。保留SDK緩衝/回應助手與既有接受源的顯式指標邊界ABI exports；不改共同SDK或ABI。

成本是源Host callback數而非實機網路測量：首頁1GET；後續Browse各2GET（首頁+所需實際頁）；作品身份按真目錄逐頁找，首屏作品1GET、第2頁作品2GET…第6頁作品6GET，找到即停。首篇詳情／章／頁／正文計畫各1目錄+1詳情（160,237原metadata字節）；首屏封面各1GET，缺pageId的cover為排除歧義讀6頁。無共享cache或憑舊get_pages信任外部圖片URI假設。SDK metadata正常UA/目錄Referer由離線橋核字段；最初原公開GET為Mac Firefox且無Referer，離線回放不證明SDK UA/Referer已在197通過。

其餘27詳情／正文實體（含頁數/格式）、Native Cpp/WAMR集成、CMYK實際呈現、其他布局/圖標槽、下載/cold、並發速度與未來目錄變更未驗收。Root負責全差異審查及197串行普通導入、真5→28分頁、首篇完整原圖、下載與cold；build或Host成功不提前稱Native PASS。沒有App/Kit/SDK/舊源/WIP/設備/IDE/簽名操作，未提交/推送/發布。

图标更新（0.1.1，2026-10-20）：官网 https://scitechvista.nat.gov.tw/ 的 follow-fixed-fb 明确链接 https://www.facebook.com/scitechvista/ ，该官方账号标题「科技大觀園 | Taipei」及简介声明Q博。采用其准确og:image完整720×720 JPEG（39742字节），Q博角色与原环保科技图形/浅色画布保持；未提取旧V、裁切、重绘或新增底板/阴影/遮罩，App容器保持原样。Root已审完整原图；两处小槽实际对比另验，不以分辨率代替通过。


## 237 首篇普通阅读失败（2026-10-23）

普通本地导入正式0.1.1后，登记由61项增至62项，仅新增本源，其余61项完整记录及相对顺序保持；普通Browse首屏5篇与《幽冥六劍客： 蚊子讓你又癢又腫的秘密》1章成功。开始阅读后先显示“图片解码中”1/1，约61秒后日志明确为ReaderImage http_request_failed code=2300028（操作超时），继而resolve_failed page=1；实际终态“无法加载页面/重试”，没有点击重试。未取得正文实体或解码成功，本次不能判断CMYK颜色、解码或内存兼容，也不把元数据HTTP200当正文成功。

普通返回Browse喵趣分组，原10000ms/AWAKE已读回；只清理本次上传包及两个临时取证文件，均核不存在，租约已释放，哈希关闭。自然进度pageIndex=0/totalPages=1/completed=false、章节isRead=false。原件在Koma既有237 category-membership的`scitech011-cmyk237-reader-final-001/{screen.png,run-metadata.json}`（business-log含01:25:54错误）、`scitech011-cmyk237-imported-001/registry.json`及`scitech011-cmyk237-final-restored-001`。不由此推断其余27篇、下载或冷启动通过，不修改源业务或App超时。
