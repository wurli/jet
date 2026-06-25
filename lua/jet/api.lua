local engine = require("jet.core.engine")
local config = require("jet.config")

local Api = {}

local connect_impl = function(spec_path)
	local buf = vim.api.nvim_create_buf(false, true)

	vim.api.nvim_buf_call(buf, function()
		vim.fn.jobstart({
			config.options.jet_binary,
			"connect",
			spec_path,
		}, {
			term = true,
		})
	end)

	vim.print(buf)
end

---@param callback fun(choice)
local choose_kernel = function(callback)
	vim.ui.select(engine.list_kernels(), {
		prompt = "Select a kernel to connect to",
		format_item = function(item)
			return string.format("%s   (%s)", item.spec.display_name, item.path)
		end,
	}, function(choice)
		if choice then
			callback(choice.path)
		end
	end)
end

---@class jet.api.connect.opts
---@field spec_path string
---@field connection_file string?
---@field session_name string?
---@field persist boolean Default `true`

---@param opts? jet.api.connect.opts
Api.connect = function(opts)
	opts = opts or {}
	if not opts.spec_path then
		choose_kernel(connect_impl)
	end
end

Api.connect()

return Api
