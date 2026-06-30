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

---@class jet.api.list_kernels.opts
---@field connected? boolean
---@field external? boolean | jet.kernel.init_external.opts
---@field inactive? boolean | jet.kernel.init_owned.opts

---List all kernels, connected (running in nvim), external (running in a Jet session outside of nvim), and inactive (not running).
---
---TODO: document me!
---
---@param status jet.api.list_kernels.opts
---@return jet.kernel[]
Api.list_kernels = function(status)
	status = status or { connected = true, external = true, inactive = true }

	for k, v in pairs(status) do
		if v == true then
			status[k] = {}
		end
	end

	---@type jet.kernel[]
	local out = {}

	if status.connected then
		for _, k in pairs(manager.kernels) do
			table.insert(out, k)
		end
	end

	if status.external then
		for _, k in ipairs(require("jet.core.engine").list_sessions()) do
			-- Don't include sessions that are already connected to Neovim
			if not manager.kernels[k.session_id] then
				local opts = vim.tbl_extend("keep", { session_id = k.session_id }, status.external)
				table.insert(out, kernel.init_external(opts))
			end
		end
	end

	if status.inactive then
		for _, k in ipairs(require("jet.core.engine").list_kernels()) do
			local opts = vim.tbl_extend("keep", { spec_path = k.path, spec = k.spec }, status.inactive)
			table.insert(out, kernel.init_owned(opts))
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

---@param kernels jet.kernel[]
---@param opts? jet.api.filter_kernels.opts
---@return jet.kernel[]
Api.filter_kernels = function(kernels, opts)
	opts = opts or {}
	---@param k jet.kernel
	return vim.tbl_filter(function(k)
		-- spec_path: present for all kernels
		if opts.spec_path and k.spec_path ~= opts.spec_path then
			return false
		end

		-- display_name: resent for all kernels
		if opts.display_name and not k.spec.display_name:lower():match(opts.display_name:lower()) then
			return false
		end

		-- session_id: present for connected and external kernels
		if opts.session_id and k.session_id ~= opts.session_id then
			return false
		end

		-- filetype: present for connected kernels (if we could resolve it) and for other kernels if explicitly configured
		if opts.filetype and opts.filetype ~= k.filetype then
			return false
		end

		if
			opts.primary
			and not (k.session_id and vim.tbl_contains(vim.tbl_values(manager.filetype_primary), k.session_id))
		then
			return false
		end

		return true
	end, kernels)
end

---@param kernels jet.kernel[]
---@param opts? table
local select_kernel = function(kernels, msg, callback, opts)
	vim.ui.select(kernels, {
		prompt = msg,
		---@param k jet.kernel
		format_item = function(k)
			return string.format(
				"%s  %s  %s",
				k.client_id and "" or k.session_id and "󰺕" or "",
				k.spec.display_name,
				utils.path_shorten(k.spec_path)
			)
		end,
	}, function(choice)
		if choice then
			callback(choice, opts)
		end
	end)
end

---Start a fresh kernel
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_inactive = function(opts, callback)
	opts = opts or {}

	---@diagnostic disable-next-line: assign-type-mismatch
	local kernels = Api.filter_kernels(Api.list_kernels({ inactive = opts.init_opts or true }), opts.filter)

	if #kernels == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	end

	-- Show user the choices even if only 1 kernel available
	---@param k jet.kernel
	select_kernel(kernels, "Select a kernel to start", function(k)
		k:start_lua_client(callback)
	end)
end

---Attach to a kernel which is running externally (not in Neovim)
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_external = function(opts, callback)
	opts = opts or {}

	---@diagnostic disable-next-line: assign-type-mismatch
	local external = Api.filter_kernels(Api.list_kernels({ external = opts.init_opts or true }), opts.filter)

	if #external == 0 then
		vim.notify("No external running kernels to attach to", vim.log.levels.WARN)
	else
		---@param k jet.kernel
		select_kernel(external, "Select an external kernel to open", function(k)
			k:start_lua_client(callback)
		end)
	end
end

---Get a kernel which is already running in Neovim
---
---@param opts? jet.api.get_all.opts
---@param callback fun(k: jet.kernel)
Api.get_connected = function(opts, callback)
	opts = opts or {}

	local matches = Api.filter_kernels(Api.list_kernels({ connected = true }), opts.filter)

	if #matches == 0 then
		vim.notify("No running kernels to attach to", vim.log.levels.WARN)
	elseif #matches == 1 then
		callback(matches[1])
	else
		---@param k jet.kernel
		select_kernel(matches, "Select a running kernel to open", function(k)
			callback(k)
		end)
	end
end

--- (1) try `get` (2) try `attach` (3) try `start` (4) fail
---
---@param k jet.kernel
---@param callback fun(k: jet.kernel)
local get_all_impl = function(k, callback)
	if k:status() == "connected" then
		callback(k)
	else
		k:start_lua_client(callback)
	end
end

---@class jet.api.get_all.opts
---@field init_opts? jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@field filter? jet.api.filter_kernels.opts

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

	local running = Api.filter_kernels(
		---@diagnostic disable-next-line: assign-type-mismatch
		Api.list_kernels({ connected = true, external = opts.init_opts or true, inactive = opts.init_opts or true }),
		opts.filter
	)

	if #running == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
	elseif #running == 1 then
		get_all_impl(running[1], callback)
	else
		---@param k jet.kernel
		select_kernel(running, "Open a Jupyter REPL", function(k)
			get_all_impl(k, callback)
		end)
	end
end

return Api
