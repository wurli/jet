local M = {}

M.setup = function()
	local api = require("jet.core.api")

	vim.api.nvim_create_user_command("Jet", function(opts)
		local args = opts.fargs
		local open = require("jet.core.kernel").open_term

		if args[1] == "repl" then
			return api.get_any({}, {}, open)
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
	end, {
		desc = "Jet: work with Jupyter kernels",
		nargs = "*",
		---@diagnostic disable-next-line: unused-local
		complete = function(prefix, line, col)
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

			if args[2] == "start" then
				local specs = require("jet.core.engine").list_kernels()
				return vim.tbl_map(function(spec)
					return spec.path
				end, specs)
			end
		end,
	})
end

return M
