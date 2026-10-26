# 再漫画

当前源版本 `0.1.7`，稳定身份 `com.zaimanhua.koma`；官方移动主页 `https://m.zaimanhua.com`，API 保持 `https://v4api.zaimanhua.com/app/v1`。官方原图标及素材归属见 `ASSET_SOURCES.md`。

## 原有公开合同

热门、最近更新、搜索、详情、目录、正文和图片请求沿用原源实现。目录完整数组、写入失败传播、PC 二次请求前的原章节快照、直接 JSON 成员解析及原分页规则保持。章节 language 不从未声明字段猜补。正文继续先检查 `canRead`；权限不足不返回正文或评论，不绕过账户与等级限制。原 ID、页序、默认图片选择、请求 UA、Platform 和超时保持。

## 章末吐槽页

设置键为 `COMMENTS`，默认 `false`，标题沿固定 Keiyoushi 参考“章末吐槽页”。宿主必须同时声明 `text_page_image_v1`、`text_page_image_zaimanhua_comments_v1` 与 `source_text_page_request_v1`，源才显示和消费此项；缺能力时，即使保存过 `true` 也不请求评论、不输出文字页。

显式开启后，正文权限检查和原图片解析成功，只在原页表末尾追加 `text_request` 描述，不在 `get_pages` 阶段请求评论。宿主准备该末页时，使用其真实 `chapter-comments:<chapterId>` 身份调用原 `get_image_request`；源再请求 `/viewpoint/list?comicId=...&chapterId=...`。评论请求失败或非空 `errmsg` 只归该页失败与正常重试，原 URL 正文页仍可读，不吞掉错误，也不使整个章节失效。每条评论取数组末尾的 JSON primitive 文本；空列表显示“没有吐槽”，非空 `errmsg` 返回错误而非冒充无评论。文本是普通 Unicode，保留原文中的 HTML 实体字样，不执行 HTML 或重复解码 JSON。文字页仅追加在正文之后，使用 `zaimanhua-chapter-comments` 固定模板，由宿主原图准备与缓存链生成本地 PNG；不伪造网络图片地址。

设置变化由宿主现有页表设置摘要在下次实际打开章节时重新取页；不要求清图片缓存。用户名、密码和 Token 不作为普通明文设置开放；不新增账号登录流程或改变原匿名访问边界。

## 构建与验证范围

标准入口为 `bash build.sh --source zaimanhua`。构建、受控 API 回放、真实网络、安装和 Native 内容画面分别成立，不以其中一项冒其余通过。当前交付证据归项目唯一工作单。
