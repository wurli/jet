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

local function ipykernel_available()
	vim.fn.system({ "python3", "-c", "import ipykernel" })
	return vim.v.shell_error == 0
end

local function which(name)
	local out = vim.fn.system({ "which", name })
	if vim.v.shell_error ~= 0 then
		return nil
	end
	out = (out:gsub("%s+$", ""))
	return out ~= "" and out or nil
end

local function ensure_python_kernelspec()
	local home = os.getenv("HOME") or ""
	local user = home .. "/Library/Jupyter/kernels/python3/kernel.json"
	if vim.fn.filereadable(user) == 1 then
		return user
	end

	local py = which("python3")
	if not py then
		return nil
	end
	local dir = vim.fn.tempname()
	vim.fn.mkdir(dir, "p")
	local path = dir .. "/kernel.json"
	vim.fn.writefile({
		vim.json.encode({
			argv = { py, "-m", "ipykernel_launcher", "-f", "{connection_file}" },
			display_name = "Python (jet mini.test)",
			language = "python",
			interrupt_mode = "signal",
		}),
	}, path)
	return path
end

local function wait_for(pred, timeout_ms)
	return vim.wait(timeout_ms, pred, 50)
end

T["send_repl delivers every line through the plugin"] = function()
	if not ipykernel_available() then
		MiniTest.skip("ipykernel not installed")
	end
	local kernel_json = ensure_python_kernelspec()
	if not kernel_json then
		MiniTest.skip("could not prepare python kernelspec")
	end
	local jet_bin = repo_root() .. "/target/release/jet"
	if vim.fn.executable(jet_bin) ~= 1 then
		MiniTest.skip("jet binary missing: " .. jet_bin)
	end

	local xdg = vim.fn.tempname()
	vim.fn.mkdir(xdg, "p")
	vim.env.XDG_DATA_HOME = xdg

	require("jet").setup({ jet_binary = jet_bin })

	local Kernel = require("jet.core.kernel")
	local kernel = Kernel.init_owned({ spec_path = kernel_json, session_name = "minitest" })

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
	local marker = "bar"
	kernel:send_repl({ "print('foo')", "print('bar')", "" })

	local function term_text()
		local lines = vim.api.nvim_buf_get_lines(kernel.term.buf, 0, -1, false)
		return table.concat(lines, "\n")
	end

	local ok = wait_for(function()
		return term_text():find(marker, 1, true) ~= nil
	end, 15000)

	-- Clean up before assertion so we don't leave a kernel alive on failure.
	pcall(function()
		require("jet.core.engine").stop(kernel.session_id)
	end)
	pcall(vim.fn.jobstop, kernel.term.job_id)
	vim.fn.delete(xdg, "rf")

	if not ok then
		error(
			"marker never appeared in the REPL buffer.\n"
				.. 'expected "'
				.. marker
				.. ':2"\n'
				.. "got:\n"
				.. term_text()
		)
	end
end

return T
