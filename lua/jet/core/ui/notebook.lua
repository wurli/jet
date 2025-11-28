local utils = require("jet.core.utils")

local USE_BORDER = true

---@class Jet.Ui.Notebook
---@field kernel Jet.Kernel
---@field results Jet.Notebook.Chunk[]
---@field ns integer
---@field augroup integer
---@field bufnr integer
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
	vim.b[opts.bufnr].jet = { type = "notebook" }
	self.results = {}
	self.kernel = kernel
	self.bufnr = opts.bufnr
	self.ns = vim.api.nvim_create_namespace("jet_notebook__" .. opts.bufnr .. "__" .. kernel.id)
	self.augroup = vim.api.nvim_create_augroup("jet_notebook__" .. opts.bufnr .. "__" .. kernel.id, {})

	vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI", "WinResized" }, {
		group = self.augroup,
		buffer = self.bufnr,
		callback = function()
			self:show()
		end,
	})

	return self
end

---@param line integer
---@param opts? vim.api.keyset.set_extmark
---@return integer
function Notebook:set_mark(line, opts)
	opts = opts or {}
	opts.invalidate = true
	return vim.api.nvim_buf_set_extmark(self.bufnr, self.ns, line, 0, opts)
end

---@param id integer
---@return integer?
function Notebook:get_mark(id)
	local mark = vim.api.nvim_buf_get_extmark_by_id(self.bufnr, self.ns, id, { details = true })
	if mark[1] and not mark[3].invalid then
		return mark[1]
	else
		return nil
	end
end

---@param id integer
---@return boolean
function Notebook:del_mark(id)
	return vim.api.nvim_buf_del_extmark(self.bufnr, self.ns, id)
end

---@class Jet.Notebook.Chunk.Source
---@field start_mark integer
---@field end_mark integer
---@field bufnr integer
---@field winnr? integer

---@class Jet.Notebook.Chunk.Output
---@field text string
---@field channel integer
---@field bufnr integer
---@field winnr integer?
---@field backdrop integer

---@class Jet.Notebook.Chunk
---@field id integer
---@field source Jet.Notebook.Chunk.Source
---@field output Jet.Notebook.Chunk.Output
---@field notebook Jet.Ui.Notebook
local Chunk = {}
Chunk.__index = Chunk
setmetatable(Chunk, {
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param id integer
---@param source Jet.Execute.Chunk
---@param notebook Jet.Ui.Notebook
function Chunk.new(id, source, notebook)
	local self = setmetatable({}, Chunk)
	self.id = id
	self.notebook = notebook
	self.source = {
		-- -1 because we want to invalidate the chunk when the user deletes
		-- the chunk delimiter; not when they change the source code.
		start_mark = notebook:set_mark(source.start_row - 1),
		end_mark = notebook:set_mark(source.end_row),
		bufnr = source.bufnr,
		winnr = source.winnr,
	}
	local output_buf = vim.api.nvim_create_buf(false, true)
	self.output = {
		text = "",
		channel = vim.api.nvim_open_term(output_buf, {}),
		bufnr = output_buf,
		backdrop = -99,
	}

	self:backdrop_update()

	self:execute()

	return self
end

function Chunk:backdrop_update()
	local backdrop = self.notebook:get_mark(self.output.backdrop)
	local line = self:get_source().end_row

	if not (line and self:is_visible()) then
		self:backdrop_remove()
		return
	end

	if not backdrop then
		self.output.backdrop = self.notebook:set_mark(line, {})
	end

	local chunk_backdrop_lines = {}
	for _ = 1, self:n_lines() + (USE_BORDER and 2 or 0) do
		table.insert(chunk_backdrop_lines, { { "", "Normal" } })
	end

	self.output.backdrop = self.notebook:set_mark(self:get_source().end_row, {
		id = self.output.backdrop,
		virt_lines = chunk_backdrop_lines,
	})
end

function Chunk:backdrop_remove()
	self.notebook:del_mark(self.output.backdrop)
end

---@return { code: string[]?, start_row: integer?, end_row: integer? }
function Chunk:get_source()
	local start_row = self.notebook:get_mark(self.source.start_mark)
	local end_row = self.notebook:get_mark(self.source.end_mark)

	if not (start_row and end_row) then
		return { start_row = start_row and (start_row + 1), end_row = end_row }
	end

	return {
		code = vim.api.nvim_buf_get_lines(self.notebook.bufnr, start_row + 1, end_row, false),
		start_row = start_row + 1,
		end_row = end_row,
	}
end

function Chunk:execute()
	self:reset_output()
	self.notebook.kernel:execute(self:get_source().code, function(msg)
		-- No need to echo the input for notebook chunks.
		if msg.type == "execute_input" then
			return
		end
		local text = utils.msg_to_string(msg)
		if text then
			self.output.text = self.output.text .. text
			-- Term messages seem to disappear if we don't give them enough
			-- space, so make sure the window is the right size before sending.
			self:show()
			pcall(vim.api.nvim_chan_send, self.output.channel, text)
		end
	end)
end

function Chunk:reset_output()
	local prev_bufnr = self.output.bufnr
	self.output.bufnr = vim.api.nvim_create_buf(false, true)
	self.output.channel = vim.api.nvim_open_term(self.output.bufnr, {})
	self.output.text = ""
	if vim.api.nvim_win_is_valid(self.output.winnr or -99) then
		vim.api.nvim_win_set_buf(self.output.winnr, self.output.bufnr)
	end

	if vim.api.nvim_buf_is_valid(prev_bufnr or -99) then
		vim.api.nvim_buf_delete(prev_bufnr, { force = true })
	end
end

---Create or update a results block
function Chunk:show()
	if not self:has_source() then
		self:destroy()
	end

	if vim.api.nvim_win_get_buf(self.source.winnr) ~= self.source.bufnr then
		self:hide()
		return
	end

	local n_lines = self:n_lines()

	local win_info = vim.fn.getwininfo(self.source.winnr)[1]
	local chunk_win_width = win_info.width - win_info.textoff - 1

	local win_config = {
		relative = "win",
		win = self.source.winnr,
		bufpos = { self:get_source().end_row, 0 },
		height = n_lines,
		width = chunk_win_width,
		-- border = USE_BORDER and { "▔", "▔", "▔", " ", "▁", "▁", "▁", " " } or "none",
		border = USE_BORDER and { "─", "─", "─", " ", "─", "─", "─", " " } or "none",
		style = "minimal",
		zindex = 50,
	}

	if not self:is_visible() then
		if vim.api.nvim_buf_is_valid(self.output.bufnr or -99) then
			self.output.winnr = vim.api.nvim_open_win(self.output.bufnr, false, win_config)
		end
	else
		vim.api.nvim_win_set_config(self.output.winnr, win_config)
	end

	self:backdrop_update()
end

function Chunk:hide()
	if self:is_visible() then
		vim.api.nvim_win_close(self.output.winnr, true)
		self:backdrop_remove()
	end
end

function Chunk:destroy()
	self:backdrop_remove()
	if vim.api.nvim_win_is_valid(self.output.winnr or -99) then
		vim.api.nvim_win_close(self.output.winnr, true)
	end
	if vim.api.nvim_buf_is_valid(self.output.bufnr or -99) then
		vim.api.nvim_buf_delete(self.output.bufnr, { force = true })
	end
	self.notebook.results[self.id] = nil
end

---@return boolean
function Chunk:is_visible()
	return vim.api.nvim_win_is_valid(self.output.winnr or -99)
end

---Are the extmarks which delimit the chunk still present? If not the user has
---likely deleted the source code and we should remove the results.
---@return boolean
function Chunk:has_source()
	local src_start = self.notebook:get_mark(self.source.start_mark)
	local src_end = self.notebook:get_mark(self.source.end_mark)

	if src_start and src_end then
		return true
	else
		return false
	end
end

---@return integer
function Chunk:n_lines()
	return #vim.split(self.output.text, "\n")
end

---@param src Jet.Execute.Chunk
function Notebook:execute_chunk(src)
	local existing = self:get_chunks(src.start_row, src.end_row)
	-- In theory there shouldn't be any overlaps, but this seems prudent
	for _, chunk in ipairs(existing.overlaps) do
		chunk:destroy()
	end

	if existing.match then
		existing.match:execute()
		return
	end

	local id = math.max(0, unpack(vim.tbl_keys(self.results))) + 1
	self.results[id] = Chunk(id, src, self)
end

---@param start_row integer
---@param end_row number
---@return { match: Jet.Notebook.Chunk?, overlaps: Jet.Notebook.Chunk[] }
function Notebook:get_chunks(start_row, end_row)
	local match
	local overlaps = {}
	for _, chunk in pairs(self.results) do
		local src = chunk:get_source()
		if src.start_row == start_row and src.end_row == end_row then
			match = chunk

		-- Stare at the following to convince yourself about the condition:
		--     Existing (src) chunk boundaries : ......a.........b.....
		--     New chunk boundaries            : ...c......d...........
		-- If a <= d and c <= b then we have overlap, assuming that c <= d and a <= b.
		elseif src.start_row <= end_row and start_row <= src.end_row then
			table.insert(overlaps, chunk)
		end
	end

	return { match = match, overlaps = overlaps }
end

function Notebook:show()
	for _, chunk in pairs(self.results) do
		chunk:show()
	end
end

function Notebook:hide()
	for _, chunk in pairs(self.results) do
		chunk:hide()
	end
end

return Notebook
