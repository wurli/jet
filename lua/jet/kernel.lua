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
                return vim.defer_fn(check_callback, 50)
            end
            self:_handle_result(result)
        end
    end

    check_callback()
end

function jet_kernel:open_repl()
    self.repl_output_winnr = vim.api.nvim_open_win(self.repl_output_bufnr, true, {
        split = "right"
    })

    self.repl_input_winnr = vim.api.nvim_open_win(self.repl_input_bufnr, true, {
        relative = "win",
        win = self.repl_output_winnr,
        col = 1,
        height = 1,
        row = vim.api.nvim_win_get_height(self.repl_output_winnr) - 1,
        width = vim.api.nvim_win_get_width(self.repl_output_winnr),
        border = "none"
    })

    local jet_ns = vim.api.nvim_create_namespace("jet_repl_input")

    vim.api.nvim_buf_set_extmark(self.repl_input_bufnr, jet_ns, 0, 0, {
        sign_text = "> ",
    })

    vim.wo[self.repl_output_winnr].number = false
    vim.wo[self.repl_output_winnr].relativenumber = false
    vim.wo[self.repl_output_winnr].listchars = ""
    vim.wo[self.repl_input_winnr].number = false
    vim.wo[self.repl_input_winnr].relativenumber = false
    vim.wo[self.repl_input_winnr].cursorline = false

    -- vim.bo[self.repl_input_bufnr].filetype = "R"
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

    -- TODO: Improve keymaps
    for _, key in ipairs({ "i", "I", "a", "A", "c", "C", "s", "S", "o", "O", "p", "P" }) do
        vim.keymap.set("n", key, function()
            self:_with_input_win(function(winnr)
                vim.api.nvim_set_current_win(winnr)
                vim.cmd.normal(key)
            end)
        end, { buffer = self.repl_output_bufnr })
    end

    vim.api.nvim_create_autocmd("WinClosed", {
        buffer = self.repl_output_bufnr,
        callback = function()
            self:_with_input_win(function(winnr)
                vim.api.nvim_win_close(winnr, true)
            end)
        end
    })

    vim.api.nvim_create_autocmd("WinResized", {
        callback = function()
            if vim.api.nvim_get_current_win() == self.repl_input_winnr then
                vim.api.nvim_win_set_config(
                    self.repl_output_winnr,
                    {
                        width = vim.api.nvim_win_get_width(self.repl_input_winnr),
                    }
                )
            elseif vim.api.nvim_get_current_win() == self.repl_output_winnr then
                vim.api.nvim_win_set_config(
                    self.repl_input_winnr,
                    {
                        relative = "win",
                        win = self.repl_output_winnr,
                        col = 1,
                        height = 1,
                        row = vim.api.nvim_win_get_height(self.repl_output_winnr) - 1,
                        width = vim.api.nvim_win_get_width(self.repl_output_winnr),
                    }
                )
            end
        end
    })
end

function jet_kernel:_with_input_buf(fn)
    if vim.api.nvim_buf_is_valid(self.repl_input_bufnr) then
        fn(self.repl_input_bufnr)
    end
end

function jet_kernel:_with_output_buf(fn)
    if vim.api.nvim_buf_is_valid(self.repl_output_bufnr) then
        fn(self.repl_output_bufnr)
    end
end

function jet_kernel:_with_input_win(fn)
    if vim.api.nvim_win_is_valid(self.repl_input_winnr) then
        fn(self.repl_input_winnr)
    end
end

function jet_kernel:_with_output_win(fn)
    if vim.api.nvim_win_is_valid(self.repl_output_winnr) then
        fn(self.repl_output_winnr)
    end
end

---@param msg Jet.ExecuteCallback.Result
function jet_kernel:_handle_result(msg)
    if not msg.data then
        return
    end

    if msg.type == "execute_result" then
        self:_handle_text_output(msg.data.data["text/plain"] .. "\n")
    elseif msg.type == "stream" then
        self:_handle_text_output(msg.data.text)
    elseif msg.type == "error" then
        self:_handle_text_output(msg.data.evalue)
    elseif msg.type == "input_request" then
        self:_handle_text_output(msg.data.prompt)
    end

    self:_scroll_to_end()
end

function jet_kernel:_scroll_to_end()
    self:_with_output_buf(function(output_buf)
        vim.api.nvim_buf_call(output_buf, function()
            vim.fn.cursor(vim.fn.line("$"), 0)
        end)
    end)
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
