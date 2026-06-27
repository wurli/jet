local M = {}

function M.tbl_len(x)
	local n_items = 0
	for _, _ in pairs(x) do
		n_items = n_items + 1
	end
	return n_items
end

-- String representation of a Lua table
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
local function iter(cb)
	return function()
		while true do
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

---@param jet jet.engine
M.start_kernel = function(jet, spec)
	---@type jet.init.response
	local con = await(jet.start(spec))

	assert(type(con.client_id) == "string" and #con.client_id > 0, "expected session id from start")
	assert(type(con.kernel_info) == "table", "expected kernel info table")

	return {
		client_id = con.client_id,
		session_id = con.session_id,
		kernel_info = con.kernel_info,
		-- Firehose iterator (no-filter listen registered at boot). Long-lived:
		-- only ends on kernel shutdown, so consumers must `break` when they
		-- have what they need.
		stream = function()
			return iter(con.stream)
		end,
		execute = function(code)
			return iter(jet.execute_code(con.client_id, code, {}))
		end,
		comm_open = function(target_name, data)
			local comm_id, cb = jet.comm_open(con.client_id, target_name, data)
			return comm_id, iter(cb)
		end,
		comm_info = function(target_name)
			return iter(jet.comm_info(con.client_id, target_name))
		end,
		---@param comm_id string
		comm_listen = function(comm_id)
			return iter(jet.comm_listen(con.client_id, comm_id))
		end,
		provide_stdin = function(parent_id, value)
			jet.provide_stdin(con.client_id, parent_id, value)
		end,
		-- Register a filtered listener once and return a factory yielding
		-- a fresh iterator over the same underlying poll closure. Same
		-- long-lived contract as `stream`.
		---@param opts jet.listen.opts
		listen = function(opts)
			local cb = jet.listen(con.client_id, opts)
			return function()
				return iter(cb)
			end
		end,
		stop = function()
			jet.stop(con.session_id)
		end,
	}
end

return M
