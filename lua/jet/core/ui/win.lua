---@class Jet.Ui.Win.Opts
---@field cfg? vim.api.keyset.win_config
---@field name? string
---@field bo? table<string, any>
---@field b? table<string, any>
---@field wo? table<string, any>
---@field show? boolean
---@field keymaps? Jet.Ui.Win.Keymap[]
---@field augroup? integer
---@field ns? integer

---@class Jet.Ui.Win.Keymap
---@field [1] string | string[]
---@field [2] string
---@field [3] string | fun()
---@field [4]? vim.keymap.set.Opts

---@class Jet.Ui.Win
---@field win integer?
---@field buf integer?
---@field cfg vim.api.keyset.win_config
---@field name? string
---@field bo table<string, any>
---@field b table<string, any>
---@field wo table<string, any>
---@field keymaps Jet.Ui.Win.Keymap[]
---@field augroup integer?
---@field ns? integer
local win = {}
win.__index = win

setmetatable(win, {
	---@return Jet.Ui.Win
	__call = function(self, ...)
		return self.init(...)
	end,
})

---@param opts? Jet.Ui.Win.Opts
---@return Jet.Ui.Win
function win.new(opts)
	local self = setmetatable({}, win)
	opts = opts or {}

	self:create_buf()
	self:set_bo(opts.bo)
	self:set_b(opts.b)
	self:set_keymaps(opts.keymaps)
	self:set_cfg(opts.cfg)
	self:set_wo(opts.wo)
	self:set_name(opts.name)
	self:set_augroup(opts.augroup)
	self:set_ns(opts.ns)

	if opts.show == nil or opts.show then
		self:show()
	end

	return self
end

---@param enter? boolean Default `false`
---@param cfg? vim.api.keyset.win_config
function win:show(enter, cfg)
	if self:is_visible() then
		vim.api.nvim_win_set_config(self.win, self.cfg)
	else
		self:create_buf()
		self.win = vim.api.nvim_open_win(self.buf, enter or false, vim.tbl_extend("force", self.cfg or {}, cfg or {}))
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
	self:set_bo()
	self:set_b()
end

---@param opts? table<string, any>
---@param how? "force" | "keep" | "error"
function win:set_b(opts, how)
	self.b = vim.tbl_extend(how or "force", self.b or {}, opts or {})
	if vim.tbl_count(self.b) > 0 then
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
	self.bo = vim.tbl_extend(how or "force", self.bo or {}, opts or {})
	if vim.tbl_count(self.bo) > 0 then
		self:with_buf(function(b)
			-- TODO: why is pcall needed?
			pcall(function()
				for k, v in pairs(self.bo) do
					vim.bo[b][k] = v
				end
			end)
		end)
	end
end

---@param opts? table<string, any>
---@param how? "force" | "keep" | "error"
function win:set_wo(opts, how)
	self.wo = vim.tbl_extend(how or "force", self.wo or {}, opts or {})
	if vim.tbl_count(self.wo) > 0 then
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
	self.cfg = vim.tbl_extend(how or "force", self.cfg or {}, opts or {})
	if vim.tbl_count(self.cfg) > 0 then
		if self:is_visible() then
			vim.api.nvim_win_set_config(self.win, self.cfg)
		end
	end
end

---@param name? string
function win:set_name(name)
	self.name = name or self.name
	if self.name and self:buf_exists() then
		vim.api.nvim_buf_set_name(self.buf, self.name)
	end
end

---@param augroup? integer
function win:set_augroup(augroup)
	self.augroup = augroup or self.augroup or vim.api.nvim_create_augroup("jet_win__" .. self.win, { clear = true })
end

---@param ns? integer
function win:set_ns(ns)
	self.ns = ns or self.ns or vim.api.nvim_create_namespace("jet_buf__" .. self.buf)
end

---@param line_start number
---@param line_end number
function win:clear_ns(line_start, line_end)
	self:with_buf(function(b)
		vim.api.nvim_buf_clear_namespace(b, self.ns, line_start or 0, line_end or -1)
	end)
end

---@param line integer
---@param col integer
---@param opts vim.api.keyset.set_extmark
function win:set_extmark(line, col, opts)
	vim.api.nvim_buf_set_extmark(self.buf, self.ns, line, col, opts)
end

---@param line_start number
---@param line_end number
---@param lines string[]
function win:set_lines(line_start, line_end, lines)
	vim.api.nvim_buf_set_lines(self.buf, line_start, line_end, false, lines)
end

function win:get_lines(line_start, line_end)
	return vim.api.nvim_buf_get_lines(self.buf, line_start or 0, line_end or -1, false)
end

---@param keymaps Jet.Ui.Win.Keymap[]
function win:set_keymaps(keymaps)
	if keymaps then
		self.keymaps = vim.tbl_extend("force", self.keymaps or {}, keymaps)
		self:with_buf(function(b)
			for _, k in ipairs(self.keymaps) do
				---@diagnostic disable-next-line: param-type-mismatch
				local mode = type(k[1]) == "string" and vim.split(k[1], "") or k[1]
				vim.keymap.set(mode, k[2], k[3], vim.tbl_extend("force", { buffer = b }, k[4] or {}))
			end
		end)
	end
end

---@param event string | string[]
---@param opts vim.api.keyset.create_autocmd
function win:autocmd(event, opts)
	opts.buffer = self.buf
	opts.group = self.augroup
	vim.api.nvim_create_autocmd(event, opts)
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
