local utils = require("jet.core.utils")

---@class Jet.Ui.Repl
---The REPL input buffer number
---@field repl_input_bufnr number
---
---The REPL input window number
---@field repl_input_winnr number
---
---The REPL input window number
---@field repl_output_bufnr number
---
---The REPL input window number
---@field repl_output_winnr number
---
---The REPL background buffer number
---@field repl_background_bufnr number
---
---The REPL background window number
---@field repl_background_winnr number
---
---The REPL output channel
---@field repl_channel number
---
---The augroup for autocommands
---@field _augroup number
---
---@field prompt { input: string, continue: string }
---@field prompt_template { input: string, continue: string }
---
---@field last_win number
---@field last_normal_win number
---@field last_jet_win number
---
---The namespace for virtual text indent text
---@field _ns number
local repl = {}
repl.__index = repl

setmetatable(repl, {
	---@return Jet.Ui.Repl
	__call = function(self, ...)
		return self.start(...)
	end,
})

---@param id string
---@param banner string
function repl.start(id, banner)
	local self = setmetatable({}, repl)
	self.prompt = vim.tbl_deep_extend("keep", self.prompt or {}, {
		input = ">",
		continue = "+",
	})
	self.prompt_template = vim.tbl_deep_extend("keep", self.prompt_template or {}, {
		input = "%s ",
		continue = "%s ",
	})
	self._ns = vim.api.nvim_create_namespace("jet_repl_" .. id)
	self._augroup = vim.api.nvim_create_augroup("jet_repl_" .. id, {})
	self:_init_repl()
	self:_filetype_set()
	self:ui_show()
	self:_display_repl_text(banner and utils.add_linebreak(banner))

	vim.api.nvim_create_autocmd("WinLeave", {
		group = self._augroup,
		callback = function()
			self.last_win = vim.api.nvim_get_current_win()
			self[vim.b.jet and vim.b.jet.id == id and "last_jet_win" or "last_normal_win"] = self.last_win
		end,
	})

	return self
end
