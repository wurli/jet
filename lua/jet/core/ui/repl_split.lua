local utils = require("jet.core.utils")
local ReplFloat = require("jet.core.ui.repl_float")

---@class Jet.Ui.ReplSplit:Jet.Ui.ReplFloat
local ReplSplit = {}
ReplSplit.__index = ReplSplit
setmetatable(ReplSplit, {
	__index = ReplFloat,
	__call = function(self, ...)
		return self.new(...)
	end,
})

function ReplSplit.new()
	return setmetatable({}, ReplSplit)
end

function ReplSplit:_init_ui()
	for _, ui in ipairs({ "prompt", "output" }) do
		if self[ui] and vim.api.nvim_buf_is_valid(self[ui].bufnr) then
			utils.log_warn("REPL %s buffer already exists with bufnr %s", ui, self[ui].bufnr)
		else
			local bufnr = vim.api.nvim_create_buf(false, true)
			self[ui] = { bufnr = bufnr }
			vim.bo[bufnr].buftype = "nofile"
			-- We set some buffer variables for use with autocommands. NB, many
			-- plugins use a custom filetype, but we often want to use filetype
			-- for other stuff in Jet.
			vim.b[bufnr].jet = {
				type = "repl_" .. ui,
				kernel_id = self.kernel.id,
			}
		end
	end

	-- Jet sends output from the kernel to a terminal channel in order to
	-- format ansi formatting.
	if self.repl_channel then
		utils.log_warn("REPL output channel `%s` already exists!", self.repl_channel)
	else
		self.repl_channel = vim.api.nvim_open_term(self.output.bufnr, {})
	end

	self:_indent_reset()
	self:_filetype_set(self.kernel.filetype)

	--- Set keymaps
	vim.keymap.set({ "n", "i" }, "<CR>", function()
		self:maybe_execute_prompt()
	end, {
		buffer = self.prompt.bufnr,
		desc = "Jet REPL: execute code",
	})

	-- TODO: Improve keymaps
	for _, key in ipairs({ "i", "I", "a", "A", "c", "C", "s", "S", "o", "O", "p", "P" }) do
		vim.keymap.set("n", key, function()
			self:_with_prompt_win(function(winnr)
				vim.api.nvim_set_current_win(winnr)
				vim.fn.feedkeys(key, "n")
			end)
		end, { buffer = self.output.bufnr })
	end

	vim.keymap.set({ "n", "i" }, "<c-p>", function()
		self:_prompt_set(self.kernel:history_get(-1))
	end, { buffer = self.prompt.bufnr })

	vim.keymap.set({ "n", "i" }, "<c-n>", function()
		self:_prompt_set(self.kernel:history_get(1))
	end, { buffer = self.prompt.bufnr })

	--- Set autocommands
	--- Attach LSP to the REPL input buffer
	--- (TODO: give the user the ability to disable this)
	vim.api.nvim_create_autocmd("BufEnter", {
		group = self._augroup,
		buffer = self.prompt.bufnr,
		callback = function()
			for _, cfg in pairs(vim.lsp._enabled_configs) do
				if cfg.resolved_config then
					local ft = cfg.resolved_config.filetypes
					if ft and vim.tbl_contains(ft, self.kernel.filetype) or not ft then
						vim.lsp.start(cfg.resolved_config, {
							bufnr = self.prompt.bufnr,
						})
					end
				end
			end
		end,
	})

	vim.api.nvim_create_autocmd("BufUnload", {
		group = self._augroup,
		callback = function(e)
			if vim.tbl_contains({ self.prompt.bufnr, self.output.bufnr }, e.buf) then
				self:delete()
			end
		end,
	})

	vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
		group = self._augroup,
		buffer = self.prompt.bufnr,
		callback = function()
			self:_indent_reset()
		end,
	})

	vim.api.nvim_create_autocmd("WinClosed", {
		group = self._augroup,
		callback = function(e)
			local repl_wins = { self.prompt.winnr, self.output.winnr }
			local repl_bufs = { self.prompt.bufnr, self.output.bufnr }
			if not vim.tbl_contains(repl_bufs, e.buf) then
				return
			end
			for _, winnr in ipairs(repl_wins) do
				if vim.api.nvim_win_is_valid(winnr) then
					vim.api.nvim_win_close(winnr, true)
				end
			end
		end,
	})
end

function ReplSplit:show()
	local cur_win = vim.api.nvim_get_current_win()

	self.output.winnr = vim.api.nvim_open_win(self.output.bufnr, true, {
		split = "right",
		style = "minimal",
	})

	self.prompt.winnr = vim.api.nvim_open_win(self.prompt.bufnr, false, {
		split = "below",
		height = 1,
		style = "minimal",
	})

	vim.api.nvim_set_current_win(cur_win)

	vim.wo[self.output.winnr].listchars = ""
	vim.wo[self.output.winnr].statusline = self.kernel.instance.spec.display_name

	self:_spinner_maybe_show()

	self:_set_layout()
end

function ReplSplit:_set_layout()
	if not (vim.api.nvim_win_is_valid(self.prompt.winnr) and vim.api.nvim_win_is_valid(self.output.winnr)) then
		return
	end

	vim.api.nvim_win_set_config(self.prompt.winnr, {
		-- We need to subtract 1 to account for the borders (the output's
		-- bottom border should overlap with the input's top border)
		height = vim.api.nvim_buf_line_count(self.prompt.bufnr),
	})

	self:_indent_reset()
end

function ReplFloat:_spinner_start()
	-- Clean up any existing spinner
end

-- Will only show if there is an active spinner and the REPL itself is visible
function ReplFloat:_spinner_maybe_show() end

---@param opts? { delete: boolean }
function ReplFloat:_spinner_hide(opts) end

---@return boolean
function ReplFloat:_has_spinner() end

return ReplSplit
