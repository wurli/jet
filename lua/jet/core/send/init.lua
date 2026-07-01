local get = require("jet.core.send.get_code")
local utils = require("jet.core.send.utils")

local M = {}

M.send_chunk = function()
	--
end

M.send_auto = function()
	local code = get.get_auto()
	local lnum = vim.fn.line(".")

	if not code or #code == 0 then
		return
	end

	local ft, commentstring = utils.local_lang_info()

	---@param line string
	local is_significant = function(line)
		return line and line:match("%S") and not utils.is_comment(line, commentstring)
	end

	local code_filtered = vim.tbl_filter(is_significant, code)

	require("jet.core.api").get_connected({ filetype = ft, primary = true }, function(k)
		table.insert(code_filtered, "")
		k:send_repl(code_filtered)

		local new_lnum = lnum + #code
		while not is_significant(vim.api.nvim_buf_get_lines(0, new_lnum - 1, new_lnum, false)[1]) do
			new_lnum = new_lnum + 1
		end
		vim.fn.cursor(new_lnum, 0)

		if vim.fn.mode():lower() == "v" then
			local esc_termcode = "\27"
			vim.api.nvim_feedkeys(esc_termcode, "n", false)
		end
	end)
end

return M
