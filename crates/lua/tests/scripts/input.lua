-------------------------------------------------------------------------------
-- Smoke test: input_request round-trip.
-- Run `v = input(); print('GOT:'..v)`, drain until input_request shows up,
-- send a reply via provide_stdin, keep draining, assert GOT:hello arrives.
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
local kernel_id = jet.connect("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

-- Drive input_request round-trip ---------------------------------------------
local received_input_request = false
local received_value = ""

utils.try_run(jet, kernel_id, "v = input('ASK> '); print('GOT:' + v)", function(res)
	if res.status == "busy" then
		if res.type == "input_request" then
			received_input_request = true
			jet.provide_stdin(kernel_id, "", "bananas")
		elseif res.type == "stream" and res.data and res.data.text then
			received_value = received_value .. res.data.text
		end
	end
	return received_input_request and received_value:find("GOT:bananas") ~= nil
end, "expected input_request followed by 'GOT:bananas' in stream output")

-- Shut down kernel -----------------------------------------------------------
jet.stop(kernel_id)
