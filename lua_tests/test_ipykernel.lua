local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

local startup_message = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

-- Print the startup message
utils.cat_header("startup message", "=")
print(startup_message)

-- Try running some code
utils.execute(carpo, "%config Completer.use_jedi=True")

utils.execute(carpo, "1 + 1")
utils.execute(carpo, "input('Enter something')")
utils.execute(carpo, [[
import time
time.sleep(1)
1 + 1
]])

-- -- Try testing completeness
-- utils.is_complete(carpo, "1 +")
-- utils.is_complete(carpo, "1 + 1")
-- utils.is_complete(carpo, "$")

-- Try getting completions (ark doesn't do these)
utils.get_completions(
    carpo,
[[import pandas as pd
df = pd.DataFrame(dict(foo = [1, 2, 3], bar = ["a", "b", "c"]))
df.f]],
    88
)


