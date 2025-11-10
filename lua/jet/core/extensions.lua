local utils = require("jet.core.utils")

---@class Jet.Extension
---
---A function that takes a Jet.Kernel.Instance and returns a boolean indicating
---whether the extension should be used for that kernel.
---@field filter_kernels fun(kernel: Jet.Kernel.Instance): boolean
---
---A list of filetypes (as per `:help 'filetype'`) that the extension should
---be active for.
---@field filetypes? string[]
---
---A function that takes a buffer number and returns a boolean indicating
---whether the extension should be used for that buffer. This may be useful to
---set if you need more granular control than `filetypes`.
---@field filter_buffers? fun(bufnr: number): boolean
---
---A function that resolves the code which should be sent to the kernel, e.g.
---on `<CR>`. Note that this function is only invoked when _not_ in a notebook
---context (in notebook chunks are always sent in their entirety).
---TODO: outline expected behaviour, e.g. moving the cursor, excluding
---comments, etc.
---@field get_expr fun(): string[]

---@class Jet.Extensions
---@field extensions table<string, Jet.Extension>
local M = {
    is_initialised = false,
    extensions = {}
}

function M:init()
    if self.is_initialised then
        return
    end
    self.is_initialised = true

    local modules = vim.iter(vim.api.nvim_get_runtime_file("*/jet/*/init.lua", true))
        :map(function(file)
            return vim.fs.basename(vim.fs.dirname(file))
        end)
        :totable()

    for _, mod in ipairs(modules) do
        local ok, ext = pcall(require, "jet." .. mod)

        if ok then
            utils.log_trace("Loaded Jet extension '%s'", mod)
            self.extensions[mod] = ext
        else
            utils.log_error("Failed to load Jet extension '%s': %s", mod, ext)
        end
    end
end

return M
