local engine = require("jet.core.rust")
local utils = require("jet.core.utils")

---@class Jet.Manager
---
---The `kernels` field contains info about kernels on the Neovim side, e.g.
---buffer IDs, etc. This is not necessarily an exhaustive list of all active
---kernels (although usually it is). For a complete list of active kernels, use
---the Rust engine.
---@field kernels table<string, Jet.Kernel>
local manager = {
    kernels = {},
}
manager.__index = manager

setmetatable(manager, {
    ---@return Jet.Manager
    __call = function(self, ...)
        return self.start(...)
    end
})

---@class Jet.Manager.Filter
---
---Case-insensitive Lua pattern; matched against the kernel spec path
---@field spec_path? string
---
---Case-insensitive language name (not a pattern); matched against the language
---as given in the kernel spec
---@field language? string
---
---Case-insensitive pattern; matched against the kernel display name
---@field name? string
---
---Active status
---@field status? "active" | "inactive"

---@alias Jet.Manager.Kernel {spec_path: string, spec: Jet.Kernel.Spec, instances: { id: string, instance: Jet.Kernel.Instance}}

---@param filter? Jet.Manager.Filter
---@return Jet.Manager.Kernel[]
function manager:list(filter)
    local available = engine.list_available_kernels()
    local running = engine.list_running_kernels()

    local kernels = {}

    for path, spec in pairs(available) do
        local info = {
            spec_path = path,
            spec = spec,
            instances = {}
        }

        for id, instance in pairs(running) do
            if instance.spec_path == path then
                table.insert(info.instances, {
                    id = id,
                    instance = instance,
                })
                running[id] = nil
            end
        end

        table.insert(kernels, info)
    end

    if vim.tbl_count(running) > 0 then
        utils.log_warn(
            "Some kernels from `list_running_kernels()` were not returned by `list_available_kernels()`"
        )
    end

    if filter then
        kernels = self._filter(kernels, filter)
    end

    return kernels
end

function manager:kernel_from_buf(buf)
    local kernels = self:list()

    if vim.bo[buf].filetype then
        local filtered = self._filter(kernels, {
            language = vim.bo[buf].filetype,
        })
        if vim.tbl_count(filtered) > 0 then
            kernels = filtered
        end
    end

    if vim.tbl_count(kernels) == 1 then
        return kernels[1]
    end

    return self:_select(kernels)
end

function manager:_select(kernels)
    local out
    vim.ui.select(
        kernels or self:list(),
        {
            prompt = "Select a kernel to start",
            format_item = function(item)
                local text = item.spec.display_name
                if #item.instances > 0 then
                    local s = #item.instances == 1 and "" or "s"
                    text = text .. (" (%d running instance%s)"):format(#item.instances, s)
                end
                return text
            end,
        },
        function(choice)
            out = choice
        end
    )
    return out
end

---@param kernels Jet.Manager.Kernel[]
---@param f Jet.Manager.Filter
function manager._filter(kernels, f)
    if not f then return kernels end

    if f.spec_path then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.Kernel
            function(k) return k.spec_path:lower():find(f.spec_path:lower()) ~= nil end,
            kernels
        )
    end

    if f.language then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.Kernel
            function(k) return k.spec.language:lower() == f.language:lower() end,
            kernels
        )
    end

    if f.name then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.Kernel
            function(k) return k.spec.display_name:lower():find(f.name:lower()) ~= nil end,
            kernels
        )
    end

    if f.status then
        kernels = vim.tbl_filter(
        ---@param k Jet.Manager.Kernel
            function(k)
                if f.status == "active" then
                    return vim.tbl_count(k.instances) > 0
                elseif f.status == "inactive" then
                    return vim.tbl_count(k.instances) == 0
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
