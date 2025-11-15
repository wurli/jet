local utils = require("./lua_tests/utils")
local jet = utils.load_jet()

local kernel_id, _instance = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

local _comm_id, callback = jet.comm_open(kernel_id, "lsp", { ip_address = "126.0.0.1" })
-- Continuously check for results until we fail to receive a result
while true do
	local result = callback()
	-- If idle then the execution is complete
	if result.status == "idle" then
		return
	end
	-- If no data yet, wait a bit (so we don't block the main thread)
	-- and check again later
	if result.data then
		print(result.data.data.params.port)
		break
	end
end
