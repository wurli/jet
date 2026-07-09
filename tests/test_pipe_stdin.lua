-- Regression test: `jet start` spawned via plain `jobstart` (no `term=true`)
-- must receive every chansend line over its piped stdin.
--
-- The original bug: crates/cli/src/repl.rs's interrupt-byte watcher was
-- reading from STDIN_FILENO unconditionally. In pipe mode it raced
-- BufReader::read_line and silently swallowed every byte that wasn't
-- 0x03 (^C). Symptom from the user: only the FIRST chansend line reached
-- the kernel; everything after was dropped.
--
-- The plugin's own `Kernel:send_repl` runs over a pty (via `term=true`),
-- so this scenario is reached when callers use `jobstart` directly —
-- exactly what the bug report described. We reproduce it here.

local new_set = MiniTest.new_set
local expect = MiniTest.expect

local KERNEL_JSON = os.getenv("JET_KERNEL_JSON")
assert(KERNEL_JSON, "JET_KERNEL_JSON env var must be set to a kernel.json path")

local child = MiniTest.new_child_neovim()

local T = new_set({
	hooks = {
		pre_case = function()
			child.restart({ "-u", "scripts/minimal_init.lua" })
		end,
		post_once = child.stop,
	},
})

T["chansend over plain pipe stdin delivers every line"] = function()
	-- jobstart's on_stdout/on_stderr callbacks are functions, which can't
	-- cross the RPC boundary — they have to be created inside the child.
	-- Everything else (chansend, polling _G.output) goes through redirectors.
	child.lua(
		[[
			_G.output = {}
			local function collect(_, data, _)
				for _, l in ipairs(data) do table.insert(_G.output, l) end
			end
			_G.job_id = vim.fn.jobstart({ "jet", "start", ... }, {
				on_stdout = collect,
				on_stderr = collect,
			})
		]],
		{ KERNEL_JSON }
	)

	local job_id = child.lua_get("_G.job_id")
	expect.no_equality(job_id, 0)
	expect.no_equality(job_id, -1)

	vim.wait(3000)

	local marker = "JETPIPEOK-" .. os.time() .. "-" .. math.random(1e9)
	child.fn.chansend(job_id, { "x = 1", "" })
	child.fn.chansend(job_id, { "x = x + 1", "" })
	child.fn.chansend(job_id, { 'print("' .. marker .. ':" + str(x))', "" })

	local needle = marker .. ":2"
	local ok = vim.wait(15000, function()
		for _, line in ipairs(child.lua_get("_G.output")) do
			if line:find(needle, 1, true) then
				return true
			end
		end
		return false
	end, 100)

	if not ok then
		error(
			'jet swallowed chansend lines over pipe stdin.\nexpected "'
				.. needle
				.. '" in stdout.\ngot:\n  '
				.. table.concat(child.lua_get("_G.output"), "\n  ")
		)
	end
end

return T
