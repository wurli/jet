---@meta Jet

---@class jupyter.KernelSpec
---@field argv string[]
---@field display_name string
---@field language string
---@field interrupt_mode? "signal" | "message"
---@field env table<string, string>?
---@field metadata? table<string, any>
---@field kernel_protocol_version string?

---@class jupyter.LanguageInfo
---@field name string
---@field version string
---@field mimetype string
---@field file_extension string
---@field pygments_lexer string?
---@field codemirror_mode table?
---@field nbconvert_exporter string?
---@field positron table?

---@class jupyter.KernelInfo
---@field status "ok" | "error"
---@field protocol_version? string
---@field implementation? string
---@field language_info jupyter.LanguageInfo
---@field banner string
---@field debugger? boolean
---@field help_links table<string, string>
---@field supported_features? string[]

---@alias jupyter.channel "shell" | "iopub" | "stdin" | "control"

---@alias jupyter.msg_type
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

---@class jupyter.Msg.Header
---@field msg_id string
---@field session string
---@field username string
---@field date string
---@field msg_type jupyter.msg_type
---@field version string
---@field subshell_id string?

---@class jupyter.Msg
---@field channel jupyter.channel
---@field header jupyter.Msg.Header
---@field parent_header jupyter.Msg.Header?
---@field metadata table
---@field content table

---@class Jet.listen.Opts
---@field channel? jupyter.channel | jupyter.channel[]
---@field msg_type? jupyter.msg_type | jupyter.msg_type[]

---@class Jet.history.Opts
---@field output? boolean # Whether to include the (output) tuple. Defaults to false.
---@field raw? boolean # Whether to return the raw or transformed input. Defaults to true.
---@field session? integer # `range` only.
---@field start? integer # `range` only.
---@field stop? integer # `range` only.
---@field n? integer # `tail` and `search` only.
---@field pattern? string # `search` only (required).
---@field unique? boolean # `search` only. Defaults to false.

---@alias jet.callback<Val> fun(): { status: "done" | "pending" } | { status: "ready", value: Val }

---@class Jet.start.Result
---@field session_id? string
---@field client_id string
---@field kernel_info jupyter.KernelInfo
---@field lsp_port integer
---@field stream jet.callback<jupyter.Msg>

---@class Jet.stop.Result
---@field success boolean
---@field failure_msg string?

---@class jet.SessionInfo
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

---@class Jet
---@field start fun(spec_path: string, connection_file: string?, session_name: string?): jet.callback<Jet.start.Result>, jet.SessionInfo?
---@field attach fun(session_id: string?, connection_file: string?, session_name: string?): jet.callback<Jet.start.Result>, jet.SessionInfo?
---@field stop fun(session_id: string): jet.callback<Jet.stop.Result>
---@field interrupt fun(client_id: string)
---@field list_connections fun(): { client_id: string, session_id: string? }[]
---@field list_sessions fun(opts?: { status?: "open" | "closed" | "all", all_dirs?: boolean }): jet.callback<jet.SessionInfo[]>
---@field list_kernels fun(): { path: string, spec: jupyter.KernelSpec }[]
---@field show_spec fun(path: string): jupyter.KernelSpec
---@field show_session fun(session_id: string): { session: jet.SessionInfo, spec: jupyter.KernelSpec }
---@field execute_code fun(client_id: string, code: string, silent: boolean, allow_stdin: boolean, user_expressions: table?): jet.callback<jupyter.Msg>, string
---@field is_complete fun(client_id: string, code: string): jet.callback<jupyter.Msg>, string
---@field get_completions fun(client_id: string, code: string): jet.callback<jupyter.Msg>, string
---@field comm_open fun(client_id: string, target_name: string, data: table): jet.callback<jupyter.Msg>, string, string
---@field comm_send fun(client_id: string, comm_id: string, data: table): jet.callback<jupyter.Msg>, string
---@field comm_close fun(client_id: string, comm_id: string, data: table?): jet.callback<jupyter.Msg>, string
---@field comm_info fun(client_id: string, target_name: string?): jet.callback<jupyter.Msg>, string
---@field comm_listen fun(client_id: string, comm_id: string): jet.callback<jupyter.Msg>
---@field inspect fun(client_id: string, code: string, cursor_pos: integer, detail_level: integer?): jet.callback<jupyter.Msg>, string
---@field history fun(client_id: string, mode: "range" | "tail" | "search", opts: Jet.history.Opts): jet.callback<jupyter.Msg>, string
---@field kernel_info fun(client_id: string): jet.callback<jupyter.Msg>, string
---@field debug fun(client_id: string, content: table): jet.callback<jupyter.Msg>, string
---@field listen fun(client_id: string, opts?: Jet.listen.Opts): jet.callback<jupyter.Msg>
---@field provide_stdin fun(client_id: string, parent_msg_id: string, value: string): string
---@field make_session_id fun(lang: string): string
---@field version fun(): string -- Get the current version of Jet

---@type Jet
local M = {}
return M
