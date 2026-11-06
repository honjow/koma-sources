# Zerobyw

参考 `extensions-source/src/zh/zerobyw/.../Zerobyw.kt`：站点目录实为最近更新，文本搜索与筛选互斥，搜索页按站点页号请求并根据下一页链接继续。旧域和用户自定义地址沿原设置处理。

0.1.10 修复搜索读取 App `args.page.cursor`：第 N 页使用站点同一关键词的第 N 页；不改变筛选、域名、作品 ID 或目录行为。
