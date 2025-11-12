Manager = require("jet.core.manager")
Manager:open_kernel()
vim.keymap.set({ "n", "v" }, "<cr>", function() require("jet").send() end)

vim.print(Manager.map_kernel_filetype)
-- Kernel = require("jet.core.kernel")
-- Ark = Kernel.start("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")
-- Ipy = Kernel.start("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json")

-- vim.ui.select(
--     Manager:list_kernels({ language = "python" }),
--     {
--         ---@param item Jet.Manager.Kernel
--         format_item = function(item)
--             local out = item.spec.display_name
--             if #item.instances > 0 then
--                 local s = #item.instances == 1 and "" or "s"
--                 out = out .. (" (%d running instance%s)"):format(#item.instances, s)
--             end
--             return out
--         end
--     },
--     function(choice)
--         vim.print(choice)
--     end
-- )

-- Ark:execute({ "jkhist(rnorm(100))" })
-- Ark:execute({ "dplyr::tibble(x = 1:5, y = rnorm(5))" })
-- Ark:execute({ "for (i in 1:3) {Sys.sleep(0.5); print(i)}" })


local _comm_id, callback = Jet.comm_open(Ark.id, "lsp", { ip_address = "127.0.0.1" })
local function check_callback()
    -- Continuously check for results until we fail to receive a result
    while true do
        local result = callback()
        -- If idle then the execution is complete
        if result.status == "idle" then
            return
        end
        -- If no data yet, wait a bit (so we don't block the main thread)
        -- and check again later
        if not result.data then
            return vim.defer_fn(check_callback, 100)
        end
        local port = result.data.data.params.port
        print(("'Connecting to LSP on port %s'"):format(port))
        vim.lsp.config.ark = {
            cmd = vim.lsp.rpc.connect("127.0.0.1", port),
            root_markers = { ".git", ".Rprofile", ".Rproj", "DESCRIPTION" },
            filetypes = { "r", "R" },
        }
        vim.lsp.enable("ark")
    end
end
check_callback()

Ark:execute({ "my_df <- dplyr::tibble(x = 1:5, y = rnorm(5))" })

VariablesId, VariablesCb = Jet.comm_open(Ark.id, "positron.variables", {})

os.execute("sleep 0.5")

VariablesReqCb = Jet.comm_send(Ark.id, VariablesId, {
    method = "show_help_topic",
    params = {
        topic = "mean"
    }
})

os.execute("sleep 0.5")

for _ = 1, 5 do
    local result = VariablesReqCb()
    print(vim.inspect(result))
    if result.status == "idle" then
        break
    end
    os.execute("sleep 0.5")
end


for _ = 1, 5 do
    local result = VariablesCb()
    print(vim.inspect(result))
    if result.status == "idle" then
        break
    end
    os.execute("sleep 0.5")
end
