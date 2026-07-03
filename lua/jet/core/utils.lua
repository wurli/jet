local M = {}

---Repeatedly run a callback until a particular result is returned
---
---Opts:
---	- interval: number (default: 50) - polling interval in milliseconds
---	- handler: function(result) - called with the result of the callback, should return
---	    either "exit", "continue", or "wait" to control the polling behavior
---
---@param callback fun(): any
---@param handler fun(result): nil | "wait" | "continue" | "exit"
---@param opts? { interval?: integer }
M.poll = function(callback, handler, opts)
	opts = opts or {}
	local function run()
		while true do
			local result = callback()
			local action = handler(result) or "wait"

			if action == "exit" then
				return
			elseif action == "wait" then
				return vim.defer_fn(run, opts.interval or 50)
			elseif action ~= "continue" then
				-- If we've got a valid result, process it and then and then
				-- immediately (i.e. with no delay) poll again.
				error(("Unexpected action '%s'"):format(tostring(action)))
			end
		end
	end

	run()
end

-- vim.keymap.set("n", "<cr>", function()
-- 	vim.print(M.get_filetype(0, { vim.fn.line("."), vim.fn.col(".") }))
-- end, {})

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
	return vim.fn.simplify(vim.fn.fnamemodify(path, ":~:."))
end

---@return string[]
M.get_all_filetypes = function()
	return vim.fn.getcompletion("", "filetype")
end

M.log_debug = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.DEBUG, {})
end
M.log_error = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.ERROR, {})
end
M.log_info = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.INFO, {})
end
M.log_off = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.OFF, {})
end
M.log_trace = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.TRACE, {})
end
M.log_warn = function(msg, ...)
	vim.notify("[jet] " .. msg:format(...), vim.log.levels.WARN, {})
end

return M
