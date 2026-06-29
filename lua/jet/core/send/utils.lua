local M = {}

-- Adapted from https://github.com/neovim/neovim/blob/master/runtime/lua/vim/_comment.lua
---@param buf? integer
---@param pos? [integer, integer]
---@return string, string # Filetype and commentstring
M.local_lang_info = function(buf, pos)
	buf = buf or 0
	pos = pos or { vim.fn.line("."), vim.fn.col(".") }
	local buf_ft = vim.bo[buf].filetype
	local buf_cs = vim.bo[buf].commentstring

	local ts_parser = vim.treesitter.get_parser(buf, "")
	if not ts_parser then
		return buf_ft, buf_cs
	end

	-- Try to get 'commentstring' associated with local tree-sitter language.
	-- This is useful for injected languages (like markdown with code blocks).
	local row, col = pos[1] - 1, pos[2]
	local ref_range = { row, col, row, col + 1 }

	-- Get 'commentstring' from tree-sitter captures' metadata.
	-- Traverse backwards to prefer narrower captures.
	local captures = vim.treesitter.get_captures_at_pos(buf, row, col)
	for i = #captures, 1, -1 do
		local id, metadata = captures[i].id, captures[i].metadata
		local metadata_cs = metadata["bo.commentstring"] or metadata[id] and metadata[id]["bo.commentstring"] --[[@as string?]]
		local metadata_ft = metadata["bo.filetype"] or metadata[id] and metadata[id]["bo.filetype"] --[[@as string?]]

		if metadata_cs and metadata_ft then
			return metadata_ft, metadata_cs
		end
	end

	-- - Get 'commentstring' from the deepest LanguageTree which both contains
	--   reference range and has valid 'commentstring' (meaning it has at least
	--   one associated 'filetype' with valid 'commentstring').
	--   In simple cases using `parser:language_for_range()` would be enough, but
	--   it fails for languages without valid 'commentstring' (like 'comment').
	local treesitter_ft, treesitter_cs, res_level = nil, nil, 0

	---@param lang_tree vim.treesitter.LanguageTree
	local function traverse(lang_tree, level)
		if not lang_tree:contains(ref_range) then
			return
		end

		treesitter_ft = lang_tree:lang()
		local filetypes = vim.treesitter.language.get_filetypes(treesitter_ft)
		for _, ft in ipairs(filetypes) do
			local cur_cs = vim.filetype.get_option(ft, "commentstring")
			if cur_cs ~= "" and level > res_level then
				treesitter_cs = cur_cs
				break
			end
		end

		for _, child_lang_tree in pairs(lang_tree:children()) do
			traverse(child_lang_tree, level + 1)
		end
	end
	traverse(ts_parser, 1)

	return (treesitter_ft or buf_ft), (treesitter_cs or buf_cs)
end

---@param text string
---@param commentstring string
M.is_comment = function(text, commentstring)
	local cs_left, cs_right = commentstring:match("^(.-)%s*%%s%s*(.-)$")

	local startswith = function(s, prefix)
		if #prefix == 0 then
			return true
		end
		return s:sub(1, #prefix) == prefix
	end
	local endswith = function(s, suffix)
		if #suffix == 0 then
			return true
		end
		return s:sub(-#suffix) == suffix
	end

	text = vim.trim(text)
	return startswith(text, cs_left) and endswith(text, cs_right)
end

return M
