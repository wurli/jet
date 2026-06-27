local M = {}

---@class jet.autocmd.data.JetKernelStarted
---@field session_id string
---@field client_id string
---@field spec jet.kernel.spec
---@field kernelspec_path string
---@field kernel_info jet.kernel.info

---@param data jet.autocmd.data.JetKernelStarted
M.kernel_started = function(data)
	vim.api.nvim_exec_autocmds("User", { pattern = "JetKernelStarted", data = data })
end

return M
