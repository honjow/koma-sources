# Koma Sources

Source repository for [Koma](https://github.com/honjow/Koma) manga reader.

## Structure

```
koma-sources/
├── sdk/                    ← Shared SDK (koma_source_sdk)
├── sources/
│   ├── baozimh/            ← 包子漫画 source
│   └── mangadex/           ← MangaDex source
├── tools/
│   └── koma-source-dev/    ← Local dev runner (runs sources with real HTTP)
├── build.sh                ← Build + package all sources
└── Cargo.toml              ← Workspace root
```

## Development

```bash
# Build everything
cargo build --release -p koma-source-dev
cargo build --release --target wasm32-unknown-unknown

# Run a single operation
./target/release/koma-source-dev run \
  --op search --request '{"query":"one piece"}' \
  target/wasm32-unknown-unknown/release/koma_mangadex_source.wasm

# Run full test suite
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_first_real_source.wasm
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

Minimum requirements:
1. A `sources/your-source/` crate depending on `koma_source_sdk`
2. A `build.sh` or CI that produces `.koma` packages + `index.json`
3. Host releases with downloadable assets
