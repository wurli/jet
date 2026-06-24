-------------------------------------------------------------------------------
-- Smoke test: start a Python kernel, run print(1+1), drain frames until idle,
-- assert "2" appears in stream output. Exits 0 on success, nonzero on failure.
-------------------------------------------------------------------------------

-- Find libs ------------------------------------------------------------------

-- Try jet.core.engine for convenience when testing in Neovim
local ok, jet = pcall(require, "jet.core.engine")
if not ok then
	jet = require("jet") --[[@as jet.engine]]
end

-- Make sibling `utils.lua` requirable regardless of cwd.
local script_dir = debug.getinfo(1, "S").source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start kernel ---------------------------------------------------------------
local kernel_id, info = jet.connect("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

assert(type(kernel_id) == "string" and #kernel_id > 0, "expected session id from connect")
assert(type(info) == "table", "expected kernel info table")

-- Simple addition ------------------------------------------------------------
utils.try_run(jet, kernel_id, "print(1 + 1)", function(res)
	return res.status == "busy" and res.type == "stream" and res.data and res.data.text and res.data.text:find("2")
end, "expected '2' in stream output")

-- Error message --------------------------------------------------------------
utils.try_run(jet, kernel_id, "raise ValueError('bananas')", function(res)
	return res.status == "busy"
		and res.type == "error"
		and res.data
		and res.data.traceback
		and table.concat(res.data.traceback):find("bananas")
end, "expected 'bananas' in error message")

-- Shut down kernel -----------------------------------------------------------
jet.stop(kernel_id)

-- Check kernel stopped -------------------------------------------------------
for session_name, _ in pairs(jet.list_sessions()) do
	assert(session_name ~= kernel_id, "expected kernel to be stopped")
end
