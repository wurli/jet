---@class Jet.Config: Jet.Config.Opts
local M = {}

---@class Jet.Config.Opts
local defaults = {
	image = {
		---@type string
		dir = vim.fn.stdpath("data") .. "/jet/images/",
		---@type "snacks"
		provider = "snacks",
	},
}

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
