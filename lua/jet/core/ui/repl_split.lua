local utils = require("jet.core.utils")
local spinners = require("jet.core.ui.spinners")

---@class _Display
---@field bufnr number
---@field winnr? number

---@class _Spinner:_Display
---@field _stop? fun()

---@class Jet.Ui.ReplSplit
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
local ReplSplit = {}
ReplSplit.__index = ReplSplit

setmetatable(ReplSplit, {
	---@return Jet.Ui.ReplSplit
	__call = function(self, ...)
		return self.new(...)
	end,
})

function ReplSplit.new()
	return setmetatable({}, ReplSplit)
end

---@param kernel Jet.Kernel
---@param opts? { show: boolean }
function ReplSplit:init(kernel, opts)
	opts = vim.tbl_extend("force", opts or {}, {
		show = true,
	})
	self.indent_chars = vim.tbl_deep_extend("keep", self.indent_chars or {}, {
		main = ">",
		continue = "+",
	})
	self.indent_templates = vim.tbl_deep_extend("keep", self.indent_templates or {}, {
		main = "%s ",
		continue = "%s ",
	})
	self.zindex = 0
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

function ReplSplit:_make_namespaces()
	local make_ns = function(name)
		return vim.api.nvim_create_namespace("jet_repl_" .. name .. "_" .. self.kernel.id)
	end
	self.ns = {
		indent = make_ns("indent"),
		spinner = make_ns("spinner"),
	}
end

function ReplSplit:delete()
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

function ReplSplit:show()
	if self:_is_visible() then
		return
	end
	self:_show()
end

function ReplSplit:_show()
	self.output.winnr = vim.api.nvim_open_win(self.output.bufnr, false, {
		split = "right",
		style = "minimal",
	})

	-- Create the prompt split window as as if from the output window
	vim.api.nvim_win_call(self.output.winnr, function()
		self.prompt.winnr = vim.api.nvim_open_win(self.prompt.bufnr, false, {
			split = "below",
			height = 1,
			style = "minimal",
		})
	end)

	vim.wo[self.output.winnr].listchars = ""
	vim.wo[self.output.winnr].winbar = table.concat({
		"%#FloatTitle#",
		"%=",
		self.kernel.instance.spec.display_name,
		"%=",
		-- "%#Normal#",
	}, "")
	self:_set_statusline()

	self:_spinner_maybe_show()

	self:_set_layout()
end


function ReplSplit:hide()
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
---
---@param opts? { left: string?, center: string?, right: string? }
function ReplSplit:_set_statusline(opts)
	if not vim.api.nvim_win_is_valid(self.output.winnr) then
		return
	end

	opts = opts or {}

	vim.wo[self.output.winnr].statusline = table.concat({
		"%#FloatTitle#",
		opts.left or "",
		"%=",
		opts.center or "",
		"%=",
		opts.right or "",
	}, "")
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

---Executes code in the kernel and displays results in the REPL.
---Leaves the REPL input window unchanged.
---Shows a fancy spinner. Swish!
---@param code string[]
function ReplSplit:execute(code)
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
function ReplSplit:execute_prompt()
	local code = self:_prompt_get()
	self:_prompt_set({})
	self:execute(code)
	vim.api.nvim_win_set_config(self.prompt.winnr, { height = 1 })
end

--Check for incompleteness before possibly executing.
function ReplSplit:maybe_execute_prompt()
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
function ReplSplit:_with_prompt_buf(fn)
	if vim.api.nvim_buf_is_valid(self.prompt.bufnr or -99) then
		fn(self.prompt.bufnr)
	end
end

---@param fn fun(bufnr: number?)
function ReplSplit:_with_output_buf(fn)
	if vim.api.nvim_buf_is_valid(self.output.bufnr or -99) then
		fn(self.output.bufnr)
	end
end

---@param fn fun(winnr: number?)
function ReplSplit:_with_prompt_win(fn)
	if vim.api.nvim_win_is_valid(self.prompt.winnr or -99) then
		fn(self.prompt.winnr)
	end
end

---@param fn fun(winnr: number?)
function ReplSplit:_with_output_win(fn)
	if vim.api.nvim_win_is_valid(self.output.winnr or -99) then
		fn(self.output.winnr)
	end
end

function ReplSplit:_indent_reset()
	self:_with_prompt_win(function(prompt_win)
		local n_lines = vim.api.nvim_buf_line_count(self.prompt.bufnr)
		vim.api.nvim_win_set_config(prompt_win, { height = n_lines })
	end)
	self:_indent_clear(0, -1)
	for i = 1, vim.fn.line("$", self.prompt.winnr) do
		self:_indent_set(i - 1)
	end
end

---@param line_start number
---@param line_end number
function ReplSplit:_indent_clear(line_start, line_end)
	self:_with_prompt_buf(function(prompt_buf)
		vim.api.nvim_buf_clear_namespace(prompt_buf, self.ns.indent, line_start, line_end)
	end)
end

---@param lnum number 0-indexed
---@param text? string Defaults to the repl indent for `lnum`
function ReplSplit:_indent_set(lnum, text)
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

function ReplSplit:_indent_get_main()
	return self.indent_templates.main:format(self.indent_chars.main)
end

function ReplSplit:_indent_get_continue()
	return self.indent_templates.continue:format(self.indent_chars.continue)
end

---@param text string[]
function ReplSplit:_prompt_set(text)
	if not text then
		return
	end
	vim.api.nvim_buf_set_lines(self.prompt.bufnr, 0, -1, false, text)
	self:_indent_reset()
end

---@return string[]
function ReplSplit:_prompt_get()
	return vim.api.nvim_buf_get_lines(self.prompt.bufnr, 0, -1, false)
end

function ReplSplit:_scroll_to_end()
	self:_with_output_buf(function(output_buf)
		vim.api.nvim_buf_call(output_buf, function()
			vim.fn.cursor(vim.fn.line("$"), 0)
		end)
	end)
end

function ReplSplit:_spinner_start()
	-- Clean up any existing spinner
	self:_spinner_hide()
	self:_spinner_maybe_show()

	self.spinner._stop = spinners.run(function(frame)
		if self:_has_spinner() then
			self:_set_statusline({ right = frame })
		end
	end, function()
		self:_spinner_hide()
	end, 100)
end

-- Will only show if there is an active spinner and the REPL itself is visible
function ReplSplit:_spinner_maybe_show()

end

function ReplSplit:_spinner_hide()
    self:_set_statusline()
end

---@return boolean
function ReplSplit:_has_spinner() end

function ReplSplit:_is_visible()
	return vim.api.nvim_win_is_valid(self.output.winnr or -99)
end

function ReplSplit:_filetype_set(filetype)
	vim.bo[self.output.bufnr].filetype = "jet"
	vim.bo[self.prompt.bufnr].filetype = filetype
end

---@param text? string
function ReplSplit:_display_output(text)
	if not text then
		return
	end

	vim.api.nvim_chan_send(self.repl_channel, text)
end

return ReplSplit
