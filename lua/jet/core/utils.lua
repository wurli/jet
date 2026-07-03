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

---@param opts { extension?: string, language?: string }
---@return string
M.resolve_filetype = function(opts)
	if opts.extension then
		if opts.extension:sub(1, 1) ~= "." then
			opts.extension = "." .. opts.extension
		end

		local ft, _ = vim.filetype.match({ filename = "file" .. opts.extension })

		if ft then
			return ft
		end
	end

	-- TODO: add a mapped list which users can add to.
	-- Turns out this is quite handy to have, e.g. if you have a ftplugin named
	-- 'R.lua' then 'R' will be returned by `vim.fn.getcompletion("", "filetype")`.
	-- Obvs you should probs just rename your ftplugin, but it's not an obvious fix.
	if opts.language then
		-- :'(
		local vim_filetypes = vim.fn.getcompletion("", "filetype")

		-- If vim has a built-in filetype which matches the language then we
		-- can be pretty sure that's the one.
		for _, ft in ipairs(vim_filetypes) do
			if ft:lower() == opts.language:lower() then
				return ft
			end
		end

		-- If vim has no matching built-in filetype then use the kernel
		-- language anyway.
		M.log_debug(
			"Could not resolve kernel filetype for extension `%s`; falling back to language `%s`",
			opts.extension,
			opts.language
		)

		return opts.language
	end

	error(("Could not resolve filetype based on extension `%s`"):format(opts.extension))
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
	local p = vim.fn.expand(path)
	path = type(p) == "string" and p or path
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
