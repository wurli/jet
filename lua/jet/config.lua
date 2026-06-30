local M = {}

M.version = "4.14.1" -- x-release-please-version

---@class jet.config
M.defaults = {
	jet_binary = "jet",
	stop_on_buf_wipeout = true,
	stop_on_nvim_quit = true,
	auto_set_primary = true,
	---key=filetype, value=kernelspec path
	---@type table<string, string>
	default_kernels = {},
	repl_win_opts = {}, ---@type vim.api.keyset.win_config
	hooks = {
		on_kernel_init = {}, ---@type fun(k: jet.kernel)[] end,
		on_lua_client_start = {}, ---@type fun(k: jet.kernel)[] end,
		on_kernel_close = {}, ---@type fun(k: jet.kernel)[] end,
	},
	send = {
		---If `false` (the default), then when sending several complete
		---expressions at once, all will be sent at once and results will be
		---shown afterwards. If `true` then each expression will be sent and
		---results shown one at a time.
		send_by_expr = false, ---@type boolean
	},
}

---@class jet.data
M.data = {
	jet_nvim_data_dir = vim.fn.stdpath("data") .. "/jet",
}

---@type jet.config
M.options = nil

---@param options? jet.config
function M.set(options)
	if options and options.jet_binary then
		options.jet_binary = vim.fn.expand(options.jet_binary)
	end

	M.options = vim.tbl_deep_extend("force", M.defaults, options or {})
end

return M
