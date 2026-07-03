local utils = require("jet.core.send.utils")

local M = {}

M.send_chunk = function()
	--
end

--TODO: I think this API probably needs some polish. Doesn't feel particularly
--elegant to me. This function probs does too much:
-- * (optionally) gets code to send
-- * filters out comments and blank lines
-- * Finds a kernel
-- * Sends the code to the kernel
-- * (optionally) moves the cursor
---@param r jet.send.Range?
---@param move_cursor boolean?
M.send_auto = function(r, move_cursor)
	move_cursor = move_cursor == nil and true or false

	if not r then
		local curr_row = vim.fn.line(".") - 1
		local expr_row = utils.next_significant_line({ buf = 0, row = curr_row - 1, col = 0 }) or curr_row
		r = require("jet.core.send.get_code").get_auto({ buf = 0, row = expr_row, col = 0 })
		if not r then
			return
		end
	end

	local text = vim.api.nvim_buf_get_text(r.buf, r.start_row, r.start_col, r.end_row, r.end_col, {})

	if #text == 0 then
		return
	end

	local lang_info = utils.local_lang_info({ buf = r.buf, row = r.start_row, col = r.start_col })
	local ft, commentstring = lang_info.filetype, lang_info.commentstring

	local code_filtered = vim.tbl_filter(function(line)
		return line:match("%S") ~= nil and not utils.is_comment(line, commentstring)
	end, text)

	if #code_filtered == 0 then
		return
	end

	require("jet.core.api").get_connected({ filetype = ft, primary = true }, function(k)
		table.insert(code_filtered, "")
		k:send_repl(code_filtered)

		if move_cursor then
			local next_line = r.end_row
			local next_significant_line = utils.next_significant_line({
				buf = r.buf,
				row = next_line,
				col = 0,
			}) or (next_line + 1)
			vim.fn.cursor(next_significant_line + 1, 0)
		end

		if vim.fn.mode():lower() == "v" then
			local esc_termcode = "\27"
			vim.api.nvim_feedkeys(esc_termcode, "n", false)
		end
	end)
end

M.send_motion = function()
	return require("jet.core.send.get_code").get_motion(function(rng)
		M.send_auto(rng, false)
	end)
end

return M
