local M = {}

---@class jet.autocmd.data.JetKernelStarted
---@field session_id string

---@param data jet.autocmd.data.JetKernelStarted
M.kernel_started = function(data)
	vim.api.nvim_exec_autocmds("User", { pattern = "JetKernelStarted", data = data })
end

return M
