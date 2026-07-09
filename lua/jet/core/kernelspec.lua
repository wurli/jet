local M = {}

---@class jet.kernel.spec
---@field argv string[]
---@field display_name string
---@field language string
---@field interrupt_mode? "signal" | "message"
---@field env table<string, string>?
---@field metadata table<string, any>
---@field kernel_protocol_version string?

---@class jet.kernelspec.install.opts
---@field path? string
---@field name? string

M.make_path = function(name)
	return require("jet.config").data.jet_nvim_data_dir .. "/kernels/" .. name .. "/kernel.json"
end

---Install a kernelspec in Jet's Neovim data dir
---
---@param k jet.kernel.spec
---@param path string Path to a kernel.json file
M.install = function(k, path)
	local json = vim.json.encode(k, { indent = "    ", sort_keys = true })

	path = vim.fn.expand(path)

	local spec_already_exists, existing_json = pcall(vim.fn.readfile, path)
	if spec_already_exists and existing_json then
		if table.concat(existing_json, "\n") == json then
			-- require("jet.core.utils").log_info("Kernelspec is already installed at %s", path)
			return
		end
	end

	local dir = vim.fs.dirname(path)
	local stat = vim.uv.fs_stat(dir)

	if not (stat and stat.type == "directory") then
		vim.fn.mkdir(dir, "p")
	end

	local file, err = io.open(path, "w")
	if not file or err then
		error("Failed to open kernel.json for writing in " .. path .. ": " .. err)
	end

	file:write(json)

	assert(file:close(), "Failed to close file after writing " .. path)

	-- require("jet.core.utils").log_info(
	-- 	"%s new kernelspec %s",
	-- 	spec_already_exists and "Reinstalled" or "Installed",
	-- 	path
	-- )
end

return M
