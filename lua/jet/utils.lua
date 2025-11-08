local M = {}

---@class Jet.Utils.Listen.Options
---
---Polling interval in milliseconds. Default is 50.
---@field interval number
---
---@field action fun(result):
---| "exit" Terminate
---| "handle" Pass the result to `handler()`
---| "retry" Continue polling after `interval` milliseconds
---@field handler fun(result: any): any
---@field on_exit? fun(): any

---@param callback fun(): any
---@param opts Jet.Utils.Listen.Options
M.listen = function(callback, opts)
    local handler = opts.handler or function() end
    local on_exit = opts.on_exit or function() end

    local function loop()
        while true do
            local result = callback()
            local action = opts.action(result)

            if action == "exit" then
                on_exit()
                return
            elseif action == "retry" then
                return vim.defer_fn(loop, opts.interval or 50)
            elseif action == "handle" then
                -- If we've got a valid result, process it and then and then
                -- immediately (i.e. with no delay) poll again.
                handler(result)
            else
                error(("Invalid action '%s'"):format(tostring(action)))
            end
        end
    end

    loop()
end

M.ext_to_filetype = function(ext)
    if ext:sub(1, 1) == "." then
        ext = ext:sub(2)
    end
    local ft, _ = vim.filetype.match({ filename = "file." .. ext })
    return ft
end

return M
