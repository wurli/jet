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
local reply = utils.await_msg_type(cb, "kernel_info_reply", 20)

assert(reply.channel == "shell", "expected kernel_info_reply on shell, got " .. tostring(reply.channel))
assert(
	reply.content and reply.content.language_info and reply.content.language_info.name == "python",
	"expected language_info.name == 'python'"
)

kernel:stop()
