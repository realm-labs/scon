local M = {}

function M.setup(opts)
    opts = opts or {}
    vim.filetype.add({ extension = { scon = "scon" } })
    local lspconfig = require("lspconfig")
    local configs = require("lspconfig.configs")
    if not configs.scon_lsp then
        configs.scon_lsp = {
            default_config = {
                cmd = opts.cmd or { "scon-lsp" },
                filetypes = { "scon" },
                root_dir = lspconfig.util.root_pattern(".git"),
            },
        }
    end
    lspconfig.scon_lsp.setup(opts)
end

return M
