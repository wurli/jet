-- vim.api.nvim_set_hl(ns_id, name, val)
local M = {}

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

    -- Spinner shown when waiting for REPL response
	JetReplSpinner        = { link = "LineNr", blend = 0 },
}

M.set = function()
	for name, val in pairs(hlgroups) do
		if vim.fn.hlexists(name) == 0 then
			vim.api.nvim_set_hl(0, name, val)
		end
	end
end

return M
