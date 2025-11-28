local utils = require("jet.core.utils")

---@class Jet.Ui.Notebook
---@field kernel Jet.Kernel
---@field results table<string, Jet.Notebook.Chunk>
---@field ns number
---@field bufnr number
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
	opts = opts or {}
	opts.bufnr = opts.bufnr or vim.api.nvim_get_current_buf()
	self.results = {}
	self.kernel = kernel
	self.bufnr = opts.bufnr
	self.ns = vim.api.nvim_create_namespace("jet_notebook__" .. opts.bufnr .. "__" .. kernel.id)
	return self
end

---@class Jet.Notebook.Chunk
---@field bufnr number
---@field winnr number
---@field extmark number
---@field channel number
---@field _src Jet.Execute.Chunk
---@field notebook Jet.Ui.Notebook
---@field output string[]
local NotebookChunk = {}
NotebookChunk.__index = NotebookChunk
setmetatable(NotebookChunk, {
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param chunk Jet.Execute.Chunk
---@param notebook Jet.Ui.Notebook
function NotebookChunk.new(chunk, notebook)
	local self = setmetatable({}, NotebookChunk)
	self._src = chunk
	self.bufnr = vim.api.nvim_create_buf(false, true)
	self.channel = vim.api.nvim_open_term(self.bufnr, {})
	self.notebook = notebook
	self.output = {}
	self:execute()
	return self
end

---@return { code: string[], start_row: number, start_col: number, end_row: number, end_col: number }
function NotebookChunk:src()
	local rng = { self._src.node:range() }
	local code = vim.treesitter.get_node_text(self._src.node, self._src.bufnr)
	return {
		code = vim.split(code, "\n", { trimempty = false }),
		start_row = rng[1],
		start_col = rng[2],
		end_row = rng[3],
		end_col = rng[4],
	}
end

function NotebookChunk:execute()
	self:clear_output()
	self.notebook.kernel:execute(self:src().code, function(msg)
		if msg.type == "execute_input" then
			return
		end
		local text = utils.msg_to_string(msg)
		if text then
			vim.api.nvim_chan_send(self.channel, text)
			for _, line in ipairs(vim.split(text, "\n")) do
				table.insert(self.output, line)
			end
			self:show()
		end
	end)
end

function NotebookChunk:clear_output()
	local prev_bufnr = self.bufnr
	self.bufnr = vim.api.nvim_create_buf(false, true)
	self.channel = vim.api.nvim_open_term(self.bufnr, {})
	self.output = {}
	if vim.api.nvim_win_is_valid(self.winnr or -99) then
		vim.api.nvim_win_set_buf(self.winnr, self.bufnr)
	end

	if vim.api.nvim_buf_is_valid(prev_bufnr or -99) then
		vim.api.nvim_buf_delete(prev_bufnr, { force = true })
	end
end

---Create or update a results block
function NotebookChunk:show()
	local n_lines = math.max(1, #self.output)

	if not self:is_visible() then
		self.winnr = vim.api.nvim_open_win(self.bufnr, false, {
			relative = "win",
			win = self._src.winnr,
			bufpos = { self:src().end_row, 0 },
			height = n_lines,
			width = vim.api.nvim_win_get_width(self._src.winnr),
			border = "none",
			style = "minimal",
		})
	end

	local chunk_backdrop_lines = {}
	for _ = 1, n_lines do
		table.insert(chunk_backdrop_lines, { { "", "Normal" } })
	end

    vim.print({
        output = self.output,
        backdrop = chunk_backdrop_lines
    })

	if #vim.api.nvim_buf_get_extmark_by_id(self.notebook.bufnr, self.notebook.ns, self.extmark or -99, {}) == 0 then
		self.extmark = vim.api.nvim_buf_set_extmark(self.notebook.bufnr, self.notebook.ns, self:src().end_row, 0, {
			virt_lines = chunk_backdrop_lines,
		})
	else
		vim.api.nvim_buf_set_extmark(self.notebook.bufnr, self.notebook.ns, self:src().end_row, 0, {
			id = self.extmark,
			virt_lines = chunk_backdrop_lines,
		})
	end

	vim.api.nvim_win_set_config(self.winnr, {
		height = n_lines,
	})
end

function NotebookChunk:is_visible()
	return vim.api.nvim_win_is_valid(self.winnr or -99)
end

---@param chunk Jet.Execute.Chunk
function Notebook:execute_chunk(chunk)
	local id = chunk.node:id()
    vim.print(id)
	self.results[id] = self.results[id] or NotebookChunk(chunk, self)
end

return Notebook
