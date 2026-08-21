-------------------------------------------------------------------------------
-- jet-lua smoke test: `history_request`. Execute a statement so the kernel
-- has something to remember, then exercise all three modes (`tail`, `range`,
-- `search`) and assert a `history_reply` comes back for each. History
-- content is kernel-dependent (ipykernel stores in-process history), so we
-- only assert on shape, not entries.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("python3")

-- Populate a bit of history.
---@diagnostic disable-next-line: empty-block
for _ in kernel:execute("x = 41", 10) do
end

local function assert_history(mode, opts)
	local cb, _msg_id = utils.jet.history(kernel.client_id, mode, opts)
	local reply = utils.await_msg_type(cb, "history_reply", 20)
	assert(reply.channel == "shell", "expected history_reply on shell, got " .. tostring(reply.channel))
	assert(
		type(reply.content) == "table" and type(reply.content.history) == "table",
		"expected content.history table on " .. mode
	)
end

assert_history("tail", { n = 5, output = false, raw = true })
assert_history("range", { session = 0, start = 0, stop = 10, output = false, raw = true })
assert_history("search", { pattern = "x*", n = 5, unique = false, output = false, raw = true })

-- Bad mode -> Lua-level error.
local ok, err = pcall(utils.jet.history, kernel.client_id, "nope", {})
assert(not ok, "expected history() to error on unknown mode")
assert(tostring(err):find("unknown mode"), "expected 'unknown mode' error, got: " .. tostring(err))

kernel:stop()
