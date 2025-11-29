---@class Jet.Config: Jet.Config.Opts
local M = {}

---@class Jet.Config.Opts
local defaults = {}

---@as Jet.Config.Opts
local config = vim.deepcopy(defaults)

---@param opts? Jet.Config.Opts
function M.set(opts)
	config = vim.tbl_deep_extend("force", {}, defaults, opts or {})
end

setmetatable(M, {
	__index = function(_, key)
		return config[key]
	end,
})

return M
