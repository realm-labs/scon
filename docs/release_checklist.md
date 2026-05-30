# Release Checklist

Use the same version for `scon-core`, `scon-cli`, and `scon-lsp` during early `0.x` releases.

## Required Checks

```sh
cargo fmt --check
cargo test --workspace --exclude scon-fuzz
cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings
cargo build --workspace --exclude scon-fuzz --release
cargo +nightly fuzz run parse_str -- -runs=10000
cargo +nightly fuzz run format_source -- -runs=10000
```

## Crates

- `scon-core`: verify serde APIs, parser, evaluator, formatter, and fuzz targets.
- `scon-cli`: verify `check`, `fmt`, `print`, `to-json`, `get`, and `--version`.
- `scon-lsp`: verify diagnostics, formatting, completion, hover, go-to-definition, and document symbols.

## GitHub Release

- Build macOS, Linux, and Windows binaries for `scon` and `scon-lsp`.
- Attach checksums.
- Include install notes and known limitations from `docs/lsp.md`.

## Editor Frontends

- VS Code: verify `scon.server.path`, `scon.format.enable`, `scon.includeRoot`, `scon.diagnostics.resolveOnChange`, and `scon.maxFileSize`.
- Neovim: verify `nvim-lspconfig` setup from `editors/neovim/README.md`.
- Zed: verify `editors/zed/extension.toml` and `editors/zed/README.md`.

## Manual Smoke

- Open a `.scon` file with an include.
- Edit the included file and confirm dependent diagnostics update.
- Format the document and confirm the result parses.
- Use completion inside `${...}`.
- Use go-to-definition from a substitution and an include path.
