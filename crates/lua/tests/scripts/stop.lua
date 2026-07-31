-- Make sibling `utils.lua` requirable regardless of cwd.
local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("ark")

local kernel_alive = function()
	local poll = utils.jet.list_sessions()
	while true do
		local res = poll()
		assert(res ~= nil, "list_sessions poll ended before ready")
		if res.status == "ready" then
			for _, session in ipairs(res.sessions) do
				if session.session_id == kernel.session_id then
					return true
				end
			end
			return false
		end
	end
end

assert(kernel_alive(), "Kernel should be alive after start")

kernel:stop()

assert(not kernel_alive(), "Kernel should not be alive after stop")
