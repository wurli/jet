local M = {}

M.version = "4.14.1" -- x-release-please-version

---@class jet.config
M.defaults = {
	jet_binary = "jet",
	stop_on_buf_wipeout = true,
	stop_on_nvim_quit = true,
	auto_set_primary = true,
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
