local kernel = require("jet.core.kernel")
local utils = require("jet.core.utils")
local manager = require("jet.core.manager")

local Api = {}

-- jet ui mockup
--
-- +--------------------------------------------------------------+
-- |                           Jet                                |
-- |                                                              |
-- | (<Enter>) Open (auto)  (n) New session  (x) Shut down        |
-- |                                                              |
-- | Ark R Kernel                                   (kernelspec)  |
-- |   session 1 (nvim)                            (session_id)  |
-- |   session 2 (nvim)                            (session_id)  |
-- | 󰺕  session 3 (external)                        (session_id)  |
-- |                                                              |
-- | Ipython                                        (kernelspec)  |
-- |   session 1 (nvim)                            (session_id)  |
-- |                                                              |
-- | Rust                                           (kernelspec)  |
-- |   start a new session                                       |
-- |                                                              |
-- +--------------------------------------------------------------+

---@class jet.kernels.item
---@field spec? jet.kernel.paritalspec | jet.kernel.spec
---@field spec_path? string
---@field session_id? string?
---@field connected_instance jet.kernel?

---@param type ("external" | "connected" | "inactive")[]?
---@return jet.kernels.item[]
Api.list_kernels = function(type)
	type = type or { "external", "connected", "inactive" }

	---@type jet.kernels.item[]
	local out = {}

	if vim.tbl_contains(type, "inactive") then
		for _, k in ipairs(require("jet.core.engine").list_kernels()) do
			table.insert(out, {
				spec_path = k.path,
				spec = k.spec,
			})
		end
	end

	if vim.tbl_contains(type, "connected") then
		for _, k in pairs(manager.kernels) do
			table.insert(out, {
				spec_path = k.spec_path,
				spec = k.spec,
				session_id = k.session_id,
				connected_instance = k,
			})
		end
	end

	if vim.tbl_contains(type, "external") then
		for _, k in ipairs(require("jet.core.engine").list_sessions()) do
			-- Don't include sessions that are already connected to Neovim
			if not manager.kernels[k.session_id] then
				table.insert(out, {
					spec_path = k.kernelspec_path,
					spec = { display_name = k.display_name, language = k.language },
					session_id = k.session_id,
				})
			end
		end
	end

	return out
end

---@class jet.api.filter_kernels.opts
---@field session_id? string
---@field spec_path? string
---@field filetype? string
---@field display_name? string
---@field primary? boolean

---@param kernels jet.kernels.item[]
---@param opts? jet.api.filter_kernels.opts
---@return jet.kernels.item[]
Api.filter_kernels = function(kernels, opts)
	opts = opts or {}
	---@param item jet.kernels.item
	return vim.tbl_filter(function(item)
		-- spec_path: present for all kernels
		if opts.spec_path and item.spec_path ~= opts.spec_path then
			print("spec_path mismatch")
			return false
		end

		-- display_name: resent for all kernels
		if opts.display_name and not item.spec.display_name:lower():match(opts.display_name:lower()) then
			print("display_name mismatch")
			return false
		end

		-- session_id: present for connected and external kernels
		if opts.session_id and item.session_id ~= opts.session_id then
			print("session_id mismatch")
			return false
		end

		-- filetype: present for connected kernels (if we could resolve it) and for other kernels if explicitly configured
		if opts.filetype and opts.filetype ~= (item.connected_instance and item.connected_instance.filetype or nil) then
			print("filetype mismatch")
			return false
		end

		if
			opts.primary
			and not (item.session_id and vim.tbl_contains(vim.tbl_values(manager.filetype_primary), item.session_id))
		then
			return false
		end

		return true
	end, kernels)
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

---@param opts jet.kernel.init_owned.opts
---@param callback fun(k: jet.kernel)
local get_inactive_impl = function(opts, callback)
	kernel.init_owned(opts):start_lua_client(callback)
end

---Start a fresh kernel
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_inactive = function(opts, callback)
	opts = opts or {}

	local kernels = Api.filter_kernels(Api.list_kernels({ "inactive" }), opts.filter)

	if #kernels == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	end

	-- Show user the choices even if only 1 kernel available
	---@param item jet.kernels.item
	select_kernel(kernels, "Select a kernel to start", function(item)
		get_inactive_impl({
			spec_path = item.spec_path,
			spec = item.spec,
			unpack(opts.init_opts or {}),
		}, callback)
	end)
end

---@param opts jet.kernel.init_external.opts
---@param callback fun(k: jet.kernel)
local get_external_impl = function(opts, callback)
	kernel.init_external(opts):start_lua_client(callback)
end

---Attach to a kernel which is running externally (not in Neovim)
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_external = function(opts, callback)
	opts = opts or {}

	-- TODO: fancy filter?
	local external = Api.filter_kernels(Api.list_kernels({ "external" }), opts.filter)

	if #external == 0 then
		vim.notify("No external running kernels to attach to", vim.log.levels.WARN)
	else
		---@param item jet.kernels.item
		select_kernel(external, "Select an external kernel to open", function(item)
			get_external_impl({
				session_id = item.session_id,
				unpack(opts.init_opts or {}),
			}, callback)
		end)
	end
end

---@param k jet.kernel
---@param callback fun(k: jet.kernel)
local get_connected_impl = function(k, callback)
	callback(k)
end

---Get a kernel which is already running in Neovim
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_connected = function(opts, callback)
	opts = opts or {}

	local matches = Api.filter_kernels(Api.list_kernels({ "connected" }), opts.filter)

	if #matches == 0 then
		vim.notify("No running kernels to attach to", vim.log.levels.WARN)
	elseif #matches == 1 then
		get_connected_impl(matches[1].connected_instance, callback)
	else
		---@param item jet.kernels.item
		select_kernel(matches, "Select a running kernel to open", function(item)
			get_connected_impl(item.connected_instance, callback)
		end)
	end
end

--- (1) try `get` (2) try `attach` (3) try `start` (4) fail
---
---@param choice jet.kernels.item
---@param init_opts? jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@param callback fun(k: jet.kernel)
local get_all_impl = function(choice, init_opts, callback)
	if choice.connected_instance then
		get_connected_impl(choice.connected_instance, callback)
	elseif choice.session_id then
		get_external_impl({
			-- Probs will add more fields soon
			session_id = choice.session_id,
			unpack(init_opts or {}),
		}, callback)
	elseif choice.spec_path then
		get_inactive_impl({
			spec_path = choice.spec_path,
			spec = choice.spec,
			unpack(init_opts or {}),
		}, callback)
	else
		error("Invalid input (must have `connected_instance`, `session_id` or `spec_path`): " .. vim.inspect(choice))
	end
end

---@class jet.api.get_all.opts
---@field init_opts? jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@field filter? jet.api.filter_kernels.opts
--TODO: pass startup opts

-- Rule 1: has session_id -> (1) connect (2) attach (3) fail
-- Rule 2: has spec_path -> (1) connect (2) attach (3) start (4) fail
-- Rule 3: has filetype -> (1) connect (2) attach (3) start (4) fail

---Open a REPL for a kernel.
---
---The kernel may be running in Neovim already, running in a Jet session
---outside of Neovim, or not yet running.
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_all = function(opts, callback)
	opts = opts or {}

	local running = Api.filter_kernels(Api.list_kernels(), opts.filter)

	if #running == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	elseif #running == 1 then
		get_all_impl(running[1], opts.init_opts, callback)
	else
		---@param item jet.kernels.item
		select_kernel(running, "Open a Jupyter REPL", function(item)
			get_all_impl(item, opts.init_opts, callback)
		end)
	end
end

return Api
