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

function M.try_run(jet, id, code, check, err_msg, timeout)
	timeout = timeout or 30
	local cb = jet.execute_code(id, code, {})
	local deadline = os.time() + timeout
	local messages = {}
	while os.time() < deadline do
		local res = cb()
		if res and res.status ~= "pending" then
			table.insert(messages, res)
		end
		if res == nil then
			error((err_msg or ("Check didn't pass for code " .. code)) .. "\nMessages: " .. M.dump(messages))
		end
		if check(res) then
			return true
		end
	end
	error("Timeout waiting for kernel to finish executing code" .. "\nMessages: " .. M.dump(messages))
end

return M
