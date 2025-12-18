local utils = require("jet.core.utils")
local win = require("jet.core.ui.win")
local spinners = require("jet.core.ui.spinners")

---@class Jet.Ui.ReplSplit
---@field ui { prompt: Jet.Ui.Win, output: Jet.Ui.Win }
---@field zindex number
---@field output_channel number
---@field _augroup number
---@field indent_chars { main: string, continue: string }
---@field indent_templates { main: string, continue: string }
---@field spinner? { stop: fun() }
---@field kernel Jet.Kernel
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

---@class Jet.Ui.Init.Opts
---@field show boolean
---@field bufnr? number The bufnr if the UI is a notebook

---@param kernel Jet.Kernel
---@param opts? Jet.Ui.Init.Opts
function ReplSplit:init(kernel, opts)
	opts = opts or {}
	self.kernel = kernel
	self.indent_chars = vim.tbl_deep_extend("keep", self.indent_chars or {}, {
		main = ">",
		continue = "+",
	})
	self.indent_templates = vim.tbl_deep_extend("keep", self.indent_templates or {}, {
		main = "%s ",
		continue = "%s ",
	})
	self.zindex = 0
	self._augroup = vim.api.nvim_create_augroup("jet_repl_" .. self.kernel.id, {})
	self:_init_ui()
	self:_display_output(utils.add_linebreak(self.kernel.instance.info.banner))
	if opts.show == nil or opts.show then
		self:show()
	end
	return self
end

function ReplSplit:delete()
	self:hide()
	self.ui.output:destroy()
	self.ui.prompt:destroy()
	vim.api.nvim_delete_augroup_by_id(self._augroup)
end

---@param first? "prompt" | "output"
function ReplSplit:show(first)
	if first == "prompt" then
		self:_show_prompt_win()
		self:_show_output_win()
	else
		self:_show_output_win()
		self:_show_prompt_win()
	end
	self:_statusline_set()
	self:_set_win_heights()
end

function ReplSplit:set_layout()
	if self:_is_visible() then
		self:show()
	end
end

function ReplSplit:_is_visible()
	return self.ui.output:is_visible() or self.ui.prompt:is_visible()
end

function ReplSplit:_show_output_win()
	if self.ui.output:is_visible() then
		return
	end

	if self.ui.prompt:is_visible() then
		self.ui.prompt:win_call(function()
			self.ui.output:show(false, {
				split = "above",
				style = "minimal",
			})
		end)
	else
		self.ui.output:show(false, {
			split = "right",
			style = "minimal",
		})
	end
end

function ReplSplit:_show_prompt_win()
	if self.ui.prompt:is_visible() then
		return
	end

	self:_show_output_win()

	self.ui.output:win_call(function()
		self.ui.prompt:show(false, {
			split = "below",
			height = 1,
			style = "minimal",
		})
	end)
end

function ReplSplit:_set_win_heights()
	if self.ui.output:is_visible() and self.ui.prompt:is_visible() then
		self.ui.prompt:set_cfg({
			height = vim.api.nvim_buf_line_count(self.ui.prompt.buf),
		})
		self:_indent_reset()
	end
end

function ReplSplit:hide()
	self.ui.output:hide()
	self.ui.prompt:hide()
end

---@param opts? { left: string?, center: string?, right: string? }
function ReplSplit:_statusline_set(opts)
	opts = opts or {}
	self.ui.output:set_wo({
		statusline = table.concat({
			"%#Normal#",
			opts.left or "",
			"%=",
			opts.center or "",
			"%=",
			"%#JetReplSpinner#",
			opts.right or "",
		}, ""),
	})
end

function ReplSplit:_init_ui()
	self.ui = {
		prompt = win.new({
			show = false,
			bo = {
				buftype = "nofile",
				filetype = self.kernel:_filetype_get(),
			},
			b = {
				jet = {
					type = "repl_input",
					kernel_id = self.kernel.id,
				},
			},
			wo = {
				signcolumn = "no",
			},
			name = self.kernel:name(),
			augroup = self._augroup,
			keymaps = {
				{
					"ni",
					"<CR>",
					function()
						self:maybe_execute_prompt()
					end,
					{ desc = "Jet REPL: execute code" },
				},
				{
					"n",
					"<C-x>",
					function()
						self:interrupt()
					end,
					{ desc = "Jet REPL: interrupt execution" },
				},
				{
					"ni",
					"<c-p>",
					function()
						self:_prompt_set(self.kernel:history_get(-1))
					end,
					{ desc = "Jet REPL: history previous" },
				},
				{
					"ni",
					"<c-n>",
					function()
						self:_prompt_set(self.kernel:history_get(1))
					end,
					{ desc = "Jet REPL: history next" },
				},
			},
		}),
		output = win.new({
			show = false,
			bo = {
				buftype = "nofile",
				modifiable = false,
				filetype = "jet",
			},
			b = {
				jet = {
					type = "repl_output",
					kernel_id = self.kernel.id,
				},
			},
			wo = {
				listchars = "",
			},
			augroup = self._augroup,
			keymaps = vim.tbl_map(function(key)
				return {
					"n",
					key,
					function()
						self.ui.prompt:with_win(function(w)
							vim.api.nvim_set_current_win(w)
							vim.fn.feedkeys(key, "n")
						end)
					end,
				}
			end, { "i", "I", "a", "A", "c", "C", "s", "S", "o", "O", "p", "P" }),
		}),
	}

	-- Jet sends output from the kernel to a terminal channel in order to
	-- format ansi formatting.
	if self.output_channel then
		utils.log_warn("REPL output channel `%s` already exists!", self.output_channel)
	else
		self.output_channel = vim.api.nvim_open_term(self.ui.output.buf, {})
	end

	self:_indent_reset()

	--- Set autocommands
	--- Attach LSP to the REPL input buffer
	--- (TODO: give the user the ability to disable this)
	self.ui.prompt:autocmd("BufEnter", {
		callback = vim.schedule_wrap(function()
			for _, cfg in pairs(vim.lsp._enabled_configs) do
				if cfg.resolved_config then
					-- vim.print({ lsp_ft = cfg.resolved_config.filetypes, repl_ft = self.kernel.filetype })
					local ft = cfg.resolved_config.filetypes
					if vim.tbl_contains(ft or {}, self.kernel.filetype) then
						vim.lsp.start(cfg.resolved_config, {
							bufnr = self.ui.prompt.buf,
						})
					end
				end
			end
		end),
	})

	vim.api.nvim_create_autocmd("BufUnload", {
		group = self._augroup,
		callback = function(e)
			if vim.tbl_contains({ self.ui.prompt.buf, self.ui.output.buf }, e.buf) then
				print("BufUnload: deleting")
				self:delete()
			end
		end,
	})

	vim.api.nvim_create_autocmd("WinClosed", {
		group = self._augroup,
		callback = function(e)
			local to_hide, type
			if e.buf == self.ui.output.buf then
				to_hide = self.ui.prompt
				type = "output"
			elseif e.buf == self.ui.prompt.buf then
				to_hide = self.ui.output
				type = "prompt"
			end
			if to_hide then
				vim.schedule(function()
					print("WinClosed: hiding " .. type)
					to_hide:hide()
				end)
			end
		end,
	})

	-- If the user removes the Jet repl buffer from one of the Jet repl windows
	-- (e.g. ctrl-i/ctrl-0), hide the other window as well.
	self.ui.prompt:autocmd("BufWinLeave", {
		callback = vim.schedule_wrap(function()
			self.ui.prompt:with_eventignore("BufWinLeave", function()
				print("BufWinLeave prompt: hiding")
				self.ui.output:hide()
			end)
		end),
	})
	self.ui.output:autocmd("BufWinLeave", {
		callback = vim.schedule_wrap(function()
			self.ui.output:with_eventignore("BufWinLeave", function()
				print("BufWinLeave output: hiding")
				self.ui.prompt:hide()
			end)
		end),
	})

	-- Conversely, if either window is entered, show both windows.
	for _, ui in pairs(self.ui) do
		ui:autocmd("BufWinEnter", {
			callback = vim.schedule_wrap(function()
				print("BufEnter: showing")
				self:show()
			end),
		})
	end

	vim.api.nvim_create_autocmd("WinResized", {
		group = self._augroup,
		callback = function()
			local layout_needs_reset = false

			-- This mildly cursed section handles the case where part of the
			-- REPL UI, i.e. the prompt or output window, is sent to a
			-- different window or tabpage. I actually use this quite a lot,
			-- e.g. when the output is too wide to fit comfortably in a split.
			for _, w in ipairs(vim.v.event.windows) do
				local buf = vim.api.nvim_win_get_buf(w)
				if buf == self.ui.prompt.buf then
					self.ui.prompt.win = w
					layout_needs_reset = true
				end
				if buf == self.ui.output.buf then
					self.ui.output.win = w
					layout_needs_reset = true
				end
			end

			if layout_needs_reset then
				utils.schedule(10, function()
					print("WinResized: resetting layout")
					self:set_layout()
				end)
			end
		end,
	})

	self.ui.prompt:autocmd({ "TextChanged", "TextChangedI" }, {
		callback = function()
			self:_indent_reset()
		end,
	})
end

---Executes code in the kernel and displays results in the REPL.
---Leaves the REPL input window unchanged.
---Shows a fancy spinner. Swish!
---@param code string[]
---@param callback? fun(msg: Jet.Callback.Execute.Result)
---@param on_complete? fun()
function ReplSplit:execute_code(code, callback, on_complete)
	self:_spinner_start()

	self.kernel:execute(code, function(msg)
		if callback then
			callback(msg)
		end
		if msg.type == "execute_input" then
			-- Add the prompt indent to input code, otherwise it can be hard to
			-- tell what's input and what's output.
			msg.data.code = self:_indent_get_main() .. msg.data.code:gsub("\n", "\n" .. self:_indent_get_continue())
		end

		if msg.type == "display_data" then
			self:_display_image(utils.display_data_to_file(msg, self.kernel))
			return
		end

		self:_display_output(utils.msg_to_string(msg))
		self:_scroll_to_end()
	end, function()
		if on_complete then
			on_complete()
		end
		self:_display_output("\n")
		self:_scroll_to_end()
		self:_spinner_stop()
	end)
end

---Execute and clear the prompt
function ReplSplit:execute_prompt()
	local code = self:_prompt_get()
	self:_prompt_set({})
	self:execute_code(code)
	self.ui.prompt:set_cfg({ height = 1 })
end

--Check for incompleteness before possibly executing.
function ReplSplit:maybe_execute_prompt()
	self.kernel:if_complete(self:_prompt_get(), {
		complete = function()
			self:execute_prompt()
		end,
		incomplete = function()
			if vim.fn.bufnr() == self.ui.prompt.buf then
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

function ReplSplit:_indent_reset()
	self.ui.prompt:set_cfg({
		height = vim.api.nvim_buf_line_count(self.ui.prompt.buf),
	})
	self:_indent_clear(0, -1)
	for i = 1, vim.fn.line("$", self.ui.prompt.win) do
		self:_indent_set(i - 1)
	end
end

---@param line_start number
---@param line_end number
function ReplSplit:_indent_clear(line_start, line_end)
	self.ui.prompt:clear_ns(line_start, line_end)
end

---@param lnum number 0-indexed
---@param text? string Defaults to the repl indent for `lnum`
function ReplSplit:_indent_set(lnum, text)
	text = text or (lnum == 0 and self:_indent_get_main() or self:_indent_get_continue())
	local hl_group = lnum == 0 and "JetReplIndentMain" or "JetReplIndentContinue"
	self.ui.prompt:set_extmark(lnum, 0, {
		-- virt_text = { { text, hl_group } },
		virt_text = { { text, hl_group } },
		virt_text_pos = "inline",
		right_gravity = false,
	})
end

function ReplSplit:_indent_get_main()
	return self.indent_templates.main:format(self.indent_chars.main)
end

function ReplSplit:_indent_get_continue()
	return self.indent_templates.continue:format(self.indent_chars.continue)
end

---@param text string[]?
function ReplSplit:_prompt_set(text)
	if text then
		self.ui.prompt:set_lines(0, -1, text)
		self:_indent_reset()
	end
end

---@return string[]
function ReplSplit:_prompt_get()
	return self.ui.prompt:get_lines()
end

function ReplSplit:_scroll_to_end()
	self.ui.output:buf_call(function()
		vim.fn.cursor(vim.fn.line("$"), 0)
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

---@param text? string
function ReplSplit:_display_output(text)
	if not text then
		return
	end

	vim.api.nvim_chan_send(self.output_channel, text)
end

---@param path? string
function ReplSplit:_display_image(path)
	if not path then
		return
	end

	local h_scale = 0.75
	local v_scale = 0.75
	local height = vim.o.lines
	local width = vim.o.columns

	local buf = vim.api.nvim_create_buf(false, true)
	local _ = vim.api.nvim_open_win(buf, true, {
		relative = "editor",
		height = math.floor(height * v_scale),
		width = math.floor(width * h_scale),
		row = math.floor(height * (1 - v_scale) / 2),
		col = math.floor(width * (1 - h_scale) / 2),
	})

	utils.image_to_buf(path, buf)
end

return ReplSplit
