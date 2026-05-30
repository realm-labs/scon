# SCON Zed Setup

Install `scon-lsp` on your `PATH`, or configure Zed to use an explicit binary path when your local extension wiring supports it.

Supported server settings:

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

The extension manifest wires `.scon` files to the `scon-lsp` language server name. The server prefers open editor buffers over filesystem reads when resolving local includes.
