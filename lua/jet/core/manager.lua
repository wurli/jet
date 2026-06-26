---@class jet.manager
---@field kernels table<string, jet.kernel>
---@field primary table<string, string> key=filetype, value=session_id
local Manager = {
	kernels = {},
	primary = {},
}

---@param k jet.kernel
function Manager:insert(k)
	assert(not self.kernels[k.session_id], "Kernel with session_id " .. k.session_id .. " already exists")
	self.kernels[k.session_id] = k
end

---@param ft string
---@return jet.kernel[]
function Manager:get_ft_all(ft)
	---@param k jet.kernel
	return vim.tbl_filter(function(k)
		return k.filetype == ft
	end, self.kernels)
end

---@param ft string
---@return jet.kernel?
function Manager:get_ft_last_used(ft)
	local session_id = self.primary[ft]
	return session_id and self.kernels[session_id]
end

return Manager
