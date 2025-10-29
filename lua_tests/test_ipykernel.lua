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

-- User expressions
utils.execute(carpo, "1 + 1", { test = "2 ** 2" })

-- Try testing completeness
-- These seem brittle... Often this hangs.
utils.is_complete(carpo, "for i in range(3):")
utils.is_complete(carpo, "1 + 1")
utils.is_complete(carpo, "$")

utils.execute(
    carpo,
    [[import pandas as pd
df = pd.DataFrame(dict(foo = [1, 2, 3], bar = ["a", "b", "c"]))]]
)

-- Try getting completions (ark doesn't do these)
utils.get_completions(carpo, "df.f", 4)
