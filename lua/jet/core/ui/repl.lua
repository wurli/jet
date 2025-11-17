local utils = require("jet.core.utils")
local spinners = require("jet.core.ui.spinners")

---@class _Display
---@field bufnr number
---@field winnr? number

---@class _Spinner:_Display
---@field _stop? fun()

---@class Jet.Ui.Repl
---The REPL input buffer number
---@field prompt _Display
---@field output _Display
---@field background _Display
---@field spinner _Spinner
---@field zindex number
---
---@field spinner_bufnr number
---@field spinner_winnr number
---
---The REPL output channel
---@field repl_channel number
---
---The augroup for autocommands
---@field _augroup number
---
---@field indent_chars { main: string, continue: string }
---@field indent_templates { main: string, continue: string }
---
---TODO: do we need these?
---@field last_win number
---@field last_normal_win number
---@field last_jet_win number
---
---A reference to the kernel this UI belongs to. NB, it might seem odd to
---include the kernel as a field of the UI while the UI is itself a field of
---the kernel, but it makes lots of things very convenient, e.g. getting
---history from the kernel.
---@field kernel Jet.Kernel
---
---Namespaces for extmarks and highlights.
---@field ns { indent: number, spinner: number }
local repl = {}
repl.__index = repl

setmetatable(repl, {
	---@return Jet.Ui.Repl
	__call = function(self, ...)
		return self.start(...)
	end,
})

---@param kernel Jet.Kernel
---@param opts? { show: boolean }
function repl.init(kernel, opts)
	opts = vim.tbl_extend("force", opts or {}, {
		show = true,
	})
	local self = setmetatable({}, repl)
	self.indent_chars = vim.tbl_deep_extend("keep", self.indent_chars or {}, {
		main = ">",
		continue = "+",
	})
	self.indent_templates = vim.tbl_deep_extend("keep", self.indent_templates or {}, {
		main = "%s ",
		continue = "%s ",
	})
	self.kernel = kernel
	self:_make_namespaces()
	self._augroup = vim.api.nvim_create_augroup("jet_repl_" .. self.kernel.id, {})
	self:_init_ui()
	self:_display_output(utils.add_linebreak(self.kernel.instance.info.banner))
	if opts.show then
		self:show()
	end
	return self
end

function repl:_make_namespaces()
	local make_ns = function(name)
		return vim.api.nvim_create_namespace("jet_repl_" .. name .. "_" .. self.kernel.id)
	end
	self.ns = {
		indent = make_ns("indent"),
		spinner = make_ns("spinner"),
	}
end

function repl:delete()
	--- Hide the UI
	self:hide()
	--- Delete REPL buffers
	for _, buf in ipairs({
		self.background.bufnr,
		self.prompt.bufnr,
		self.output.bufnr,
	}) do
		if vim.api.nvim_buf_is_valid(buf) then
			vim.api.nvim_buf_delete(buf, { force = true })
		end
	end
	--- Delete autocommands
	vim.api.nvim_delete_augroup_by_id(self._augroup)
end

function repl:show()
	self.zindex = 0
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

function repl:hide()
	for _, winnr in ipairs({
		self.background.winnr,
		self.prompt.winnr,
		self.output.winnr,
	}) do
		if vim.api.nvim_win_is_valid(winnr) then
			vim.api.nvim_win_close(winnr, true)
		end
	end
end

function repl:_init_ui()
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

function repl:_set_layout()
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

---Executes code in the kernel and displays results in the REPL.
---Leaves the REPL input window unchanged.
---Shows a fancy spinner. Swish!
---@param code string[]
function repl:execute(code)
	self:_spinner_start()

	self.kernel:execute(code, function(msg)
		if msg.type == "execute_input" then
			-- Add the prompt indent to input code, otherwise it can be hard to
			-- tell what's input and what's output.
			msg.data.code = self:_indent_get_main() .. msg.data.code:gsub("\n", "\n" .. self:_indent_get_continue())
		end
		self:_display_output(utils.msg_to_string(msg))
		self:_scroll_to_end()
	end, function()
		self:_display_output("\n")
		self:_scroll_to_end()
		self:_spinner_hide({ delete = true })
	end)
end

---Execute and clear the prompt
function repl:execute_prompt()
	local code = self:_prompt_get()
	self:_prompt_set({})
	self:execute(code)
	vim.api.nvim_win_set_config(self.prompt.winnr, { height = 1 })
end

--Check for incompleteness before possibly executing.
function repl:maybe_execute_prompt()
	self.kernel:if_complete(self:_prompt_get(), {
		complete = function()
			self:execute_prompt()
		end,
		incomplete = function()
			if vim.fn.bufnr() == self.prompt.bufnr then
				vim.api.nvim_feedkeys("\r", "n", false)
			end
		end,
	})
end

---@param fn fun(bufnr: number?)
function repl:_with_prompt_buf(fn)
	if vim.api.nvim_buf_is_valid(self.prompt.bufnr or -99) then
		fn(self.prompt.bufnr)
	end
end

---@param fn fun(bufnr: number?)
function repl:_with_output_buf(fn)
	if vim.api.nvim_buf_is_valid(self.output.bufnr or -99) then
		fn(self.output.bufnr)
	end
end

---@param fn fun(winnr: number?)
function repl:_with_prompt_win(fn)
	if vim.api.nvim_win_is_valid(self.prompt.winnr or -99) then
		fn(self.prompt.winnr)
	end
end

---@param fn fun(winnr: number?)
function repl:_with_output_win(fn)
	if vim.api.nvim_win_is_valid(self.output.winnr or -99) then
		fn(self.output.winnr)
	end
end

function repl:_indent_reset()
	self:_with_prompt_win(function(prompt_win)
		local n_lines = #vim.api.nvim_buf_get_lines(self.prompt.bufnr, 0, -1, false)
		vim.api.nvim_win_set_config(prompt_win, { height = n_lines })
	end)
	self:_indent_clear(0, -1)
	for i = 1, vim.fn.line("$", self.prompt.winnr) do
		self:_indent_set(i - 1)
	end
end

---@param line_start number
---@param line_end number
function repl:_indent_clear(line_start, line_end)
	self:_with_prompt_buf(function(prompt_buf)
		vim.api.nvim_buf_clear_namespace(prompt_buf, self.ns.indent, line_start, line_end)
	end)
end

---@param lnum number 0-indexed
---@param text? string Defaults to the repl indent for `lnum`
function repl:_indent_set(lnum, text)
	text = text or (lnum == 0 and self:_indent_get_main() or self:_indent_get_continue())
	local hl_group = lnum == 0 and "JetReplIndentMain" or "JetReplIndentContinue"

	self:_with_prompt_buf(function(prompt_buf)
		vim.api.nvim_buf_set_extmark(prompt_buf, self.ns.indent, lnum, 0, {
			-- virt_text = { { text, hl_group } },
			virt_text = { { text, hl_group } },
			virt_text_pos = "inline",
			right_gravity = false,
		})
	end)
end

function repl:_indent_get_main()
	return self.indent_templates.main:format(self.indent_chars.main)
end

function repl:_indent_get_continue()
	return self.indent_templates.continue:format(self.indent_chars.continue)
end

---@param text string[]
function repl:_prompt_set(text)
	if not text then
		return
	end
	vim.api.nvim_buf_set_lines(self.prompt.bufnr, 0, -1, false, text)
	self:_indent_reset()
end

---@return string[]
function repl:_prompt_get()
	return vim.api.nvim_buf_get_lines(self.prompt.bufnr, 0, -1, false)
end

function repl:_scroll_to_end()
	self:_with_output_buf(function(output_buf)
		vim.api.nvim_buf_call(output_buf, function()
			vim.fn.cursor(vim.fn.line("$"), 0)
		end)
	end)
end

function repl:_spinner_start()
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
function repl:_spinner_maybe_show()
	if not (self:_is_visible() and self:_has_spinner()) then
		return
	end

	self.spinner.winnr = vim.api.nvim_open_win(self.spinner.bufnr, false, {
		relative = "win",
		anchor = "SE",
		win = self.background.winnr,
		col = vim.api.nvim_win_get_width(self.output.winnr),
		row = vim.api.nvim_win_get_height(self.output.winnr) + 1,
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
function repl:_spinner_hide(opts)
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
function repl:_has_spinner()
	return vim.api.nvim_buf_is_valid(self.spinner and self.spinner.bufnr or -99)
end

function repl:_is_visible()
	return vim.api.nvim_win_is_valid(self.background.winnr)
end

function repl:_filetype_set(filetype)
	vim.bo[self.prompt.bufnr].filetype = filetype
end

---@param text? string
function repl:_display_output(text)
	if not text then
		return
	end

	vim.api.nvim_chan_send(self.repl_channel, text)
end

return repl
