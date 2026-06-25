local engine = require("jet.core.engine")
local manager = require("jet.core.manager")
local config = require("jet.config")

---@class jet.term
---@field job_id integer
---@field buf integer
---@field win? integer

---@class jet.kernel
---@field spec jet.kernel.spec
---@field spec_path string
---@field kernel_info table
---@field session_id? string
---@field client_id? string
---@field term? jet.term
local Kernel = {}
Kernel.__index = Kernel

setmetatable(Kernel, {
	---@return jet.kernel
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param spec jet.kernel.spec
---@param spec_path string
function Kernel.new(spec, spec_path)
	local out = setmetatable({
		spec = spec,
		spec_path = spec_path,
	}, Kernel)
	table.insert(manager.kernels, out)
	return out
end

function Kernel:connect_cmd()
	assert(self.spec_path, "Kernel spec path is not set")
	assert(self.session_id, "Kernel session ID is not set")

	return {
		config.options.jet_binary,
		"start",
		self.spec_path,
		"--session-id",
		self.session_id,
		"--session-name",
		"nvim",
	}
end

function Kernel:attach_cmd()
	assert(self.session_id, "Kernel session ID is not set")

	return {
		config.options.jet_binary,
		"attach",
		self.session_id,
		"--session-name",
		"nvim",
	}
end

function Kernel:init_term(buf)
	if self.session_id then
		error("TODO: implement attach")
	end

	self.session_id = engine.make_session_id(self.spec.language)

	---@diagnostic disable-next-line: missing-fields
	self.term = {}
	self.term.buf = buf or vim.api.nvim_create_buf(false, true)

	vim.api.nvim_buf_call(self.term.buf, function()
		self.term.job_id = vim.fn.jobstart(self:connect_cmd(), { term = true })
	end)
end

---@param opts vim.api.keyset.win_config
function Kernel:open_term(opts)
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

function Kernel:attach()
	if not self.term then
		self:init_term()
	end
	local out = engine.attach(self.session_id, nil, "nvim")
	self.client_id = out.client_id
	self.kernel_info = out.kernel_info
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
