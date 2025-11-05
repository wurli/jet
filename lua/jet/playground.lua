-- Ipython = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
-- Ipython:execute("1 + 1")

-- local term_buf = vim.api.nvim_create_buf(false, true)
-- vim.api.nvim_open_win(term_buf, true, { split = "right" })

Jet = require("jet.rust")
-- Kernel = require("jet.kernel")
-- Ark = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")
-- Ark:execute("options(cli.num_colors = 256)")
-- Ark:execute("dplyr::tibble(x = 1:5, y = rnorm(5))")
local id = Jet.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")


local term_buf = vim.api.nvim_create_buf(false, true)
vim.api.nvim_open_win(term_buf, true, { split = "right" })
local term_chan = vim.api.nvim_open_term(term_buf, {})

local callback = Jet.execute_code(id, "for (i in 1:3) {Sys.sleep(0.5); print(i)}", {})
local w = vim.uv.new_work(function()
    local res = callback()
    vim.print(res)
    vim.api.nvim_chan_send(term_chan, vim.inspect(res) .. "\n")
end, function(...) vim.print({ ... }) end)
vim.uv.queue_work(w, callback)

-- local function run()
--     vim.schedule(function()
--         coroutine.resume(coroutine.create(function()
--             local result = callback()
--             if vim.tbl_count(result) == 0 then return end
--             vim.api.nvim_chan_send(term_chan, vim.inspect(result) .. "\n")
--             run()
--         end))
--     end)
-- end
-- run()


-- local function send_next()
--     local result = generate_result()
--     if not result then
--         return
--     end
--     vim.api.nvim_chan_send(term_chan, result .. "\n")
--     vim.schedule(send_next)
-- end
-- vim.schedule(send_next)
