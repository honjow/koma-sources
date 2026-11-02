# Koma 书源库

为 [Koma 阅读器](https://github.com/honjow/Koma) 提供可独立安装、更新的漫画书源。Koma 不内置书源；请只添加你信任的订阅。

持续更新订阅地址：

~~~text
https://github.com/honjow/koma-sources/releases/latest/download/index.json
~~~

## 在 Koma 中使用

1. **添加订阅**：打开 Koma 的“浏览”，点右上角“书源管理”，再点右上角四点菜单中的“订阅地址”。在订阅地址页点右上角添加，粘贴上面的地址并保存。可以添加多个订阅；名称可自行填写，留空时使用订阅提供的名称。
2. **安装或更新**：返回书源管理，选择需要的书源安装。下拉书源管理页面可检查更新；有更新的书源会集中显示在顶部，可分别更新。Koma 也会在后台检查，发现更新时在“浏览”显示角标。
3. **管理订阅**：在“订阅地址”页可编辑、复制或移除已添加的地址；订阅提供项目主页时，也可从对应菜单打开。
4. **本地导入**：也可以在 [最新发行版](https://github.com/honjow/koma-sources/releases/latest) 下载单个 .koma 文件，在 Koma 的书源管理右上角四点菜单中选择“本地导入”，通过文件选择器导入。

书源更新会通过 GitHub Actions 自动发布至[最新发行版](https://github.com/honjow/koma-sources/releases/latest)。

[index.json](https://github.com/honjow/koma-sources/releases/latest/download/index.json) 是 Koma 可安装源列表；[catalogue.json](https://github.com/honjow/koma-sources/releases/latest/download/catalogue.json) 是完整生产源目录。包和图标与索引位于同一发行目录，条目中的 pkg、icon 使用相对路径。

同目录可选的 `repo.json` 使用 `{"meta":{"name":"订阅名","website":"https://项目页"}}` 提供订阅信息；两个字段均可省略，website 为 HTTP(S) 地址。它独立于数组格式的安装索引，旧客户端无需读取即可继续安装和更新。

## 图片处理插件

书源和图片处理插件使用独立的订阅地址；在同一个“订阅地址”入口添加后，Koma 会自动识别并分组。图片处理插件目录地址：

~~~text
https://github.com/honjow/koma-sources/releases/download/image-plugins-v1/index.json
~~~

添加订阅后，在图片处理插件管理中选择并安装插件；添加目录本身不会安装插件。插件可以配置和卸载，新处理算法通过外部 WASM 插件提供。“去除包子漫画横幅”插件可去除同款顶部及底部横幅。

在“设置 → 阅读 → 横幅过滤（页面裁边分组）”中选择过滤方式，也可在单本漫画的阅读设置中单独选择，或使用全局设置。下载原文件和普通 CBZ 导出保留原图。详见 [Koma 横幅过滤说明](https://github.com/honjow/Koma#横幅过滤)。

## 常见问题

### 订阅加载失败

先确认地址与上方完全一致，并检查设备能否访问 GitHub。地址尚未保存时，可在添加或编辑对话框中重试；已保存的订阅可返回书源管理页下拉检查。也可从[发行记录](https://github.com/honjow/koma-sources/releases)打开对应版本确认资产是否存在。

### 检查更新没有反应

先确认已添加持续更新的订阅地址，且订阅中的版本高于已安装版本。在书源管理页下拉检查更新；若是本地导入的旧包，也可添加对应订阅后再检查。仍无反应时，请附上源名、已安装版本、订阅中的版本和操作步骤反馈。

### 书源能安装，但打不开或没有内容

请在[源库 Issues](https://github.com/honjow/koma-sources/issues)反馈源名、操作步骤和页面提示。

### 如何反馈问题

请在[源库 Issues](https://github.com/honjow/koma-sources/issues)提交，并写明：

- 源名和索引中显示的版本；
- 可重复的操作步骤；
- 页面提示、HTTP 状态或日志中的错误。

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

漫画、图片及相关内容的权利归原权利人所有。
