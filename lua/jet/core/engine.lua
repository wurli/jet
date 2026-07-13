local lib_path = require("jet.config").data.jet_library_path
assert(lib_path, "Could not resolve path to the Jet library")

local loader = package.loadlib(lib_path, "luaopen_jet")

---@class jet.kernel.languageinfo
---@field name string
---@field version string
---@field mimetype string
---@field file_extension string
---@field pygments_lexer string?
---@field codemirror_mode table?
---@field nbconvert_exporter string?
---@field positron table?

---@class jet.kernel.info
---@field status "ok" | "error"
---@field protocol_version? string
---@field implementation? string
---@field language_info jet.kernel.languageinfo
---@field banner string
---@field debugger? boolean
---@field help_links table<string, string>
---@field supported_features? table<string>

---@alias jet.channel "shell" | "iopub" | "stdin" | "control"

---@alias jet.msg_type
---| "execute_request"
---| "execute_reply"
---| "execute_input"
---| "execute_result"
---| "inspect_request"
---| "inspect_reply"
---| "complete_request"
---| "complete_reply"
---| "history_request"
---| "history_reply"
---| "is_complete_request"
---| "is_complete_reply"
---| "comm_info_request"
---| "comm_info_reply"
---| "kernel_info_request"
---| "kernel_info_reply"
---| "shutdown_request"
---| "shutdown_reply"
---| "interrupt_request"
---| "interrupt_reply"
---| "debug_request"
---| "debug_reply"
---| "stream"
---| "display_data"
---| "update_display_data"
---| "clear_output"
---| "error"
---| "status"
---| "debug_event"
---| "input_request"
---| "input_reply"
---| "comm_open"
---| "comm_msg"
---| "comm_close"

---@class jet.jupyter.msg.header
---@field msg_id string
---@field session string
---@field username string
---@field date string
---@field msg_type jet.msg_type
---@field version string
---@field subshell_id string?

---@class jet.jupyter.msg
---@field channel jet.channel
---@field header jet.jupyter.msg.header
---@field parent_header jet.jupyter.msg.header?
---@field metadata table
---@field content table

---@class jet.kernel.response
---@field status "busy" | "pending"
---@field msg jet.jupyter.msg

---@class jet.listen.opts
---@field channel? jet.channel | jet.channel[]
---@field msg_type? jet.msg_type | jet.msg_type[]

---@class jet.init.response
---@field status "ready" | "pending"
---@field session_id? string
---@field client_id string
---@field kernel_info jet.kernel.info
---@field stream fun(): jet.kernel.response?

---@alias jet.init.callback fun(): jet.init.response?

---@class jet.session_info
---@field session_id string
---@field closed_at string?
---@field connection_file string
---@field created_at string # E.g. 2026-07-07T20:11:08Z
---@field kernel_pid number?
---@field kernelspec_path string
---@field language string
---@field display_name string
---@field status "open" | "closed"
---@field working_dir string

---@class jet.engine
---@field start fun(spec_path: string, connection_file: string?, session_name: string?): jet.init.callback, jet.session_info?
---@field attach fun(session_id: string?, connection_file: string?, session_name: string?): jet.init.callback, jet.session_info?
---@field stop fun(session_id: string)
---@field interrupt fun(client_id: string)
---@field list_connections fun(): { client_id: string, session_id: string? }
---@field list_sessions fun(opts?: { status?: "open" | "closed" | "all", all_dirs?: boolean }): jet.session_info[]
---@field list_kernels fun(): { path: string, spec: jet.kernel.spec }[]
---TODO: should accept short paths like ~/...
---@field show_spec fun(path: string): jet.kernel.spec
---@field show_session fun(session_id: string): { session: jet.session_info, spec: jet.kernel.spec }
---@field execute_code fun(client_id: string, code: string, silent: bool, allow_stdin: bool, user_expression: table?): fun(): jet.kernel.response?
---@field is_complete fun(client_id: string, code: string): fun(): jet.kernel.response?
---@field get_completions fun(client_id: string, code: string): table?
---@field comm_open fun(client_id: string, comm_id: string, data: table): string, fun(): jet.kernel.response?
---@field comm_send fun(client_id: string, comm_id: string, data: table): fun(): jet.kernel.response?
---@field comm_info fun(client_id: string, target_name: string?): fun(): jet.kernel.response?
---@field comm_listen fun(client_id: string, comm_id: string): fun(): jet.kernel.response?
---@field listen fun(client_id: string, opts?: jet.listen.opts): fun(): jet.kernel.response?
---@field provide_stdin fun(client_id: string, parent_msg_id: string, value: string)
---@field make_session_id fun(lang: string): string
---@field version fun(): string -- Get the current version of Jet
local out = loader()

return out
