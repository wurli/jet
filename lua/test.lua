Jet = require("jet.core.engine")

-- local k = jet.attach("con.json")
K = Jet.connect("/Users/JACOB.SCOTT1/Repos/jet/kernels/ark/kernel.json", "con.json")

Cb = Jet.execute_code(K, "print('hello'); Sys.sleep(5); print('jacob')", {})

local drain = function(cb)
	while true do
		local res = cb()
		if not res then
			break
		end
		if res.data and res.data.text then
			print(res.data.text)
		end
		os.execute("sleep 1")
	end
end

drain(Cb)
