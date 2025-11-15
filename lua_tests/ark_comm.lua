local utils = require("./lua_tests/utils")
local jet = utils.load_jet()

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

local execute_cb = jet.execute_code(kernel_id, "help(lm)", {})

-- Continuously check for results until we fail to receive a result
while true do
	local result = callback()
	-- If idle then the execution is complete
	-- if result.status == "idle" then
	-- 	return
	-- end

	-- If no data yet, wait a bit (so we don't block the main thread)
	-- and check again later
	if result.data then
		utils.print(result)
		-- break
	end

	result = execute_cb()
	-- If idle then the execution is complete
	-- if result.status == "idle" then
	-- 	return
	-- end

	-- If no data yet, wait a bit (so we don't block the main thread)
	-- and check again later
	if result.data then
		utils.print(result)
		-- break
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
