local M = {}

---@class Jet.Execute.Code
---@field code string[]
---@field region? Jet.Execute.Region

---@class Jet.Execute.Region
---@field bufnr number
---@field winnr number
---@field filetype string The filetype of the code is not always the filetype of the buffer
---@field start_row number
---@field start_col number
---@field end_row number
---@field end_col number

---@return string[]
M.get_code_auto = function()
	local mode = vim.fn.mode()
	if vim.tbl_contains({ "v", "V", "" }, mode) then
		return M.get_visual()
	end
	return M.get_curr_expr()
end

---@return string[]
M.get_curr_expr = function()
	local buf_ft = vim.bo.filetype

	local ok, ft_module = pcall(require, "jet.filetype." .. buf_ft)
	if not ok or not ft_module.get_expr then
		return M.get_curr_line()
	end

	return ft_module.get_expr()
end

---@return string[]
M.get_curr_line = function()
	return { vim.api.nvim_get_current_line() }
end

---@return string[]
M.get_visual = function()
	local mode = vim.fn.mode()
	if vim.tbl_contains({ "v", "V", "" }, mode) then
		return vim.fn.getregion(vim.fn.getpos("v"), vim.fn.getpos("."), { type = mode })
	end
	return M.get_curr_line()
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
---@param callback fun(code: string[])
---@return fun(): "g@" # A function that can be used in an operator-pending mapping
M.handle_motion = function(callback)
	return function()
		-- Unfortunately doesn't seem to work if the callback is a member of this module
		_G.JET_OP_PENDING_CALLBACK = callback
		vim.o.operatorfunc = "v:lua.require'jet.core.execute'._handle_curr_motion"
		return "g@"
	end
end

---@param mode "line" | "block" | "char"
M._handle_curr_motion = function(mode)
	if not _G.JET_OP_PENDING_CALLBACK then
		return
	end

	local code = vim.fn.getregion(vim.fn.getpos("'["), vim.fn.getpos("']"), {
		type = mode == "line" and "V" or mode == "block" and "" or mode == "char" and "v",
	})

	_G.JET_OP_PENDING_CALLBACK(code)
	_G.JET_OP_PENDING_CALLBACK = nil
end

return M
