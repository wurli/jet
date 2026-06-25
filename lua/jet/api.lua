local engine = require("jet.core.engine")
local config = require("jet.config")

local Api = {}

---@param opts { path: string, spec: jet.kernel.spec }
local connect_impl = function(opts) end

---@param callback fun(choice)
local choose_kernel = function(callback)
	vim.ui.select(engine.list_kernels(), {
		prompt = "Select a kernel to start to",
		format_item = function(item)
			return string.format("%s   (%s)", item.spec.display_name, item.path)
		end,
	}, function(choice)
		if choice then
			callback(choice)
		end
	end)
end

---@class jet.api.start.opts
---@field spec_path string
---@field connection_file string?
---@field session_name string?
---@field persist boolean Default `true`

---@param opts? jet.api.start.opts
Api.start = function(opts)
	opts = opts or {}
	if not opts.spec_path then
		choose_kernel(connect_impl)
	end
end

Api.start()

return Api
