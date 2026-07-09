-------------------------------------------------------------------------------
-- jet-lua smoke test: input_request round-trip.
-- Run `v = input(); print('GOT:'..v)`, drain until input_request shows up,
-- send a reply via provide_stdin, keep draining, assert GOT:hello arrives.
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

-- Drive input_request round-trip ---------------------------------------------
local received_input_request = false
local received_value = ""

for res in kernel:execute("v = input('ASK> '); print('GOT:' + v)", 5) do
	local msg = res.msg
	if res.status == "busy" then
		if msg.header.msg_type == "input_request" then
			received_input_request = true
			kernel:provide_stdin("", "bananas")
		elseif msg.header.msg_type == "stream" and msg.content and msg.content.text then
			received_value = received_value .. msg.content.text
		end
	end
end

assert(
	received_input_request and received_value:find("GOT:bananas") ~= nil,
	"expected input_request followed by 'GOT:bananas' in stream output"
)

-- Shut down kernel -----------------------------------------------------------
kernel:stop()
