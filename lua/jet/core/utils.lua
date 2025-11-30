local M = {}

---@class Jet.Utils.Listen.Options
---
---Polling interval in milliseconds. Default is 50.
---@field interval number
---
---@field action fun(result): "exit" Terminate
---| "handle" Pass the result to `handler()`
---| "retry" Continue polling after `interval` milliseconds
---@field handler? fun(result: any): any
---@field on_exit? fun(): any

---Repeatedly run a callback until a particular result is returned
---
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

---Gets the filetype, first at the position, then for the buffer if that fails.
---
---@param bufnr? number
---@param pos? number[]
---@return string|nil
M.get_cur_filetype = function(bufnr, pos)
	bufnr = bufnr or vim.api.nvim_get_current_buf()
	pos = pos or { vim.fn.line("."), vim.fn.col(".") }
	local buf_ft = vim.bo[bufnr].filetype
	local ft = buf_ft == "" and nil or buf_ft

	if not pos then
		return ft
	end

	local parser = vim.treesitter.get_parser(bufnr, nil, { error = false })
	if not parser then
		return ft
	end

	return parser
		:language_for_range({
			pos[1] - 1,
			pos[2] - 1,
			pos[1] - 1,
			pos[2],
		})
		:lang()
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

---@param msg Jet.Callback.Execute.Result
---@return string | nil
M.msg_to_string = function(msg)
	if not msg.data then
		return
	end

	local out

	if msg.type == "execute_input" then
		-- TODO: restore logic to get the prompt
		-- local code = self:_prompt_get_input() .. msg.data.code:gsub("\n", "\n" .. self:_prompt_get_continue())
		out = M.add_linebreak(msg.data.code)
	elseif msg.type == "execute_result" then
		out = M.add_linebreak(msg.data.data["text/plain"])
	elseif msg.type == "stream" then
		out = msg.data.text
	elseif msg.type == "error" then
		local err = msg.data.evalue
		local trace = msg.data.traceback
		out = M.add_linebreak(err)
		-- Sometimes the traceback is just the error itself, a feature of
		-- Jupyter that truly makes me sick
		if #trace > 0 and not (#trace == 1 and trace[1] == err) then
			out = out .. M.add_linebreak(table.concat(trace, "\n"))
		end
	elseif msg.type == "input_request" then
		out = msg.data.prompt
	elseif msg.type == "display_data" then
		-- TODO
		out = vim.inspect(msg.data)
	else
		M.log_warn("Dropping unexpected message type: '%s'", msg.type)
	end

	return out
end

---@param path string
---@param buf integer
---@param opts? { row?: integer, col?: integer, height?: integer, width?: integer }
M.image_to_buf = function(path, buf, opts)
	opts = opts or {}
	Snacks.image.buf.attach(buf, {
		src = path,
		inline = true,
		type = "image",
		pos = { opts.row or 1, opts.col or 0 },
		width = opts.width,
		height = opts.height,
	})
end

---@param msg Jet.Msg.DisplayData
---@param kernel Jet.Kernel
---@return string? # The path to the created file
M.display_data_to_file = function(msg, kernel)
	if vim.tbl_count(msg.data.data) > 1 then
		M.log_warning("Display data has more than one entry: %s", vim.inspect(vim.tbl_keys(msg.data.data)))
	end
	for mimetype, data in pairs(msg.data.data) do
		return M.base64_to_file(mimetype, data, {
			filetype = kernel and kernel.filetype,
			kernel_name = kernel and kernel.instance.spec.display_name,
		})
	end
end

---@param x string
---@param multiple integer
---@param pad string Should be a single character
---@return string
local pad_to_multiple = function(x, multiple, pad)
	local n_pad = (multiple - (#x % multiple)) % multiple
	return x .. pad:rep(n_pad)
end

---@param mimetype string E.g. `"image/png"`
---@param data string Base64-encoded data
---@param opts? { kernel_name: string?, filetype: string? }
---@return string?
M.base64_to_file = function(mimetype, data, opts)
	local bytes = vim.base64.decode(pad_to_multiple(data, 4, "="))

	local dir = require("jet.core.config").image.dir
	pcall(vim.fn.mkdir, dir, "p")

	---@param s string?
	local sanitise = function(s)
		return (s or ""):lower():gsub("%W", "-"):gsub("^-*", ""):gsub("-*$", ""):gsub("-+", "-")
	end

	opts = opts or {}
	local file_name = table.concat({
		os.date("%Y-%m-%d-%H:%M:%S"),
		sanitise(opts.filetype or vim.bo.filetype),
		sanitise(opts.kernel_name or "jet"),
	}, "__")

	local extension = mimetype:gsub("^[^/]*/", "")
	local file_path = vim.fs.joinpath(dir, file_name .. "." .. extension)
	-- local file_path = file_name .. "." .. extension

	local file, error = io.open(file_path, "wb")
	if not file then
		M.log_error("Failed to write file `%s`: %s", file_path, error)
		return
	end

	file:write(bytes)
	file:close()

	return file_path
end

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

return M
