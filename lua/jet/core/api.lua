local engine = require("jet.core.engine")
local kernel = require("jet.core.kernel")
local utils = require("jet.core.utils")
local manager = require("jet.core.manager")

local Api = {}

---@param choice jet.kernels.item
local start_impl = function(choice, opts)
	local k = kernel.init_owned({
		spec_path = choice.spec_path,
		spec = choice.spec,
		connection_file = opts.connection_file,
		session_name = opts.session_name,
	})

	k:open_term()
end

---@param choice jet.kernels.item
---@param opts? table
local open_impl = function(choice, opts)
	if not choice.connected_instance then
		-- Should never happen
		error("No instance to attach to")
	end

	choice.connected_instance:open_term()
end

---@param choice jet.kernels.item
---@param opts? table
local attach_impl = function(choice, opts)
	if not choice.external_instance then
		-- Should never happen
		error("No external instance to attach to")
	end

	local k = kernel.init_external({ session_id = choice.external_instance.session_id })
	k:open_term()
end

---@param choice jet.kernels.item
---@param opts? table
local repl_impl = function(choice, opts)
	if choice.connected_instance then
		open_impl(choice, opts)
	elseif choice.external_instance then
		attach_impl(choice, opts)
	else
		start_impl(choice, opts)
	end
end

---@class jet.kernels.item
---@field spec_path string
---@field spec jet.kernel.paritalspec | jet.kernel.spec
---@field connected_instance jet.kernel?
---@field external_instance jet.session_info?

---@return jet.kernels.item[]
local list_kernels = function()
	local connected_kernels = manager.kernels
	local active_sessions = engine.list_sessions()
	local all_kernels = engine.list_kernels()

	---@type table<string, { spec: { display_name: string, language: string }, connected_instances: jet.kernel[], external_instances: jet.session_info[] }>
	local all = {}

	-- Add all kernels to the table, even if they have no connected instances
	for _, k in ipairs(all_kernels) do
		all[k.path] = {
			spec = k.spec,
			connected_instances = {},
			external_instances = {},
		}
	end

	-- Add connected (nvim) instances
	for _, k in pairs(connected_kernels) do
		if not all[k.spec_path] then
			all[k.spec_path] = {
				spec = k.spec,
				connected_instances = {},
				external_instances = {},
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
				external_instances = {},
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
			table.insert(all[session.kernelspec_path].external_instances, session)
		end
	end

	local out = {}

	for spec_path, item in pairs(all) do
		if #item.connected_instances == 0 and #item.external_instances == 0 then
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
			for _, session in ipairs(item.external_instances) do
				table.insert(out, {
					spec_path = spec_path,
					spec = item.spec,
					external_instance = session,
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
				item.connected_instance and "" or item.external_instance and "󰺕" or "",
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
		open_impl(running[1])
	else
		select_kernel(running, "Select a running kernel to open", open_impl, opts)
	end
end

---Attach to a kernel which is running externally (not in Neovim)
---
---@param opts? jet.api.open.opts
Api.attach = function(opts)
	opts = opts or {}

	if opts.session_id then
		error("Passing session id is TODO")
	end

	local external = vim.tbl_filter(function(k)
		return k.external_instance
	end, list_kernels())

	if #external == 0 then
		vim.notify("No external running kernels to attach to", vim.log.levels.WARN)
	else
		select_kernel(external, "Select a running kernel to open", attach_impl, opts)
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

-- vim.print(engine.list_sessions({}))
Api.repl()

return Api
