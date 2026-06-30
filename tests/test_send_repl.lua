-- End-to-end test driving the jet neovim plugin: start a kernel via
-- `Kernel.init_owned`, open its REPL term, send three lines with
-- `Kernel:send_repl`, and verify each is echoed into the term buffer.
-- This covers the plugin's day-to-day pty path. The companion file
-- test_pipe_stdin.lua covers the raw-jobstart-over-pipes path where
-- the original bug actually lived.

local MiniTest = require("mini.test")

local T = MiniTest.new_set()

local function repo_root()
	return vim.fn.fnamemodify(vim.fn.resolve(debug.getinfo(1).source:sub(2)), ":p:h:h")
end

local KERNEL_JSON = os.getenv("JET_KERNEL_JSON")
assert(KERNEL_JSON, "JET_KERNEL_JSON env var must be set to a kernel.json path")

local function wait_for(pred, timeout_ms)
	return vim.wait(timeout_ms, pred, 50)
end

T["send_repl delivers every line through the plugin"] = function()
	local jet_bin = repo_root() .. "/target/release/jet"
	if vim.fn.executable(jet_bin) ~= 1 then
		MiniTest.skip("jet binary missing: " .. jet_bin)
	end

	local xdg = vim.fn.tempname()
	vim.fn.mkdir(xdg, "p")
	vim.env.XDG_DATA_HOME = xdg

	require("jet").setup({ jet_binary = jet_bin })

	local Kernel = require("jet.core.kernel")
	local kernel = Kernel.init_owned({ spec_path = KERNEL_JSON, session_name = "minitest" })

	local opened = false
	kernel:open_term(function()
		opened = true
	end)
	MiniTest.expect.equality(
		wait_for(function()
			return opened
		end, 15000),
		true
	)

	-- Give jet a moment to draw its banner before the first send.
	vim.wait(2000)

	-- Per the chansend contract, a trailing "" forces a real newline.
	-- Small inter-send gaps let each echo land before the next; the bug
	-- only required multi-line sends, not specifically back-to-back ones.
	kernel:send_repl({ "print('foo')", "print('bar')", "" })

	local function term_text()
		local lines = vim.api.nvim_buf_get_lines(kernel.term.buf, 0, -1, false)
		return table.concat(lines, "\n")
	end

	-- Single execute: foo and bar must appear together, with no `>`
	-- (a fresh prompt — i.e. a new execute) between them.
	local pattern = "foo[^>]*bar"
	local ok = wait_for(function()
		return term_text():find(pattern) ~= nil
	end, 15000)

	-- Clean up before assertion so we don't leave a kernel alive on failure.
	pcall(function()
		require("jet.core.engine").stop(kernel.session_id)
	end)
	pcall(vim.fn.jobstop, kernel.term.job_id)
	vim.fn.delete(xdg, "rf")

	if not ok then
		error("marker never appeared in the REPL buffer.\n" .. 'expected "' .. pattern .. "\ngot:\n" .. term_text())
	end
end

return T
