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

for msg in ipython:stream(5, "break") do
	if msg.header.msg_type == "stream" then
		assert(not msg.content.text:find("this is ark"), "ipython stream should not contain ark output")
	end
end

for msg in ark:stream(1, "break") do
	if msg.header.msg_type == "stream" then
		assert(not msg.content.text:find("this is ipython"), "ark stream should not contain ipython output")
	end
end
