local function get_lib_extension()
    if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then return '.dylib' end
    if jit.os:lower() == 'windows' then return '.dll' end
    return '.so'
end

-- local base_path = vim.fn.simplify(debug.getinfo(1).source:match('@?(.*/)') .. '../../target/release/')
local base_path = debug.getinfo(1).source:match('@?(.*/)') .. '../../target/release/'
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


------ Message Types ----------------------------------------------------------
-- The Jet engine (rust) doesn't do any pre-processing of jupyter messages
-- besides filtering out the ones which aren't useful in Lua. The idea is that
-- providing the raw jupyter messages should make Jet more extensible. The
-- following are the message types that Jet currently uses.
-------------------------------------------------------------------------------
---@class Jet.Msg.CommClose
---@field comm_id Jet.Comm.Id

---@class Jet.Msg.CommMsg
---@field comm_id Jet.Comm.Id
---@field data table<string, any>

---@class Jet.Msg.CommOpen
---@field comm_id Jet.Comm.Id
---@field target_name string
---@field data table<string, any>

---@class Jet.Msg.CompleteReply
---@field status "ok" | "error"
---@field matches string[]
---@field cursor_start number
---@field cursor_end number
---@field metadata table<string, any>

---@class Jet.Msg.DisplayData
---@field data table<string, any>
---@field metadata table<string, any>
---@field transient table<string, any>

---@class Jet.Msg.ExecuteError
---@field ename string
---@field evalue string
---@field traceback string[]

---@class Jet.Msg.ExecuteInput
---@field code string
---@field execution_count number

---@class Jet.Msg.ExecuteResult
---@field data table<string, any>
---@field execution_count number
---@field metadata table<string, any>

---@class Jet.Msg.InputRequest
---@field prompt string
---@field password boolean

---@class Jet.Msg.IsCompleteReply
---@field status "complete" | "incomplete" | "invalid" | "unknown"
---@field indent string?

---@class Jet.Msg.Stream
---@field name "stdout" | "stderr"
---@field text string

------ Message Groups ---------------------------------------------------------
-- Not all functions return all message types; they are grouped here for
-- clarity.
-------------------------------------------------------------------------------
---@alias Jet.MsgGroup.Execute
---| Jet.Msg.DisplayData
---| Jet.Msg.ExecuteError
---| Jet.Msg.ExecuteInput
---| Jet.Msg.ExecuteResult
---| Jet.Msg.InputRequest
---| Jet.Msg.Stream

---@alias Jet.MsgType.Execute
---| "display_data"
---| "error"
---| "execute_input"
---| "execute_result"
---| "input_request"
---| "stream"

---@alias Jet.MsgGroup.Comm
---| Jet.Msg.CommClose
---| Jet.Msg.CommMsg
---| Jet.Msg.CommOpen

---@alias Jet.MsgType.Comm
---| "comm_close"
---| "comm_msg"
---| "comm_open"

---@alias Jet.ExecutionStatus
---| "busy"
---| "idle"

---@alias Jet.MsgType.IsCompleteReply
---| "is_complete_reply"


------ Kernel information -----------------------------------------------------
-- When a kernel starts up, Jet stores the following information about it.
-------------------------------------------------------------------------------
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

------ Kernel spec ------------------------------------------------------------
-- Kernels make thesmelves available through JSON spec files. Jet detects and
-- parses these into this format.
-------------------------------------------------------------------------------
---@class Jet.Kernel.Spec
---@field argv string[]
---@field display_name string
---@field language string
---@field interrupt_mode "signal" | "message" | nil
---@field env table<string, string>?
---@field metadata table<string, any>?
---@field kernel_protocol_version string?

------ Jet engine -------------------------------------------------------------
-- The Jet engine (rust) exposes the following functions to interact with
-- Jupyter kernels.
-------------------------------------------------------------------------------
---@class Jet.Engine
---@field comm_open fun(kernel_id: string, target_name: string, data: table): (Jet.Comm.Id, Jet.Callback.Comm)
---@field comm_send fun(kernel_id: string, comm_id: Jet.Comm.Id, data: table): Jet.Callback.Comm
---@field execute_code fun(kernel_id: Jet.Kernel.Id, code: string, user_expression: table?): Jet.Callback.Execute
---@field get_completions fun(kernel_id: Jet.Kernel.Id, code: string): Jet.Msg.CompleteReply
---@field is_complete fun(kernel_id: Jet.Kernel.Id, code: string): Jet.Callback.IsComplete
---@field list_available_kernels fun(): table<Jet.Kernel.Spec.Path, Jet.Kernel.Spec>
---@field list_running_kernels fun(): table<Jet.Kernel.Id, Jet.Kernel.Info>
---@field provide_stdin fun(kernel_id: Jet.Kernel.Id, value: string): nil
---@field request_restart fun(kernel_id: Jet.Kernel.Id): table?
---@field request_shutdown fun(kernel_id: Jet.Kernel.Id): nil
---@field start_kernel fun(spec_path: string): (Jet.Kernel.Id, Jet.Kernel.Info)

---@alias Jet.Comm.Id string
---@alias Jet.Callback.Comm fun(): Jet.Callback.Comm.Result
---@alias Jet.Callback.Comm.Result { status: Jet.ExecutionStatus, type: Jet.MsgType.Comm?, data: Jet.MsgGroup.Comm? }
---@alias Jet.Callback.Execute fun(): Jet.Callback.Execute.Result
---@alias Jet.Callback.Execute.Result { status: Jet.ExecutionStatus, type: Jet.MsgType.Execute?, data: Jet.MsgGroup.Execute? }
---@alias Jet.Callback.IsComplete fun(): Jet.Callback.IsComplete.Result
---@alias Jet.Callback.IsComplete.Result { status: Jet.ExecutionStatus, type: Jet.MsgType.IsCompleteReply?, data: Jet.Msg.IsCompleteReply? }
---@alias Jet.Kernel.Id string
---@alias Jet.Kernel.Spec.Path string

---@type Jet.Engine
local out = loader()

return out
