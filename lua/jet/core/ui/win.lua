---@class Jet.Ui.Win
---@field win integer?
---@field buf integer?
---@field cfg vim.api.keyset.win_config
---@field bo table<string, any>
---@field b table<string, any>
---@field wo table<string, any>
local win = {}
win.__index = win

setmetatable(win, {
	---@return Jet.Ui.Win
	__call = function(self, ...)
		return self.init(...)
	end,
})

---@param opts? { show?: boolean, cfg?: vim.api.keyset.win_config, bo?: table<string, any>, b?: table<string, any>, wo?: table<string, any> }
---@return Jet.Ui.Win
function win.init(opts)
	local self = setmetatable({}, win)
	opts = opts or {}
	self:set_bo(opts.bo)
	self:set_b(opts.b)
	self:set_wo(opts.wo)
	self:set_cfg(opts.cfg)

	if opts.show == nil or opts.show then
		self:show()
	end

	return self
end

---@param enter? boolean Default `false`
function win:show(enter)
	if self:is_visible() then
		vim.api.nvim_win_set_config(self.win, self.cfg)
	else
		self:create_buf()
		self.win = vim.api.nvim_open_win(self.buf, enter or false, self.cfg)
	end

	self:set_wo()
end

---@return boolean
function win:is_visible()
	return self:win_exists() and self:buf_exists() and (vim.api.nvim_win_get_buf(self.win) == self.buf)
end

---@return boolean
function win:win_exists()
	return vim.api.nvim_win_is_valid(self.win or -1)
end

---@return boolean
function win:buf_exists()
	return vim.api.nvim_buf_is_valid(self.buf or -1)
end

function win:create_buf()
	if not self:buf_exists() then
		self.buf = vim.api.nvim_create_buf(false, true)
	end
end

---@param opts? table<string, any>
---@param how? "force" | "keep" | "error"
function win:set_b(opts, how)
	if opts then
		self.b = vim.tbl_extend(how or "force", self.b or {}, opts)
		self:with_buf(function(b)
			for k, v in pairs(self.b) do
				vim.b[b][k] = v
			end
		end)
	end
end

---@param opts? table<string, any>
---@param how? "force" | "keep" | "error"
function win:set_bo(opts, how)
	if opts then
		self.bo = vim.tbl_extend(how or "force", self.bo or {}, opts)
		self:with_buf(function(b)
			for k, v in pairs(self.bo) do
				vim.bo[b][k] = v
			end
		end)
	end
end

---@param opts? table<string, any>
---@param how? "force" | "keep" | "error"
function win:set_wo(opts, how)
	if opts then
		self.wo = vim.tbl_extend(how or "force", self.wo or {}, opts)
		self:with_win(function(w)
			for k, v in pairs(self.wo) do
				vim.wo[w][k] = v
			end
		end)
	end
end

---@param opts? vim.api.keyset.win_config
---@param how? "force" | "keep" | "error"
function win:set_cfg(opts, how)
	if opts then
		self.cfg = vim.tbl_extend(how or "force", self.cfg or {}, opts)
		if self:is_visible() then
			vim.api.nvim_win_set_config(self.win, self.cfg)
		end
	end
end

---@param f fun(): any
---@return any
function win:win_call(f)
	if self:win_exists() then
		return vim.api.nvim_win_call(self.win, f)
	end
end

---@param f fun(): any
---@return any
function win:buf_call(f)
	if self:buf_exists() then
		return vim.api.nvim_buf_call(self.buf, f)
	end
end

---@param f fun(w: integer): any
---@return any
function win:with_win(f)
	if self:is_visible() then
		return f(self.win)
	end
end

---@param f fun(b: integer): any
---@return any
function win:with_buf(f)
	if self:buf_exists() then
		return f(self.buf)
	end
end

function win:hide()
	if self:win_exists() then
		vim.api.nvim_win_close(self.win, true)
	end
end

function win:destroy()
	self:hide()
	if self:buf_exists() then
		vim.api.nvim_buf_delete(self.buf, { force = true })
	end
end

return win
