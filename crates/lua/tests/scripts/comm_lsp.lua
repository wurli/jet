-------------------------------------------------------------------------------
-- jet-lua smoke test: ark's 'lsp' comm. Open the comm with an ip_address,
-- then send a comm_info_request and assert the reply lists a comm whose
-- target_name is 'lsp'.
-------------------------------------------------------------------------------

-- Make sibling `utils.lua` requirable regardless of cwd.
local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start ark ------------------------------------------------------------------
local kernel = utils.start_kernel("ark")

-- Open LSP comm --------------------------------------------------------------
local lsp_comm_id, lsp_comm_msgs = kernel:comm_open("lsp", { ip_address = "127.0.0.1" })
assert(type(lsp_comm_id) == "string" and #lsp_comm_id > 0, "expected comm_id from comm_open")

-- Wait for the first reply to come back so we know the comm is ready
lsp_comm_msgs()

-- Check open comms -----------------------------------------------------------
local found_lsp = false
for res in kernel:comm_info("lsp", 10) do
	local msg = res.msg
	if msg.header.msg_type == "comm_info_reply" and msg.content and msg.content.comms then
		for _, info in pairs(msg.content.comms) do
			if info.target_name == "lsp" then
				found_lsp = true
				break
			end
		end
	end
end
assert(found_lsp, "expected comm_info_reply to list a comm with target_name='lsp'")

-- Open UI comm ---------------------------------------------------------------
local ui_comm_id, ui_comm_messages = kernel:comm_open("positron.ui", {})
assert(type(lsp_comm_id) == "string" and #lsp_comm_id > 0, "expected comm_id from comm_open")

-- Wait for the first reply to come back so we know the comm is ready
ui_comm_messages()

-- Listen on the UI comm ------------------------------------------------------
local ui_comm_notifications = kernel:comm_listen(ui_comm_id, 5)
local msg1 = ui_comm_notifications()
local msg2 = ui_comm_notifications()

local method1 = msg1 and msg1.msg and msg1.msg.content and msg1.msg.content.data and msg1.msg.content.data.method
local method2 = msg2 and msg2.msg and msg2.msg.content and msg2.msg.content.data and msg2.msg.content.data.method

if method1 == method2 then
	error(
		"Should not receive duplicate messages on the UI comm\n"
			.. "Got 1: "
			.. utils.dump(msg1)
			.. "\n"
			.. "Got 2: "
			.. utils.dump(msg2)
	)
end

kernel:stop()
