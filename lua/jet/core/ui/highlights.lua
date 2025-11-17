-- vim.api.nvim_set_hl(ns_id, name, val)
local M = {}

local hl_modify = function(group, opts)
	local hl = vim.api.nvim_get_hl(0, { name = group, create = false })
	if vim.tbl_count(hl) == 0 then
		return
	end

	for k, v in pairs(opts) do
		hl[k] = v ~= "none" and v or nil
	end

	return hl
end

-- stylua: ignore
local hlgroups = {
    -- Background for the REPL floating window itself
    JetRepl               = { link = "Float" },
    JetReplInput          = { link = "JetRepl" },
    JetReplOutput         = { link = "JetRepl" },
	JetReplIndent         = { link = "WarningMsg" },

    -- 'Indent' character for the REPL input
	JetReplIndentMain     = { link = "JetReplIndent" },
	JetReplIndentContinue = { link = "JetReplIndent" },

    JetReplSpinner        = hl_modify("LineNr", { bg = "none" })
}

M.set = function()
	for name, val in pairs(hlgroups) do
		val.default = true
		---@diagnostic disable-next-line: param-type-mismatch
		vim.api.nvim_set_hl(0, name, val)
	end
end

return M
