-------------------------------------------------------------------------------
-- jet-lua smoke test: `debug_request` on the control channel. Requires a
-- kernel that reports `debugger: true` in kernel_info (recent ipykernel).
-- Sends a DAP `initialize` and asserts a debug_reply comes back on control.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("python3")

-- The `kernel_info` returned at boot is the source of truth for whether
-- the kernel speaks DAP. Skip cleanly if it doesn't.
if not (kernel.kernel_info and kernel.kernel_info.debugger) then
	print("SKIP: kernel does not report debugger=true")
	kernel:stop()
	os.exit(0)
end

local cb, _msg_id = utils.jet.debug(kernel.client_id, {
	type = "request",
	command = "initialize",
	arguments = {
		clientID = "jet-test",
		clientName = "jet",
		adapterID = "ipykernel",
		pathFormat = "path",
		linesStartAt1 = true,
		columnsStartAt1 = true,
		supportsRunInTerminalRequest = false,
		locale = "en",
	},
})

local saw_reply = false
local start_time = os.clock()
while true do
	assert(os.clock() - start_time < 20, "debug_request timed out")
	local res = cb()
	if res.status == "done" then
		break
	end
	if res.status == "ready" then
		local msg = res.value
		if msg.header and msg.header.msg_type == "debug_reply" then
			saw_reply = true
			assert(
				msg.channel == "control",
				"expected debug_reply on control channel, got " .. tostring(msg.channel)
			)
		end
	end
end
assert(saw_reply, "never received a debug_reply")

kernel:stop()
