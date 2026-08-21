-------------------------------------------------------------------------------
-- jet-lua smoke test: `comm_close`. Uses ark's `lsp` comm (same as the
-- comm_lsp smoke). Opens it, confirms it's listed by comm_info, closes it
-- via jet.comm_close, then drains the close poll.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("ark")

-- Open LSP comm and wait for its first reply so we know it's ready.
local comm_id, comm_msgs = kernel:comm_open("lsp", { ip_address = "127.0.0.1" })
assert(type(comm_id) == "string" and #comm_id > 0, "expected comm_id from comm_open")
comm_msgs()

-- Confirm the comm shows up in comm_info.
local found_open = false
for msg in kernel:comm_info("lsp", 10) do
	if msg.header.msg_type == "comm_info_reply" and msg.content and msg.content.comms and msg.content.comms[comm_id] then
		found_open = true
		break
	end
end
assert(found_open, "expected freshly-opened comm to appear in comm_info_reply")

-- Close from the frontend side. The reply is optional (kernels don't have
-- to ack a comm_close), so just drive the poll to `done`.
local close_cb, _close_msg_id = utils.jet.comm_close(kernel.client_id, comm_id)
assert(type(close_cb) == "function", "expected comm_close to return a poll closure")
utils.drain(close_cb, 10)

kernel:stop()
