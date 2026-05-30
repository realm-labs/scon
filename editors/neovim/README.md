# SCON Neovim Setup

Install `scon-lsp` on your `PATH`, then configure `nvim-lspconfig`:

```lua
vim.filetype.add({ extension = { scon = "scon" } })

require("lspconfig.configs").scon_lsp = {
  default_config = {
    cmd = { "scon-lsp" },
    filetypes = { "scon" },
    root_dir = require("lspconfig.util").root_pattern(".git"),
  },
}

require("lspconfig").scon_lsp.setup({
  settings = {
    scon = {
      includeRoot = "",
      format = { enable = true },
      diagnostics = { resolveOnChange = true },
      maxFileSize = 1048576,
    },
  },
})
```

Use an explicit binary path by changing `cmd`, for example:

```lua
cmd = { "/path/to/scon-lsp" }
```
