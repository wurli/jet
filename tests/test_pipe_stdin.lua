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

T["chansend over plain pipe stdin delivers every line"] = function()
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

	local marker = "JETPIPEOK-" .. tostring(os.time()) .. "-" .. tostring(math.random(1e9))
	local output = {}
	local job_id = vim.fn.jobstart({ jet_bin, "start", kernel_json }, {
		env = { XDG_DATA_HOME = xdg },
		on_stdout = function(_, data, _)
			for _, l in ipairs(data) do
				table.insert(output, l)
			end
		end,
		on_stderr = function(_, data, _)
			for _, l in ipairs(data) do
				table.insert(output, l)
			end
		end,
	})
	MiniTest.expect.no_equality(job_id, 0)
	MiniTest.expect.no_equality(job_id, -1)

	vim.wait(3000)

	-- Three separate chansends, mirroring the bug report's invocation.
	vim.fn.chansend(job_id, { "x = 1", "" })
	vim.fn.chansend(job_id, { "x = x + 1", "" })
	vim.fn.chansend(job_id, { 'print("' .. marker .. ':" + str(x))', "" })

	local ok = vim.wait(15000, function()
		for _, line in ipairs(output) do
			if line:find(marker .. ":2", 1, true) then
				return true
			end
		end
		return false
	end, 100)

	vim.fn.jobstop(job_id)
	vim.fn.delete(xdg, "rf")

	if not ok then
		error(
			"jet swallowed chansend lines over pipe stdin.\n"
				.. 'expected to see "'
				.. marker
				.. ':2" in stdout.\n'
				.. "got:\n  "
				.. table.concat(output, "\n  ")
		)
	end
end

return T
