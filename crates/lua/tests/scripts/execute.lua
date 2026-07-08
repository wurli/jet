-------------------------------------------------------------------------------
-- jet-lua smoke test: start a Python kernel, run print(1+1), drain frames
-- until idle, assert "2" appears in stream output. Exits 0 on success, nonzero
-- on failure.
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

-- Simple addition ------------------------------------------------------------
local ok1 = false
for msg in kernel:execute("print(1 + 1)", 5) do
	ok1 = msg.status == "busy"
		and msg.msg.header
		and msg.msg.header.msg_type == "stream"
		and msg.msg.content
		and msg.msg.content.text
		and msg.msg.content.text:find("2")
	if ok1 then
		break
	end
end
assert(ok1, "expected '2' in stream output")

-- Error message --------------------------------------------------------------
local ok2 = nil
for msg in kernel:execute("raise ValueError('bananas')", 5) do
	ok2 = msg.status == "busy"
		and msg.msg.header.msg_type == "error"
		and msg.msg.content
		and msg.msg.content.traceback
		and table.concat(msg.msg.content.traceback):find("bananas")
	if ok2 then
		break
	end
end

-- Shut down kernel -----------------------------------------------------------
kernel:stop()

-- Check kernel stopped -------------------------------------------------------
-- list_connections() = clients open in this process, keyed by client_id.
-- list_sessions() now reads from disk and is keyed by session_id; that's a
-- different concept (a Closed session.json still appears there).
for client_id, _ in pairs(utils.jet.list_connections()) do
	assert(client_id ~= kernel.client_id, "expected kernel to be stopped")
end
