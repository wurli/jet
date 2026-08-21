-------------------------------------------------------------------------------
-- jet-lua smoke test: `inspect_request`. Ask ipykernel for info about
-- `print` and assert the reply is `ok` with `found=true`.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("python3")

local code = "print"
local cb, _msg_id = utils.jet.inspect(kernel.client_id, code, #code, 0)

local saw_reply = false
local start_time = os.clock()
while true do
	assert(os.clock() - start_time < 20, "inspect timed out")
	local res = cb()
	if res.status == "done" then
		break
	end
	if res.status == "ready" then
		local msg = res.value
		if msg.header and msg.header.msg_type == "inspect_reply" then
			saw_reply = true
			assert(msg.channel == "shell", "expected inspect_reply on shell, got " .. tostring(msg.channel))
			assert(
				msg.content and msg.content.status == "ok",
				"expected content.status == 'ok', got " .. tostring(msg.content and msg.content.status)
			)
			assert(msg.content.found == true, "expected content.found == true for `print`")
		end
	end
end
assert(saw_reply, "never received an inspect_reply")

kernel:stop()
