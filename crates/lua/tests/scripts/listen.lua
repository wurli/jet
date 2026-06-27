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

local lib_ok, jet = pcall(require, "jet.core.engine")
if not lib_ok then
	jet = require("jet") --[[@as jet.engine]]
end

local spec = os.getenv("JET_TEST_KERNEL")
assert(spec and #spec > 0, "JET_TEST_KERNEL must be set")

-- Start kernel, grab the boot-time stream from the response. ----------------
local start_poll = jet.start(spec)
local con
while true do
	local r = start_poll()
	assert(r ~= nil, "start poll returned nil before ready")
	if r.status == "ready" then
		con = r
		break
	end
end
assert(type(con.stream) == "function", "expected start response to include a `stream` poll")

local kernel = {
	client_id = con.client_id,
	session_id = con.session_id,
	execute = function(code)
		local cb = jet.execute_code(con.client_id, code, {})
		return function()
			while true do
				local res = cb()
				if not res then
					return nil
				end
				if res.status ~= "pending" then
					return res
				end
			end
		end
	end,
	stop = function()
		jet.stop(con.session_id)
	end,
}

-- A separate filtered listen registered explicitly. -------------------------
local iopub_streams_cb = jet.listen(con.client_id, { channel = "iopub", msg_type = "stream" })

-- Execute something and drain to idle (drives traffic through the listeners).
for _ in kernel.execute("print(1 + 1)") do
end

-- Now drain the firehose stream non-blocking: pull until we hit "pending"
-- twice in a row (the kernel is idle, nothing more is coming for now).
local function drain_pending(cb)
	local out = {}
	local empties = 0
	while empties < 5 do
		local res = cb()
		if res == nil then
			break
		elseif res.status == "pending" then
			empties = empties + 1
		else
			empties = 0
			table.insert(out, res)
		end
	end
	return out
end

local stream_frames = drain_pending(con.stream)
local filtered_frames = drain_pending(iopub_streams_cb)

-- All firehose frames carry a valid channel field. --------------------------
local saw_iopub_stream_2 = false
for _, f in ipairs(stream_frames) do
	assert(
		f.channel == "shell"
			or f.channel == "iopub"
			or f.channel == "stdin"
			or f.channel == "control",
		"expected channel field, got: " .. tostring(f.channel)
	)
	if f.channel == "iopub" and f.type == "stream" and f.data and f.data.text and f.data.text:find("2") then
		saw_iopub_stream_2 = true
	end
end
assert(saw_iopub_stream_2, "expected to see 'iopub'/stream frame containing '2' in firehose")

-- Filtered listen sees stream frames only, all on iopub. --------------------
assert(#filtered_frames > 0, "expected filtered listen to see at least one stream frame")
for _, f in ipairs(filtered_frames) do
	assert(f.channel == "iopub", "filter violated channel constraint: " .. tostring(f.channel))
	assert(f.type == "stream", "filter violated type constraint: " .. tostring(f.type))
end

-- Shutting the kernel down ends the streams (poll returns nil). -------------
kernel.stop()

-- Give the kernel_exit watcher a moment to flip status.
os.execute("sleep 0.5")

local function poll_to_nil(cb, label)
	for _ = 1, 200 do
		local res = cb()
		if res == nil then
			return true
		end
		-- Skip pending; we just want to confirm the stream eventually ends.
	end
	error(label .. ": stream never returned nil after kernel stopped")
end

poll_to_nil(con.stream, "firehose")
poll_to_nil(iopub_streams_cb, "filtered")
