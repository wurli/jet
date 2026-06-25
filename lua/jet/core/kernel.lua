local engine = require("jet.core.engine")

---@class jet.kernel
---@field spec string
---@field id string
---@field info table
local Kernel = {}
Kernel.__index = Kernel

setmetatable(Kernel, {
	---@return jet.kernel
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param kernelspec string
function Kernel.new(kernelspec)
	local id, info = engine.connect(kernelspec)
	return setmetatable({ id = id, info = info, spec = kernelspec }, Kernel)
end

---@param code string | string[]
---@param user_expressions table<string, string>?
function Kernel:execute(code, user_expressions)
	if type(code) == "table" then
		code = table.concat(code, "\n")
	end

	local callback = engine.execute_code(self.id, code, user_expressions or {})
end

return Kernel
