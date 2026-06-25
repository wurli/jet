local engine = require("jet.core.engine")
local kernel = require("jet.core.kernel")
local utils = require("jet.core.utils")
local manager = require("jet.core.manager")

local Api = {}

---@param choice jet.kernels.item
local start_impl = function(choice, opts)
	local k = kernel.new({
		spec_path = choice.spec_path,
		spec = choice.spec,
		connection_file = opts.connection_file,
		session_name = opts.session_name,
	})

	k:open_term()
end

---@param choice jet.kernels.item
local attach_impl = function(choice)
	if not choice.instance then
		-- Should never happen
		error("No instance to attach to")
	end

	choice.instance:open_term()
end

---@param opts jet.kernels.item
local repl_impl = function(opts)
	if opts.instance then
		attach_impl(opts)
	else
		start_impl(opts)
	end
end

---@alias jet.kernels.item { spec_path: string, spec: jet.kernel.spec, instance: jet.kernel }

--TODO: include kernels which are running but not in nvim
---@return jet.kernels.item[]
local list_kernels = function()
	local active_kernels = manager.kernels
	local all_kernels = engine.list_kernels()

	---@type table<string, { spec: jet.kernel.spec, instances: jet.kernel[] }>
	local all = {}

	for _, k in ipairs(all_kernels) do
		all[k.path] = {
			spec = k.spec,
			instances = {},
		}
	end

	for _, k in ipairs(active_kernels) do
		if not all[k.spec_path] then
			all[k.spec_path] = {
				spec = k.spec,
				instances = {},
			}
		end
		table.insert(all[k.spec_path].instances, k)
	end

	local out = {}

	for spec_path, item in pairs(all) do
		if #item.instances == 0 then
			table.insert(out, {
				spec_path = spec_path,
				spec = item.spec,
			})
		else
			for _, instance in ipairs(item.instances) do
				table.insert(out, {
					spec_path = spec_path,
					spec = item.spec,
					instance = instance,
				})
			end
		end
	end

	return out
end

---@param kernels jet.kernels.item[]
---@param opts? table
local select_kernel = function(kernels, msg, callback, opts)
	vim.ui.select(kernels, {
		prompt = msg,
		---@param item jet.kernels.item
		format_item = function(item)
			return string.format(
				"%s %s   (%s)",
				item.instance and "" or "",
				item.spec.display_name,
				utils.path_shorten(item.spec_path)
			)
		end,
	}, function(choice)
		if choice then
			callback(choice, opts)
		end
	end)
end

---@class jet.api.start.opts
---@field spec_path? string
---@field connection_file string?
---@field session_name string?
---@field persist boolean efault `true`

---Start a fresh kernel
---
---@param opts? jet.api.start.opts
Api.start = function(opts)
	opts = opts or {}

	if opts.spec_path then
		error("Passing spec path is TODO")
	end

	local kernels = list_kernels()

	if #kernels == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	else
		-- Select even if only 1 kernel available
		select_kernel(kernels, "Select a kernel to start", start_impl, opts)
	end
end

---@class jet.api.open.opts
---@field session_id string
---@field connection_file string?
---@field session_name string?
---@field persist boolean Default `true`

---Open a kernel which is already running in Neovim
---
---@param opts? jet.api.open.opts
Api.open = function(opts)
	opts = opts or {}

	if opts.session_id then
		error("Passing session id is TODO")
	end

	local running = vim.tbl_filter(function(k)
		return k.instance
	end, list_kernels())

	if #running == 0 then
		vim.notify("No running kernels to attach to", vim.log.levels.WARN)
	elseif #running == 1 then
		attach_impl(running[1])
	else
		select_kernel(running, "Select a running kernel to open", attach_impl, opts)
	end
end

---@class jet.api.repl.opts
---@field spec_path? string
---@field session_id string
---@field connection_file string?
---@field session_name string?
---@field persist boolean Default `true`

---Open a REPL for a kernel.
---
---The kernel may be running in Neovim already, running in a Jet session
---outside of Neovim, or not yet running.
---
---@param opts? jet.api.repl.opts
Api.repl = function(opts)
	opts = opts or {}

	if opts.session_id or opts.spec_path then
		error("Passing session id/spec path is TODO")
	end

	local running = list_kernels()

	if #running == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	elseif #running == 1 then
		repl_impl(running[1])
	else
		select_kernel(running, "Start a kernel or attach to a running one", repl_impl, opts)
	end
end

Api.start()

return Api
