local M = {}

--------------------------------------------------
-- Get the carpo library
--------------------------------------------------
local libname = "carpo"

local get_lib_extension = function()
    if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then return '.dylib' end
    if jit.os:lower() == 'windows' then return '.dll' end
    return '.so'
end

M.carpo_loader = package.loadlib(
    "./target/release/lib" .. libname .. get_lib_extension(),
    "luaopen_" .. libname
)

--------------------------------------------------
-- Helpers
--------------------------------------------------

--- Get the number of keys in a table
M.tbl_len = function(t)
    local count = 0
    for _ in pairs(t) do count = count + 1 end
    return count
end

--- Dump a Lua object as a string
function M.dump(o, level)
    level = level or 4
    local indent = (" "):rep(level)
    local prev_indent = (" "):rep(level - 4)
    if type(o) == "table" and M.tbl_len(o) > 0 then
        local s = "{\n"
        for k, v in pairs(o) do
            if type(k) ~= "number" then k = '"' .. k .. '"' end
            s = s .. indent .. "[" .. k .. "] = " .. M.dump(v, level + 4) .. ",\n"
        end
        return s .. prev_indent .. "}"
    elseif type(o) == "table" then
        return "{}"
    elseif type(o) == "string" then
        return '"' .. o .. '"'
    else
        return tostring(o)
    end
end

---@param x string?
---@param pad string?
M.cat_header = function(x, pad)
    local out_len = 80
    pad = pad or "-"
    x = x and " " .. x .. " " or ""
    local x_len = x:len()
    print(pad:rep(2) .. x .. pad:rep(math.max(out_len - 2 - x_len, 0)))
end

--- Execute code in the carpo kernel and print results until the execution finishes
function M.execute(carpo, kernel_id, code, user_expressions)
    user_expressions = user_expressions or {}

    M.cat_header(nil, "=")
    print("Executing code in kernel: " .. kernel_id)
    if M.tbl_len(user_expressions) > 0 then
        print("User expressions: " .. M.dump(user_expressions))
    end
    M.cat_header(nil, "=")
    print("```")
    print(code)
    print("```")

    local callback = carpo.execute_code(kernel_id, code, user_expressions)

    local i = 0
    while true do
        i = i + 1
        M.cat_header("Result " .. i)
        local result = callback()
        print(M.dump(result))
        if M.tbl_len(result) == 0 then break end

        if result.type == "input_request" then
            local stdin = "Hello from Lua!"
            M.cat_header(("Sending dummy val '%s'"):format(stdin), ".")
            if kernel_id then
                carpo.provide_stdin(kernel_id, stdin)
            else
                carpo.provide_stdin(stdin)
            end
        end

        os.execute("sleep 0.1")
    end
end

function M.is_complete(carpo, kernel_id_or_code, code_or_nil)
    local kernel_id, code

    if code_or_nil ~= nil then
        kernel_id = kernel_id_or_code
        code = code_or_nil
    else
        kernel_id = nil
        code = kernel_id_or_code
    end

    M.cat_header(nil, "=")
    print("Testing completeness")
    M.cat_header(nil, "=")
    print("```")
    print(code)
    print("```")

    if kernel_id then
        print(M.dump(carpo.is_complete(kernel_id, code)))
    else
        error("is_complete() requires a kernel_id parameter")
    end
end

function M.get_completions(carpo, kernel_id, code, cursor_pos)
    M.cat_header(nil, "=")
    print("Getting completions")
    M.cat_header(nil, "=")
    print("Cursor pos: " .. cursor_pos)
    print("```")
    print(code)
    print("```")

    if kernel_id then
        print(M.dump(carpo.get_completions(kernel_id, code, cursor_pos)))
    else
        error("get_completions() requires a kernel_id parameter")
    end
end

return M
