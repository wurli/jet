local config = require("jet.config")

local M = {}

---@param opts jet.config
M.setup = function(opts)
	config.set(opts)
end

return M
