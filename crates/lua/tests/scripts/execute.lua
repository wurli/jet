-------------------------------------------------------------------------------
-- jet-lua smoke test: start a Python kernel, run print(1+1), drain frames
-- until idle, assert "2" appears in stream output. Exits 0 on success, nonzero
-- on failure.
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

-- Simple addition ------------------------------------------------------------
local ok1 = false
for msg in kernel.execute("print(1 + 1)") do
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
local ok2 = false
for msg in kernel.execute("raise ValueError('bananas')") do
	ok2 = msg.status == "busy"
		and msg.msg.header.msg_type == "error"
		and msg.msg.content
		and msg.msg.content.traceback
		and table.concat(msg.msg.content.traceback):find("bananas")
	if ok2 then
		break
	end
end
assert(ok2, "expected 'bananas' in error message")

-- Shut down kernel -----------------------------------------------------------
kernel.stop()

-- Check kernel stopped -------------------------------------------------------
-- list_connections() = clients open in this process, keyed by client_id.
-- list_sessions() now reads from disk and is keyed by session_id; that's a
-- different concept (a Closed session.json still appears there).
for client_id, _ in pairs(jet.list_connections()) do
	assert(client_id ~= kernel.id, "expected kernel to be stopped")
end
