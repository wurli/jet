local engine = require("jet.core.rust")
local utils = require("jet.core.utils")

---@class Jet.Manager
---
---The `kernels` field contains info about kernels on the Neovim side, e.g.
---buffer IDs, etc. This is not necessarily an exhaustive list of all active
---kernels (although usually it is). For a complete list of active kernels, use
---the Rust engine.
---@field running table<string, Jet.Kernel>
---
---A mapping of filetypes to kernel IDs. This map is checked when choosing
---which kernel to execute with in the event that multiple kernels are running
---for a particular filetype.
---@field map_kernel_filetype table<string, string>
---
---A mapping of buffer numbers to kernels. This supports running dedicated
---kernel instances for particular buffers. This is most often used for
---notebooks. NB, linking buffers (rather than files) to kernels brings the
---limitation that if the buffer quits and reopens (i.e. with a differnt bufnr)
---the link is broken. This feels like intuitive behaviour, so this is a
---conscious design decision.
---@field map_kernel_buffer table<string, string>
local manager = {
    running = {},
    map_kernel_filetype = {},
    map_kernel_buffer = {},
}
manager.__index = manager

local jet_global_augroup = vim.api.nvim_create_augroup("jet-global", {})

vim.api.nvim_create_autocmd({ "BufEnter", "BufWinEnter" }, {
    group = jet_global_augroup,
    callback = function()
        local id = (vim.b.jet and vim.b.jet.id)
        local ft = vim.bo.filetype

        if id and ft ~= "" then
            manager.map_kernel_filetype[ft:lower()] = id
        end
    end,
})

setmetatable(manager, {
    ---@return Jet.Manager
    __call = function(self, ...)
        return self.start(...)
    end,
})

---@param opts? Jet.Manager.Filter
function manager:open_kernel(opts)
    self:get_kernel(function(spec_path, id)
        if id then
            self.running[id].ui:show()
        elseif spec_path then
            require("jet.core.kernel").start(spec_path)
        end
    end, opts)
end

---@param opts? Jet.Manager.Filter
---@param callback fun(spec_path: string?, id: string?)
function manager:get_kernel(callback, opts)
    local kernels = self:_filter(self:list_kernels(), opts)

    if vim.tbl_count(kernels) <= 1 then
        local k = kernels[1]
        callback(k and k.spec_path, k and k.id)
        return
    end

    -- Formatting stuff for a nicer display
    kernels = vim.tbl_map(function(k)
        local time = k.start_time and utils.time_since(k.start_time)
        return {
            kernel = k,
            name = k.spec.display_name,
            status = (k.id and time and "(running for " .. time .. ")") or (k.id and "(running)") or "",
            path = utils.path_shorten(k.spec_path):gsub("/kernel%.json$", ""),
        }
    end, kernels)

    local widths = {}
    for _, field in ipairs({ "name", "status", "path" }) do
        widths[field] = math.max(unpack(vim.tbl_map(function(k)
            return #k[field]
        end, kernels)))
    end

    local pad = function(s, w)
        return s .. (#s < w and string.rep(" ", w - #s) or "")
    end

    ---@param k { kernel: Jet.Manager.ListItem, path: string, name: string, status: string }
    kernels = vim.tbl_map(function(k)
        return {
            kernel = k.kernel,
            desc = ("%s %s %s"):format(
                pad(k.name, widths.name),
                pad(k.status, widths.status),
                pad(k.path, math.min(widths.path, 40))
            ),
        }
    end, kernels)

    -- TODO: more helpful sorting
    table.sort(kernels, function(a, b)
        return a.desc < b.desc
    end)

    vim.ui.select(
        kernels,
        {
            prompt = "Select a kernel",
            ---@param k { kernel: Jet.Manager.ListItem, desc: string }
            format_item = function(k)
                return k.desc
            end,
        },
        ---@param k { kernel: Jet.Manager.ListItem, desc: string }
        function(k)
            local kernel = k and k.kernel or {}
            callback(kernel.spec_path, kernel.id)
        end
    )
end

---@class Jet.Manager.ListItem
---@field spec_path string
---@field spec Jet.Kernel.Spec
---@field id? string
---@field info? Jet.Kernel.Info
---@field start_time? number

---@return Jet.Manager.ListItem[]
function manager:list_kernels()
    local available = engine.list_available_kernels()
    local running = engine.list_running_kernels()

    local kernels = {}

    for path, spec in pairs(available) do
        table.insert(kernels, {
            spec_path = path,
            spec = spec,
        })
    end

    for id, instance in pairs(running) do
        table.insert(kernels, {
            spec_path = instance.spec_path,
            spec = instance.spec,
            id = id,
            info = instance.info,
            start_time = instance.start_time,
        })
    end

    return kernels
end

---@param kernels Jet.Manager.ListItem[]
---@param opts? Jet.Manager.Filter
function manager:_filter(kernels, opts)
    if not opts then
        return kernels
    end

    if opts.buf then
        -- First we try and get a kernel which may have been explicitly linked
        -- to the buffer
        opts.buf = opts.buf ~= 0 and opts.buf or vim.api.nvim_get_current_buf()
        opts.id = self.map_kernel_buffer[opts.buf]
        -- If no linked kernel, then try and get the 'primary' kernel for the
        -- current filetype
        if not opts.id then
            -- Try to get kernel from filetype mapping
            local ft = opts.language or vim.bo[opts.buf].filetype
            opts.id = self.map_kernel_filetype[ft:lower()]
        end

        -- If we still haven't found a kernel then there isn't an active one
        -- for the buffer
        if not opts.id then
            return {}
        end
    end

    if opts.id then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.ListItem
            function(k)
                return k.id == opts.id
            end,
            kernels
        )
    end

    if opts.spec_path then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.ListItem
            function(k)
                return k.spec_path:lower():find(opts.spec_path:lower()) ~= nil
            end,
            kernels
        )
    end

    if opts.language then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.ListItem
            function(k)
                return k.spec.language:lower() == opts.language:lower()
            end,
            kernels
        )
    end

    if opts.name then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.ListItem
            function(k)
                return k.spec.display_name:lower():find(opts.name:lower()) ~= nil
            end,
            kernels
        )
    end

    if opts.status then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.ListItem
            function(k)
                if opts.status == "active" then
                    return k.id ~= nil
                elseif opts.status == "inactive" then
                    return k.id == nil
                else
                    return true
                end
            end,
            kernels
        )
    end

    return kernels
end

return manager
