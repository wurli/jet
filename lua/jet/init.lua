local M = {}

M.setup = function(_)

end

Jet = require("jet.core.rust")
vim.print(Jet.list_available_kernels())

return M

