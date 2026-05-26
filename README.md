# Koma Sources

Source repository for [Koma](https://github.com/honjow/Koma) manga reader.

## Quick Start

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
4. Add to `SOURCE_MAP`/`NSFW_MAP` in `build.sh` and workspace `Cargo.toml`
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
| `manifest.json` | ✓ | Source metadata (id, name, version, lang, etc.) |
| `source.wasm` | ✓ | Compiled WASM module |
| `icon.png` | | Source icon (256×256 recommended) |

## Release

Push a tag `v*` to trigger CI → builds all sources → creates a GitHub Release with:
- `index.json` — source index with download URLs
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
      "url": "https://github.com/you/koma-sources/releases/download/v0.1.0/mysource-0.1.0.koma"
    }
  ]
}
```

## Current Sources (20)

| Source | Type | Status | Notes |
|--------|------|--------|-------|
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
