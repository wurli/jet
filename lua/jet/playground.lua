-- Ark:execute("options(cli.num_colors = 256)")
-- Ark:execute("dplyr::tibble(x = 1:5, y = rnorm(5))")
-- Ipython = Kernel.new("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")
-- Ipython:execute("1 + 1")

local term_buf = vim.api.nvim_create_buf(false, true)
local term_chan = vim.api.nvim_open_term(term_buf, {})
vim.api.nvim_open_win(term_buf, false, { split = "right" })

local outputs = { "First", "Second", "Third", }

local generate_result = function()
    os.execute("sleep 1")
    return table.remove(outputs, 1)
end

vim.schedule(function()
    while true do
        local result = { coroutine.wrap(generate_result)() }
        if vim.tbl_count(result) == 0 then
            break
        end
        vim.api.nvim_chan_send(term_chan, vim.inspect(result) .. "\n")
    end
end)

-- local function send_next()
--     local result = generate_result()
--     if not result then
--         return
--     end
--     vim.api.nvim_chan_send(term_chan, result .. "\n")
--     vim.schedule(send_next)
-- end
-- vim.schedule(send_next)
