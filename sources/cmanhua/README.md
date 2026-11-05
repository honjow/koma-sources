# CManhua source

Public routes, bounded catalogue parsing and stable work/chapter/page IDs follow the source contract in [NOTICE](NOTICE).

The HTML decoder preserves ordinary literal ampersands and decodes decimal or hexadecimal numeric references with or without a trailing semicolon. This follows the HTML numeric-reference tokenization rule and handles the site's real chapter titles and descriptions without changing the text. Unknown named entities, invalid scalar values and output-buffer overflow retain explicit failure; no markup is executed. Changes must preserve these boundaries and the unchanged catalogue IDs and order.

Version 0.1.7 fixes the 31-chapter catalogue for 放肆 (a title contains 秦&唐) and the description of 新52超人动作漫画 (numeric references omit semicolons). Real-response replay matches the old parser's result for the equivalent correctly escaped HTML.

明确空目录：仅当当前作品网页有唯一的章节数节点且值为0、唯一章节列表容器完全为空，并无目录区间时，get_chapters返回no_chapters。缺失、重复或未知节点仍保持解析失败；不把失败或空数组猜成下架。
