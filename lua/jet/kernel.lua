local jet_engine = require("jet.rust")

---@alias Jet.Execution.TextOutput string
---@alias Jet.Repl.Bufnr number
---@alias Jet.Repl.Channel number

---@class Jet.Kernel
---@field id Jet.Kernel.Id
---@field info Jet.Kernel.Info
---@field repl_output_bufnr Jet.Repl.Bufnr
---@field repl_channel Jet.Repl.Channel
local jet_kernel = {}
jet_kernel.__index = jet_kernel

setmetatable(jet_kernel, {
    __call = function(self, ...)
        return self.new(...)
    end
})

---@param spec_path Jet.Kernel.Spec.Path
function jet_kernel.new(spec_path)
    local self = setmetatable({}, jet_kernel)
    self:_init_repl()
    self:open_repl()
    self.id, self.info = jet_engine.start_kernel(spec_path)
    self:_handle_text_output(self.info.banner)
    return self
end

---@param code string
function jet_kernel:execute(code)
    if not self.id then
        error("Kernel is not active; use `start()` to activate the kernel.")
    end

    self:_handle_text_output("\n> " .. code .. "\n")
    local callback = jet_engine.execute_code(self.id, code, {})

    while true do
        local msg = callback()
        if vim.tbl_count(msg) > 0 then
            self:_handle_result(msg)
        else
            break
        end
    end
end

function jet_kernel:open_repl()
    local output_winid = vim.api.nvim_open_win(self.repl_output_bufnr, true, {
        split = "right"
    })

    local input_winid = vim.api.nvim_open_win(self.repl_input_bufnr, true, {
        relative = "win",
        win = output_winid,
        col = 1,
        height = 1,
        row = vim.api.nvim_win_get_height(output_winid) - 1,
        width = vim.api.nvim_win_get_width(output_winid),
        border = "none"
    })

    for _, key in ipairs({ "i", "I", "a", "A", "c", "C", "s", "S", "o", "O" }) do
        vim.keymap.set("n", key, function()
            vim.api.nvim_set_current_win(input_winid)
            vim.cmd.startinsert()
        end, { buffer = self.repl_output_bufnr })
    end

    local jet_ns = vim.api.nvim_create_namespace("jet_repl_input")

    vim.api.nvim_buf_set_extmark(self.repl_input_bufnr, jet_ns, 0, 0, {
        sign_text = "> ",
    })

    vim.wo[output_winid].number = false
    vim.wo[output_winid].relativenumber = false
    vim.wo[output_winid].listchars = ""
    vim.wo[input_winid].number = false
    vim.wo[input_winid].relativenumber = false
    vim.wo[input_winid].cursorline = false

    vim.bo[self.repl_input_bufnr].filetype = "R"
end

function jet_kernel:_init_repl()
    if self.repl_output_bufnr then
        print("REPL output buffer already exists!")
    else
        self.repl_output_bufnr = vim.api.nvim_create_buf(false, true)
        vim.bo[self.repl_output_bufnr].modifiable = false
    end

    if self.repl_input_bufnr then
        print("REPL input buffer already exists!")
    else
        self.repl_input_bufnr = vim.api.nvim_create_buf(false, true)
        vim.keymap.set({ "n", "i" }, "<CR>", function()
            local code = vim.api.nvim_buf_get_lines(self.repl_input_bufnr, 0, -1, false)
            vim.schedule(function()
                vim.api.nvim_buf_set_lines(self.repl_input_bufnr, 0, -1, false, {})
                self:execute(table.concat(code, "\n"))
            end)
        end, { buffer = self.repl_input_bufnr })
    end

    if self.repl_channel then
        print("REPL channel already exists!")
    else
        self.repl_channel = vim.api.nvim_open_term(self.repl_output_bufnr, {})
    end
end

---@param msg Jet.MsgGroup.ExecuteCode
function jet_kernel:_handle_result(msg)
    if msg.type == "execute_result" then
        self:_handle_text_output(msg.data.data["text/plain"])
    elseif msg.type == "stream" then
        self:_handle_text_output(msg.data.text)
    elseif msg.type == "error" then
        self:_handle_text_output(msg.data.evalue)
    elseif msg.type == "input_request" then
        self:_handle_text_output(msg.data.prompt)
    end
end

---@param text string
function jet_kernel:_handle_text_output(text)
    if not text then
        return
    end
    -- if not self.repl_channel then
    --     self._init_repl(self)
    -- end

    vim.api.nvim_chan_send(self.repl_channel, text)

    -- local last_line = vim.api.nvim_buf_line_count(self.repl_output_bufnr)
    -- vim.api.nvim_win_set_cursor(vim.fn.bufwinid(self.repl_output_bufnr), {last_line, 0})
end

return jet_kernel
