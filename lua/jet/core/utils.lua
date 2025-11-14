local M = {}

---@class Jet.Utils.Listen.Options
---
---Polling interval in milliseconds. Default is 50.
---@field interval number
---
---@field action fun(result):
---| "exit" Terminate
---| "handle" Pass the result to `handler()`
---| "retry" Continue polling after `interval` milliseconds
---@field handler fun(result: any): any
---@field on_exit? fun(): any

---@param callback fun(): any
---@param opts Jet.Utils.Listen.Options
M.listen = function(callback, opts)
	local handler = opts.handler or function() end
	local on_exit = opts.on_exit or function() end

	local function loop()
		while true do
			local result = callback()
			local action = opts.action(result)

			if action == "exit" then
				on_exit()
				return
			elseif action == "retry" then
				return vim.defer_fn(loop, opts.interval or 50)
			elseif action == "handle" then
				-- If we've got a valid result, process it and then and then
				-- immediately (i.e. with no delay) poll again.
				handler(result)
			else
				error(("Invalid action '%s'"):format(tostring(action)))
			end
		end
	end

	loop()
end

--- Convert a file extension (with or without leading period) to a vim filetype.
---
--- Wraps `vim.filetype.match()`.
---
---@param ext string File extension (e.g. ".py" or "py")
---@return string|nil Filetype (e.g. "python") or `nil` if not found
M.ext_to_filetype = function(ext)
	if ext:sub(1, 1) == "." then
		ext = ext:sub(2)
	end
	local ft, _ = vim.filetype.match({ filename = "file." .. ext })
	return ft
end

---Get the time since some time as a nicely formatted string
---@param t number
---@return string
M.time_since = function(t)
	local seconds = math.floor(os.difftime(os.time(), t))

	if seconds < 60 then
		return string.format("%ds", seconds)
	end

	local minutes = math.floor(seconds / 60)
	if minutes < 60 then
		return string.format("%dm", minutes)
	end

	local hours = math.floor(minutes / 60)
	local remaining_minutes = minutes % 60

	if hours < 24 then
		if remaining_minutes == 0 then
			return string.format("%dh", hours)
		else
			return string.format("%dh%dm", hours, remaining_minutes)
		end
	end

	local days = math.floor(hours / 24)
	local remaining_hours = hours % 24

	if remaining_hours == 0 then
		return string.format("%dd", days)
	else
		return string.format("%dd%dh", days, remaining_hours)
	end
end

---Attempts to shorten a path by either using `~` for the home directory
---or `.` for the current working directory.
---
---@param path string
---@return string
M.path_shorten = function(path)
	path = vim.fn.expand(path)
	for _, x in ipairs({
		-- CWD should be preferred over HOME - hence why `pairs` not used
		{ abbv = ".", expansion = vim.fn.getcwd() },
		{ abbv = "~", expansion = vim.fn.expand("~") },
	}) do
		if path:sub(1, #x.expansion) == x.expansion then
			return x.abbv .. path:sub(#x.expansion + 1)
		end
	end
	return path
end

M.add_linebreak = function(x)
	return x .. (x:sub(-1) == "\n" and "" or "\n")
end

-- vim.fn.expand(string, nosuf, list)

M.log_debug = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.DEBUG, {})
end
M.log_error = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.ERROR, {})
end
M.log_info = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.INFO, {})
end
M.log_off = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.OFF, {})
end
M.log_trace = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.TRACE, {})
end
M.log_warn = function(msg, ...)
	vim.notify(msg:format(...), vim.log.levels.WARN, {})
end

-- local get_win_move_keymaps = function()
-- 	local patterns = {}
-- 	for _, key in ipairs({ "h", "j", "k", "l" }) do
-- 		table.insert(patterns, "<c%-w><c%-" .. key .. ">")
-- 		table.insert(patterns, "<c%-w>" .. key)
-- 	end
-- 	local move_maps = vim.tbl_filter(function(x)
-- 		if not x.rhs then
-- 			return false
-- 		end
-- 		for _, p in ipairs(patterns) do
-- 			if x.rhs:lower():match(p) then
-- 				return true
-- 			end
-- 		end
-- 		return false
-- 	end, vim.api.nvim_get_keymap("n"))
-- 	return vim.tbl_map(function(x)
-- 		return x.lhs
-- 	end, move_maps)
-- end
-- vim.print(get_win_move_keymaps())

return M
