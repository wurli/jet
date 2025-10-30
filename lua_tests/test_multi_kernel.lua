local utils = require("./lua_tests/utils")
local carpo = utils.carpo_loader()

-- Discover available kernels
utils.cat_header("Available kernels", "=")
local kernels = carpo.discover_kernels()
for path, spec in pairs(kernels) do
    print(string.format("%s: %s", spec.display_name, path))
end

-- Start multiple kernels
utils.cat_header("Starting multiple kernels", "=")

local python_kernel_id = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
print("Started Python kernel: " .. python_kernel_id)

local r_kernel_id = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")
print("Started R kernel: " .. r_kernel_id)

-- List all running kernels
utils.cat_header("Running kernels", "=")
local running = carpo.list_kernels()
for i, kernel_id in ipairs(running) do
    print(string.format("%d. %s", i, kernel_id))
end

-- Execute code in Python kernel
utils.cat_header("Executing code in Python kernel", "=")
utils.execute(carpo, python_kernel_id, "x = 1 + 1")
utils.execute(carpo, python_kernel_id, "print(f'Python result: {x}')")

-- Execute code in R kernel
utils.cat_header("Executing code in R kernel", "=")
utils.execute(carpo, r_kernel_id, "x <- 2 + 2")
utils.execute(carpo, r_kernel_id, "cat('R result:', x, '\\n')")

-- Show that kernels maintain separate state
utils.cat_header("Kernels maintain separate state", "=")
utils.execute(carpo, python_kernel_id, "print(f'Python x = {x}')")
utils.execute(carpo, r_kernel_id, "cat('R x =', x, '\\n')")

print("\nMulti-kernel test completed successfully!")
