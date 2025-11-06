-- Ipython = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
-- Ipython:execute("1 + 1")

-- local term_buf = vim.api.nvim_create_buf(false, true)
-- vim.api.nvim_open_win(term_buf, true, { split = "right" })

Jet = require("jet.rust")
Kernel = require("jet.kernel")
Ark = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")
Ark:execute("options(cli.num_colors = 256)")
Ark:execute("dplyr::tibble(x = 1:5, y = rnorm(5))")
Ark:execute("for (i in 1:3) {Sys.sleep(0.5); print(i)}")


-- vim.print({Callback()})



-- while true do
--     local result = callback()
--     if vim.tbl_count(result) == 0 then
--         break
--     end
--     vim.api.nvim_chan_send(term_chan, vim.inspect(result) .. "\n")
-- end


-- local function send_next()
--     local result = generate_result()
--     if not result then
--         return
--     end
--     vim.api.nvim_chan_send(term_chan, result .. "\n")
--     vim.schedule(send_next)
-- end
-- vim.schedule(send_next)
