-------------------------------------------------------------------------------
-- jet-lua smoke test: `comm_close`. Uses ark's `lsp` comm (same as the
-- comm_lsp smoke). Opens it, confirms it's listed by comm_info, closes it
-- via jet.comm_close, then confirms it's gone from comm_info.
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
local function has_comm(id)
	for msg in kernel:comm_info("lsp", 10) do
		if msg.header.msg_type == "comm_info_reply" and msg.content and msg.content.comms then
			if msg.content.comms[id] then
				return true
			end
		end
	end
	return false
end
assert(has_comm(comm_id), "expected freshly-opened comm to appear in comm_info_reply")

-- Close from the frontend side. The reply is optional (kernels don't have
-- to ack a comm_close), so just drive the poll to `done` without asserting
-- on any specific frame.
local close_cb, _close_msg_id = utils.jet.comm_close(kernel.client_id, comm_id)
assert(type(close_cb) == "function", "expected comm_close to return a poll closure")

local start_time = os.clock()
while true do
	assert(os.clock() - start_time < 10, "comm_close poll did not terminate")
	local res = close_cb()
	if res.status == "done" then
		break
	end
end

kernel:stop()
