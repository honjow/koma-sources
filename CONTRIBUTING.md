# Contributing to Koma Sources

Thanks for your interest in contributing! This guide covers the process for adding new manga sources or fixing existing ones.

## Quick Start

```bash
# 1. Fork & clone
git clone https://github.com/YOUR_USERNAME/koma-sources.git
cd koma-sources

# 2. Create a new source (recommended)
./build.sh --scaffold mysource

# 3. Implement your source
# Edit sources/mysource/src/lib.rs
# See SKILL.md or docs/WRITING_A_SOURCE.md for the full guide

# 4. Build
cargo build --release --target wasm32-unknown-unknown -p koma_mysource_source

# 5. Test
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm

# 6. Package
bash build.sh --source mysource
```

## Requirements

- **Rust** with `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- **jq** and **zip** for packaging (install via your package manager)

## Source Code Rules

1. **Use all 3 SDK macros** — every source must use `koma_source_buffers!`, `koma_source_helpers!`, and `koma_source_exports!`
2. **`#![no_std]`** — no `std` crate. No `String`, `Vec`, `HashMap`, `format!`, etc.
3. **No non-ASCII in byte literals** — use hex UTF-8 arrays with comments: `&[0xe4, 0xb8, 0xad] // 中文`
4. **JSON schemas** — follow the exact request/response schemas in `SKILL.md`
5. **Page images use nested `image.url`** — not top-level `url`
6. **Links use array format** — `[{"kind":"source","url":"..."}]`, not flat strings
7. **Always close HtmlDescriptor** — call `html_close()` when done with any descriptor

## Testing

All sources must pass a full chain test before submission:

```bash
# Build the dev runner first
cargo build --release -p koma-source-dev

# Test each source
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_YOURSOURCE_source.wasm
```

The CI pipeline also runs `test-all` on `example-demo` (the only source with a public API suitable for CI).

## Pull Request Process

1. **One source per PR** — makes review easier
2. **Include `Cargo.toml` changes** — workspace members must include your source
3. **Update `build.sh`** — add your source to `SOURCE_MAP` and `NSFW_MAP`
4. **CI must pass** — the pipeline builds all sources and runs tests
5. **Add your source to the README table** — with type (JSON/HTML), auth requirements, and status

## Documentation

- **`SKILL.md`** — AI-friendly development reference (SDK API, macros, patterns, pitfalls)
- **`docs/WRITING_A_SOURCE.md`** — Detailed developer guide (~530 lines)
- **`sources/example-demo/`** — Minimal working source using JSONPlaceholder public API
- **`template/`** — Starter template (or use `./build.sh --scaffold`)

## Package Format

A `.koma` package is a zip archive containing:

```
manifest.json    — SourceInfo metadata + capabilities
source.wasm      — Compiled WASM module
```

The `build.sh` script handles packaging automatically. You can also distribute sources individually — users place `.koma` files in their Koma source directory.

## Questions?

Open an issue on GitHub or refer to the existing sources in `sources/` as examples.
