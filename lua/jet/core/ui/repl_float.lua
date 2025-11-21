local spinners = require("jet.core.ui.spinners")
local utils = require("jet.core.utils")
local ReplSplit = require("jet.core.ui.repl_split")

---@class Jet.Ui.ReplFloat:Jet.Ui.ReplSplit
local ReplFloat = {}
ReplFloat.__index = ReplFloat
setmetatable(ReplFloat, {
	__index = ReplSplit,
	__call = function(self, ...)
		return self.new(...)
	end,
})

function ReplFloat.new()
	return setmetatable({}, ReplFloat)
end

function ReplFloat:_init_ui()
	for _, ui in ipairs({ "background", "prompt", "output" }) do
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

	vim.api.nvim_create_autocmd("WinEnter", {
		group = self._augroup,
		buffer = self.background.bufnr,
		callback = function()
			-- When we enter the background window we want to automatically
			-- enter a different window. The approach is:
			-- *  Entering from the repl input     => go to repl output
			-- *  Entering from the repl output    => go to last normal window
			-- *  Entering from last normal window => go to repl input
			-- This should hopefully make entering/leaving the REPL windows
			-- feel natural and work well with the user's existing keymaps.
			vim.api.nvim_set_current_win(
				(self.last_win == self.prompt.winnr and self.output.winnr)
					or (self.last_win == self.output.winnr and self.last_normal_win)
					or (self.last_win == self.last_normal_win and self.prompt.winnr)
					or self.last_win
			)
		end,
	})

	vim.api.nvim_create_autocmd("WinClosed", {
		group = self._augroup,
		callback = function(e)
			local repl_wins = { self.background.winnr, self.prompt.winnr, self.output.winnr }
			local repl_bufs = { self.background.bufnr, self.prompt.bufnr, self.output.bufnr }

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

	vim.api.nvim_create_autocmd("WinResized", {
		group = self._augroup,
		callback = function()
			self:_set_layout()
		end,
	})

	vim.api.nvim_create_autocmd("WinLeave", {
		group = self._augroup,
		callback = function()
			self.last_win = vim.api.nvim_get_current_win()
			local buf_is_repl = vim.b.jet and vim.b.jet.kernel_id == self.kernel.id
			self[buf_is_repl and "last_jet_win" or "last_normal_win"] = self.last_win
		end,
	})
end

function ReplFloat:_show()
	-- ╭───────────╮
	-- │ box chars │
	-- ╰───────────╯

	self.background.winnr = vim.api.nvim_open_win(self.background.bufnr, false, {
		split = "right",
		focusable = false,
	})

	self.output.winnr = vim.api.nvim_open_win(self.output.bufnr, false, {
		relative = "win",
		win = self.background.winnr,
		col = 0,
		row = 0,
		height = vim.api.nvim_win_get_height(self.background.winnr) - 4,
		width = vim.api.nvim_win_get_width(self.background.winnr) - 4,
		border = { "╭", "─", "╮", "│", "│", " ", "│", "│" },
		zindex = self.zindex + 1,
		style = "minimal",
		title = self.kernel.instance.spec.display_name,
		title_pos = "center",
	})

	self.prompt.winnr = vim.api.nvim_open_win(self.prompt.bufnr, false, {
		relative = "win",
		win = self.background.winnr,
		height = 1,
		col = 0,
		row = vim.api.nvim_win_get_height(self.background.winnr),
		width = vim.api.nvim_win_get_width(self.background.winnr) - 4,
		border = { "│", "─", "│", "│", "╯", "─", "╰", "│" },
		zindex = self.zindex + 2,
		style = "minimal",
	})

	vim.wo[self.output.winnr].listchars = ""

	self:_spinner_maybe_show()

	self:_set_layout()
end

function ReplFloat:_set_layout()
	-- TODO: reset vertical layout when we resize other windows. This seems to
	-- get unborkably borked if we resize vim.
	if not (vim.api.nvim_win_is_valid(self.prompt.winnr) and vim.api.nvim_win_is_valid(self.output.winnr)) then
		return
	end

	-- First, if we're in either the input or output window, resize the background
	-- according to the current window width
	local cur_win = vim.api.nvim_get_current_win()
	if cur_win == self.output.winnr or cur_win == self.prompt.winnr then
		vim.api.nvim_win_set_config(self.background.winnr, {
			width = vim.api.nvim_win_get_width(cur_win) + 2,
		})
	end

	-- Now we're sure the background is the right size, set both input and output
	-- to match its width
	for _, win in ipairs({ self.prompt.winnr, self.output.winnr }) do
		vim.api.nvim_win_set_config(win, {
			width = math.max(vim.api.nvim_win_get_width(self.background.winnr) - 2, 1),
		})
	end

	-- TODO: if we've just resized the output window vertically, adjust the input
	-- window height accordingly
	-- if cur_win == self.repl_output.winnr then
	--     vim.api.nvim_win_set_config(self.repl_input.winnr, {
	--         height =
	--     })
	-- end

	local bg_height = vim.api.nvim_win_get_height(self.background.winnr)
	local prompt_height = vim.api.nvim_win_get_height(self.prompt.winnr)
	vim.api.nvim_win_set_config(self.output.winnr, {
		-- We need to subtract 1 to account for the borders (the output's
		-- bottom border should overlap with the input's top border)
		height = math.max(bg_height - prompt_height - 2, 1),
	})

	-- TODO
	-- if self:_has_spinner() then
	-- end

	self:_indent_reset()
end

function ReplFloat:_spinner_start()
	-- Clean up any existing spinner
	self:_spinner_hide({ delete = true })
	self.spinner = { bufnr = vim.api.nvim_create_buf(false, true) }

	self:_spinner_maybe_show()

	self.spinner._stop = spinners.run(function(frame)
		if self:_has_spinner() then
			vim.api.nvim_buf_set_extmark(self.spinner.bufnr, self.ns.spinner, 0, 0, {
				id = 1,
				virt_text_pos = "right_align",
				virt_text = { { frame, "JetReplSpinner" } },
				hl_mode = "combine",
			})
		end
	end, function()
		self:_spinner_hide({ delete = true })
	end, 100)
end

-- Will only show if there is an active spinner and the REPL itself is visible
function ReplFloat:_spinner_maybe_show()
	if not (self:_is_visible() and self:_has_spinner()) then
		return
	end

	self.spinner.winnr = vim.api.nvim_open_win(self.spinner.bufnr, false, {
		relative = "win",
		anchor = "SE",
		win = self.background.winnr,
		col = vim.api.nvim_win_get_width(self.output.winnr) - 1,
		row = vim.api.nvim_win_get_height(self.output.winnr),
		height = 1,
		width = 4,
		border = "none",
		zindex = self.zindex + 3,
		style = "minimal",
	})

	-- vim.wo[self.spinner.winnr].winhighlight = "NormalFloat:JetRepl"
	--    vim.wo[self.spinner.winnr].winblend = 100
end

---@param opts? { delete: boolean }
function ReplFloat:_spinner_hide(opts)
	opts = opts or {}

	if not self:_has_spinner() then
		return
	end

	if vim.api.nvim_win_is_valid(self.spinner.winnr) then
		vim.api.nvim_win_hide(self.spinner.winnr)
	end

	if opts.delete then
		vim.api.nvim_buf_delete(self.spinner.bufnr, {})
		if self.spinner._stop then
			self.spinner._stop()
		end
	end

	self.spinner = nil
end

---@return boolean
function ReplFloat:_has_spinner()
	return vim.api.nvim_buf_is_valid(self.spinner and self.spinner.bufnr or -99)
end

return ReplFloat
