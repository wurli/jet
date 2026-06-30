-- End-to-end test driving the jet neovim plugin: start a kernel via
-- `Kernel.init_owned`, open its REPL term, send two lines with
-- `Kernel:send_repl`, and verify each is echoed into the term buffer.
-- This covers the plugin's day-to-day pty path. The companion file
-- test_pipe_stdin.lua covers the raw-jobstart-over-pipes path where
-- the original bug actually lived.

local new_set = MiniTest.new_set

local KERNEL_JSON = os.getenv("JET_KERNEL_JSON")
assert(KERNEL_JSON, "JET_KERNEL_JSON env var must be set to a kernel.json path")

local child = MiniTest.new_child_neovim()

-- `_G.kernel.term` is populated asynchronously by `open_term`, so guard
-- against it being nil while polling.
local TERM_TEXT = [[
	_G.kernel and _G.kernel.term
		and table.concat(vim.api.nvim_buf_get_lines(_G.kernel.term.buf, 0, -1, false), "\n")
		or ""
]]

local T = new_set({
	hooks = {
		pre_case = function()
			child.restart({ "-u", "scripts/minimal_init.lua" })
			child.lua([[require("jet").setup({})]])
		end,
		post_once = child.stop,
	},
})

T["send_repl delivers every line through the plugin"] = function()
	-- Kernel + open_term's callback live as closures in the child; can't
	-- cross the RPC boundary, so the lifecycle is set up in one block.
	-- Trailing "" in send_repl is required by the chansend contract.
	child.lua(
		[[
			local Kernel = require("jet.core.kernel")
			_G.kernel = Kernel.init_owned({ spec_path = ..., session_name = "minitest" })
			_G.kernel:open_term(function()
				_G.kernel:send_repl({ "print('foo')", "print('bar')", "" })
			end)
		]],
		{ KERNEL_JSON }
	)

	-- Single execute: foo and bar must appear together, with no `>`
	-- (a fresh prompt — i.e. a new execute) between them.
	local pattern = "foo[^>]*bar"
	local ok = vim.wait(15000, function()
		return child.lua_get(TERM_TEXT):find(pattern) ~= nil
	end, 100)

	if not ok then
		error("marker never appeared in the REPL buffer.\nexpected " .. pattern .. "\ngot:\n" .. child.lua_get(TERM_TEXT))
	end
end

return T
