---@class Jet.Extension.FileType
---@field get_expr? fun(opts: Jet.GetExpr.Opts): Jet.GetExpr.Result?
local M = {}

---@param node TSNode
---@param target_type string
---@return TSNode?
local ascend_tree_until = function(node, target_type)
	while node do
		if node:type() == target_type then
			return node
		end
		---@diagnostic disable-next-line: cast-local-type
		node = node:parent()
	end
end

---@param node TSNode
---@param ... string
---@return TSNode?
local descend_tree_until = function(node, ...)
	local get_child_by_type1 = function(n, t)
		for child in n:iter_children() do
			if child:type() == t then
				return child
			end
		end
	end

	for _, t in ipairs({ ... }) do
		node = get_child_by_type1(node, t)
		if not node then
			return
		end
	end
	return node
end

---@class Jet.GetExpr.Result
---@field bufnr number
---@field winnr number
---@field filetype string The filetype of the code is not always the filetype of the buffer
---@field start_row number
---@field start_col number
---@field end_row number
---@field end_col number
---@field code string[]

---@class Jet.GetExpr.Opts
---
---Defaults to `vim.fn.line(".")`
---@field cursor_row number
---
---Defaults to `vim.fn.col(".")`
---@field cursor_col number
---
---Defaults to 0 the current buffer
---@field bufnr number
---
---Needed in order to know where to place the execution results
---@field winnr number

---@param opts Jet.GetExpr.Opts?
---@return Jet.GetExpr.Result?
M.get_expr = function(opts)
	opts = opts or {}
	if not opts.bufnr or opts.bufnr == 0 then
		opts.bufnr = vim.api.nvim_get_current_buf()
	end

	opts.cursor_row = opts.cursor_row or vim.fn.line(".")
	opts.cursor_col = opts.cursor_col or vim.fn.col(".")

	local node = vim.treesitter.get_node({
		bufnr = opts.bufnr,
		pos = { opts.cursor_row - 1, opts.cursor_col - 1 },
		ignore_injections = true,
	})

	if not node then
		return nil
	end

	local chunk_node = ascend_tree_until(node, "fenced_code_block")

	if not chunk_node then
		return nil
	end

	local lang_node = descend_tree_until(chunk_node, "info_string", "language")
	local code_node = descend_tree_until(chunk_node, "code_fence_content")

	if not (lang_node and code_node) then
		return nil
	end

	local language = vim.treesitter.get_node_text(lang_node, opts.bufnr, {})
	local code = vim.treesitter.get_node_text(code_node, opts.bufnr, {})
	local range = { chunk_node:range(false) }

	return {
		bufnr = opts.bufnr,
		winnr = opts.winnr,
		filetype = language,
		code = vim.split(code, "\n", { trimempty = false }),
		start_row = range[1],
		start_col = range[2],
		end_row = range[3],
		end_col = range[4],
	}
end

-- vim.keymap.set("n", "<cr>", function()
-- 	vim.print(M.get_expr())
-- end, { desc = "Get Markdown Code Block Expr" })

return M
