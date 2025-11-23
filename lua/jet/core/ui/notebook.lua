---@class Jet.Ui.Notebook
---@field kernel Jet.Kernel
---@field results Jet.CodeChunk[]

---@class Jet.CodeChunk
---@field bufnr number
---@field winnr number
---@field channel number
---@field start_row number
---@field start_col number
---@field end_row number
---@field end_col number
---@field code string[]
---@field spinner table

local Notebook = {}

Notebook.__index = Notebook
setmetatable(Notebook, {
	__call = function(self, ...)
		return self.new(...)
	end,
})

function Notebook.new()
	return setmetatable({}, Notebook)
end

---@param kernel Jet.Kernel
function Notebook:init(kernel)
	self.kernel = kernel
end

---@param code Jet.GetExpr.Result
function Notebook:execute(code)
	local buf = vim.api.nvim_create_buf(false, true)
	local channel = vim.api.nvim_open_term(buf, {})
	local win = vim.api.nvim_open_win(buf, false, {
        relative = "win",
        win =
    })
end
