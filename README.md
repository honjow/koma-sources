# Koma Sources

Source repository for [Koma](https://github.com/honjow/Koma) manga reader.

## Package Format

Each source is distributed as a `.koma` file (zip archive) containing:

| File | Required | Description |
|------|----------|-------------|
| `manifest.json` | ✓ | Source metadata (id, name, version, lang, etc.) |
| `source.wasm` | ✓ | Compiled WASM module |
| `icon.png` | | Source icon (256×256 recommended) |

## Index Format

The `index.json` file lists all available sources:

```json
[
  {
    "id": "online.baozimh.koma",
    "name": "包子漫画 (Baozimh)",
    "version": "0.1.0",
    "lang": "zh-Hant",
    "nsfw": false,
    "pkg": "sources/baozimh/baozimh-0.1.0.koma",
    "icon": "sources/baozimh/icon.png",
    "minAppVersion": "0.1.0"
  }
]
```

## Third-Party Sources

Koma supports custom source repositories. Create a repo with the same `index.json` format and host it anywhere (GitHub Pages, static CDN, etc.). Users can add your index URL in the app settings.

## Building

```bash
./build.sh              # Build all sources
./build.sh --source mangadex  # Build one source
```

Requires:
- Rust toolchain with `wasm32-unknown-unknown` target
- `koma-source-dev` runner (from main Koma repo)
- `jq`, `zip`
