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
local reply = utils.await_msg_type(cb, "debug_reply", 20)

assert(
	reply.channel == "control",
	"expected debug_reply on control channel, got " .. tostring(reply.channel)
)

kernel:stop()
