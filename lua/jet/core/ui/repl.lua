local utils = require("jet.core.utils")
local spinners = require("jet.core.ui.spinners")

---@class Jet.Ui.Repl
---The REPL input buffer number
---@field prompt_bufnr number
---
---The REPL input window number
---@field prompt_winnr number
---
---The REPL input window number
---@field output_bufnr number
---
---The REPL input window number
---@field output_winnr number
---
---The REPL background buffer number
---@field background_bufnr number
---
---The REPL background window number
---@field background_winnr number
---
---The REPL output channel
---@field repl_channel number
---
---The augroup for autocommands
---@field _augroup number
---
---@field indent_chars { main: string, continue: string }
---@field indent_templates { main: string, continue: string }
---
---TODO: do we need these?
---@field last_win number
---@field last_normal_win number
---@field last_jet_win number
---
---A reference to the kernel this UI belongs to. NB, it might seem odd to
---include the kernel as a field of the UI while the UI is itself a field of
---the kernel, but it makes lots of things very convenient, e.g. getting
---history from the kernel.
---@field kernel Jet.Kernel
---
---The namespace for virtual text indent text
---@field _ns number
local repl = {}
repl.__index = repl

setmetatable(repl, {
    ---@return Jet.Ui.Repl
    __call = function(self, ...)
        return self.start(...)
    end,
})

---@param kernel Jet.Kernel
---@param opts? { show: boolean }
function repl.init(kernel, opts)
    opts = vim.tbl_extend("force", opts or {}, {
        show = true
    })
    local self = setmetatable({}, repl)
    self.indent_chars = vim.tbl_deep_extend("keep", self.indent_chars or {}, {
        main = ">",
        continue = "+",
    })
    self.indent_templates = vim.tbl_deep_extend("keep", self.indent_templates or {}, {
        main = "%s ",
        continue = "%s ",
    })
    self.kernel = kernel
    self._ns = vim.api.nvim_create_namespace("jet_repl_" .. self.kernel.id)
    self._augroup = vim.api.nvim_create_augroup("jet_repl_" .. self.kernel.id, {})
    self:_init_ui()
    self:_filetype_set()
    self:_display_output(utils.add_linebreak(self.kernel.instance.info.banner))
    if opts.show then
        self:show()
    end
    return self
end

function repl:delete()
    --- Hide the UI
    self:hide()
    --- Delete REPL buffers
    for _, buf in ipairs({
        self.background_bufnr,
        self.prompt_bufnr,
        self.output_bufnr,
    }) do
        if vim.api.nvim_buf_is_valid(buf) then
            vim.api.nvim_buf_delete(buf, { force = true })
        end
    end
    --- Delete autocommands
    vim.api.nvim_delete_augroup_by_id(self._augroup)
end

function repl:show()
    -- ╭───────────╮
    -- │ box chars │
    -- ╰───────────╯

    self.background_winnr = vim.api.nvim_open_win(self.background_bufnr, false, {
        split = "right",
        focusable = false,
    })

    self.output_winnr = vim.api.nvim_open_win(self.output_bufnr, false, {
        relative = "win",
        win = self.background_winnr,
        col = 0,
        row = 0,
        height = vim.api.nvim_win_get_height(self.background_winnr) - 4,
        width = vim.api.nvim_win_get_width(self.background_winnr) - 4,
        border = { "╭", "─", "╮", "│", "│", " ", "│", "│" },
        zindex = 10,
        style = "minimal",
    })

    self.prompt_winnr = vim.api.nvim_open_win(self.prompt_bufnr, false, {
        relative = "win",
        win = self.background_winnr,
        height = 1,
        col = 0,
        row = vim.api.nvim_win_get_height(self.background_winnr),
        width = vim.api.nvim_win_get_width(self.background_winnr) - 4,
        border = { "│", "─", "│", "│", "╯", "─", "╰", "│" },
        zindex = 20,
        style = "minimal",
    })

    vim.wo[self.output_winnr].listchars = ""

    self:_set_layout()
end

function repl:hide()
    for _, winnr in ipairs({
        self.background_winnr,
        self.prompt_winnr,
        self.output_winnr,
    }) do
        if vim.api.nvim_win_is_valid(winnr) then
            vim.api.nvim_win_close(winnr, true)
        end
    end
end

function repl:_init_ui(id)
    for _, jet_ui in ipairs({ "background", "prompt", "output" }) do
        local buf_name = jet_ui .. "_bufnr"
        if self[buf_name] and vim.api.nvim_buf_is_valid(self[buf_name]) then
            utils.log_warn("REPL %s buffer already exists with bufnr %s", buf_name, self[buf_name])
        else
            local buf = vim.api.nvim_create_buf(false, true)
            self[buf_name] = buf
            vim.bo[buf].buftype = "nofile"
            vim.b[buf].jet = {
                type = "repl_" .. jet_ui,
                id = id,
            }
        end
    end

    -- Jet sends output from the kernel to a terminal channel in order to
    -- format ansi formatting.
    if self.repl_channel then
        utils.log_warn("REPL output channel `%s` already exists!", self.repl_channel)
    else
        self.repl_channel = vim.api.nvim_open_term(self.output_bufnr, {})
    end

    self:_indent_reset()

    --- Set keymaps
    vim.keymap.set({ "n", "i" }, "<CR>", function()
        self:maybe_execute_prompt()
    end, {
        buffer = self.prompt_bufnr,
        desc = "Jet REPL: execute code"
    })

    -- TODO: Improve keymaps
    for _, key in ipairs({ "i", "I", "a", "A", "c", "C", "s", "S", "o", "O", "p", "P" }) do
        vim.keymap.set("n", key, function()
            self:_with_prompt_win(function(winnr)
                vim.api.nvim_set_current_win(winnr)
                vim.fn.feedkeys(key, "n")
            end)
        end, { buffer = self.output_bufnr })
    end

    vim.keymap.set({ "n", "i" }, "<c-p>", function()
        self:_prompt_set(self.kernel:history_get(-1))
    end, { buffer = self.prompt_bufnr })

    vim.keymap.set({ "n", "i" }, "<c-n>", function()
        self:_prompt_set(self.kernel:history_get(1))
    end, { buffer = self.prompt_bufnr })

    --- Set autocommands
    --- Attach LSP to the REPL input buffer
    --- (TODO: give the user the ability to disable this)
    vim.api.nvim_create_autocmd("BufEnter", {
        group = self._augroup,
        buffer = self.prompt_bufnr,
        callback = function()
            for _, cfg in pairs(vim.lsp._enabled_configs) do
                if cfg.resolved_config then
                    local ft = cfg.resolved_config.filetypes
                    if ft and vim.tbl_contains(ft, self.kernel.filetype) or not ft then
                        vim.lsp.start(cfg.resolved_config, {
                            bufnr = self.prompt_bufnr,
                        })
                    end
                end
            end
        end,
    })

    vim.api.nvim_create_autocmd("BufUnload", {
        group = self._augroup,
        callback = function(e)
            if vim.tbl_contains({ self.prompt_bufnr, self.output_bufnr }, e.buf) then
                self:delete()
            end
        end,
    })

    vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
        group = self._augroup,
        buffer = self.prompt_bufnr,
        callback = function()
            self:_indent_reset()
        end,
    })

    vim.api.nvim_create_autocmd("WinEnter", {
        group = self._augroup,
        buffer = self.background_bufnr,
        callback = function()
            -- When we enter the background window we want to automatically
            -- enter a different window. The approach is:
            -- *  Entering from the repl input     => go to repl output
            -- *  Entering from the repl output    => go to last normal window
            -- *  Entering from last normal window => go to repl input
            -- This should hopefully make entering/leaving the REPL windows
            -- feel natural and work well with the user's existing keymaps.
            vim.api.nvim_set_current_win(
                (self.last_win == self.prompt_winnr and self.output_winnr)
                or (self.last_win == self.output_winnr and self.last_normal_win)
                or (self.last_win == self.last_normal_win and self.prompt_winnr)
                or self.last_win
            )
        end,
    })

    vim.api.nvim_create_autocmd("WinClosed", {
        group = self._augroup,
        callback = function(e)
            local repl_wins = { self.background_winnr, self.prompt_winnr, self.output_winnr }
            local repl_bufs = { self.background_bufnr, self.prompt_bufnr, self.output_bufnr }

            if not vim.tbl_contains(repl_bufs, e.buf) then
                return
            end

            for _, winnr in ipairs(repl_wins) do
                if vim.api.nvim_win_is_valid(winnr) then
                    vim.api.nvim_win_close(winnr, true)
                end
            end
        end,
    })

    vim.api.nvim_create_autocmd("WinResized", {
        group = self._augroup,
        callback = function()
            self:_set_layout()
        end,
    })



    vim.api.nvim_create_autocmd("WinLeave", {
        group = self._augroup,
        callback = function()
            self.last_win = vim.api.nvim_get_current_win()
            local buf_is_repl = vim.b.jet and vim.b.jet.id == self.kernel.id
            self[buf_is_repl and "last_jet_win" or "last_normal_win"] = self.last_win
        end,
    })
end

function repl:_set_layout()
    if not (vim.api.nvim_win_is_valid(self.prompt_winnr) and vim.api.nvim_win_is_valid(self.output_winnr)) then
        return
    end

    -- First, if we're in either the input or output window, resize the background
    -- according to the current window width
    local cur_win = vim.api.nvim_get_current_win()
    if cur_win == self.output_winnr or cur_win == self.prompt_winnr then
        vim.api.nvim_win_set_config(self.background_winnr, {
            width = vim.api.nvim_win_get_width(cur_win) + 2,
        })
    end

    -- Now we're sure the background is the right size, set both input and output
    -- to match its width
    for _, win in ipairs({ self.prompt_winnr, self.output_winnr }) do
        vim.api.nvim_win_set_config(win, {
            width = math.max(vim.api.nvim_win_get_width(self.background_winnr) - 2, 1),
        })
    end

    -- TODO: if we've just resized the output window vertically, adjust the input
    -- window height accordingly
    -- if cur_win == self.repl_output_winnr then
    --     vim.api.nvim_win_set_config(self.repl_input_winnr, {
    --         height =
    --     })
    -- end

    local bg_height = vim.api.nvim_win_get_height(self.background_winnr)
    local prompt_height = vim.api.nvim_win_get_height(self.prompt_winnr)
    vim.api.nvim_win_set_config(self.output_winnr, {
        -- We need to subtract 1 to account for the borders (the output's
        -- bottom border should overlap with the input's top border)
        height = math.max(bg_height - prompt_height - 2, 1),
    })

    self:_indent_reset()
    self:_title_set()
end

---Executes code in the kernel and displays results in the REPL.
---Leaves the REPL input window unchanged.
---Shows a fancy spinner. Swish!
---@param code string[]
function repl:execute(code)
    local stop_spinner = spinners.run(function(frame)
        self:_subtitle_set(frame)
    end, function()
        self:_subtitle_set()
    end, 100)

    self.kernel:execute(
        code,
        function(msg)
            if msg.type == "execute_input" then
                -- Add the prompt indent to input code, otherwise it can be hard to
                -- tell what's input and what's output.
                msg.data.code = self:_indent_get_main() .. msg.data.code:gsub("\n", "\n" .. self:_indent_get_continue())
            end
            self:_display_output(utils.msg_to_string(msg))
        end,
        function()
            self:_display_output("\n")
            stop_spinner()
        end
    )
end

---Execute and clear the prompt
function repl:execute_prompt()
    local code = self:_prompt_get()
    self:_prompt_set({})
    self:execute(code)
    vim.api.nvim_win_set_config(self.prompt_winnr, { height = 1 })
end

--Check for incompleteness before possibly executing.
function repl:maybe_execute_prompt()
    self.kernel:if_complete(self:_prompt_get(), {
        complete = function()
            self:execute_prompt()
        end,
        incomplete = function()
            if vim.fn.bufnr() == self.prompt_bufnr then
                vim.api.nvim_feedkeys("\r", "n", false)
            end
        end
    })
end

---@param fn fun(bufnr: number?)
function repl:_with_prompt_buf(fn)
    if vim.api.nvim_buf_is_valid(self.prompt_bufnr) then
        fn(self.prompt_bufnr)
    end
end

---@param fn fun(bufnr: number?)
function repl:_with_output_buf(fn)
    if vim.api.nvim_buf_is_valid(self.output_bufnr) then
        fn(self.output_bufnr)
    end
end

---@param fn fun(winnr: number?)
function repl:_with_prompt_win(fn)
    if vim.api.nvim_win_is_valid(self.prompt_winnr) then
        fn(self.prompt_winnr)
    end
end

---@param fn fun(winnr: number?)
function repl:_with_output_win(fn)
    if vim.api.nvim_win_is_valid(self.output_winnr) then
        fn(self.output_winnr)
    end
end

function repl:_indent_reset()
    if self.prompt_winnr then
        local n_lines = #vim.api.nvim_buf_get_lines(self.prompt_bufnr, 0, -1, false)
        vim.api.nvim_win_set_config(self.prompt_winnr, { height = n_lines })
    end
    self:_indent_clear(0, -1)
    for i = 1, vim.fn.line("$", self.prompt_winnr) do
        self:_indent_set(i - 1)
    end
end

---@param line_start number
---@param line_end number
function repl:_indent_clear(line_start, line_end)
    self:_with_prompt_buf(function(prompt_buf)
        vim.api.nvim_buf_clear_namespace(prompt_buf, self._ns, line_start, line_end)
    end)
end

---@param lnum number 0-indexed
---@param text? string Defaults to the repl indent for `lnum`
function repl:_indent_set(lnum, text)
    text = text or (lnum == 0 and self:_indent_get_main() or self:_indent_get_continue())

    self:_with_prompt_buf(function(prompt_buf)
        vim.api.nvim_buf_set_extmark(prompt_buf, self._ns, lnum, 0, {
            -- TODO: add Jet highlight groups
            virt_text = { { text, "FloatTitle" } },
            virt_text_pos = "inline",
            right_gravity = false,
        })
    end)
end

function repl:_indent_get_main()
    return self.indent_templates.main:format(self.indent_chars.main)
end

function repl:_indent_get_continue()
    return self.indent_templates.continue:format(self.indent_chars.continue)
end

---@param text string[]
function repl:_prompt_set(text)
    if not text then
        return
    end
    vim.api.nvim_buf_set_lines(self.prompt_bufnr, 0, -1, false, text)
    self:_indent_reset()
end

---@return string[]
function repl:_prompt_get()
    return vim.api.nvim_buf_get_lines(self.prompt_bufnr, 0, -1, false)
end

function repl:_scroll_to_end()
    self:_with_output_buf(function(output_buf)
        vim.api.nvim_buf_call(output_buf, function()
            vim.fn.cursor(vim.fn.line("$"), 0)
        end)
    end)
end

---@param title string?
function repl:_title_set(title)
    -- local inst = self.instance
    -- local spec = inst and inst.spec
    -- local info = inst and inst.info
    -- title = title or (spec and spec.display_name) or (info and info.language_info.name)

    if title then
        vim.api.nvim_win_set_config(self.output_winnr, {
            title = title,
            title_pos = "center",
        })
    end
end

---@param info string?
function repl:_subtitle_set(info)
    self:_with_prompt_win(function(win)
        vim.api.nvim_win_set_config(win, {
            title = info or "",
            title_pos = "right",
        })
    end)
end

function repl:_filetype_set(filetype)
    vim.bo[self.prompt_bufnr].filetype = filetype
end

---@param text? string
function repl:_display_output(text)
    if not text then
        return
    end

    vim.api.nvim_chan_send(self.repl_channel, text)
end

return repl
