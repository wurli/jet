Jet = require("jet.core.engine")

-- local k = jet.attach("con.json")
-- K = Jet.start("/Users/JACOB.SCOTT1/Repos/jet/kernels/ark/kernel.json", "con.json")
K = Jet.start("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json", "con.json")

vim.print({ session = K })

Cb = Jet.execute_code(K, "import pandas as pd\npd.DataFrame({'x': [1, 2, 3]})", {})
-- Cb = Jet.execute_code(K, "123", {})

Jet.execute_code(K, "library(tidyverse)", {})

-- local drain = function(cb)
-- 	while true do
-- 		local res = cb()
-- 		if not res then
-- 			break
-- 		end
-- 		if res.data and res.data.text then
-- 			print(res.data.text)
-- 		end
-- 		os.execute("sleep 1")
-- 	end
-- end
--
-- drain(Cb)
