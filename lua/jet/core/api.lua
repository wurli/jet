local kernel = require("jet.core.kernel")
local utils = require("jet.core.utils")
local manager = require("jet.core.manager")

local M = {}

---@class jet.api.list_kernels.filters
---@field session_id? string Implies `status` = "connected" or "external"
---@field spec_path? string
---@field filetype? string | boolean `true` gets the filetype at the current position
---@field display_name? string
---@field primary? boolean Implies `status` = "connected"
---@field default? boolean Only gets the default kernel for `filetype` (see `config.default_kernels`)
---@field status? jet.kernel.status | jet.kernel.status[]

---@param kernels jet.kernel[]
---@param opts? jet.api.list_kernels.filters
---@return jet.kernel[]
M.filter_kernels = function(kernels, opts)
	opts = opts or {}
	opts.status = opts.status or { "connecting", "connected", "external", "inactive" }
	opts.status = type(opts.status) == "string" and { opts.status } or opts.status
	opts.filetype = opts.filetype == true and vim.bo.filetype or opts.filetype

	---@param k jet.kernel
	return vim.tbl_filter(function(k)
		local status, _ = k:status()
		if not vim.tbl_contains(opts.status, status) then
			return false
		end

		if opts.spec_path and k.spec_path ~= opts.spec_path then
			return false
		end

		if opts.display_name and not k.spec.display_name:lower():match(opts.display_name:lower()) then
			return false
		end

		-- implies `status` = "connected" or "external"
		if opts.session_id and k.session_id ~= opts.session_id then
			return false
		end

		if opts.filetype then
			-- filetype is present for connected kernels if added through hooks,
			-- and for other kernels if explicitly configured
			if opts.filetype ~= k.filetype then
				return false
			end

			if opts.default then
				local spec_path = require("jet.config").options.default_kernels[opts.filetype]
				if type(spec_path) == "function" then
					spec_path = spec_path()
				end
				if not spec_path or not utils.path_eq(k.spec_path, spec_path) then
					return false
				end
			end
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

---@param filters jet.api.list_kernels.filters
---@param init_opts? {} | jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@return jet.kernel[]
M.list_kernels = function(filters, init_opts)
	filters = filters or {}
	filters.status = filters.status or { "connecting", "connected", "external", "inactive" }
	filters.status = type(filters.status) == "string" and { filters.status } or filters.status

	---@type jet.kernel[]
	local kernels = {}

	if vim.tbl_contains(filters.status, "connected") then
		for _, k in pairs(manager.kernels) do
			table.insert(kernels, k)
		end
	end

	if vim.tbl_contains(filters.status, "external") then
		for _, k in ipairs(require("jet.core.engine").list_sessions()) do
			-- Don't include sessions that are already connected to Neovim
			if not manager.kernels[k.session_id] then
				local init = vim.tbl_extend("keep", { session_id = k.session_id }, init_opts or {}) --[[@as jet.kernel.init_external.opts]]
				table.insert(kernels, kernel.init_external(init))
			end
		end
	end

	if vim.tbl_contains(filters.status, "inactive") then
		for _, k in ipairs(require("jet.core.engine").list_kernels()) do
			local init = vim.tbl_extend("keep", { spec_path = k.path, spec = k.spec }, init_opts or {}) --[[@as jet.kernel.init_owned.opts]]
			table.insert(kernels, kernel.init_owned(init))
		end
	end

	return M.filter_kernels(kernels, filters)
end

---@param kernels jet.kernel[]
local select_kernel = function(kernels, msg, callback)
	vim.ui.select(kernels, {
		prompt = msg,
		---@param k jet.kernel
		format_item = function(k)
			local _, status_icon = k:status()
			return string.format("%s  %s  %s", status_icon, k.spec.display_name, utils.path_shorten(k.spec_path))
		end,
	}, function(choice)
		if choice then
			callback(choice)
		end
	end)
end

---Run `callback()` on a kernel which is not yet running
---
---@param filters jet.api.list_kernels.filters
---@param init_opts {} | jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@param callback fun(k: jet.kernel)
M.get_inactive = function(filters, init_opts, callback)
	filters = filters or {}
	filters.status = { "inactive" }

	local kernels = M.list_kernels(filters, init_opts)

	if #kernels == 0 then
		vim.notify("Could not find any kernels on the system", vim.log.levels.WARN)
		return
	end

	-- Show user the choices even if only 1 kernel available
	---@param k jet.kernel
	select_kernel(kernels, "Select a kernel to start", function(k)
		callback(k)
	end)
end

---Run `callback()` on a kernel which is running but not connected to Neovim
---
---@param filters jet.api.list_kernels.filters
---@param init_opts {} | jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@param callback fun(k: jet.kernel)
M.get_external = function(filters, init_opts, callback)
	filters = filters or {}
	filters.status = { "external" }

	local external = M.list_kernels(filters, init_opts)

	if #external == 0 then
		vim.notify("No external running kernels to attach to", vim.log.levels.WARN)
		return
	end

	---@param k jet.kernel
	select_kernel(external, "Select an external kernel to open", function(k)
		callback(k)
	end)
end

---Run `callback()` on a kernel which is running and connected to Neovim
---
---@param filters? jet.api.list_kernels.filters
---@param callback fun(k: jet.kernel)
M.get_connected = function(filters, callback)
	filters = filters or {}
	filters.status = { "connected" }

	local matches = M.list_kernels(filters)

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

---Perform some action on a kernel
---
---The kernel may be running in Neovim already, running in a Jet session
---outside of Neovim, or not yet running.
---
---1. Starts with a list of all kernels, including those connected to nvim,
---   inactive kernels, and kernels running externally (e.g. in tmux or in
---   another nvim session in the same directory).
---2. If the list contains only one *running* kernel, this is passed to
---   `callback`.
---3. If the list contains only one *inactive* kernel and this kernel is marked
---   as 'default' for its filetype (see `config.default_kernels`), this kernel
---   is passed to `callback`.
---4. If neither 2 or 3 apply, the user is prompted to select a kernel via
---   `vim.ui.select()`.
---
---TODO: document available filters here
---
---@param filters jet.api.list_kernels.filters
---@param init_opts {} | jet.kernel.init_owned.opts | jet.kernel.init_external.opts
---@param callback fun(k: jet.kernel)
M.get_any = function(filters, init_opts, callback)
	local filters1 = vim.tbl_extend("keep", { status = { "connected", "connecting" } }, filters)
	local matches1 = M.list_kernels(filters1, init_opts)

	if #matches1 == 1 then
		callback(matches1[1])
		return
	end

	local inactive_kernels = M.list_kernels({ status = { "inactive" } }, init_opts)

	local filters2 = vim.tbl_extend("keep", { status = { "inactive" }, default = 2 }, filters)
	local matches2 = M.filter_kernels(inactive_kernels, filters2)

	if #matches2 == 1 then
		callback(matches2[1])
		return
	end

	if #inactive_kernels == 1 then
		callback(inactive_kernels[1])
		return
	end

	-- If we reach this point, there are multiple kernels to choose from.
	select_kernel(M.list_kernels({}, init_opts), "Select a kernel", callback)
end

return M
