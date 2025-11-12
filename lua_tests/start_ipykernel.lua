local utils = require("./lua_tests/utils")
local jet = utils.jet_loader()

local kernel_id, info = jet.start_kernel("/Users/JACOB.SCOTT1/Repos/jet/.venv/share/jupyter/kernels/python3/kernel.json")

print("Kernel started with ID:", kernel_id)
utils.print(info)
