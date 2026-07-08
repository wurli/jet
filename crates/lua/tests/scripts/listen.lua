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

-- Try jet.core.engine for convenience when testing in Neovim
local lib_ok, jet = pcall(require, "jet.core.engine")
if not lib_ok then
	jet = require("jet") --[[@as jet.engine]]
end

-- Make sibling `utils.lua` requirable regardless of cwd.
local script_dir = debug.getinfo(1, "S").source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start kernel ---------------------------------------------------------------
local kernel = utils.start_kernel(jet, utils.kernel_spec())
assert(type(kernel.stream) == "function", "expected start response to include a `stream` poll")

-- A separate filtered listen registered explicitly. --------------------------
local iopub_streams = kernel.listen({ channel = "iopub", msg_type = "stream" })

-- Execute something and drain to idle (drives traffic through the listeners).
---@diagnostic disable-next-line: empty-block
for _ in kernel.execute("print(1 + 1)") do
	-- Drain to idle
end

-- Check for messages in 'global' listener ------------------------------------
local saw_iopub_stream_2 = false
for res in kernel.stream() do
	local msg = res.msg
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
		saw_iopub_stream_2 = true
	end
	if
		msg.channel == "iopub"
		and msg.header.msg_type == "status"
		and msg.content
		and msg.content.execution_state == "idle"
	then
		break
	end
end
-- TODO: this seems to (very rarely) fail.
assert(saw_iopub_stream_2, "expected to see 'iopub'/stream frame containing '2' in firehose")

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
kernel.stop()

-- Drain iterators ------------------------------------------------------------
---@diagnostic disable-next-line: empty-block
for _ in kernel.stream() do
end
---@diagnostic disable-next-line: empty-block
for _ in iopub_streams() do
end
