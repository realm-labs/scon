# Troubleshooting

## `scon-lsp` Does Not Start

- Confirm the binary is installed: `scon-lsp --version` is not currently a CLI mode, so use `which scon-lsp` or run the binary directly.
- In VS Code, set `scon.server.path` to the absolute binary path.
- In Neovim or Zed, ensure the configured command can be found from the editor process environment.

## Includes Fail In The Editor

- Check `scon.includeRoot`.
- Relative includes resolve from the including file's directory by default.
- Open buffers take priority over filesystem reads, so unsaved include changes should be reflected.
- Absolute paths, `~`, environment-variable paths, globs, and URL-like include paths are rejected.

## Formatting Does Nothing

- Confirm `scon.format.enable` is `true`.
- Formatting is full-document only.
- Formatting is source-preserving; resolved canonical output is available through `scon print`, not LSP formatting.

## Diagnostics Are Slow

- Set `scon.diagnostics.resolveOnChange` to `false` and rely on save-time diagnostics.
- Lower or raise `scon.maxFileSize` based on workspace size.
- Large include graphs should be reduced or checked through `scon check` in CI.

## Useful Local Checks

```sh
cargo fmt --check
cargo test --workspace --exclude scon-fuzz
cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings
cargo +nightly fuzz run parse_str -- -runs=10000
cargo +nightly fuzz run format_source -- -runs=10000
```
