local get = require("jet.core.send.get_code")

local M = {}

---@code string[]
local send_impl = function(code)
	if not code or #code == 0 then
		return
	end

	local ft, _ = require("jet.core.send.utils").local_lang_info()

	require("jet.core.api").get_connected({ filetype = ft, primary = true }, function(k)
		table.insert(code, "")
		k:send_repl(code)
	end)
end

M.send_chunk = function()
	--
end

M.send_auto = function()
	send_impl(get.get_auto())
end

return M
