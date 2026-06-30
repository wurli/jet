-- Companion to test_send_repl.lua: with `send.send_by_expr = true`,
-- Kernel:send_repl bypasses the bracketed-paste path and instead sends
-- the code as a list of lines via chansend. Each expression is meant to
-- be evaluated one at a time. The bug: only the first line ever made it
-- into the REPL because the list lacked a trailing "" to force the final
-- newline (per the chansend contract).

local new_set = MiniTest.new_set

local KERNEL_JSON = os.getenv("JET_KERNEL_JSON")
assert(KERNEL_JSON, "JET_KERNEL_JSON env var must be set to a kernel.json path")

local child = MiniTest.new_child_neovim()

local T = new_set({
	hooks = {
		pre_case = function()
			child.restart({ "-u", "scripts/minimal_init.lua" })
			child.lua([[require("jet").setup()]])
		end,
		post_once = child.stop,
	},
})

local run_send = function()
	child.lua(
		[[
			local Kernel = require("jet.core.kernel")
			_G.kernel = Kernel.init_owned({ spec_path = ..., session_name = "minitest" })
			_G.kernel:open_term(function()
				vim.uv.sleep(500)
				_G.kernel:send_repl({ "print('first line')", "print('second line')" })
			end)
		]],
		{ KERNEL_JSON }
	)

	local TERM_TEXT = [[
		_G.kernel and _G.kernel.term
			and table.concat(vim.api.nvim_buf_get_lines(_G.kernel.term.buf, 0, -1, false), "\n")
			or ""
	]]

	-- In send_by_expr mode each line is evaluated separately, so both
	-- markers must appear. The previous bug stopped after the first.
	local repl_text = ""
	local ok = vim.wait(4000, function()
		repl_text = child.lua_get(TERM_TEXT)
		return repl_text:find("first line", 1, true) ~= nil and repl_text:find("second line", 1, true) ~= nil
	end, 100)

	if not ok then
		error(
			"send_by_expr dropped lines.\n"
				.. "Expected:\n"
				.. "    'first line' and 'second line' in REPL buffer\n"
				.. "Got:\n"
				.. "    "
				.. vim.trim(repl_text):gsub("\n", "\n    ")
		)
	end
end

T["send_repl with send_by_expr=false delivers every line"] = function()
	child.lua([[ require("jet.config").options.send.send_by_expr = false ]])
	run_send()
end

T["send_repl with send_by_expr=true delivers every line"] = function()
	child.lua([[ require("jet.config").options.send.send_by_expr = true ]])
	run_send()
end

return T
