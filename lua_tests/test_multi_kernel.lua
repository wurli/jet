local utils = require("./lua_tests/utils")
local jet = utils.jet_loader()

-- Discover available kernels
utils.cat_header("Available kernels", "=")
print()
for path, spec in pairs(jet.list_available_kernels()) do
    print(("- %s (%s)"):format(spec.display_name, path))
end
print()

-- Start multiple kernels
utils.cat_header("Starting kernels", "=")

local ipy_id, ipy_info = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
local ark_id, ark_info = jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- List all running kernels
utils.list_running_kernels(jet)

-- Execute code in kernels, interleaving results
utils.execute(jet, ipy_id, "x = 1 + 1", ipy_info.display_name)
utils.execute(jet, ark_id, "x <- 2 + 2", ark_info.display_name)
utils.execute(jet, ipy_id, "print(f'Result: {x}')", ipy_info.display_name)
utils.execute(jet, ark_id, "cat('Result:', x, '\\n')", ark_info.display_name)

utils.request_shutdown(jet, ipy_id)

-- List all running kernels
utils.list_running_kernels(jet)

-- utils.execute(jet, ipy_id, "1 + 1", ipy_info.display_name)
utils.execute(jet, ark_id, "2 + 2", ark_info.display_name)

utils.request_shutdown(jet, ark_id)
