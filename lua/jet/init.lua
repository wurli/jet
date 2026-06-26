local config = require("jet.config")

local M = {}

---@param opts jet.config
M.setup = function(opts)
	config.set(opts)
	require("jet.core.cmd").setup()
end

return M
