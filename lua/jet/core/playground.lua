Jet = require("jet.core.engine")
Manager = require("jet.core.manager")

local start_ark_lsp = function(id)
	local _comm_id, callback = Jet.comm_open(id, "lsp", { ip_address = "127.0.0.1" })
	local function drain_callback()
		while true do
			local result = callback()
			if result.status == "idle" then
				return
			end
			if not result.data then
				return vim.defer_fn(drain_callback, 100)
			end
			local port = result.data.data.params.port
			vim.lsp.config.ark = {
				cmd = vim.lsp.rpc.connect("127.0.0.1", port),
				root_markers = { ".git", ".Rprofile", ".Rproj", "DESCRIPTION" },
				filetypes = { "r", "R" },
			}
			vim.lsp.enable("ark")
			return
		end
	end
	drain_callback()
end

local ark_id = Manager:list_kernels({ status = "active" })[1].id or ""
Manager.running[ark_id]:execute({ "options(cli.default_num_colors = 256L)" })
start_ark_lsp(ark_id)

