local M = {}
local api = require("jet.core.api")
local utils = require("jet.core.utils")

-- jet ui mockup
--
-- +--------------------------------------------------------------+
-- |                           Jet                                |
-- |                                                              |
-- | (<Enter>) Open (auto)  (n) New session  (x) Shut down        |
-- |                                                              |
-- | Ark R Kernel                                   (kernelspec)  |
-- |   session 1 (nvim)                            (session_id)  |
-- |   session 2 (nvim)                            (session_id)  |
-- | 󰺕  session 3 (external)                        (session_id)  |
-- |                                                              |
-- | Ipython                                        (kernelspec)  |
-- |   session 1 (nvim)                            (session_id)  |
-- |                                                              |
-- | Rust                                           (kernelspec)  |
-- | <no running sesssion>                                        |
-- |                                                              |
-- +--------------------------------------------------------------+

local ns = vim.api.nvim_create_namespace("jet.ui")

---@generic T
---@class jet.ui.line<T>
---@field data T
---@field parts { [1]: string, [2]: string? }[]
---@field refresh fun(self)
local line = {}
line.__index = line

function line.refresh()
	-- Default implementation does nothing
end

---@generic T
---@param data T
---@return jet.ui.line<T>
local function new_line(data)
	return setmetatable({ data = data }, line)
end

---@param data jet.kernel
local active_kernel_line = function(data)
	assert(data.session_id, "Kernel must have a session_id")
	local out = new_line(data)

	function out:refresh()
		local status, status_icon = self.data:status()
		assert(self.data.session_info, "Kernel must have session info")
		self.parts = {
			{ status_icon .. "  ", status == "external" and "@variable.builtin" or "@string.regexp" },
			{ "(" .. utils.time_since(self.data.session_info.created_at) .. ") ", "Comment" },
			{ self.data.session_id .. " " },
		}
	end

	return out
end

---@param data jet.ui.kernel_group
local kernel_info_line = function(data)
	local out = new_line(data)

	function out:refresh()
		self.parts = { { self.data.kernel.spec.display_name } }

		local n_running = #self.data.connected + #self.data.external
		if n_running > 0 then
			table.insert(self.parts, { "  " })
			table.insert(self.parts, {
				string.format("(%s running instance%s)", n_running, n_running > 1 and "s" or ""),
				"Comment",
			})
		end

		table.insert(self.parts, { "    " })
		table.insert(self.parts, { utils.path_shorten(self.data.kernel.spec_path), "Directory" })
	end

	return out
end

local header_line = function()
	local out = new_line(nil)
	out.parts = { { "Jet ", "Title" }, { " ", "OkMsg" } }
	return out
end

local url_line = function()
	local out = new_line(nil)
	out.parts = { { "https://github.com/wurli/jet", "Comment" } }
	return out
end

local keymaps_line = function()
	local out = new_line(nil)
	out.parts = {
		{ "[<Enter>] Open (auto)", "OkMsg" },
		{ "  " },
		{ "[n] New session", "OkMsg" },
		{ "  " },
		{ "[x] Shut down", "OkMsg" },
		{ "  " },
		{ "[q] Quit", "OkMsg" },
	}
	return out
end

local blank_line = function()
	local out = new_line(nil)
	out.parts = { { "" } }
	return out
end

---@class jet.ui.kernel_group
---@field kernel jet.kernel
---@field external jet.kernel[]
---@field connected jet.kernel[]

---@return jet.ui.kernel_group[]
local list_kernel_groups = function()
	local kernel_list = api.list_kernels({}, {})

	---@type table<string, { kernel: jet.kernel, external: jet.kernel[], connected: jet.kernel[] }>
	local kernels_grouped = {}
	for _, k in ipairs(kernel_list) do
		local path = utils.path_normalise(k.spec_path)
		if not kernels_grouped[path] then
			kernels_grouped[path] = {
				kernel = k, -- Doesn't matter which kernel this one is, since we only use its basic info (name, path, etc.)
				external = {},
				connected = {},
			}
		end
	end

	for _, k in ipairs(api.filter_kernels(kernel_list, { status = { "connected", "connecting" } })) do
		table.insert(kernels_grouped[utils.path_normalise(k.spec_path)].connected, k)
	end

	for _, k in ipairs(api.filter_kernels(kernel_list, { status = { "external" } })) do
		table.insert(kernels_grouped[utils.path_normalise(k.spec_path)].external, k)
	end

	-- Makes sorting possible
	local out = vim.tbl_values(kernels_grouped)

	table.sort(out, function(a, b)
		---@type table<jet.kernel.status, integer>
		local status_ranks = {
			connected = 1,
			connecting = 1,
			external = 2,
			inactive = 3,
		}

		local a_min_status = status_ranks[(a.connected[1] or a.external[1] or a.kernel):status()]
		local b_min_status = status_ranks[(b.connected[1] or b.external[1] or b.kernel):status()]

		if a_min_status ~= b_min_status then
			return a_min_status < b_min_status
		end

		return a.kernel.spec.display_name < b.kernel.spec.display_name
	end)

	for _, running in pairs(out) do
		table.sort(running.connected, function(a, b)
			return a.session_id < b.session_id
		end)
		table.sort(running.external, function(a, b)
			return a.session_id < b.session_id
		end)
	end

	return out
end

M.show = function()
	local groups = list_kernel_groups()

	----------------------------------------------
	--               Write lines                --
	----------------------------------------------
	local lines = {
		header_line(),
		url_line(),
		blank_line(),
		keymaps_line(),
		blank_line(),
	}

	for _, group in ipairs(groups) do
		table.insert(lines, kernel_info_line(group))
		local any_connected = false
		for _, k in ipairs(group.connected) do
			table.insert(lines, active_kernel_line(k))
			any_connected = true
		end
		for _, k in ipairs(group.external) do
			table.insert(lines, active_kernel_line(k))
			any_connected = true
		end
		if any_connected then
			table.insert(lines, blank_line())
		end
	end

	local buf = vim.api.nvim_create_buf(false, true)

	---@type { lnum: integer, start_col: integer, end_col: integer, hl: string }[]
	local extmarks = {}
	---@type string[]
	local text = {}
	for lnum, l in ipairs(lines) do
		l:refresh()
		local line_text = "    "
		for _, part in ipairs(l.parts) do
			line_text = line_text .. part[1]
			if part[2] then
				table.insert(extmarks, {
					lnum = lnum,
					end_col = #line_text,
					start_col = #line_text - #part[1],
					hl = part[2],
				})
			end
		end
		table.insert(text, line_text)
	end

	vim.api.nvim_buf_set_lines(buf, 0, -1, false, text)
	for _, mark in ipairs(extmarks) do
		vim.api.nvim_buf_set_extmark(buf, ns, mark.lnum - 1, mark.start_col, {
			end_col = mark.end_col,
			hl_group = mark.hl,
		})
	end

	----------------------------------------------
	--                  Keymaps                 --
	----------------------------------------------
	vim.keymap.set("n", "q", function()
		vim.api.nvim_win_close(0, true)
	end)

	----------------------------------------------
	--               Display Buffer             --
	----------------------------------------------
	local screen_width = vim.o.columns
	local screen_height = vim.o.lines
	local scale = function(x, y)
		return math.floor(x * y)
	end
	vim.api.nvim_open_win(buf, true, {
		relative = "editor",
		width = scale(screen_width, 0.8),
		height = scale(screen_height, 0.8),
		row = scale(screen_height, 0.2 / 2),
		col = scale(screen_width, 0.2 / 2),
		style = "minimal",
		border = "rounded",
	})
end

return M
