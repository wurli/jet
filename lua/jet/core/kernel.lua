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
---@field comms table<string, string> comm_name -> id
local Kernel = {}
Kernel.__index = Kernel

setmetatable(Kernel, {
	---@return jet.kernel
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@class jet.kernel.init_owned.opts
---@field spec_path string
---@field session_name? string
---@field spec? jet.kernel.spec | jet.kernel.paritalspec
---@field connection_file? string

---@param opts jet.kernel.init_owned.opts
function Kernel.init_owned(opts)
	if not opts.spec then
		opts.spec = require("jet.core.engine").show_spec(opts.spec_path)
	end

	local obj = vim.tbl_extend("force", opts, {
		session_name = opts.session_name or "nvim",
		owned = true,
		comms = {},
	})

	local out = setmetatable(obj, Kernel)

	for _, hook in ipairs(config.hooks.on_kernel_init) do
		hook(out)
	end

	return out
end

---@class jet.kernel.init_external.opts
---@field session_id string

---@param opts jet.kernel.init_external.opts
---@return jet.kernel
function Kernel.init_external(opts)
	assert(opts.session_id, "Kernel session ID is not set")
	local view = require("jet.core.engine").show_session(opts.session_id)

	local out = setmetatable({
		session_id = opts.session_id,
		spec = view.spec,
		spec_path = view.session.kernelspec_path,
		owned = false,
		comms = {},
	}, Kernel)

	for _, hook in ipairs(config.hooks.on_kernel_init) do
		hook(out)
	end

	return out
end

---@param session_id string
local make_attach_cmd = function(session_id)
	return {
		config.jet_binary,
		"attach",
		session_id,
		"--banner",
		"--session-name",
		"nvim",
	}
end

---@param callback? fun(k: jet.kernel)
---@param win_config? vim.api.keyset.win_config
function Kernel:open_term(callback, win_config)
	local open = function()
		self.term.win = vim.api.nvim_open_win(
			self.term.buf,
			false,
			vim.tbl_extend("force", {
				split = "right",
				style = "minimal",
			}, win_config or config.repl_win_opts or {})
		)

		vim.wo[self.term.win].number = false
		vim.wo[self.term.win].relativenumber = false

		if callback then
			callback(self)
		end
	end

	if self.term then
		open()
	else
		self:create_term(open)
	end
end

---@param callback? fun(k: jet.kernel)
function Kernel:create_term(callback)
	local connect = function()
		---@diagnostic disable-next-line: missing-fields
		self.term = { buf = vim.api.nvim_create_buf(false, true) }

		--TODO: document this
		vim.b[self.term.buf].jet = { session_id = self.session_id }

		vim.api.nvim_create_autocmd("BufWipeout", {
			buffer = self.term.buf,
			group = augroup,
			callback = function()
				self:remove()
			end,
		})

		-- buf_call since the buf is not yet attached to a window.
		vim.api.nvim_buf_call(self.term.buf, function()
			self.term.job_id = vim.fn.jobstart(make_attach_cmd(self.session_id), { term = true })
		end)

		-- On TermEnter, record this kernel as the last used
		-- TODO: configure whether or not this should automatically happen
		if config.auto_set_primary then
			vim.api.nvim_create_autocmd("TermEnter", {
				buffer = self.term.buf,
				group = augroup,
				callback = function()
					self:set_as_filetype_primary()
				end,
			})
		end

		if callback then
			callback(self)
		end
	end

	if self.client_id then
		connect()
	else
		self:start_lua_client(connect)
	end
end

---@alias jet.kernel.status "connected" | "attached" | "inactive"

---@return jet.kernel.status
function Kernel:status()
	if self.client_id then
		return "connected"
	elseif self.session_id then
		return "attached"
	elseif self.spec_path then
		return "inactive"
	else
		error("Kernel has neither client_id, session_id or spec_path: " .. vim.inspect(self))
	end
end

function Kernel:set_as_filetype_primary()
	assert(self.filetype, "Kernel has no filetype")
	manager.filetype_primary[self.filetype] = self.session_id
end

---@return boolean
function Kernel:has_lua_client()
	return self.client_id ~= nil
end

---@param callback? fun(k: jet.kernel)
function Kernel:start_lua_client(callback)
	if self:has_lua_client() then
		return
	end

	local cb
	if self.owned then
		assert(self.spec_path, "Kernel spec_path is not set")
		cb = require("jet.core.engine").start(self.spec_path, self.connection_file, self.session_name)
	else
		assert(self.session_id, "Kernel session_id is not set")
		cb = require("jet.core.engine").attach(self.session_id, nil, self.session_name)
	end

	---@param res jet.init.response?
	utils.poll(cb, function(res)
		if res.status == "ready" then
			self.session_id = res.session_id
			self.client_id = res.client_id
			self.kernel_info = res.kernel_info
			self.stream = res.stream

			manager:insert(self)

			-- Try resolving filetype after kernel started autocmd so the user
			-- has a chance to override it.
			self:try_resolve_filetype()

			-- Even though the kernel has not yet been shown in a REPL, if
			-- there isn't another kernel for this filetype already set as
			-- primary we should set this one for convenience.
			if self.filetype and not manager.filetype_primary[self.filetype] then
				self:set_as_filetype_primary()
			end

			for _, hook in ipairs(config.hooks.on_lua_client_start) do
				hook(self)
			end

			if callback then
				callback(self)
			end
			return "exit"
		else
			return "wait"
		end
	end, { interval = 30 })
end

-- function Kernel:attach_lua_client()
-- 	if not self.term then
-- 		self:run()
-- 	end
-- 	local out = require("jet.core.engine").attach(self.session_id, nil, self.session_name)
-- 	self.client_id = out.client_id
-- 	self.kernel_info = out.kernel_info
-- end

--- Can only be done after the kernel is connected and we have the kernel info,
--- since we need the file extension to resolve the filetype (kernelspec has
--- language, but this is not the same).
---
--- TODO: let the user override the filetype per-kernel
function Kernel:try_resolve_filetype()
	if self.filetype then
		return
	end
	if self.kernel_info and self.kernel_info.language_info and self.kernel_info.language_info.file_extension then
		local ft, _, is_fallback = vim.filetype.match({
			-- Idk if 'dummy-file' is ever gonna make a difference, felt right tho
			filename = "dummy-file" .. self.kernel_info.language_info.file_extension,
		})
		if ft and not is_fallback then
			self.filetype = ft
		end
	else
		--TODO: advertise autocmd help page as a way to override this!
		utils.log_warn("Could not resolve filetype for kernel '%s'.", self.spec.display_name, self.session_id)
	end
end

function Kernel:remove()
	assert(self.session_id, "Kernel has no session id")

	manager.kernels[self.session_id] = nil

	for ft, session_id in pairs(manager.filetype_primary) do
		if session_id == self.session_id then
			manager.filetype_primary[ft] = nil
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

---@class jet.kernel.comm_open.opts
---@field listener? fun(res: jet.kernel.response)
---@field listener_interval? number In milliseconds, default 50ms

---@param name string
---@param data? table
---@param opts? jet.kernel.comm_open.opts
---@return string comm_id
function Kernel:comm_open(name, data, opts)
	assert(self.client_id, "Kernel has no client id")
	local comm_id, _ = require("jet.core.engine").comm_open(self.client_id, name, data or {})

	self.comms[name] = comm_id

	opts = opts or {}

	if opts.listener then
		local get_comm_msg = require("jet.core.engine").comm_listen(self.client_id, comm_id)

		---@param res? jet.kernel.response
		utils.poll(get_comm_msg, function(res)
			if not res then
				-- The comm has been closed, so stop polling
				return "exit"
			elseif res.status == "busy" then
				opts.listener(res)
				return "continue"
			else
				return "wait"
			end
		end, { interval = opts.listener_interval })
	end

	return comm_id
end

---@class jet.kernel.listen.opts : jet.listen.opts
---This function can return:
--- - `"wait"`: The listener will be called again after the `interval`
--- - `"continue"`: The listener will be called again immediately
--- - `"exit"`: Stop listening
---@field listener fun(res: jet.kernel.response): "wait" | "continue" | "exit"
---@field interval? number In milliseconds, default 50ms

---@param opts jet.kernel.listen.opts
function Kernel:listen(opts)
	local listener = require("jet.core.engine").listen(self.client_id, opts or {})
	utils.poll(listener, opts.listener, { interval = opts.interval })
end

---@param comm_id string
---@param data table
function Kernel:comm_send(comm_id, data)
	require("jet.core.engine").comm_send(self.client_id, comm_id, data)
end

---@param code string | string[]
function Kernel:send_repl(code)
	assert(self.term and self.term.job_id, "Kernel has no repl job id")

	-- Wrap in a bracketed-paste sequence so the REPL on the other end
	-- accumulates the whole block as one cell instead of evaluating each
	-- line separately, then submit with a single CR (Enter, in raw mode).
	-- This is exactly what a terminal emits on Cmd/Ctrl+V — works with
	-- any REPL that honors bracketed paste.
	if type(code) == "table" then
		code = table.concat(code, "\n")
	end

	code = code:gsub("\n-$", "")

	-- We use bracketed paste so the Jet REPL knows not to evaluate the code
	-- until the end of the paste. This matches behaviour of Positron.
	-- TODO: make this configurable?
	local bracketed_paste_start = "\x1b[200~"
	local bracketed_paste_end = "\x1b[201~"

	local payload = bracketed_paste_start .. code .. bracketed_paste_end .. "\r"

	vim.fn.chansend(self.term.job_id, payload)
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
