local M = {}

local FILTER_ARGSPEC = {
	session_id = "text",
	spec_path = function()
		return vim.tbl_map(function(res)
			return res.path
		end, require("jet.core.engine").list_kernels())
	end,
	filetype = "text",
	display_name = "text",
	status = { "connecting", "connected", "inactive", "external" },
	primary = "flag",
	default = "flag",
}

local argspec_to_completions = function(spec)
	local completions = {}
	for k, v in pairs(spec) do
		if type(v) == "function" then
			v = v()
		end
		if type(v) == "table" or type(v) == "string" and v == "text" then
			table.insert(completions, k .. "=")
		elseif type(v) == "string" and v == "flag" then
			table.insert(completions, k)
		end
	end
	return completions
end

---@param args string[]
---@param argspec table
local parse_args = function(args, argspec, from)
	local check = {}
	for i = from or 1, #args do
		table.insert(check, args[i])
	end

	local invalid_msg = function(opt)
		local valid = table.concat(vim.tbl_keys(argspec), ", ")
		return string.format("Invalid option '%s', allowed options are: %s", opt, valid)
	end

	local parsed = {}
	for _, arg in ipairs(check) do
		if arg:find("=") then
			local kv = vim.split(arg, "=", { trimempty = true })
			assert(kv[1] and kv[2] and argspec[kv[1]], invalid_msg(kv[1]))

			local allowed = argspec[kv[1]]
			if type(allowed) == "function" then
				allowed = allowed()
			end

			if type(allowed) == "table" then
				assert(
					vim.tbl_contains(allowed, kv[2]),
					string.format(
						"Invalid value '%s' for option '%s', allowed values are: %s",
						kv[2],
						kv[1],
						table.concat(allowed, ", ")
					)
				)
				parsed[kv[1]] = kv[2]
			elseif type(allowed) == "string" and allowed == "text" then
				parsed[kv[1]] = kv[2]
			else
				error(string.format("Invalid argspec for option '%s'", kv[1]))
			end
		else
			if argspec[arg] and argspec[arg] == "flag" then
				parsed[arg] = true
			else
				error(invalid_msg(arg))
			end
		end
	end
	return parsed
end

M.setup = function()
	local ui = require("jet.core.ui")
	local api = require("jet.core.api")

	vim.api.nvim_create_user_command("Jet", function(opts)
		local args = opts.fargs
		local open = require("jet.core.kernel").open_term

		if #args == 0 then
			return ui.show()
		end

		if args[1] == "repl" then
			return api.get_any(parse_args(args, FILTER_ARGSPEC, 2), {}, open)
		end

		if args[1] == "open" then
			return api.get_external({}, {}, open)
		end

		if args[1] == "start" then
			return api.get_inactive({ spec_path = args[2] }, {}, open)
		end

		if args[1] == "attach" then
			return api.get_external({}, {}, open)
		end

		if args[1] == "send" and args[2] then
			---@param k jet.kernel
			return api.get_any({}, {}, function(k)
				k:send_lua(args[2], false)
			end)
		end
	end, {
		desc = "Jet: work with Jupyter kernels",
		nargs = "*",
		---@diagnostic disable-next-line: unused-local
		complete = function(_, line, _)
			local args = vim.split(line, " +", { trimempty = true })
			if args[1] ~= "Jet" then
				return {}
			end

			if #args == 1 then
				return {
					"repl",
					"open",
					"start",
					"attach",
				}
			end

			if #args == 2 then
				return argspec_to_completions(FILTER_ARGSPEC)
			end
		end,
	})
end

return M
