local M = {}
local utils = require("jet.core.utils")

---@param args string[]
---@param callback fun(res?: table, stdout: string)
M.curl = function(args, callback)
	vim.system({ "curl", unpack(args) }, {
		text = true,
	}, function(res)
		if res.code ~= 0 then
			error("curl call failed with code " .. res.code .. ":\n  " .. res.stderr)
		end

		local ok, json = pcall(vim.json.decode, res.stdout, { luanil = { object = true, array = true } })

		callback(ok and json or nil, res.stdout)
	end)
end

---@param dir? string
---@return { dir: string, bin_path?: string, lib_path?: string }
M.jet_resource_paths = function(dir)
	dir = dir or (vim.fn.stdpath("data") .. "/jet")
	local bin_dir = dir .. "/bin/"
	local lib_dir = dir .. "/lib/"

	if not vim.uv.fs_stat(dir) then
		mkdir(dir)
	end
	if not vim.uv.fs_stat(bin_dir) then
		mkdir(bin_dir)
	end
	if not vim.uv.fs_stat(lib_dir) then
		mkdir(lib_dir)
	end

	local jet_bin_path
	for file, type in vim.fs.dir(bin_dir) do
		if type == "file" and file:match("jet") then
			if jet_bin_path then
				error("Multiple files found in jet bin directory: " .. bin_dir)
			else
				jet_bin_path = bin_dir .. file
			end
		end
	end

	local jet_lib_path
	for file, type in vim.fs.dir(lib_dir) do
		if type == "file" and file:match("jet") then
			if jet_lib_path then
				error("Multiple files found in jet lib directory: " .. lib_dir)
			else
				jet_lib_path = lib_dir .. file
			end
		end
	end

	return {
		-- Always present
		dir = dir,
		bin_dir = bin_dir,
		lib_dir = lib_dir,
		-- If these are nil then the files haven't been downloaded yet.
		bin_path = jet_bin_path,
		lib_path = jet_lib_path,
	}
end

M.get_system_string = function()
	local u = vim.uv.os_uname()

	local arch = ({
		x86_64 = "x86_64",
		amd64 = "x86_64",
		aarch64 = "aarch64",
		arm64 = "aarch64",
	})[u.machine]

	local os_part = ({
		Darwin = "apple-darwin",
		Linux = "unknown-linux-gnu",
		-- No windows support yet.
		-- Windows_NT = "pc-windows-msvc",
	})[u.sysname]

	assert(arch, "Unsupported architecture: " .. u.machine)
	assert(os_part, "Unsupported platform: " .. u.sysname)

	return arch .. "-" .. os_part
end

---@param version string either `"latest"` or a version like `"v0.0.1"`
---@param callback fun(urls: string[])
local get_jet_download_urls = function(version, callback)
	M.curl({ "-s", "https://api.github.com/repos/wurli/jet/releases" }, function(res)
		assert(res[1], "No releases found for wurli/jet")

		-- Determine the release to use ----------------
		local release
		if version == "latest" then
			table.sort(res, function(a, b)
				return utils.version_compare(a.name, b.name)
			end)
			release = res[1]
		else
			if not version:match("^v") then
				version = "v" .. version
			end

			for _, r in ipairs(res) do
				if r.name == version then
					release = r
					break
				end
			end

			if not release then
				error("Release with version " .. version .. " not found")
			end
		end

		-- Get the urls for the release ----------------
		local urls = vim.tbl_map(function(res)
			return res.browser_download_url
		end, release.assets)

		vim.schedule(function()
			callback(urls)
		end)
	end)
end

local mkdir = function(dir)
	-- tonumber("755", 8) = 493
	vim.uv.fs_mkdir(dir, 493)
end

---@return { bin_url: string, lib_url: string }
local get_jet_urls_for_system = function(urls)
	local system_string = M.get_system_string()

	local system_urls = vim.tbl_filter(function(url)
		return url:find(system_string, 0, true) and url:match("%.tar%.gz$")
	end, urls)

	local bin_urls = vim.tbl_filter(function(url)
		return url:match("jet%-")
	end, system_urls)

	local lib_urls = vim.tbl_filter(function(url)
		return url:match("jet_lua%-")
	end, system_urls)

	if #bin_urls == 0 then
		error("No download URLs found for binary")
	elseif #bin_urls > 1 then
		error("Multiple download URLs found for binary: " .. vim.inspect(bin_urls))
	end

	if #lib_urls == 0 then
		error("No download URLs found for lib ")
	elseif #lib_urls > 1 then
		error("Multiple download URLs found for lib: " .. vim.inspect(lib_urls))
	end

	return {
		bin_url = bin_urls[1],
		lib_url = lib_urls[1],
	}
end

local download_and_unpack = function(url, dest_dir, callback)
	local stat = vim.uv.fs_stat(dest_dir)
	if not (stat and stat.type == "directory") then
	end

	local download_path = vim.fn.tempname() .. ".tar.gz"

	M.curl({ url, "-sL", "-o", download_path }, function(res)
		vim.system({ "tar", "-xzf", download_path, "-C", dest_dir, "--strip-components=1" }, {
			text = true,
		}, function(res)
			if res.code ~= 0 then
				error("Failed to unpack tarball: " .. res.stderr)
			end
			if callback then
				callback()
			end
		end)
	end)
end

---@param version string either `"latest"` or a version like `"v0.0.1"`
---@param dir? string
---@param callback? fun()
local download_jet = function(version, dir, callback)
	local urls = get_jet_download_urls(version, function(urls)
		local system_urls = get_jet_urls_for_system(urls)
		local dests = M.jet_resource_paths(dir)

		-- This is how we parallelize in the Wild West, partner
		local done_downloads = 0

		local on_done = vim.schedule_wrap(function()
			done_downloads = done_downloads + 1
			if done_downloads == 2 and callback then
				utils.log_info("Downloaded jet to " .. dests.dir)
				callback()
			end
		end)

		download_and_unpack(system_urls.bin_url, dests.bin_dir, on_done)
		download_and_unpack(system_urls.lib_url, dests.lib_dir, on_done)
	end)
end

---@param callback? fun({ bin_path: string, lib_path: string })
---@param has_done_download boolean
function M.maybe_download_jet(callback, has_done_download)
	local config = require("jet.config").options
	local path_defaults = M.jet_resource_paths()

	---------------------------------------------------
	--    Check that Jet library and binary exist    --
	---------------------------------------------------
	local lib_path = config.jet_library_path or path_defaults.lib_path
	local bin_path = config.jet_binary_path or path_defaults.bin_path

	local has_jet_bin = bin_path
		and vim.uv.fs_stat(bin_path)
		and vim.uv.fs_stat(bin_path).type == "file"
		and vim.fn.executable(bin_path) == 1

	local has_jet_lib = lib_path and vim.uv.fs_stat(lib_path) and vim.uv.fs_stat(lib_path).type == "file"

	local needs_download = false

	---------------------------------------------------
	--        Maybe download library and binary      --
	---------------------------------------------------
	if not has_jet_bin then
		if config.jet_binary_path then
			error("Jet binary not found at custom path: " .. config.jet_binary_path)
		else
			needs_download = true
		end
	end

	if not has_jet_lib then
		if config.jet_library_path then
			error("Jet Lua library not found at custom path: " .. config.jet_library_path)
		else
			needs_download = true
		end
	end

	if needs_download then
		if has_done_download then
			utils.log_error("Jet binary and/or library not found after download attempt!")
			return
		end
		utils.input_key("Install jet?", { "y", "n" }, function(choice)
			if choice == "y" then
				-- Download and then re-call this function to make sure we all good
				download_jet("latest", path_defaults.dir, function()
					M.maybe_download_jet(callback, true)
				end)
				return
			else
				error("Jet is required for this plugin! Use `:Jet install` to download a release.")
			end
		end)
		return
	end

	-----------------------------------------------------
	--        Check that version reqts are met         --
	-----------------------------------------------------
	-- The last version of jet before the --version flag was added
	local default_version = "0.0.1"

	local jet_bin_version = vim.system({ bin_path, "--version" }, { text = true })
		:wait().stdout
		:match("$jet (%d+%.%d+%.%d+)$") or default_version

	assert(jet_bin_version, "Failed to get Jet binary version from: " .. bin_path)

	local lua_loader = package.loadlib(lib_path, "luaopen_jet")
	local jet = lua_loader()
	local jet_lib_version = jet.version and jet.version() or default_version

	assert(jet_lib_version, "Failed to get Jet library version from: " .. lib_path)

	local required = require("jet.config").data.jet_min_version

	local bin_outdated = utils.version_compare(jet_bin_version, required)
	local lib_outdated = utils.version_compare(jet_lib_version, required)

	if bin_outdated or lib_outdated then
		if has_done_download then
			utils.log_error("Jet binary and/or library is outdated after download!")
			return
		end
		local msg = string.format("Jet binary and/or library is less than required version %s. Update?", required)
		utils.input_key(msg, { "y", "n" }, function(choice)
			if choice == "y" then
				download_jet("latest", path_defaults.dir, function()
					M.maybe_download_jet(callback, true)
				end)
				return
			else
				error("Jet binary and/or library is outdated. Use `:Jet install` to download a release.")
			end
		end)
		return
	end

	callback({ bin_path = bin_path, lib_path = lib_path })
end

-- download_jet("latest", "/Users/JACOB.SCOTT1/Repos/jet/tmp/")

-- On load Jet:
-- * Check if the binary and library exist in the data path.
--   * If no:
--     * prompt the user to download them (y/n prompt)
--   * If yes:
--     * Check if the lib and binary are compatible with the nvim plugin
--       * If no:
--         * prompt the user to update them (hard)
--       * If yes:
--         * finish
return M
