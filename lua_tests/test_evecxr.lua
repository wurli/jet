local utils = require("./lua_tests/utils")
local jet = utils.load_jet()

local kernel_id, info = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/rust/kernel.json")

-- Print the startup message
utils.cat_header("startup message", "=")
print(info.banner)

-- Try running some code
utils.execute(jet, kernel_id, "1 + 1")
-- utils.execute(jet, kernel_id, "readline('Enter something: ')")
-- utils.execute(jet, kernel_id, "Sys.sleep(1); 1 + 1")

-- Try user expressions
utils.execute(jet, kernel_id, "1 + 1", info.display_name, { test = "2^2" })

-- Try testing completeness
utils.is_complete(jet, kernel_id, "1 +")
-- utils.is_complete(jet, kernel_id, "1 + 1")
-- utils.is_complete(jet, kernel_id, "_")

-- Try getting completions (ark doesn't do these)
utils.get_completions(jet, kernel_id, [[use std::back]], 13)

-- Try shutting down
utils.request_shutdown(jet, kernel_id)
