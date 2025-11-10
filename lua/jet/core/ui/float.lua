local manager = require("jet.core.manager")

---@class Jet.Ui
---@field background_bufnr number
---@field background_winnr number
local ui = {}
ui.__index = ui

setmetatable(ui, {
    __call = function(self, ...)
        return self.new(...)
    end
})

function ui:show()
    local scale = function(value, factor)
        return math.floor(value * factor)
    end

    self.background_bufnr = vim.api.nvim_create_buf(false, true)

    vim.keymap.set("n", "q", function()
        vim.api.nvim_win_close(self.background_winnr, true)
    end, { buffer = self.background_bufnr, silent = true })

    local scale_factor = 0.75

    self.background_winnr = vim.api.nvim_open_win(self.background_bufnr, false, {
        relative = "editor",
        width = scale(vim.o.columns, scale_factor),
        height = scale(vim.o.lines, scale_factor),
        row = scale(vim.o.lines, (1 - scale_factor) / 2),
        col = scale(vim.o.columns, (1 - scale_factor) / 2),
        style = "minimal",
        focusable = false,
    })


    local kernels = manager:list_kernels()

    ---@class Jet.Ui.KernelGroup
    ---@field name string
    ---@field any_active boolean
    ---@field entries Jet.Ui.KernelItem[]

    ---@class Jet.Ui.KernelItem
    ---@field language string
    ---@field status string
    ---@field spec_path string

    ---@type table<string, Jet.Ui.KernelGroup>
    local groups = {}

    for _, k in ipairs(kernels) do
        groups[k.spec.display_name] = groups[k.spec.display_name] or {
            name = k.spec.display_name,
            any_active = false,
            entries = {},
        }

        if #k.instances > 0 then
            groups[k.spec.display_name].any_active = true
        end

        table.insert(groups[k.spec.display_name].entries, {
            language = k.spec.language or "unknown",
            status = #k.instances > 0 and "active" or "inactive",
            spec_path = k.spec_path,
        })
    end


    ---@param x Jet.Ui.KernelItem
    ---@return string
    local kernel_item_to_line = function(x)
        return ("● %s (%s)"):format(x.status, x.spec_path)
    end

    ---@param group Jet.Ui.KernelGroup
    local kernel_group_to_lines = function(group)
        local lines = { "" }

        table.insert(lines, group.name)

        for _, item in ipairs(group.entries) do
            table.insert(lines, "  " .. kernel_item_to_line(item))
        end

        return lines
    end

    local all_lines = { "Jet: Jupyter Kernels", "" }

    for _, group in pairs(groups) do
        for _, line in ipairs(kernel_group_to_lines(group)) do
            table.insert(all_lines, line)
        end
    end

    vim.api.nvim_buf_set_lines(self.background_bufnr, 0, -1, false, all_lines)

    vim.bo[self.background_bufnr].readonly = true
    vim.bo[self.background_bufnr].modifiable = false
    vim.bo[self.background_bufnr].swapfile = false

    vim.api.nvim_set_current_win(self.background_winnr)
end



-- TODO
-- We shoulld maybe have somethign like this
--
-- Keymaps:
-- <x>: kill kernel
-- <enter>: open kernel (either repl or notebook)
-- <n>: new instance
-- <q>: quit
-- <tab>: expand section
--
-- ╭──────────────────────────────────────────────────────────────────────────────────────────────╮
-- │                                         Jet                                                 │
-- │          <enter> Open   <x> Stop   <n> New instance   <tab> Expand   <q> Quit                │
-- │                                                                                              │
-- │  Active Kernels (6)                                                                          │
-- │                                                                                              │
-- │    ●  Ark R Kernel   /Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark                        │
-- │        - Startup command                                                                     │
-- │          /Users/JACOB.SCOTT1/Repos/ark/target/debug/ark \                                    │
-- │            --connection_file \                                                               │
-- │            some_connection_file.json \                                                       │
-- │            --sesion-mode \                                                                   │
-- │            notebook \                                                                        │
-- │            --log                                                                             │
-- │            ark.log                                                                           │
-- │                                                                                              │
-- │        - Environment                                                                         │
-- │          - RUST_LOG=trace                                                                    │
-- │                                                                                              │
-- │       󰆍  REPL <1h31m>                                                                       │
-- │        - Last command                                                                        │
-- │          my_table |>                                                                         │
-- │            mutate(my_col = my_other_col * 2)                                                 │
-- │                                                                                              │
-- │       󰆍  REPL <1h31m>                                                                       │
-- │       󰈙  docs/test.qmd <30m>                                                                │
-- │       󰈙  docs/test2.qmd <5m>                                                                │
-- │                                                                                              │
-- │    ●  Ipykernel  /Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3                        │
-- │       󰆍  REPL <45s>                                                                         │
-- │       󰆍  REPL <45s> (inactive REPLs should be grey)                                         │
-- │                                                                                              │
-- │  Inactive Kernels (2)                                                                        │
-- │    ●  Evecxr  /Users/JACOB.SCOTT1/Library/Jupyter/kernels/rust                              │
-- │    ●  Ark R Kernel  /Users/JACOB.SCOTT1/Repos/jet/kernels/ark                               │
-- │                                                                                              │
-- ╰──────────────────────────────────────────────────────────────────────────────────────────────╯

return ui
