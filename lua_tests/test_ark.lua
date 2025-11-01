local utils = require("./lua_tests/utils")
local jet = utils.jet_loader()

local kernel_id, info = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- Print the startup message
utils.cat_header("startup message", "=")
print(info.banner)

-- Also can test image display, but the output is big
-- utils.execute(jet, kernel_id, "hist(islands)")

-- Try running some code
utils.execute(jet, kernel_id, "1 + 1")
utils.execute(jet, kernel_id, "readline('Enter something: ')")
utils.execute(jet, kernel_id, "Sys.sleep(1); 1 + 1")
utils.execute(jet, kernel_id, "cat('hi')")

-- Try user expressions
utils.execute(jet, kernel_id, "1 + 1", { test = "2^2" })
utils.execute(jet, kernel_id, "x <- 2 + 2")
utils.execute(jet, kernel_id, "cat('Result:', x)")

-- Try testing completeness
utils.is_complete(jet, kernel_id, "1 +")
utils.is_complete(jet, kernel_id, "1 + 1")
utils.is_complete(jet, kernel_id, "_")

-- Try getting completions (ark doesn't do these)
utils.get_completions(jet, kernel_id, "iris$", 4)

-- Try shutting down
-- utils.request_restart(jet, kernel_id)
-- Causes issues currently
utils.execute(jet, kernel_id, "x")
utils.request_shutdown(jet, kernel_id)
