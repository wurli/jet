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
	ok1 = msg.status == "busy" and msg.type == "stream" and msg.data and msg.data.text and msg.data.text:find("2")
end
assert(ok1, "expected '2' in stream output")

-- Error message --------------------------------------------------------------
local ok2 = false
for msg in kernel.execute("raise ValueError('bananas')") do
	print(utils.dump(msg))
	ok2 = msg.status == "busy"
		and msg.type == "error"
		and msg.data
		and msg.data.traceback
		and table.concat(msg.data.traceback):find("bananas")
end
assert(ok2, "expected 'bananas' in error message")

-- Shut down kernel -----------------------------------------------------------
kernel.stop()

-- Check kernel stopped -------------------------------------------------------
for session_name, _ in pairs(jet.list_sessions()) do
	assert(session_name ~= kernel.id, "expected kernel to be stopped")
end
