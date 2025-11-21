local jet = require("jet.core.rust")

local kernel_id, _ = jet.start_kernel("./kernels/ark/kernel.json")

-- local _, callback = jet.comm_open(kernel_id, "lsp", { ip_address = "127.0.0.1" })
-- -- Continuously check for results until we fail to receive a result
-- while true do
-- 	local result = callback()
-- 	-- If idle then the execution is complete
-- 	if result.status == "idle" then
-- 		return
-- 	end
-- 	-- If no data yet, wait a bit (so we don't block the main thread)
-- 	-- and check again later
-- 	if result.data then
-- 		utils.print(result)
-- 		break
-- 	end
-- end

-- See Positron source: positron/comms/help-frontend-openrpc.json
local comm_id, callback = jet.comm_open(kernel_id, "positron.help", {})
os.execute("sleep 2")
-- local callback2 = jet.comm_send(kernel_id, comm_id, {
-- 	show_help_topic = {
-- 		topic = "library",
-- 	},
-- })

local execute_cb = jet.execute_code(kernel_id, "?dplyr::mutate", {})

-- Continuously check for results until we fail to receive a result
for _ = 1, 20 do
	local result = callback()
	-- If idle then the execution is complete
	-- if result.status == "idle" then
	-- 	return
	-- end

	-- If no data yet, wait a bit (so we don't block the main thread)
	-- and check again later
	vim.print(result)
	if result.data then
		-- break
		local dt = result.data.data
		if dt.params and dt.params.content then
			vim.system(
				{
					"pandoc",
					"-f",
					"html",
					"-t",
					"markdown+pipe_tables+backtick_code_blocks",
					dt.params.content,
				},
				{},
				vim.schedule_wrap(function(res)
					if not res.stdout then
						return
					end
					local buf = vim.api.nvim_create_buf(false, true)
					local win = vim.api.nvim_open_win(buf, true, {
						style = "minimal",
						split = "right",
					})

					vim.wo[win].conceallevel = 3
					vim.bo[buf].filetype = "markdown"
					vim.api.nvim_buf_set_lines(buf, 0, -1, false, vim.split(res.stdout, "\n"))
				end)
			)
		end
	end
end

-- /Users/JACOB.SCOTT1/Repos/positron/src/positron-dts/positron.d.ts
-- 	export enum RuntimeClientType {
-- 		Variables = 'positron.variables',
-- 		Lsp = 'positron.lsp',
-- 		Plot = 'positron.plot',
-- 		DataExplorer = 'positron.dataExplorer',
-- 		Ui = 'positron.ui',
-- 		Help = 'positron.help',
-- 		Connection = 'positron.connection',
-- 		Reticulate = 'positron.reticulate',
-- 		IPyWidget = 'jupyter.widget',
-- 		IPyWidgetControl = 'jupyter.widget.control',
