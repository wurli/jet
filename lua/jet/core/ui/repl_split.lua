local utils = require("jet.core.utils")
local spinners = require("jet.core.ui.spinners")

---@class _Display
---@field bufnr number
---@field winnr? number

---@class Jet.Ui.ReplSplit
---@field ui { prompt: _Display, output: _Display }
---@field zindex number
---@field output_channel number
---@field _augroup number
---@field indent_chars { main: string, continue: string }
---@field indent_templates { main: string, continue: string }
---@field spinner? { stop: fun() }
---@field kernel Jet.Kernel
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
	for _, ui in pairs(self.ui) do
		if vim.api.nvim_buf_is_valid(ui.bufnr) then
			vim.api.nvim_buf_delete(ui.bufnr, { force = true })
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
	self.ui.output.winnr = vim.api.nvim_open_win(self.ui.output.bufnr, false, {
		split = "right",
		style = "minimal",
	})

	-- Create the prompt split window as as if from the output window
	vim.api.nvim_win_call(self.ui.output.winnr, function()
		self.ui.prompt.winnr = vim.api.nvim_open_win(self.ui.prompt.bufnr, false, {
			split = "below",
			height = 1,
			style = "minimal",
		})
	end)

	vim.wo[self.ui.output.winnr].listchars = ""
	vim.wo[self.ui.prompt.winnr].signcolumn = "no"
	self:_statusline_set()

	self:_set_layout()
end

function ReplSplit:hide()
	for _, ui in pairs(self.ui) do
		if vim.api.nvim_win_is_valid(ui.winnr) then
			vim.api.nvim_win_close(ui.winnr, true)
		end
	end
end

---@param opts? { left: string?, center: string?, right: string? }
function ReplSplit:_statusline_set(opts)
	if not vim.api.nvim_win_is_valid(self.ui.output.winnr) then
		return
	end

	opts = opts or {}

	vim.schedule(function()
		self:_with_output_win(function(win)
			vim.wo[win].statusline = table.concat({
				"%#Normal#",
				opts.left or "",
				"%=",
				opts.center or "",
				"%=",
				"%#JetReplSpinner#",
				opts.right or "",
			}, "")
		end)
	end)
end

---@param ... string Any extra buffers to create
function ReplSplit:_init_bufs(...)
	local names = { "prompt", "output", ... }
	self.ui = self.ui or {}
	for _, ui_name in ipairs(names) do
		self.ui[ui_name] = self.ui[ui_name] or {}
		local ui = self.ui[ui_name]
		if vim.api.nvim_buf_is_valid(ui and ui.bufnr or -99) then
			utils.log_warn("REPL %s buffer already exists with bufnr %s", ui_name, ui.bufnr)
		else
			ui.bufnr = vim.api.nvim_create_buf(false, true)
			vim.bo[ui.bufnr].buftype = "nofile"
			-- We set some buffer variables for use with autocommands. NB, many
			-- plugins use a custom filetype, but we often want to use filetype
			-- for other stuff in Jet.
			vim.b[ui.bufnr].jet = {
				type = "repl_" .. ui_name,
				kernel_id = self.kernel.id,
			}
		end
	end

	vim.bo[self.ui.output.bufnr].modifiable = false
end

function ReplSplit:_init_ui()
	self:_init_bufs()

	self:_with_prompt_buf(function(buf)
		vim.api.nvim_buf_call(buf, function()
			vim.cmd.file(self.kernel:name())
		end)
	end)

	-- Jet sends output from the kernel to a terminal channel in order to
	-- format ansi formatting.
	if self.output_channel then
		utils.log_warn("REPL output channel `%s` already exists!", self.output_channel)
	else
		self.output_channel = vim.api.nvim_open_term(self.ui.output.bufnr, {})
	end

	self:_indent_reset()
	self:_filetype_set(self.kernel.filetype)

	--- Set keymaps
	vim.keymap.set({ "n", "i" }, "<CR>", function()
		self:maybe_execute_prompt()
	end, {
		buffer = self.ui.prompt.bufnr,
		desc = "Jet REPL: execute code",
	})

	vim.keymap.set("n", "<C-x>", function()
		self:interrupt()
	end, {
		buffer = self.ui.prompt.bufnr,
		desc = "Jet REPL: interrupt execution",
	})

	-- TODO: Improve keymaps
	for _, key in ipairs({ "i", "I", "a", "A", "c", "C", "s", "S", "o", "O", "p", "P" }) do
		vim.keymap.set("n", key, function()
			self:_with_prompt_win(function(winnr)
				vim.api.nvim_set_current_win(winnr)
				vim.fn.feedkeys(key, "n")
			end)
		end, { buffer = self.ui.output.bufnr })
	end

	vim.keymap.set({ "n", "i" }, "<c-p>", function()
		self:_prompt_set(self.kernel:history_get(-1))
	end, { buffer = self.ui.prompt.bufnr })

	vim.keymap.set({ "n", "i" }, "<c-n>", function()
		self:_prompt_set(self.kernel:history_get(1))
	end, { buffer = self.ui.prompt.bufnr })

	--- Set autocommands
	--- Attach LSP to the REPL input buffer
	--- (TODO: give the user the ability to disable this)
	vim.api.nvim_create_autocmd("BufEnter", {
		group = self._augroup,
		buffer = self.ui.prompt.bufnr,
		callback = function()
			for _, cfg in pairs(vim.lsp._enabled_configs) do
				if cfg.resolved_config then
					local ft = cfg.resolved_config.filetypes
					if ft and vim.tbl_contains(ft, self.kernel.filetype) or not ft then
						vim.lsp.start(cfg.resolved_config, {
							bufnr = self.ui.prompt.bufnr,
						})
					end
				end
			end
		end,
	})

	vim.api.nvim_create_autocmd("BufUnload", {
		group = self._augroup,
		callback = function(e)
			if vim.tbl_contains({ self.ui.prompt.bufnr, self.ui.output.bufnr }, e.buf) then
				self:delete()
			end
		end,
	})

	vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
		group = self._augroup,
		buffer = self.ui.prompt.bufnr,
		callback = function()
			self:_indent_reset()
		end,
	})

	vim.api.nvim_create_autocmd("WinResized", {
		group = self._augroup,
		callback = function()
			self:_set_layout()
		end,
	})

	vim.api.nvim_create_autocmd("WinClosed", {
		group = self._augroup,
		callback = function(e)
			local repl_wins = { self.ui.prompt.winnr, self.ui.output.winnr }
			local repl_bufs = { self.ui.prompt.bufnr, self.ui.output.bufnr }
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
	if not (vim.api.nvim_win_is_valid(self.ui.prompt.winnr) and vim.api.nvim_win_is_valid(self.ui.output.winnr)) then
		return
	end

	vim.api.nvim_win_set_config(self.ui.prompt.winnr, {
		-- bottom border should overlap with the input's top border)
		-- We need to subtract 1 to account for the borders (the output's
		height = vim.api.nvim_buf_line_count(self.ui.prompt.bufnr),
	})

	self:_indent_reset()
end

---Executes code in the kernel and displays results in the REPL.
---Leaves the REPL input window unchanged.
---Shows a fancy spinner. Swish!
---@param code { code: string[] }
function ReplSplit:execute(code)
	self:_spinner_start()

	self.kernel:execute(code.code, function(msg)
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
		self:_spinner_stop()
	end)
end

---Execute and clear the prompt
function ReplSplit:execute_prompt()
	local code = self:_prompt_get()
	self:_prompt_set({})
	self:execute({ code = code })
	vim.api.nvim_win_set_config(self.ui.prompt.winnr, { height = 1 })
end

--Check for incompleteness before possibly executing.
function ReplSplit:maybe_execute_prompt()
	self.kernel:if_complete(self:_prompt_get(), {
		complete = function()
			self:execute_prompt()
		end,
		incomplete = function()
			if vim.fn.bufnr() == self.ui.prompt.bufnr then
				vim.api.nvim_feedkeys("\r", "n", false)
			end
		end,
	})
end

function ReplSplit:interrupt()
	self:_spinner_start(function(frame)
		return "Cancelling " .. frame
	end)
	self.kernel:interrupt(function() end, function()
		self:_spinner_stop()
	end)
end

---@param fn fun(bufnr: number?)
function ReplSplit:_with_prompt_buf(fn)
	if vim.api.nvim_buf_is_valid(self.ui.prompt.bufnr or -99) then
		fn(self.ui.prompt.bufnr)
	end
end

---@param fn fun(bufnr: number?)
function ReplSplit:_with_output_buf(fn)
	if vim.api.nvim_buf_is_valid(self.ui.output.bufnr or -99) then
		fn(self.ui.output.bufnr)
	end
end

---@param fn fun(winnr: number?)
function ReplSplit:_with_prompt_win(fn)
	if vim.api.nvim_win_is_valid(self.ui.prompt.winnr or -99) then
		fn(self.ui.prompt.winnr)
	end
end

---@param fn fun(winnr: number?)
function ReplSplit:_with_output_win(fn)
	if vim.api.nvim_win_is_valid(self.ui.output.winnr or -99) then
		fn(self.ui.output.winnr)
	end
end

function ReplSplit:_indent_reset()
	self:_with_prompt_win(function(prompt_win)
		local n_lines = vim.api.nvim_buf_line_count(self.ui.prompt.bufnr)
		vim.api.nvim_win_set_config(prompt_win, { height = n_lines })
	end)
	self:_indent_clear(0, -1)
	for i = 1, vim.fn.line("$", self.ui.prompt.winnr) do
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

---@param text string[]?
function ReplSplit:_prompt_set(text)
	if not text then
		return
	end
	vim.api.nvim_buf_set_lines(self.ui.prompt.bufnr, 0, -1, false, text)
	self:_indent_reset()
end

---@return string[]
function ReplSplit:_prompt_get()
	return vim.api.nvim_buf_get_lines(self.ui.prompt.bufnr, 0, -1, false)
end

function ReplSplit:_scroll_to_end()
	self:_with_output_buf(function(output_buf)
		vim.api.nvim_buf_call(output_buf, function()
			vim.fn.cursor(vim.fn.line("$"), 0)
		end)
	end)
end

---@param transform? fun(frame: string): string
---@param type? Jet.Spinner
function ReplSplit:_spinner_start(transform, type)
	-- Stop any pre-existing spinner callbacks
	self:_spinner_stop()
	self.spinner = {
		stop = spinners.run(function(frame)
			if transform then
				frame = transform(frame)
			end
			self:_statusline_set({ right = frame .. " " })
		end, function() end, 100, type),
	}
end

function ReplSplit:_spinner_stop()
	if self.spinner and self.spinner.stop then
		self.spinner.stop()
	end
	self.spinner = nil
	self:_statusline_set()
end

---@return boolean
function ReplSplit:_has_spinner()
	return self.spinner ~= nil
end

function ReplSplit:_is_visible()
	return vim.api.nvim_win_is_valid(self.ui.output.winnr or -99)
end

function ReplSplit:_filetype_set(filetype)
	vim.bo[self.ui.output.bufnr].filetype = "jet"
	vim.bo[self.ui.prompt.bufnr].filetype = filetype
end

---@param text? string
function ReplSplit:_display_output(text)
	if not text then
		return
	end

	vim.api.nvim_chan_send(self.output_channel, text)
end

return ReplSplit
