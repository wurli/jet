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

M.start_kernel = function(jet, spec)
	local id, info = jet.connect(spec)

	assert(type(id) == "string" and #id > 0, "expected session id from connect")
	assert(type(info) == "table", "expected kernel info table")

	return {
		id = id,
		execute = function(code)
			local cb = jet.execute_code(id, code, {})
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
		end,
		provide_stdin = function(parent_id, value)
			jet.provide_stdin(id, parent_id, value)
		end,
		stop = function()
			jet.stop(id)
		end,
	}
end

return M
