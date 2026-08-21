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
local reply = utils.await_msg_type(cb, "inspect_reply", 20)

assert(reply.channel == "shell", "expected inspect_reply on shell, got " .. tostring(reply.channel))
assert(
	reply.content and reply.content.status == "ok",
	"expected content.status == 'ok', got " .. tostring(reply.content and reply.content.status)
)
assert(reply.content.found == true, "expected content.found == true for `print`")

kernel:stop()
