local manager = require("jet.core.manager")
local config = require("jet.config").options
local utils = require("jet.core.utils")

local augroup = vim.api.nvim_create_augroup("jet.stop.term", { clear = true })

---@class jet.term
---@field job_id integer
---@field buf integer
---@field win? integer

---@alias jet.kernel.paritalspec { display_name: string, language: string }

---@class jet.kernel
---@field session_name string
---@field spec jet.kernel.spec | jet.kernel.paritalspec
---@field spec_path string
---@field kernel_info? jet.kernel.info
---@field session_id? string
---@field client_id? string
---@field term? jet.term
---@field connection_file? string
---@field cmd string[]
---@field owned boolean
---@field filetype? string
local Kernel = {}
Kernel.__index = Kernel

setmetatable(Kernel, {
	---@return jet.kernel
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param session_id string
---@param spec_path string
---@param connection_file? string
local connect_cmd = function(session_id, spec_path, connection_file)
	assert(spec_path, "Kernel spec path is not set")

	local out = {
		config.jet_binary,
		"start",
		spec_path,
		"--session-id",
		session_id,
		"--session-name",
		"nvim",
	}

	if not config.stop_on_nvim_quit then
		table.insert(out, "--persist")
	end

	-- TODO: remove this?
	if connection_file then
		table.insert(out, "--connection-file")
		table.insert(out, connection_file)
	end

	return out
end

---@param session_id string
local make_attach_cmd = function(session_id)
	return {
		config.jet_binary,
		"attach",
		session_id,
		"--session-name",
		"nvim",
	}
end

---@class jet.kernel.init_owned.opts
---@field session_name? string
---@field spec? jet.kernel.spec | jet.kernel.paritalspec
---@field spec_path string
---@field connection_file? string

---@param opts jet.kernel.init_owned.opts
function Kernel.init_owned(opts)
	if not opts.spec then
		opts.spec = require("jet.core.engine").show_spec(opts.spec_path)
	end

	local session_id = require("jet.core.engine").make_session_id(opts.spec.language)
	local obj = vim.tbl_extend("force", opts, {
		session_id = session_id,
		session_name = opts.session_name or "nvim",
		cmd = connect_cmd(session_id, opts.spec_path, opts.connection_file),
		owned = true,
	})

	return setmetatable(obj, Kernel)
end

---@class jet.kernel.init_external.opts
---@field session_id string

---@param opts jet.kernel.init_external.opts
---@return jet.kernel
function Kernel.init_external(opts)
	assert(opts.session_id, "Kernel session ID is not set")
	local view = require("jet.core.engine").show_session(opts.session_id)

	return setmetatable({
		session_id = opts.session_id,
		cmd = make_attach_cmd(opts.session_id),
		spec = view.spec,
		spec_path = view.session.kernelspec_path,
		owned = false,
	}, Kernel)
end

function Kernel:run()
	---@diagnostic disable-next-line: missing-fields
	self.term = {}
	self.term.buf = vim.api.nvim_create_buf(false, true)

	vim.api.nvim_create_autocmd("BufWipeout", {
		buffer = self.term.buf,
		group = augroup,
		callback = function()
			self:remove()
		end,
	})

	-- buf_call since the buf is not yet attached to a window.
	vim.api.nvim_buf_call(self.term.buf, function()
		self.term.job_id = vim.fn.jobstart(self.cmd, { term = true })
	end)

	-- On TermEnter, record this kernel as the last used
	-- TODO: configure whether or not this should automatically happen
	if config.auto_set_primary then
		vim.api.nvim_create_autocmd("TermEnter", {
			buffer = self.term.buf,
			group = augroup,
			callback = function()
				self:set_as_primary()
			end,
		})
	end

	-- Give the kernel a bit of time to start up
	-- TODO: find a more robust solution, e.g. watch for a session.json
	vim.defer_fn(function()
		self:attach_lua_client()
		self:try_resolve_filetype()
		manager:insert(self)
	end, 500)
end

function Kernel:set_as_primary()
	assert(self.filetype, "Kernel has no filetype")
	manager.primary[self.filetype] = self.session_id
end

function Kernel:attach_lua_client()
	if not self.term then
		self:run()
	end
	local out = require("jet.core.engine").attach(self.session_id, nil, self.session_name)
	self.client_id = out.client_id
	self.kernel_info = out.kernel_info
end

--- Can only be done after the kernel is connected and we have the kernel info,
--- since we need the file extension to resolve the filetype (kernelspec has
--- language, but this is not the same).
---
--- TODO: let the user override the filetype per-kernel
function Kernel:try_resolve_filetype()
	if
		not self.filetype
		and self.kernel_info
		and self.kernel_info.language_info
		and self.kernel_info.language_info.file_extension
	then
		local ft, _, is_fallback = vim.filetype.match({
			-- Idk if 'dummy-file' is ever gonna make a difference, felt right tho
			filename = "dummy-file" .. self.kernel_info.language_info.file_extension,
		})
		if ft and not is_fallback then
			self.filetype = ft
		end
	end
end

---@param opts? vim.api.keyset.win_config
function Kernel:open_term(opts)
	opts = opts or {}
	if not self.term then
		self:run()
	end

	self.term.win = vim.api.nvim_open_win(
		self.term.buf,
		false,
		vim.tbl_extend("force", {
			split = "right",
			style = "minimal",
		}, opts or {})
	)

	vim.wo[self.term.win].number = false
	vim.wo[self.term.win].relativenumber = false
end

function Kernel:remove()
	assert(self.session_id, "Kernel has no session id")

	manager.kernels[self.session_id] = nil

	for ft, session_id in pairs(manager.primary) do
		if session_id == self.session_id then
			manager.primary[ft] = nil
		end
	end

	vim.schedule(function()
		if vim.api.nvim_buf_is_valid(self.term.buf) then
			vim.api.nvim_buf_delete(self.term.buf, { force = true })
		end
	end)

	if self.owned and config.stop_on_buf_wipeout then
		local ok, err = pcall(require("jet.core.engine").stop, self.session_id)
		if ok then
			utils.log_info("Stopped kernel '%s'", self.spec.display_name)
		else
			utils.log_error("Failed to stop kernel '%s': %s", self.spec.display_name, vim.inspect(err))
		end
	end
end

-- ---@param code string | string[]
-- ---@param user_expressions table<string, string>?
-- function Kernel:execute(code, user_expressions)
-- 	if type(code) == "table" then
-- 		code = table.concat(code, "\n")
-- 	end
--
-- 	local callback = engine.execute_code(self.client_id, code, user_expressions or {})
-- end

return Kernel
