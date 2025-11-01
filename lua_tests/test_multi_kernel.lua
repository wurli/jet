local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

-- Discover available kernels
utils.cat_header("Available kernels", "=")
print()
for _, spec in pairs(carpo.discover_kernels()) do
    print("- " .. spec.display_name)
end
print()

-- Start multiple kernels
utils.cat_header("Starting kernels", "=")

local ipy_id, ipy_info = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
local ark_id, ark_info = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- List all running kernels
utils.list_running_kernels(carpo)

-- Execute code in kernels, interleaving results
utils.execute(carpo, ipy_id, "x = 1 + 1", ipy_info.display_name)
utils.execute(carpo, ark_id, "x <- 2 + 2", ark_info.display_name)
utils.execute(carpo, ipy_id, "print(f'Result: {x}')", ipy_info.display_name)
utils.execute(carpo, ark_id, "cat('Result:', x, '\\n')", ark_info.display_name)

utils.request_shutdown(carpo, ipy_id)

-- List all running kernels
utils.list_running_kernels(carpo)

-- utils.execute(carpo, ipy_id, "1 + 1", ipy_info.display_name)
utils.execute(carpo, ark_id, "2 + 2", ark_info.display_name)

utils.request_shutdown(carpo, ark_id)
