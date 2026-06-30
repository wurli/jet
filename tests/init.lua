-- Bootstrap for headless mini.test runs. Invoked by scripts/minitest.

local repo_root = vim.fn.fnamemodify(vim.fn.resolve(debug.getinfo(1).source:sub(2)), ":p:h:h")

vim.opt.runtimepath:prepend(repo_root .. "/deps/mini.nvim")
vim.opt.runtimepath:prepend(repo_root)

require("mini.test").setup({
	collect = {
		find_files = function()
			return vim.fn.globpath("tests", "test_*.lua", true, true)
		end,
	},
	execute = {
		-- stdout reporter + exit non-zero on failure so CI fails the job.
		reporter = require("mini.test").gen_reporter.stdout({ quit_on_finish = true }),
	},
})
