local utils = require("jet.core.utils")

local M = {}

---For notebook code chunks we track the treesitter node rather than the buffer
---text. This is because the user might edit the chunk between executions, and
---this lets us update rather than re-create the chunk when this happens. This
---would also be quite hard to do, since we'd need to somehow know when it's
---okay to delete old chunks.
---
---@class Jet.Execute.Chunk
---@field bufnr number
---@field winnr number
---@field filetype string Position filetype, not buffer filetype
---@field node TSNode

---@return Jet.Execute.Chunk?
M.get_chunk = function()
	-- Note: we want the filetype for the _buffer_, not at the cursor
	local ok, ft_module = pcall(require, "jet.filetype." .. vim.bo.filetype)
	if ok and ft_module.get_chunk then
		return ft_module.get_chunk()
	end

	utils.log_debug("Couldn't get Jet filetype module: " .. vim.inspect(ft_module))
end

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
	-- Note: we want the filetype at the _cursor_, not the buffer filetype
	local ok, ft_module = pcall(require, "jet.filetype." .. utils.get_cur_filetype())
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
