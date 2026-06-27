local kernel = require("jet.core.kernel")
local utils = require("jet.core.utils")
local manager = require("jet.core.manager")

local Api = {}

---@param choice jet.kernels.item
---@param opts jet.api.repl.opts
local start_impl = function(choice, opts)
	local k = kernel.init_owned({
		spec_path = choice.spec_path,
		spec = choice.spec,
		connection_file = opts.connection_file,
		session_name = opts.session_name,
	})

	if opts.hidden then
		k:start_lua_client(opts.callback)
	else
		k:open_term(opts.callback)
	end
end

---@param choice jet.kernels.item
---@param opts jet.api.repl.opts
local open_impl = function(choice, opts)
	if choice.connected_instance then
		if not opts.hidden then
			choice.connected_instance:open_term()
		end
	else
		error("No instance to attach to")
	end
end

---@param choice jet.kernels.item
---@param opts jet.api.repl.opts
local attach_impl = function(choice, opts)
	if not choice.session_id then
		-- Should never happen
		error("No external instance to attach to")
	end

	local k = kernel.init_external({ session_id = choice.session_id })

	if opts.hidden then
		k:start_lua_client()
	else
		k:open_term()
	end
end

---@param choice jet.kernels.item
---@param opts jet.api.repl.opts
local repl_impl = function(choice, opts)
	if choice.connected_instance then
		open_impl(choice, opts)
	elseif choice.session_id then
		attach_impl(choice, opts)
	else
		start_impl(choice, opts)
	end
end

---@class jet.kernels.item
---@field spec_path? string
---@field spec? jet.kernel.paritalspec | jet.kernel.spec
---@field connected_instance jet.kernel?
---@field session_id? string?

---@return jet.kernels.item[]
local list_kernels = function()
	local connected_kernels = manager.kernels
	local active_sessions = require("jet.core.engine").list_sessions()
	local all_kernels = require("jet.core.engine").list_kernels()

	---@type table<string, { spec: { display_name: string, language: string }, connected_instances: jet.kernel[], session_ids: jet.session_info[] }>
	local all = {}

	-- Add all kernels to the table, even if they have no connected instances
	for _, k in ipairs(all_kernels) do
		all[k.path] = {
			spec = k.spec,
			connected_instances = {},
			session_ids = {},
		}
	end

	-- Add connected (nvim) instances
	for _, k in pairs(connected_kernels) do
		if not all[k.spec_path] then
			all[k.spec_path] = {
				spec = k.spec,
				connected_instances = {},
				session_ids = {},
			}
		end
		table.insert(all[k.spec_path].connected_instances, k)
	end

	-- Add external (non-nvim) instances
	for _, session in ipairs(active_sessions) do
		if not all[session.kernelspec_path] then
			all[session.kernelspec_path] = {
				spec = { display_name = session.display_name, language = session.language },
				connected_instances = {},
				session_ids = {},
			}
		end
		-- connected_kernels also includes any nvim ones, so we need to filter these out
		local session_is_nvim = false
		for _, instance in ipairs(all[session.kernelspec_path].connected_instances) do
			if instance.session_id == session.session_id then
				session_is_nvim = true
				break
			end
		end
		if not session_is_nvim then
			table.insert(all[session.kernelspec_path].session_ids, session.session_id)
		end
	end

	local out = {}

	for spec_path, item in pairs(all) do
		if #item.connected_instances == 0 and #item.session_ids == 0 then
			table.insert(out, {
				spec_path = spec_path,
				spec = item.spec,
			})
		else
			for _, instance in ipairs(item.connected_instances) do
				table.insert(out, {
					spec_path = spec_path,
					spec = item.spec,
					connected_instance = instance,
				})
			end
			for _, session_id in ipairs(item.session_ids) do
				table.insert(out, {
					spec_path = spec_path,
					spec = item.spec,
					session_id = session_id,
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
				"%s  %s  %s",
				item.connected_instance and "" or item.session_id and "󰺕" or "",
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
---@field filetype? string
---@field hidden boolean

---Start a fresh kernel
---
---@param opts? jet.api.start.opts
Api.start = function(opts)
	opts = opts or {}

	if opts.filetype then
		local spec_path = require("jet.config").options.default_kernels[opts.filetype]
		assert(spec_path, string.format("No default kernel configured for filetype %s", opts.filetype))
		opts.spec_path = spec_path
	end

	if opts.spec_path then
		return kernel.init_owned({ spec_path = opts.spec_path }):open_term()
	end

	local kernels = list_kernels()

	if #kernels == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	else
		-- Show user the choices even if only 1 kernel available
		select_kernel(kernels, "Select a kernel to start", start_impl, opts)
	end
end

---@class jet.api.open.opts
---@field session_id string
---@field connection_file string?
---@field session_name string?
---@field persist boolean Default `true`
---@field filetype? string --- Must be present in config default_kernels
---@field hidden boolean

---Open a kernel which is already running in Neovim
---
---@param opts? jet.api.repl.opts
Api.open = function(opts)
	opts = opts or {}

	if opts.filetype then
		local spec_path = require("jet.config").options.default_kernels[opts.filetype]
		assert(spec_path, string.format("No default kernel configured for filetype %s", opts.filetype))
		for _, session in ipairs(require("jet.core.engine").list_sessions()) do
			if vim.fn.simplify(session.kernelspec_path) == vim.fn.simplify(spec_path) then
				opts.session_id = session.session_id
			end
		end
	end

	if opts.session_id then
		error("Passing session id is TODO")
	end

	local running = vim.tbl_filter(function(k)
		return k.instance
	end, list_kernels())

	if #running == 0 then
		vim.notify("No running kernels to attach to", vim.log.levels.WARN)
	elseif #running == 1 then
		open_impl(running[1], opts)
	else
		select_kernel(running, "Select a running kernel to open", open_impl, opts)
	end
end

---Attach to a kernel which is running externally (not in Neovim)
---
---@param opts? jet.api.open.opts
Api.attach = function(opts)
	opts = opts or {}

	-- TODO: remove duplicated code
	if opts.filetype then
		local spec_path = require("jet.config").options.default_kernels[opts.filetype]
		assert(spec_path, string.format("No default kernel configured for filetype %s", opts.filetype))
		for _, session in ipairs(require("jet.core.engine").list_sessions()) do
			if vim.fn.simplify(session.kernelspec_path) == vim.fn.simplify(spec_path) then
				opts.session_id = session.session_id
			end
		end
	end

	if opts.session_id then
		error("Passing session id is TODO")
	end

	local external = vim.tbl_filter(function(k)
		return k.session_id
	end, list_kernels())

	if #external == 0 then
		vim.notify("No external running kernels to attach to", vim.log.levels.WARN)
	else
		select_kernel(external, "Select a running kernel to open", attach_impl, opts)
	end
end

---@class jet.api.repl.opts
---@field spec_path? string
---@field session_id? string
---@field connection_file? string
---@field session_name? string
---@field persist? boolean Default `true`
---@field filetype? string --- Must be present in config default_kernels
---@field hidden? boolean If `true`, do not open a terminal window right away.
---@field callback? fun(k: jet.kernel)

---Open a REPL for a kernel.
---
---The kernel may be running in Neovim already, running in a Jet session
---outside of Neovim, or not yet running.
---
---@param opts? jet.api.repl.opts
Api.repl = function(opts)
	opts = opts or {}

	if opts.filetype then
		local spec_path = require("jet.config").options.default_kernels[opts.filetype]
		assert(spec_path, string.format("No default kernel configured for filetype %s", opts.filetype))
		opts.spec_path = spec_path
	end

	if opts.session_id then
		return attach_impl({ session_id = opts.session_id }, opts)
	elseif opts.spec_path then
		return start_impl({ spec_path = opts.spec_path }, opts)
	end

	local running = list_kernels()

	if #running == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	elseif #running == 1 then
		repl_impl(running[1], opts)
	else
		select_kernel(running, "Start a kernel or attach to a running one", repl_impl, opts)
	end
end

return Api
