local config = require("jet.config")

local M = {}

-- Jet extensions might want to install custom (nvim specific) kernelspecs.
-- Prepending ~/.local/share/nvim/jet to JUPYTER_PATH means this dir will be
-- seached first when running Jet from nvim - but not in other contexts.
local modify_jupyter_path = function()
	local pathsep = vim.fn.has("win32") == 1 and ";" or ":"
	vim.env.JUPYTER_PATH = table.concat({ config.data.jet_nvim_data_dir, vim.env.JUPYTER_PATH }, pathsep)
end

---@param opts jet.config
M.setup = function(opts)
	modify_jupyter_path()
	config.set(opts)
	require("jet.core.cmd").setup()
end

return M
