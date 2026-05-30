# SCON LSP

`scon-lsp` provides language-server support for `.scon` files.

## Install

Build from this workspace:

```sh
cargo install --path crates/scon-lsp
```

Or build release binaries:

```sh
cargo build --workspace --exclude scon-fuzz --release
```

The binary is `scon-lsp`.

## Features

- Parse and resolve diagnostics.
- Include diagnostics using open editor buffers before filesystem reads.
- Full-document formatting through `scon-core::format_source`.
- Completion for SCON paths, include paths, and keywords.
- Hover for diagnostics, definitions, and resolved value previews.
- Go to definition for substitutions, spreads, and include paths.
- Nested document symbols.

## Configuration

Settings are accepted under `scon`:

```json
{
  "scon": {
    "includeRoot": "",
    "format": { "enable": true },
    "diagnostics": { "resolveOnChange": true },
    "maxFileSize": 1048576
  }
}
```

- `includeRoot`: optional root directory for include resolution.
- `format.enable`: enables full-document formatting.
- `diagnostics.resolveOnChange`: resolves includes and substitutions on document changes.
- `maxFileSize`: maximum file size in bytes for LSP analysis.

## Editors

- VS Code: use `editors/vscode`; set `scon.server.path` when `scon-lsp` is not on `PATH`.
- Neovim: use the `nvim-lspconfig` setup in `editors/neovim/README.md`.
- Zed: use the extension manifest and settings in `editors/zed`.

## Known Limitations

- Range formatting is not implemented.
- Remote includes and glob includes are intentionally unsupported.
- Schema validation is not part of v0.1.
- npm-based binary distribution is not part of v0.1.
