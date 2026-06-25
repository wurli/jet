local engine = require("jet.core.engine")
local manager = require("jet.core.manager")
local config = require("jet.config").options

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
---@field kernel_info table
---@field session_id? string
---@field client_id? string
---@field term? jet.term
---@field connection_file? string
---@field cmd string[]
local Kernel = {}
Kernel.__index = Kernel

setmetatable(Kernel, {
	---@return jet.kernel
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@class jet.kernel.start.opts
---@field session_name? string
---@field spec jet.kernel.spec | jet.kernel.paritalspec
---@field spec_path string
---@field connection_file? string

---@param opts jet.kernel.start.opts
function Kernel.new(opts)
	opts.session_name = opts.session_name or "nvim"
	local out = setmetatable(opts, Kernel)
	out.cmd = out:connect_cmd()
	table.insert(manager.kernels, out)
	return out
end

function Kernel:connect_cmd()
	assert(self.spec_path, "Kernel spec path is not set")
	assert(not self.session_id, "Kernel session ID is already set")

	self.session_id = engine.make_session_id(self.spec.language)

	local out = {
		config.jet_binary,
		"start",
		self.spec_path,
		"--session-id",
		self.session_id,
		"--session-name",
		"nvim",
	}

	-- TODO: remove this?
	if self.connection_file then
		table.insert(out, "--connection-file")
		table.insert(out, self.connection_file)
	end

	return out
end

---@class jet.kernel_from_external.opts
---@field session_id string

---@param opts jet.kernel_from_external.opts
---@return jet.kernel
function Kernel.from_external(opts)
	assert(opts.session_id, "Kernel session ID is not set")

	local out = setmetatable({
		session_id = opts.session_id,
		spec = engine.show(opts.session_id).spec,
	}, Kernel)

	out.cmd = out:attach_cmd()

	table.insert(manager.kernels, out)
	return out
end

function Kernel:attach_cmd()
	assert(self.session_id, "Kernel session ID is not set")

	return {
		config.jet_binary,
		"attach",
		self.session_id,
		"--session-name",
		"nvim",
	}
end

function Kernel:init_term()
	---@diagnostic disable-next-line: missing-fields
	self.term = {}
	self.term.buf = vim.api.nvim_create_buf(false, true)

	if config.stop_on_exit then
		vim.api.nvim_create_autocmd("BufWipeout", {
			buffer = self.term.buf,
			group = augroup,
			callback = function()
				self:stop()
			end,
		})
	end

	vim.api.nvim_buf_call(self.term.buf, function()
		self.term.job_id = vim.fn.jobstart(self.cmd, { term = true })
	end)
end

---@param opts? vim.api.keyset.win_config
function Kernel:open_term(opts)
	opts = opts or {}
	if not self.term then
		self:init_term()
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

function Kernel:attach_lua_client()
	if not self.term then
		self:init_term()
	end
	local out = engine.attach(self.session_id, nil, "nvim")
	self.client_id = out.client_id
	self.kernel_info = out.kernel_info
end

function Kernel:stop()
	if not self.session_id then
		error("Kernel has no session id")
	end

	for i, k in ipairs(manager.kernels) do
		if k == self then
			table.remove(manager.kernels, i)
			break
		end
	end

	vim.schedule(function()
		if vim.api.nvim_buf_is_valid(self.term.buf) then
			vim.api.nvim_buf_delete(self.term.buf, { force = true })
		end
	end)

	engine.stop(self.session_id)

	vim.notify("Stopped kernel " .. self.spec.display_name)
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
