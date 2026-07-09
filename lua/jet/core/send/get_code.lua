---@class jet.send.filetype
---@field get_chunk? fun(): jet.send.Range?
---@field get_curr_expr? fun(): string[]?

---@class jet.getcode
local M = {
	---@type table<string, jet.getcode>
	filetype = {
		markdown = require("jet.core.send.markdown"),
	},
}

--- TODO: swap for vim.Pos once it's stabilised
---@class jet.send.Pos
---@field buf integer
---@field row integer 0-indexed
---@field col integer 0-indexed

--- TODO: swap for vim.Range once it's stabilised
---@class jet.send.Range
---@field buf integer
---@field start_row integer 0-indexed
---@field start_col integer 0-indexed
---@field end_row integer 0-indexed
---@field end_col integer 0-indexed

---@return jet.send.Pos
M.curr_pos = function()
	local cursor = vim.api.nvim_win_get_cursor(0)
	return {
		buf = vim.api.nvim_get_current_buf(),
		row = cursor[1] - 1,
		col = cursor[2],
	}
end

---@param pos jet.send.Pos?
---@return jet.send.Range?
M.get_auto = function(pos)
	pos = pos or M.curr_pos()
	if vim.tbl_contains({ "v", "V", "" }, vim.fn.mode()) then
		return M.get_visual()
	end
	return M.get_expr(pos)
end

---@param pos jet.send.Pos?
---@return jet.send.Range?
M.get_expr = function(pos)
	pos = pos or M.curr_pos()
	-- Note: we want the filetype at the _cursor_, not the buffer filetype
	local ft_module = M.filetype[vim.bo.filetype]
	if ft_module and ft_module.get_expr then
		return ft_module.get_expr(pos)
	end

	return M.get_line(pos)
end

---@param pos jet.send.Pos?
---@return jet.send.Range?
M.get_line = function(pos)
	pos = pos or M.curr_pos()
	local line = vim.api.nvim_buf_get_lines(pos.buf, pos.row, pos.row + 1, false)[1]
	if not line then
		return
	end
	return {
		buf = pos.buf,
		start_row = pos.row,
		start_col = 0,
		end_row = pos.row + 1,
		end_col = #line,
	}
end

---@return jet.send.Range?
M.get_visual = function()
	local mode = vim.fn.mode()
	if vim.tbl_contains({ "v", "V", "" }, mode) then
		local region = vim.fn.getregionpos(vim.fn.getpos("v"), vim.fn.getpos("."), { type = mode })
		assert(region and region[1] and region[#region], "Failed to get visual region")

		local pos1 = region[1][1]
		local pos2 = region[#region][2]

		return {
			buf = vim.api.nvim_get_current_buf(),
			start_row = pos1[2] - 1,
			start_col = pos1[3] - 1,
			end_row = pos2[2] - 1,
			end_col = pos2[3],
		}
	end
	return M.get_line({
		buf = vim.api.nvim_get_current_buf(),
		row = vim.fn.line(".") - 1,
		col = vim.fn.col(".") - 1,
	})
end

---Can be used in mappings to handle the code moved over by a motion:
---
---```lua
---vim.keymap.set(
--    { "n", "v" },
--    "gj",
--    require("jet.core.execute").handle_motion(vim.print),
--    { expr = true }
--)
---```
---
---@param callback fun(code: jet.send.Range)
---@return fun(): "g@" # A function that can be used in an operator-pending mapping
M.get_motion = function(callback)
	return function()
		---@diagnostic disable-next-line: global-in-non-module
		-- Unfortunately doesn't seem to work if the callback is a member of this module
		_G.JET_OP_PENDING_CALLBACK = callback
		vim.o.operatorfunc = "v:lua.require'jet.core.send.get_code'._handle_curr_motion"
		return "g@"
	end
end

---@param mode "line" | "block" | "char"
M._handle_curr_motion = function(mode)
	if not _G.JET_OP_PENDING_CALLBACK then
		return
	end

	local region = vim.fn.getregionpos(vim.fn.getpos("'["), vim.fn.getpos("']"), {
		type = mode == "line" and "V"
			or mode == "block" and ""
			or mode == "char" and "v"
			-- Keeps lua_ls happy
			or "Something has gone wrong!",
	})
	assert(region and region[1] and region[#region], "Failed to get motion region")
	local pos1 = region[1][1]
	local pos2 = region[#region][2]

	local code = {
		buf = vim.api.nvim_get_current_buf(),
		start_row = pos1[2] - 1,
		start_col = pos1[3] - 1,
		end_row = pos2[2] - 1,
		end_col = pos2[3],
	}

	_G.JET_OP_PENDING_CALLBACK(code)
	---@diagnostic disable-next-line: global-in-non-module
	_G.JET_OP_PENDING_CALLBACK = nil
end

return M
