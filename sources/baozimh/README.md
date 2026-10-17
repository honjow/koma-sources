# 包子漫畫 0.1.3：已有列表默认设置

基于独立生产候选 online.baozimh.koma 0.1.2，而非主树0.1.0原型。
仅 get_manga_list：region/state 缺省或 all 时采用 settings.defaultRegion/defaultState；具体非all筛选优先。设置仅接受原schema枚举，未知设置回到all。无设置/默认all的原URL及头逐字保持。
home四个固定推荐和search职责、正文解析/URI/请求头、图标原字节均不改。

已有403 challenge_required与5xx回执继续保留；本轮0网络请求，未找到可重放的完整成功原HTTP体。生产WASM+真实C++Host只以明确controlled503探针验证query、默认请求及错误语义，不宣称页面解析、当前可达或Native恢复。未部署。

未部署013 Home错误修订：四栏任一HTTP/传输错误按原分类立即返回；HTML解析失败明确parse_error，不继续后续栏。成功空DOM仍保原空栏目成功。只修错误传播，不含栏目id/listingId、500预算或章节slot变更。生产Host仅受控合同比较，无原站成功体、无新网络/Native，不部署。

Home栏目身份修订：仅四原栏目增加home:popular/cn/jp/kr稳定id，无listingId、新列表或更多按钮。原标题/顺序/条目/四请求及错误传播保持，原App按其现有栏目键消费；受控单卡片仅synthetic，不宣称实站或Native可用。

正文完整性修订：保留现500图预算；超过预算明确未完整错误，不将前500图冒完整成功。选择失败、src读取或写入失败立即报错，所有实际分配子句柄统一回收后关闭根。真正0图仍保原成功语义，正常URI/顺序/slot/HTTP保持。500缺上游依据仍OPEN，错误不等于支持长章或全源修复；验证仅synthetic生产Host，未联网部署。

正文容量修订取代上一500图错误预算：移除500图/512字节src及正文4000句柄限制，count-only后复用原BODY缓冲收句柄，原HTTP_OUT缓冲读取完整src。只保当前Native完整响应1MiB及既有实际缓冲容量，超容量明确未完成错误并清理；原四位页ID保持，>=10000保完整整数不回绕。无共享SDK/App/HTTP/slot变更，不新增buffer或网络，不部署；原站成功/Native仍未证。
原离线runner的Attr桥另有1MiB缓冲guard（Native C++无此字段cap），首2MiB尝试的失败已保存。现读取slice取Native完整响应1MiB；恰满URI连同任何JSON开销必超真实响应容量，明确报错，不将guard上限当512新字段协议。

本组合候选保留已审Home句柄所有权及详情全部tag，合入章节count/完整响应处理和未知章节language省略。分支非线性，原各候选冻结不改。全部HTML回放为受控输入，不宣称上游成功或Native可用；既有403不重试。标题/href等其它字段边界仍OPEN。
