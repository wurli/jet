---@class Jet.State
---@field last_win number
---@field last_jet_win number
---@field last_normal_win number
local state = {}
state.__index = state

setmetatable(state, {
    __call = function(self, ...)
        return self.new(...)
    end
})

state.augroup = vim.api.nvim_create_augroup("jet_state", {})

vim.api.nvim_create_autocmd("WinLeave", {
    group = state.augroup,
    callback = function()
        state.last_win = vim.api.nvim_get_current_win()
        state[vim.b.jet and "last_jet_win" or "last_normal_win"] = state.last_win
    end,
})

return state
