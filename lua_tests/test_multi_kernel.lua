local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

-- Discover available kernels
utils.cat_header("Available kernels", "=")
print()
local kernels = carpo.discover_kernels()
for _, spec in pairs(kernels) do
    print(string.format("- %s", spec.display_name))
end
print()

-- Start multiple kernels
utils.cat_header("Starting kernels", "=")

local ipykernel_id = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
local ark_id = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- List all running kernels
utils.cat_header("Running kernels", "=")
local running = carpo.list_kernels()
print(utils.dump(running))
for i, kernel_id in ipairs(running) do
    print(string.format("%d. %s", i, kernel_id))
end

-- Execute code in Python kernel
utils.cat_header("Executing code in Python kernel", "=")
utils.execute(carpo, ipykernel_id, "x = 1 + 1")
utils.execute(carpo, ipykernel_id, "print(f'Result: {x}')")

-- Execute code in R kernel
utils.cat_header("Executing code in R kernel", "=")
utils.execute(carpo, ark_id, "x <- 2 + 2")
utils.execute(carpo, ark_id, "cat('Result:', x, '\\n')")

