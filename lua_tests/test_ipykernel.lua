local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

local kernel_id, info = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

-- Print the startup message
utils.cat_header("startup message", "=")
print(info.banner)

-- Try running some code
utils.execute(carpo, kernel_id, "%config Completer.use_jedi=True")

utils.execute(carpo, kernel_id, "1 + 1")
utils.execute(carpo, kernel_id, "input('Enter something')")
utils.execute(carpo, kernel_id, [[
import time
time.sleep(1)
1 + 1
]])

-- User expressions
utils.execute(carpo, kernel_id, "1 + 1", info.display_name, { test = "2 ** 2" })

-- Try testing completeness
-- These seem brittle... Often this hangs.
utils.is_complete(carpo, kernel_id, "for i in range(3):")
utils.is_complete(carpo, kernel_id, "1 + 1")
utils.is_complete(carpo, kernel_id, "$")

utils.execute(
    carpo,
    kernel_id,
    [[import pandas as pd
df = pd.DataFrame(dict(my_inconveniently_named_col = [1, 2, 3], bar = ["a", "b", "c"]))]]
)

-- Try getting completions (ark doesn't do these)
utils.get_completions(carpo, kernel_id, "df.my_inconv", 12)

-- Try getting completions (ark doesn't do these)
utils.request_shutdown(carpo, kernel_id)
-- utils.request_restart(carpo, kernel_id)
-- utils.execute(carpo, kernel_id, "df")
