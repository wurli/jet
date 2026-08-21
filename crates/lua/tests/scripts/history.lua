-------------------------------------------------------------------------------
-- jet-lua smoke test: `history_request`. Execute a couple of statements so
-- the kernel has something to remember, then exercise all three modes
-- (`tail`, `range`, `search`) and assert a `history_reply` comes back for
-- each. History content is kernel-dependent (ipykernel stores in-process
-- history), so we only assert on shape, not entries.
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "Failed to determine script dir for kernel spec path")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local kernel = utils.start_kernel("python3")

-- Populate a bit of history first.
---@diagnostic disable-next-line: empty-block
for _ in kernel:execute("x = 41", 10) do
end
---@diagnostic disable-next-line: empty-block
for _ in kernel:execute("x + 1", 10) do
end

local function drain_history_reply(cb, tag)
	local start_time = os.clock()
	while true do
		assert(os.clock() - start_time < 20, "history[" .. tag .. "] timed out")
		local res = cb()
		if res.status == "done" then
			error("history[" .. tag .. "] ended without a reply")
		end
		if res.status == "ready" then
			local msg = res.value
			if msg.header and msg.header.msg_type == "history_reply" then
				assert(
					msg.channel == "shell",
					"expected history_reply on shell, got " .. tostring(msg.channel)
				)
				assert(
					type(msg.content) == "table" and type(msg.content.history) == "table",
					"expected content.history table on " .. tag
				)
				return
			end
		end
	end
end

local tail_cb = utils.jet.history(kernel.client_id, "tail", { n = 5, output = false, raw = true })
drain_history_reply(tail_cb, "tail")

local range_cb = utils.jet.history(
	kernel.client_id,
	"range",
	{ session = 0, start = 0, stop = 10, output = false, raw = true }
)
drain_history_reply(range_cb, "range")

local search_cb = utils.jet.history(
	kernel.client_id,
	"search",
	{ pattern = "x*", n = 5, unique = false, output = false, raw = true }
)
drain_history_reply(search_cb, "search")

-- Bad mode -> Lua-level error.
local ok, err = pcall(utils.jet.history, kernel.client_id, "nope", {})
assert(not ok, "expected history() to error on unknown mode")
assert(tostring(err):find("unknown mode"), "expected 'unknown mode' error, got: " .. tostring(err))

-- `search` without `pattern` -> Lua-level error.
local ok2, err2 = pcall(utils.jet.history, kernel.client_id, "search", {})
assert(not ok2, "expected history('search', {}) to error without `pattern`")
assert(tostring(err2):find("pattern"), "expected 'pattern' error, got: " .. tostring(err2))

kernel:stop()
