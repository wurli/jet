local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

local kernel_id = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/rust/kernel.json")

-- Print the startup message
-- utils.cat_header("startup message", "=")
-- print(startup_message)

-- Try running some code
utils.execute(carpo, kernel_id, "1 + 1")
-- utils.execute(carpo, kernel_id, "readline('Enter something: ')")
-- utils.execute(carpo, kernel_id, "Sys.sleep(1); 1 + 1")

-- Try user expressions
utils.execute(carpo, kernel_id, "1 + 1", { test = "2^2" })

-- Try testing completeness
utils.is_complete(carpo, kernel_id, "1 +")
-- utils.is_complete(carpo, kernel_id, "1 + 1")
-- utils.is_complete(carpo, kernel_id, "_")

-- Try getting completions (ark doesn't do these)
utils.get_completions(carpo, kernel_id, [[use std::back]], 13)
