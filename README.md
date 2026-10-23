# Koma 书源库

为 [Koma 阅读器](https://github.com/honjow/Koma) 提供可独立安装、更新的漫画书源。Koma 不内置书源；请只添加你信任的索引。

持续更新索引：

~~~text
https://github.com/honjow/koma-sources/releases/latest/download/index.json
~~~

## 在 Koma 中使用

1. **添加索引**：打开 Koma 的“源包管理”，进入“源索引”，粘贴上面的地址并加载。
2. **安装或更新**：在索引中选择需要的书源安装。已安装且来源相同的书源出现新版本时，可在同一索引中检查并更新。
3. **本地导入**：也可以在 [最新发行版](https://github.com/honjow/koma-sources/releases/latest) 下载单个 .koma 文件，通过 Koma 的本地文件选择器导入。

向 master 推送后，GitHub Actions 会自动编译全部生产源和教学示例，生成包、图标、安装索引与完整目录。工作流先创建草稿发行版，只在这些资产全部生成、上传并通过检查后，才把这一版公开为 latest。构建失败不会覆盖当前可用的 latest；历史发行版仍保留在[发行记录](https://github.com/honjow/koma-sources/releases)中。

[index.json](https://github.com/honjow/koma-sources/releases/latest/download/index.json) 是 Koma 可安装源列表；[catalogue.json](https://github.com/honjow/koma-sources/releases/latest/download/catalogue.json) 是完整生产源目录。包和图标与索引位于同一发行目录，条目中的 pkg、icon 使用相对路径。

## 当前覆盖

- **85 个生产源**纳入自动编译和发行；教学用 example-demo 也会编译检查，但不进入安装索引。
- 当前 latest 的安装索引与完整目录均为 **85 项**，其中 **85 项提供必需图标并可安装**。
- 版本按源独立维护，请以索引条目的 version 和 minAppVersion 为准。
- 当前有 **32 个源标记为 NSFW**。请遵守所在地法律和站点条款；nsfw: false 或 unknown 不等于已证明适合所有年龄。

图标均满足打包要求，但并非全部取得了官方高清原件：

- IKMMH、HentaiHand、Happy 和 92 漫画使用用户批准的临时名称字标 IKM / HH / Happy / 92。它们是普通的 Koma 绿色透明 512px 名称字标，不是官方 Logo，也没有额外底板、阴影或遮罩。
- 喵趣 0.1.2、XComic 简繁 0.1.1 已清理旧外底板或灰影；HComic 0.1.2 使用官网标识中的完整鲨鱼图形；MyComic 0.1.2、漫画1234 0.1.3 清理了保留下来的现有图标。这些处理保留身份，不代表取得新的官方高清原件。
- 部分黑色图标使用固定白底以保证深色界面中的辨识度，并保留原图形。鸟鸟韩漫 0.1.2 已完成浅色、深色列表验证；泰拉记事社 0.1.7 保留原白底黑字图标。

“可安装”只表示包、元数据和图标满足分发合同，不代表站点所有功能都经过验收。部分源会受登录或账户要求、站点访问限制、Cloudflare/TLS 拦截、地域限制或上游改版影响，也可能只覆盖公开免费的页面。各源的已验证范围和已知边界见 [sources/](sources) 下的说明。

## 常见问题

### 索引加载失败

先确认地址与上方完全一致，并检查设备能否访问 GitHub。若网络正常，重新加载索引并查看错误提示；也可从[发行记录](https://github.com/honjow/koma-sources/releases)打开对应版本确认资产是否存在。

### 检查更新没有反应

先确认安装来自持续更新索引，且索引条目的 version 确实高于已安装版本。重新加载索引后再检查更新。若是本地导入的旧包，可重新添加索引并检查一次；仍无反应时，请附上源名、已安装版本、索引版本和操作步骤反馈。

### 书源能安装，但打不开或没有内容

这通常来自站点侧变化、登录要求、访问限制或上游改版，与索引资产是否完整是两个问题。先更新到最新版并查看该源目录中的边界说明。不要反复请求已明确拒绝或被挑战的地址。

### 如何反馈问题

请在[源库 Issues](https://github.com/honjow/koma-sources/issues)提交，并写明：

- 源名和索引中显示的版本；
- 可重复的操作步骤；
- 页面提示、HTTP 状态或日志中的错误；
- 已尝试的更新或重新加载操作。

请勿提供账号密码、Cookie、令牌或其他登录凭据。

## 开发与贡献

书源使用 Rust 编写并编译为 wasm32-unknown-unknown。本地需要 Rust、wasm32-unknown-unknown target、jq、zip，以及带 Pillow 的 Python 3。

~~~bash
# 首次测试前构建开发运行器（已有可跳过）
cargo build --locked --release -p koma-source-dev

# 从模板创建书源
./build.sh --scaffold mysource

# 编译、校验并打包一个书源
./build.sh --source mysource

# 运行搜索 → 详情 → 目录 → 正文的完整链路测试
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm
~~~

新生产源必须同时登记到 build.sh 的 SOURCE_ENTRIES 和 Cargo workspace，并提供恰好一个符合格式、大小和解码要求的本地图标。.koma 包包含 manifest.json、source.wasm 和一个图标文件。

完整写源说明见 [docs/WRITING_A_SOURCE.md](docs/WRITING_A_SOURCE.md)，贡献流程和源码规则见 [CONTRIBUTING.md](CONTRIBUTING.md)，最小示例见 [sources/example-demo](sources/example-demo)。也可以在独立仓库中生成同样结构的索引，供用户自行添加。

## 说明与免责声明

本仓库只提供书源适配代码，不托管漫画、图片或站点内容；内容、商标和图标权利归各自权利人。编译、打包或安装成功不等于站点当前可访问，也不等于全部站点功能、图片或下载流程都通过验收。请遵守站点条款、版权要求和所在地法律，并自行判断是否使用标记为 NSFW 的源。
