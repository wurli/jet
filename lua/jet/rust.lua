local function get_lib_extension()
    if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then return '.dylib' end
    if jit.os:lower() == 'windows' then return '.dll' end
    return '.so'
end

local base_path = vim.fn.simplify(debug.getinfo(1).source:match('@?(.*/)') .. '../../target/release/')
local lib_name = 'jet'
local lib_extension = get_lib_extension()

-- Try loading with lib prefix first (Unix-style)
local lib_path = base_path .. 'lib' .. lib_name .. lib_extension
local loader = package.loadlib(lib_path, 'luaopen_' .. lib_name)

-- If that fails, try without lib prefix (Windows-style)
if not loader then
    lib_path = base_path .. lib_name .. lib_extension
    loader = package.loadlib(lib_path, 'luaopen_' .. lib_name)
end

if not loader then
    error('Failed to load native module from: ' .. lib_path)
end

---@class Jet.MsgContent.CompleteReply
---@field status "ok" | "error"
---@field matches string[]
---@field cursor_start number
---@field cursor_end number
---@field metadata table<string, any>

---@class Jet.MsgContent.DisplayData
---@field data table<string, any>
---@field metadata table<string, any>
---@field transient table<string, any>

---@class Jet.MsgContent.ExecuteError
---@field ename string
---@field evalue string
---@field traceback string[]

---@class Jet.MsgContent.ExecuteResult
---@field data table<string, any>
---@field execution_count number
---@field metadata table<string, any>

---@class Jet.MsgContent.InputRequest
---@field prompt string
---@field password boolean

---@class Jet.MsgContent.IsCompleteReply
---@field status "complete" | "incomplete" | "invalid" | "unknown"
---@field indent string?

---@class Jet.MsgContent.Stream
---@field name "stdout" | "stderr"
---@field text string

---@alias Jet.Msg.CompleteReply   { type: "complete_reply",    data: Jet.MsgContent.CompleteReply }
---@alias Jet.Msg.DisplayData     { type: "display_data",      data: Jet.MsgContent.DisplayData }
---@alias Jet.Msg.ExecuteError    { type: "error",             data: Jet.MsgContent.ExecuteError }
---@alias Jet.Msg.ExecutionResult { type: "execute_result",    data: Jet.MsgContent.ExecuteResult }
---@alias Jet.Msg.InputRequest    { type: "input_request",     data: Jet.MsgContent.InputRequest }
---@alias Jet.Msg.IsCompleteReply { type: "is_complete_reply", data: Jet.MsgContent.IsCompleteReply }
---@alias Jet.Msg.Stream          { type: "stream",            data: Jet.MsgContent.Stream }

---@alias Jet.Kernel.Id string

---@class Jet.Kernel.Info
---@field spec_path string
---@field display_name string
---@field banner string
---@field language Jet.Kernel.LanguageInfo

---@class Jet.Kernel.LanguageInfo
---@field name string
---@field version string
---@field mimetype string
---@field file_extension string
---@field pygments_lexer string?
---@field codemirror_mode table?
---@field nbconvert_exporter string?
---@field positron table?

---@class Jet.Kernel.Spec
---@field argv string[]
---@field display_name string
---@field language string
---@field interrupt_mode "signal" | "message" | nil
---@field env table<string, string>?
---@field metadata table<string, any>?
---@field kernel_protocol_version string?

---@alias Jet.Kernel.Spec.Path string

---@class Jet.Engine
---@field start_kernel fun(spec_path: string): (Jet.Kernel.Id, Jet.Kernel.Info)
---@field execute_code fun(kernel_id: string, code: string, user_expression: table?): Jet.ExecuteCallback
---@field is_complete fun(kernel_id: string, code: string): Jet.Msg.IsCompleteReply
---@field get_completions fun(kernel_id: string, code: string): Jet.Msg.CompleteReply
---@field request_shutdown fun(kernel_id: string): nil
---@field request_restart fun(kernel_id: string): table?
---@field provide_stdin fun(kernel_id: string, value: string): nil
---@field list_available_kernels fun(): table<Jet.Kernel.Spec.Path, Jet.Kernel.Spec>
---@field list_running_kernels fun(): table<Jet.Kernel.Id, Jet.Kernel.Info>

---@alias Jet.ExecuteCallback fun(): Jet.MsgGroup.ExecuteCode

---@alias Jet.MsgGroup.ExecuteCode
---| Jet.Msg.ExecutionResult
---| Jet.Msg.ExecuteError
---| Jet.Msg.Stream
---| Jet.Msg.DisplayData
---| Jet.Msg.InputRequest
---| {}

---@type Jet.Engine
local out = loader()

return out
