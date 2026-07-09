-------------------------------------------------------------------------------
-- jet-lua smoke test: the firehose `listen` API.
--
-- 1. start() returns a `stream` poll closure (a no-filter listen registered
--    at boot).
-- 2. While we execute `print(1+1)`, the stream observes every busy/idle/
--    stream/execute_input frame the kernel emits, with the correct channel.
-- 3. A filtered listen({channel="iopub", msg_type="stream"}) sees only
--    stream frames on iopub.
-------------------------------------------------------------------------------

-- Find libs ------------------------------------------------------------------

-- Make sibling `utils.lua` requirable regardless of cwd.
local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start kernel ---------------------------------------------------------------
local kernel = utils.start_kernel("python3")
assert(type(kernel.stream) == "function", "expected start response to include a `stream` poll")

-- A separate filtered listen registered explicitly. --------------------------
local iopub_streams = kernel:listen({ channel = "iopub", msg_type = "stream" }, 60)

-- Execute something and drain to idle (drives traffic through the listeners).
---@diagnostic disable-next-line: empty-block
for _ in kernel:execute("print(1 + 1)", 3) do
	-- Drain to idle
end

-- Check for messages in 'global' listener ------------------------------------
for res in kernel:stream(10) do
	local msg = res.msg
	---@diagnostic disable-next-line: unnecessary-assert
	assert(
		msg.channel == "shell" or msg.channel == "iopub" or msg.channel == "stdin" or msg.channel == "control",
		"expected channel field, got: " .. tostring(msg.channel)
	)
	if
		msg.channel == "iopub"
		and msg.header.msg_type == "stream"
		and msg.content
		and msg.content.text
		and msg.content.text:find("2")
	then
		break
	end
end

-- Check for messages in filtered listener ------------------------------------
local filtered_count = 0
for res in iopub_streams() do
	local msg = res.msg
	filtered_count = filtered_count + 1
	assert(msg.channel == "iopub", "filter violated channel constraint: " .. tostring(msg.channel))
	assert(msg.header.msg_type == "stream", "filter violated type constraint: " .. tostring(msg.header.msg_type))
	break
end
assert(filtered_count > 0, "expected filtered listen to see at least one stream frame")

-- Shut down kernel -----------------------------------------------------------
kernel:stop()

-- Drain iterators ------------------------------------------------------------
---@diagnostic disable-next-line: empty-block
for _ in kernel:stream(10) do
end
---@diagnostic disable-next-line: empty-block
for _ in iopub_streams() do
end
