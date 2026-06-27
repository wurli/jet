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
local kernel = utils.start_kernel(jet, "/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
assert(type(kernel.stream) == "function", "expected start response to include a `stream` poll")

-- A separate filtered listen registered explicitly. -------------------------
local iopub_streams = kernel.listen({ channel = "iopub", msg_type = "stream" })

-- Execute something and drain to idle (drives traffic through the listeners).
---@diagnostic disable-next-line: empty-block
for _ in kernel.execute("print(1 + 1)") do
	-- Drain to idle
end

-- Firehose: iterate until we see `iopub`/status:idle, which the kernel
-- always emits as the last frame for an execute request. Along the way
-- every frame must carry a valid channel, and we expect to spot the "2"
-- stream frame. execute() above already drained that request to idle, so
-- all of its frames are buffered ahead of us — no time-based exit needed.
local saw_iopub_stream_2 = false
for msg in kernel.stream() do
	assert(
		msg.channel == "shell" or msg.channel == "iopub" or msg.channel == "stdin" or msg.channel == "control",
		"expected channel field, got: " .. tostring(msg.channel)
	)
	if msg.channel == "iopub" and msg.type == "stream" and msg.data and msg.data.text and msg.data.text:find("2") then
		saw_iopub_stream_2 = true
	end
	if msg.channel == "iopub" and msg.type == "status" and msg.data and msg.data.execution_state == "idle" then
		break
	end
end
assert(saw_iopub_stream_2, "expected to see 'iopub'/stream frame containing '2' in firehose")

-- Filtered listen sees stream frames only, all on iopub. The print emitted
-- one stream frame; one busy frame from the iterator is our terminator.
local filtered_count = 0
for msg in iopub_streams() do
	filtered_count = filtered_count + 1
	assert(msg.channel == "iopub", "filter violated channel constraint: " .. tostring(msg.channel))
	assert(msg.type == "stream", "filter violated type constraint: " .. tostring(msg.type))
	break
end
assert(filtered_count > 0, "expected filtered listen to see at least one stream frame")

-- Shutting the kernel down ends the streams: each iterator should exit on
-- its own (the underlying poll returns nil once the sockets close).
kernel.stop()

---@diagnostic disable-next-line: empty-block
for _ in kernel.stream() do
end
---@diagnostic disable-next-line: empty-block
for _ in iopub_streams() do
end
