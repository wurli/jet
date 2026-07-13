local manager = require("jet.core.manager")
local config = require("jet.config")
local utils = require("jet.core.utils")

local augroup = vim.api.nvim_create_augroup("jet.stop.term", { clear = true })

local STARTING_KERNEL_SENTINEL = "<pending>"

---@class jet.term
---@field job_id integer
---@field buf integer

---@alias jet.kernel.paritalspec { display_name: string, language: string }
---@alias jet.kernel.execution_state "busy" | "idle" | "starting"

---@class jet.kernel
---@field session_name string
---@field spec jet.kernel.spec | jet.kernel.paritalspec
---@field spec_path string
---@field kernel_info? jet.kernel.info
---@field session_id? string
---@field session_info? jet.session_info
---@field client_id? string
---@field term? jet.term
---@field cmd string[]
---@field owned boolean
---@field filetype? string
---@field execution_state? jet.kernel.execution_state
---@field curr_execution_start_time? integer
---@field comms table<string, string> comm_name -> id
local Kernel = {}
Kernel.__index = Kernel

---@class jet.kernel.init_owned.opts
---@field spec_path string
---@field session_name? string
---@field spec? jet.kernel.spec | jet.kernel.paritalspec

---Represents a kernel which is not active. Turn it into an 'owned'/connected
---kernel using `Kernel:start_lua_client()` or `Kernel:open_term()`.
---
---@param opts jet.kernel.init_owned.opts
function Kernel.init_owned(opts)
	if not opts.spec then
		opts.spec = require("jet.core.engine").show_spec(opts.spec_path)
	end

	local obj = vim.tbl_extend("force", opts, {
		owned = true,
		on_msg_hooks = {},
		comms = {},
	})

	local out = setmetatable(obj, Kernel)
	Kernel.try_resolve_filetype(out)

	for _, hook in ipairs(config.options.hooks.on_kernel_init) do
		hook(out)
	end

	return out
end

---@class jet.kernel.init_external.opts
---@field session_id string

---@param opts jet.kernel.init_external.opts
---@return jet.kernel
function Kernel.init_external(opts)
	---@diagnostic disable-next-line: unnecessary-assert
	assert(opts.session_id, "Kernel session ID is not set")
	local view = require("jet.core.engine").show_session(opts.session_id)

	local out = setmetatable({
		session_id = opts.session_id,
		spec = view.spec,
		spec_path = view.session.kernelspec_path,
		session_info = view.session,
		owned = false,
		on_msg_hooks = {},
		comms = {},
	}, Kernel)

	Kernel.try_resolve_filetype(out)

	for _, hook in ipairs(config.options.hooks.on_kernel_init) do
		hook(out)
	end

	return out
end

---@private
---@return integer?
function Kernel:get_win()
	if not self.term or not self.term.buf then
		return nil
	end

	return vim.tbl_filter(function(w)
		return vim.api.nvim_win_get_buf(w) == self.term.buf
	end, vim.api.nvim_tabpage_list_wins(0))[1]
end

function Kernel:toggle_term()
	local term_win = self:get_win()

	if term_win then
		vim.api.nvim_win_close(term_win, true)
	else
		self:open_term()
	end
end

---@param callback? fun(k: jet.kernel)
---@param win_config? vim.api.keyset.win_config
function Kernel:open_term(callback, win_config)
	local open = function()
		assert(self.term, "kernel.term is nil")

		local term_win = self:get_win()

		if term_win then
			vim.api.nvim_set_current_win(term_win)
			vim.cmd.startinsert()
		else
			local opts = vim.tbl_extend("keep", win_config or config.options.repl_win_opts or {}, {
				split = "right",
				style = "minimal",
			})

			term_win = vim.api.nvim_open_win(self.term.buf, false, opts)

			-- When the cursor is at the bottom of the REPL you get aut-scroll when
			-- new lines appear. This is a good state to start in.
			vim.api.nvim_win_set_cursor(term_win, { vim.api.nvim_buf_line_count(self.term.buf), 0 })
		end

		vim.wo[term_win].number = false
		vim.wo[term_win].relativenumber = false

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
		local term_buf = vim.api.nvim_create_buf(false, true)

		--TODO: document this
		vim.b[term_buf].jet = { session_id = self.session_id }

		vim.api.nvim_create_autocmd("BufWipeout", {
			buffer = term_buf,
			group = augroup,
			callback = function()
				self:close()
			end,
		})

		-- buf_call since the buf is not yet attached to a window.
		vim.api.nvim_buf_call(term_buf, function()
			assert(self.session_id, "Kernel has no session id")
			local term_job_id = vim.fn.jobstart({
				config.data.jet_binary_path,
				"attach",
				self.session_id,
				"--banner",
				"--session-name",
				"nvim",
				config.options.send.send_by_expr and "--no-indent" or nil,
			}, {
				term = true,
				on_exit = function()
					-- TODO: perhaps we don't want this - e.g. a kernel crashes
					-- and suddenly all the info from the console is gone. For
					-- now it's convenient, but maybe review in future or add
					-- config.
					self:delete_term_buffer()
				end,
			})
			self.term = { job_id = term_job_id, buf = term_buf }
		end)

		-- It seems that jobstart() also sets the buf name, so this has to be
		-- done afterwards.
		vim.api.nvim_buf_set_name(term_buf, self.spec.display_name)

		-- On TermEnter, record this kernel as the last used
		-- TODO: configure whether or not this should automatically happen
		if config.options.auto_set_primary and self.term then
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

---@alias jet.kernel.status "connecting" | "connected" | "external" | "inactive"

---@return jet.kernel.status, string
function Kernel:status()
	if self.client_id == STARTING_KERNEL_SENTINEL then
		return "connecting", "󰪤 "
	elseif self.client_id then
		return "connected", "󰪥 "
	elseif self.session_id then
		return "external", "󰺕 "
	else
		return "inactive", " "
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

function Kernel:handle_stream()
	---@param msg jet.jupyter.msg
	local update_execution_state = function(msg)
		local new_state = msg.content and msg.content.execution_state
		if not new_state then
			return
		end
		if not vim.tbl_contains({ "idle", "busy", "starting" }, new_state) then
			utils.log_warn("Kernel '%s' sent unknown execution state: %s", self.spec.display_name, new_state)
			return
		end

		self.execution_state = new_state
		self.curr_execution_start_time = new_state == "busy" and os.time() or nil

		if self.term and self.term.buf and vim.api.nvim_buf_is_valid(self.term.buf) then
			local jet_b = vim.b[self.term.buf].jet or {}
			jet_b.execution_state = self.execution_state
			jet_b.curr_execution_start_time = self.curr_execution_start_time
			vim.b[self.term.buf].jet = jet_b
			vim.api.nvim__redraw({ statusline = true, buf = self.term.buf })
		end

		for _, hook in pairs(config.options.hooks.on_execution_state) do
			hook(self, new_state)
		end
	end

	---@param res jet.kernel.response
	utils.poll(self.stream, function(res)
		if not res then
			return "exit"
		elseif res.status == "busy" then
			update_execution_state(res.msg)
			for _, hook in pairs(config.options.hooks.on_message) do
				hook(self, res.msg)
			end
			return "continue"
		else
			return "wait"
		end
	end)
end

---@param callback? fun(k: jet.kernel)
function Kernel:start_lua_client(callback)
	if self:has_lua_client() then
		if callback then
			callback(self)
		end
		return
	end

	local cb
	if self.owned then
		---@diagnostic disable-next-line: unnecessary-assert
		assert(self.spec_path, "Kernel spec_path is not set")
		cb, self.session_info = require("jet.core.engine").start(self.spec_path, nil)

		assert(self.session_info, "Kernel did not return session info")
		self.session_id = self.session_info.session_id

		self.client_id = STARTING_KERNEL_SENTINEL
		---@diagnostic disable-next-line: unnecessary-assert
		assert(self.session_id, "Kernel did not return a session id")
	else
		assert(self.session_id, "Kernel session_id is not set")
		cb, self.session_info = require("jet.core.engine").attach(self.session_id, nil)
	end

	manager:insert(self)

	--TODO: stop poll on kernel close
	---@param res jet.init.response?
	utils.poll(cb, function(res)
		if not res then
			return "exit"
		end
		if res.status == "ready" then
			self.client_id = res.client_id
			self.kernel_info = res.kernel_info
			self.stream = res.stream

			-- Try resolving filetype after kernel started autocmd so the user
			-- has a chance to override it.
			self:try_resolve_filetype()

			-- Even though the kernel has not yet been shown in a REPL, if
			-- there isn't another kernel for this filetype already set as
			-- primary we should set this one for convenience.
			if self.filetype and not manager.filetype_primary[self.filetype] then
				self:set_as_filetype_primary()
			end

			for _, hook in ipairs(config.options.hooks.on_lua_client_start) do
				hook(self)
			end

			self:handle_stream()

			if callback then
				callback(self)
			end
			return "exit"
		else
			return "wait"
		end
	end, { interval = 30 })
end

--- Can only be done after the kernel is connected and we have the kernel info,
--- since we need the file extension to resolve the filetype (kernelspec has
--- language, but this is not the same).
---
--- TODO: let the user override the filetype per-kernel
function Kernel:try_resolve_filetype()
	if self.filetype then
		return
	end
	local shorten = require("jet.core.utils").path_shorten
	for ft, default_spec in pairs(config.options.default_kernels) do
		local s = type(default_spec) == "string" and default_spec or default_spec()
		if s and shorten(s) == shorten(self.spec_path) then
			self.filetype = ft
			return
		end
	end

	if self.kernel_info then
		---@diagnostic disable-next-line: unnecessary-if
		if self.kernel_info.language_info and self.kernel_info.language_info.file_extension then
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
end

function Kernel:delete_term_buffer()
	vim.schedule(function()
		if self.term and self.term.buf then
			if vim.api.nvim_buf_is_valid(self.term.buf) then
				pcall(vim.api.nvim_buf_delete, self.term.buf, { force = true })
			end
		end
	end)
end

function Kernel:close()
	assert(self.session_id, "Kernel has no session id")

	manager.kernels[self.session_id] = nil

	for ft, session_id in pairs(manager.filetype_primary) do
		if session_id == self.session_id then
			manager.filetype_primary[ft] = nil
		end
	end

	self:delete_term_buffer()

	if self.owned and config.options.stop_on_buf_wipeout then
		local ok, err = pcall(require("jet.core.engine").stop, self.session_id)
		if ok then
			utils.log_info("Stopped kernel '%s'", self.spec.display_name)
		else
			utils.log_error("Failed to stop kernel '%s': %s", self.spec.display_name, vim.inspect(err))
		end
	end

	for _, hook in ipairs(config.options.hooks.on_kernel_close) do
		hook(self)
	end
end

---@class jet.kernel.comm_open.opts
---@field listener? fun(res: jet.jupyter.msg)
---@field listener_interval? integer In milliseconds, default 50ms

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
				opts.listener(res.msg)
				return "continue"
			else
				return "wait"
			end
		end, { interval = opts.listener_interval })
	end

	return comm_id
end

-- NOTE: we might need this one day but not now, so commenting until a clear
-- use case arises.
--
-- ---@class jet.kernel.listen.opts : jet.listen.opts
-- ---This function can return:
-- --- - `"wait"`: The listener will be called again after the `interval`
-- --- - `"continue"`: The listener will be called again immediately
-- --- - `"exit"`: Stop listening
-- ---@field listener fun(res: jet.kernel.response): "wait" | "continue" | "exit"
-- ---@field interval? number In milliseconds, default 50ms
--
-- ---@param opts jet.kernel.listen.opts
-- function Kernel:listen(opts)
-- 	local listener = require("jet.core.engine").listen(self.client_id, opts or {})
-- 	utils.poll(listener, opts.listener, { interval = opts.interval })
-- end

---@param comm_id string
---@param data table
function Kernel:comm_send(comm_id, data)
	assert(self.client_id)
	require("jet.core.engine").comm_send(self.client_id, comm_id, data)
end

---@param code string | string[]
function Kernel:send_repl(code)
	assert(self.term and self.term.job_id, "Kernel has no repl job id")
	if type(code) == "string" then
		code = vim.split(code, "[\n\r]", { plain = false })
	end

	-- Remove trailing empty lines
	for i = #code, 1, -1 do
		if code[i] == "" then
			table.remove(code, i)
		else
			break
		end
	end

	-- Allow the user to modify the code before we send it. This is
	-- particularly helpful, e.g. for ipython, which requires an extra newline
	-- at the end of statements which end on an indented line in order to be
	-- actually sent to the kernel (otherwise you get the continuation prompt
	-- '+ ...').
	for _, hook in ipairs(config.options.hooks.on_send_pre) do
		hook(self, code)
	end

	-- Wrap in a bracketed-paste sequence so the REPL on the other end
	-- accumulates the whole block as one cell instead of evaluating each
	-- line separately, then submit with a single CR (Enter, in raw mode).
	-- This is exactly what a terminal emits on Cmd/Ctrl+V — works with
	-- any REPL that honors bracketed paste.
	---@diagnostic disable-next-line: param-type-mismatch
	code = table.concat(code, "\r")

	-- We use bracketed paste so the Jet REPL knows not to evaluate the code
	-- until the end of the paste. This matches behaviour of Positron.
	if not config.options.send.send_by_expr then
		-- https://en.wikipedia.org/wiki/Bracketed-paste#Description_of_bracketed-paste
		local bracketed_paste_start = "\x1b[200~"
		local bracketed_paste_end = "\x1b[201~"
		code = bracketed_paste_start .. code .. bracketed_paste_end
	end

	vim.fn.chansend(self.term.job_id, code .. "\r")
end

---Send code to the kernel via the Lua client.
---
---TODO: document difference between repl and lua clients.
---
---@param code string | string[]
---@param silent boolean
---@param callback? fun(res: jet.kernel.response)
function Kernel:send_lua(code, silent, callback)
	assert(self.client_id, "Kernel has no client id")
	if type(code) == "table" then
		code = table.concat(code, "\n")
	end
	local responder = require("jet.core.engine").execute_code(self.client_id, code, silent, true, {})

	if not callback then
		return
	end

	utils.poll(responder, function(res)
		if not res then
			return "exit"
		end
		if res.status == "busy" then
			callback(res)
			return "continue"
		else
			return "wait"
		end
	end, { interval = 30 })
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
