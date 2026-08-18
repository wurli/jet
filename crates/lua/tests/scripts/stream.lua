-- Find libs ------------------------------------------------------------------

-- Make sibling `utils.lua` requirable regardless of cwd.
local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start kernel ---------------------------------------------------------------
local ipython = utils.start_kernel("python3")
local ark = utils.start_kernel("ark")

ipython:execute("print('this is ipython')", 20)
ark:execute("print('this is ark')", 20)

for res in ipython:stream(5, "break") do
	local msg = res.msg
	if msg then
		if msg.header.msg_type == "stream" then
			assert(not msg.content.text:find("this is ark"), "ipython stream should not contain ark output")
		end
	end
end

for res in ark:stream(1, "break") do
	local msg = res.msg
	if msg then
		if msg.header.msg_type == "stream" then
			assert(not msg.content.text:find("this is ipython"), "ark stream should not contain ipython output")
		end
	end
end
