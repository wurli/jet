-- Bootstrap for headless mini.test runs. Used both as `-u` for the test
-- runner itself and as `-u` for the child neovim each test spawns (see
-- MiniTest.new_child_neovim). The `cwd` is the repo root in both cases.

vim.cmd([[let &rtp.=','.getcwd()]])

-- mini.test setup runs only in the headless runner (no UIs attached).
-- Child instances get a UI and skip this block — they just need the
-- plugin on rtp.
if #vim.api.nvim_list_uis() == 0 then
	vim.opt.runtimepath:prepend("deps/mini.nvim")
	require("mini.test").setup({
		collect = {
			find_files = function()
				return vim.fn.globpath("tests", "test_*.lua", true, true)
			end,
		},
		execute = {
			reporter = require("mini.test").gen_reporter.stdout({ quit_on_finish = true }),
		},
	})
end
