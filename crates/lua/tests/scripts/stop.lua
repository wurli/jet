-- Make sibling `utils.lua` requirable regardless of cwd.
local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

---@param k jet.testing.kernel
local kernel_alive = function(k)
	for _, session in ipairs(utils.list_sessions()) do
		if session.session_id == k.session_id then
			return true
		end
	end
	return false
end

-- Ark ------------------------------------------------------------------------
local ark = utils.start_kernel("ark")

assert(kernel_alive(ark), "Kernel should be alive after start")
ark:stop()
assert(not kernel_alive(ark), "Kernel should not be alive after stop")

-- Broken ---------------------------------------------------------------------
local sessions1 = utils.list_sessions()
local ok, _ = pcall(utils.start_kernel, "broken")
local sessions2 = utils.list_sessions()

assert(not ok, "Broken kernel should not start successfully")

for _, session in ipairs(sessions2) do
	local session_preexisting = false
	for _, session1 in ipairs(sessions1) do
		if session.session_id == session1.session_id then
			session_preexisting = true
		end
	end
	if not session_preexisting then
		error("Kernel should not be alive after failed start")
	end
end
