-------------------------------------------------------------------------------
-- jet-lua smoke test: `kernel_info_request`. jet already does one at boot
-- (returned via the `start` handshake); this exercises the on-demand
-- Lua helper.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("python3")

local cb, _msg_id = utils.jet.kernel_info(kernel.client_id)

local saw_reply = false
local start_time = os.clock()
while true do
	assert(os.clock() - start_time < 20, "kernel_info timed out")
	local res = cb()
	if res.status == "done" then
		break
	end
	if res.status == "ready" then
		local msg = res.value
		if msg.header and msg.header.msg_type == "kernel_info_reply" then
			saw_reply = true
			assert(msg.channel == "shell", "expected kernel_info_reply on shell, got " .. tostring(msg.channel))
			assert(
				msg.content and msg.content.language_info and msg.content.language_info.name == "python",
				"expected language_info.name == 'python'"
			)
		end
	end
end
assert(saw_reply, "never received a kernel_info_reply")

kernel:stop()
