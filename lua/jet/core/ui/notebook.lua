---@class Jet.Ui.Notebook
---@field kernel Jet.Kernel
---@field results Jet.CodeChunk[]
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
---@param opts? Jet.Ui.Init.Opts
function Notebook:init(kernel, opts)
	self.kernel = kernel
	return self
end

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

---@param chunk Jet.Execute.Chunk
function Notebook:execute_chunk(chunk)
	local buf = vim.api.nvim_create_buf(false, true)
	local channel = vim.api.nvim_open_term(buf, {})
	local win = vim.api.nvim_open_win(buf, false, {
		relative = "win",
	})
end

return Notebook
