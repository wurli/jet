local utils = require("./lua_tests/utils")
local jet = utils.load_jet()

local kernel_id, info = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

-- Print the startup message
utils.cat_header("startup message", "=")
print(info.banner)

utils.execute(jet, kernel_id, "1 + 1")
utils.execute(jet, kernel_id, "input('Enter something')")
utils.execute(
	jet,
	kernel_id,
	[[
import time
time.sleep(1)
1 + 1
]]
)

-- User expressions
utils.execute(jet, kernel_id, "1 + 1", { test = "2 ** 2" })

-- Try testing completeness
-- These seem brittle... Often this hangs.
utils.is_complete(jet, kernel_id, "for i in range(3):")
utils.is_complete(jet, kernel_id, "1 + 1")
utils.is_complete(jet, kernel_id, "$")

utils.execute(jet, kernel_id, [[my_inconveniently_named_var = 1]])

-- Try getting completions (ark doesn't do these)
utils.get_completions(jet, kernel_id, "my_inconv", 9)

-- Try getting completions (ark doesn't do these)
utils.request_shutdown(jet, kernel_id)
-- utils.request_restart(jet, kernel_id)
-- utils.execute(jet, kernel_id, "df")
