local utils = require("jet.core.utils")

---@class Jet.Extension.FileType
---@field get_chunk? fun(): Jet.Execute.Chunk?
---@field get_expr? fun(): string[]?
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

---@return Jet.Execute.Chunk?
M.get_chunk = function()
	local bufnr = vim.api.nvim_get_current_buf()
	local cursor_row = vim.fn.line(".")
	local cursor_col = vim.fn.col(".")

	if not vim.treesitter.get_parser(bufnr, nil, { error = false }) then
		return nil
	end

	local cursor_node = vim.treesitter.get_node({
		bufnr = bufnr,
		pos = { cursor_row - 1, cursor_col - 1 },
		ignore_injections = true,
	})

	if not cursor_node then
		return nil
	end

	local chunk_node = ascend_tree_until(cursor_node, "fenced_code_block")

	if not chunk_node then
		return nil
	end

	local lang_node = descend_tree_until(chunk_node, "info_string", "language")
	local code_node = descend_tree_until(chunk_node, "code_fence_content")

	if not (lang_node and code_node) then
		return nil
	end

	local code = vim.treesitter.get_node_text(code_node, bufnr, {})
	local range = { chunk_node:range(false) }

	return {
		bufnr = bufnr,
		winnr = vim.api.nvim_get_current_win(),
		filetype = utils.get_cur_filetype(),
		code = vim.split(code, "\n", { trimempty = false }),
		start_row = range[1],
		start_col = range[2],
		end_row = range[3],
		end_col = range[4],
	}
end

return M
