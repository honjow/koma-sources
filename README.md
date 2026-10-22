# Koma 书源库

为 [Koma 阅读器](https://github.com/honjow/Koma) 提供可独立安装、更新的书源。Koma 不内置书源；请自行添加信任的索引。

## 在 Koma 中添加和更新

1. 打开 Koma 的“源包管理”，进入“源索引”，添加下面的持续更新地址。
2. 加载索引，选择需要的书源并安装。已安装源有新版本时，可在同一索引中更新。
3. 也可在[最新源库发行版](https://github.com/honjow/koma-sources/releases/latest)下载单个 `.koma`，通过本地文件选择器导入。

```text
https://github.com/honjow/koma-sources/releases/latest/download/index.json
```

每次 `master` 更新，GitHub Actions 编译全部生产书源。包、图标和索引上传齐全后，才将这一版公开为最新发行版；构建失败不会替换原可用版本。包地址和图标地址与索引处于同一发行目录。历史发行版保留，可从[发行记录](https://github.com/honjow/koma-sources/releases)选择原版本。

## 当前覆盖与使用边界

- 85 个生产书源全部纳入自动编译；教学用 `example-demo` 单独编译检查，不进入安装索引。
- [安装索引](https://github.com/honjow/koma-sources/releases/latest/download/index.json)仅列出符合打包合同的源；[完整源目录](https://github.com/honjow/koma-sources/releases/latest/download/catalogue.json)同时列出所有生产源及尚不能发布的具体原因。
- 85 个生产源均提供必需图标并进入安装索引。IKMMH、HentaiHand、Happy、92 漫画采用用户批准的临时名称字标 `IKM / HH / Happy / 92`：普通粗体、Koma 绿色、透明512px，无底板/阴影/遮罩，不称官方Logo；九儿（92）的资产说明在本条保留。
- 喵趣0.1.2与XComic简繁0.1.1已清理原外底板/灰影，HComic0.1.2采用完整鲨鱼标识。各源说明保留资源来源及已验证范围；清理或重建的图标不代表取得了新的官方高清原件。MyComic与漫画1234仍使用现有图标，素材清晰度及来源说明仍有待完善。
- 部分黑色图标使用固定白底，以保证深色界面的辨识度并保留原图形。鸟鸟韩漫0.1.2已完成浅深色列表验证；泰拉记事社0.1.7保留原白底黑字图标。
- 编译和安装成功不表示站点所有功能均可用。部分源会受站点限制、登录要求或上游变更影响；成人内容源保留分级，具体已验证范围见[各源目录](sources)中的说明。

问题请提交到[源库 Issues](https://github.com/honjow/koma-sources/issues)，附源名、版本、普通操作步骤和错误提示。不要提供账号、Cookie 或令牌。

## 开发

以下为构建、扩展接口和保留的历史实现记录。



### 本地构建

```bash
# Build all sources
./build.sh

# Build single source
./build.sh --source baozimh

# Create a new source from template
./build.sh --scaffold mysource

# Test a source
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm

# Start dev web UI
./target/release/koma-source-dev serve \
  target/wasm32-unknown-unknown/release --port 3010
```

## Writing a Source

See **[docs/WRITING_A_SOURCE.md](docs/WRITING_A_SOURCE.md)** for the complete guide.

Quick overview:
1. `./build.sh --scaffold mysource` — creates `sources/mysource/` from template
2. Edit `SOURCE_INFO` and `SOURCE_CAPS` in `src/lib.rs`
3. Implement `run_search`, `run_get_manga`, `run_get_chapters`, `run_get_pages`
4. `--scaffold` already registers `name|crate_dir|false|0.1.0` in `SOURCE_ENTRIES` and adds the crate to workspace `Cargo.toml`; adjust the row if this source needs a newer Koma version
5. Build and test

The template uses SDK macros that reduce boilerplate from ~180 lines to ~10:

```rust
koma_source_sdk::koma_source_buffers! { payload: 128*1024, http_out: 512*1024, ... }
koma_source_sdk::koma_source_helpers!();
// ... your run_* functions ...
koma_source_sdk::koma_source_exports!("mysource");
```

## Structure

```
koma-sources/
├── sdk/                    ← Shared SDK (koma_source_sdk)
│   └── src/lib.rs          ← Host imports, JSON utils, macros (2183 lines)
├── sources/
│   ├── example-demo/       ← Minimal working example (start here!)
│   ├── baozimh/            ← HTML scraping reference
│   ├── mangadex/           ← JSON API reference
│   └── ...                 ← 17+ more sources
├── template/               ← Scaffold template for new sources
├── tools/
│   └── koma-source-dev/    ← Dev runner + web UI
├── docs/
│   └── WRITING_A_SOURCE.md ← Developer guide
├── build.sh                ← Build + package all sources
└── Cargo.toml              ← Workspace root
```

## Dev Tool

`koma-source-dev` is a local dev runner that provides the host environment (HTTP, HTML parsing) for testing sources without the Koma app.

```bash
# Build dev runner
cargo build --release -p koma-source-dev

# Run a single operation
./target/release/koma-source-dev run \
  --op search --request '{"query":"one piece"}' \
  target/wasm32-unknown-unknown/release/koma_mangadex_source.wasm

# Run full test suite (search → manga → chapters → pages)
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mangadex_source.wasm

# Show source info
./target/release/koma-source-dev info \
  target/wasm32-unknown-unknown/release/koma_mangadex_source.wasm
```

## Package Format (.koma)

Each source is distributed as a `.koma` file (zip archive) containing:

| File | Required | Description |
|------|----------|-------------|
| `manifest.json` | ✓ | Source metadata (id, name, version, lang, author, description, contentRating, minAppVersion) |
| `source.wasm` | ✓ | Compiled WASM module |
| `icon.png/jpg/jpeg/gif/webp/ico` (exactly one) | ✓ | Required source identity asset; manifest.icon is its root filename |

Every new production package requires exactly one regular local root image, at most
1 MiB and 1,048,576 decoded pixels in total. Its extension must match the actual
PNG, JPEG, GIF, WebP or ICO data. Python 3 with Pillow is required by `build.sh`;
validation decodes the asset before compilation and checks the packaged copy and
index resource. Missing, empty, unreadable, duplicate or mismatched icons stop
packaging; no placeholder is generated.

Prefer a suitable real official identity asset. Explicitly approved temporary
source-name wordmarks must be labelled as temporary, never official logos.
Do not add decorative backing plates, external masks or shadows. When an
otherwise transparent black subject is actually unreadable on dark UI, the
user's fixed-white-background permission allows retaining the subject and its
black colour on white; this is optional, not a requirement for every black icon. Keep
existing official white canvases where appropriate. Secondary processing may
remove unwanted plates/shadows while preserving the real identity; retain
provenance and before/after evidence. Prefer a suitable official original over an unsuitable
ported icon. The App's existing rounded display clipping is a separate layer
and must remain. New production requirements do not remove old installed sources
or invalidate old backups automatically.

## Release

Push to `master` to run the complete build and atomically publish a revision release as latest. A `v*` tag or manual release can also publish a named version with:
- `index.json` — installable source index with same-directory download URLs
- `catalogue.json` — every production source, including explicit distribution blockers
- `*.koma` — packaged sources

Manual: `./build.sh --tag v0.2.0`

## Index URL

App fetches the latest release index:
```
https://github.com/honjow/koma-sources/releases/latest/download/index.json
```

## Third-Party Sources

Create your own repo with the same structure. Users add your release index URL in Koma settings.

Requirements:
1. Sources depend on `koma_source_sdk` (copy the `sdk/` directory or use a git dependency)
2. Each source compiles to `wasm32-unknown-unknown` with `crate-type = ["cdylib"]`
3. A build script that produces `.koma` packages + `index.json`
4. Host releases with downloadable assets on GitHub (or any HTTP server)

`index.json` format:
```json
{
  "sources": [
    {
      "id": "com.example.mysource",
      "name": "My Source",
      "version": "0.1.0",
      "lang": "en",
      "nsfw": false,
      "author": "Example author",
      "description": "A concise source description.",
      "contentRating": "safe",
      "minAppVersion": "0.1.0",
      "pkg": "mysource-0.1.0.koma",
      "icon": "mysource-icon.png"
    }
  ]
}
```

`pkg` is the package URL Koma downloads. `minAppVersion` is a required
canonical `MAJOR.MINOR.PATCH` value for generated sources and is checked before
installation. Every generated index entry requires a non-empty `icon` reference
to the actual published asset with the same extension and bytes as the package
root image. `manifest.icon` records only that root filename (for example
`icon.gif`); the index reference is relative to its own distribution directory.

This distribution does not generate package digests. Koma's existing index
contract accepts an omitted `sha256`; indices that do provide a digest still use
the App's existing verification path. Package metadata, ZIP entries, required
identity assets, release paths and minimum App versions remain validated.

### Localized source display names

A source may provide `sources/<source>/metadata.json`. The production packager
extracts only its `localizedNames` object and copies the validated object into
both `manifest.json` and its index entry. Unrelated top-level metadata is ignored
and never merged into either output. This file is build input, not an extra
`.koma` archive member. Without this key, the existing package fields are kept;
an explicitly empty `localizedNames` object is preserved as an empty object.

Locale keys follow `^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8}){0,5}$`, are at most 63
characters, and must be unique ignoring case. There may be at most 16 entries.
Names must be nonempty trimmed strings, at most 128 UTF-16 code units, valid
Unicode, and contain no U+0000 through U+001F or U+007F characters. Invalid values
of this known field fail extraction. The JSON reader currently uses standard
`json.loads` duplicate-key handling; the old helper's duplicate-key rejection is
not claimed by the current production entry point.

`name` remains the canonical fallback. `lang` describes source content, not the
interface locale. Localized names do not change source identity, version,
ratings, capabilities or language. SDK `SourceInfo`, `get_source_info` and WASM
behavior remain separate. The schema's string limit counts Unicode code points;
the production helper also checks the exact UTF-16 length limit.

## 保留的历史实现记录

本表是早期实现记录，数量与“Full”仅描述当时实现，不代表当前全部生产源或完整站点验收。当前覆盖以完整源目录为准。

| Source | Type | Status | Notes |
|--------|------|--------|-------|
| guazimanhua | HTML | Retained | Current source 0.1.2; finite named evidence only; see source implementation and retained provenance |
| kuaikan | HTML/JSON | Retained | Current source 0.1.4; public free content only; locked chapters refused; see source implementation and retained contract |
| cmanhua | HTML | Retained | 0.1.3 adds the original doupocangqiong webpage; Host HTTP200/canonical match, actual WASM non-links fields/request equal to runtime0.1.2/Cargo0.1.0. Three-member package retains icon/256KiB stack/nsfw=true; no localized metadata exists. Device237 first import 54→55 enabled preserves original 54 ordered IDs/enabled; exact-work two webpage actions → real same-work WebView → Back retains 664/664. Normal chapter 1 “斗破苍穹 - 01”: P1/P2 distinct colour title images, P3/36 real three-panel narrative; Back history retains 3/36. No P4/other chapter/download or old 403 retry. Browse/search scope/10000ms/AWAKE restored, transfer removed, leases released. Evidence: Koma `.hvigor/outputs/user-flow-audit/cmanhua013-link-compare-offline.json`; device partition `device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/cmanhua013-{import237-001,body237-001,story237-001,story-restored237-001}` |
| example-demo | JSON API | ✅ Demo | Minimal working example for learning |
| baozimh | HTML | ✅ Full | 包子漫画 |
| mangadex | JSON API | ✅ Full | MangaDex |
| mangabz | HTML | ✅ Full | MangaBZ |
| manhuaren | JSON API | ✅ Full | 漫画人 (anon auth) |
| zaimanhua | JSON API | ✅ Full | 在漫画 |
| terrahistoricus | JSON API | ✅ Full | 明日方舟泰拉记事社 |
| komiic | GraphQL | ✅ Full | Komiic (NSFW) |
| dm5 | HTML | ✅ Full | 动漫屋 (NSFW) |
| dongmanmanhua | HTML | ✅ Full | 咚漫 (Line Webtoon CN) |
| manhuashe | HTML | ✅ Full | 漫画社 |
| iqiyi | HTML | ✅ Full | 爱奇艺叭嗒 |
| jiuermanhua | HTML | ✅ Full | 92漫画 |
| manhuawu | JSON API | ✅ Partial | 漫画屋 (CF on listing) |
| hanman18 | HTML | ✅ Code | 汉漫18 (NSFW, TLS blocked) |
| happymh | JSON+HTML | ⚠️ CF | 嗨皮漫画 (needs cookie injection) |
| manhuagui | HTML | ⚠️ TLS | 漫画柜 (TLS fails from dev env) |
| noyacg | JSON API | ⚠️ Auth | NoyACG (NSFW, needs login) |
| zerobyw | HTML | ⚠️ DNS | zero搬运网 (DNS fails) |
| mh1234 | — | 📦 New | — |
| zazhimi | — | 📦 New | — |
