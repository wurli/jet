local M = {}

---@return integer
function M.tbl_len(x)
	local n_items = 0
	for _, _ in pairs(x) do
		n_items = n_items + 1
	end
	return n_items
end

-- String representation of a Lua table
---@param obj any
---@param level integer?
function M.dump(obj, level)
	level = level or 4
	local indent = (" "):rep(level)
	local prev_indent = (" "):rep(level - 4)

	if type(obj) == "table" and M.tbl_len(obj) > 0 then
		local s = "{\n"
		for k, v in pairs(obj) do
			if type(k) ~= "number" then
				k = '"' .. k .. '"'
			end
			s = s .. indent .. "[" .. k .. "] = " .. M.dump(v, level + 4) .. ",\n"
		end
		return s .. prev_indent .. "}"
	elseif type(obj) == "table" then
		return "{}"
	elseif type(obj) == "string" then
		return '"' .. obj .. '"'
	else
		return tostring(obj)
	end
end

M.print = function(obj, level)
	print(M.dump(obj, level))
end

-- Wrap a poll closure as a stateful iterator: skips "pending" frames,
-- returns each "busy" frame, ends when the kernel goes idle (poll → nil).
--
-- Per-request streams (execute/comm) terminate naturally on idle, so
-- iterating to exhaustion in a `for` loop is fine. Long-lived streams
-- (`kernel.stream`, `kernel.listen(...)`) only terminate on kernel
-- shutdown — consumers of those must `break` out themselves once they've
-- seen enough.
---@generic T
---@param cb fun(): T
---@param timeout_seconds integer?
---@return fun(): T?
local function iter(cb, timeout_seconds)
	local start_time = os.clock()
	return function()
		while true do
			if timeout_seconds and os.clock() - start_time > timeout_seconds then
				error(string.format("Iter timeout exceeded %ss", timeout_seconds))
			end
			local res = cb()
			if not res then
				return nil
			end
			if res.status ~= "pending" then
				return res
			end
		end
	end
end

-- Block until a jet.start / jet.attach poll closure yields its ready response.
-- Errors if the closure ends without ever producing one.
local await = function(poll)
	while true do
		local res = poll()
		assert(res ~= nil, "kernel boot poll ended before ready")
		if res.status == "ready" then
			return res
		end
	end
end

local get_jet = function()
	-- Try jet.core.engine for convenience when testing in Neovim
	local lib_ok, jet = pcall(require, "jet.core.engine")
	if not lib_ok then
		---@diagnostic disable-next-line: unresolved-require
		jet = require("jet")
	end
	return jet
end

M.jet = get_jet() --[[@as jet.engine]]

-- Kernelspec path chosen by the Rust runner in lua_smoke.rs (which reads
-- from the repo's `kernels/` dir populated by scripts/install-dev-kernels.sh).
-- Fail loudly when the env var is missing so an inline hardcoded path
-- doesn't sneak back into the test scripts.
function M.kernel_spec(name)
	local debug_info = debug.getinfo(1, "S")
	assert(debug_info, "failed to determine script dir for kernel spec path")
	local script_dir = debug_info.source:sub(2):match("(.*/)") or "./"
	return script_dir .. "../../../../test-kernels/" .. name .. "/kernel.json"
end

---@class jet.testing.kernel
---@field client_id string
---@field session_id string
---@field kernel_info jet.kernel.info
---@field msg_stream fun(): jet.kernel.response?
local Kernel = {}
Kernel.__index = Kernel

function Kernel.init(spec_name)
	---@type jet.init.response
	local con = await(M.jet.start(M.kernel_spec(spec_name)))

	assert(type(con.client_id) == "string" and #con.client_id > 0, "expected session id from start")
	assert(type(con.kernel_info) == "table", "expected kernel info table")

	local out = {
		client_id = con.client_id,
		session_id = con.session_id,
		kernel_info = con.kernel_info,
		msg_stream = con.stream,
	}

	return setmetatable(out, Kernel)
end

---@param timeout_seconds integer
function Kernel:stream(timeout_seconds)
	return iter(self.msg_stream, timeout_seconds)
end

---@param code string
---@param timeout_seconds integer
function Kernel:execute(code, timeout_seconds)
	return iter(M.jet.execute_code(self.client_id, code, false, true, {}), timeout_seconds)
end

---@param target_name string
---@param data table
function Kernel:comm_open(target_name, data)
	local comm_id, cb = M.jet.comm_open(self.client_id, target_name, data)
	return comm_id, iter(cb, 10)
end

---@param comm_id string
---@param data table
function Kernel:comm_send(comm_id, data)
	local cb = M.jet.comm_send(self.client_id, comm_id, data)
	return iter(cb, 10)
end

---@param comm_id string
---@param timeout_seconds integer
function Kernel:comm_info(comm_id, timeout_seconds)
	local cb = M.jet.comm_info(self.client_id, comm_id)
	return iter(cb, timeout_seconds)
end

---@param comm_id string
---@param timeout_seconds integer
function Kernel:comm_listen(comm_id, timeout_seconds)
	return iter(M.jet.comm_listen(self.client_id, comm_id), timeout_seconds)
end

---@param parent_id string
---@param value string
function Kernel:provide_stdin(parent_id, value)
	M.jet.provide_stdin(self.client_id, parent_id, value)
end

---@param opts jet.listen.opts
---@param timeout_seconds integer
function Kernel:listen(opts, timeout_seconds)
	local cb = M.jet.listen(self.client_id, opts)
	return function()
		return iter(cb, timeout_seconds)
	end
end

function Kernel:stop()
	M.jet.stop(self.session_id)
end

---@param spec_name string
M.start_kernel = function(spec_name)
	return Kernel.init(spec_name)
end

return M
