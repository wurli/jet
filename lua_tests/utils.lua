local M = {}

--------------------------------------------------
-- Get the jet library
--------------------------------------------------
M.jet_loader = function()
    return require("../lua/jet/rust")
end

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

local function big_header(action, name, context)
    name = name and ("in kernel: " .. name) or ""
    M.cat_header(nil, "=")
    print(action .. name)
    for k, v in pairs(context or {}) do
        print("*   " .. k .. ": " .. M.dump(v))
    end
    M.cat_header(nil, "=")
end

--- Execute code in the jet kernel and print results until the execution finishes
---@param jet Jet.Engine
function M.execute(jet, kernel_id, code, user_expressions, name)
    user_expressions = user_expressions or {}

    big_header(
        "Executing code",
        name,
        { ["User expressions"] = user_expressions, ["code"] = code }
    )

    local callback = jet.execute_code(kernel_id, code, user_expressions)

    local i = 0
    while true do
        local result = callback()
        if result.status == "idle" then
            break
        end
        if result.data then
            i = i + 1
            M.cat_header("Result " .. i)
            print(M.dump(result))
            if result.type == "input_request" then
                local stdin = "Hello from Lua!"
                M.cat_header(("Sending dummy val '%s'"):format(stdin), ".")
                jet.provide_stdin(kernel_id, stdin)
            end
        end
    end
end

function M.is_complete(jet, kernel_id, code, name)
    big_header("Testing completeness", name, { ["code"] = code })

    if kernel_id then
        print(M.dump(jet.is_complete(kernel_id, code)))
    else
        error("is_complete() requires a kernel_id parameter")
    end
end

function M.get_completions(jet, kernel_id, code, cursor_pos, name)
    big_header("Getting completeions", name, {
        code = code,
        name = name,
        cursor_pos = cursor_pos
    })
    print(M.dump(jet.get_completions(kernel_id, code, cursor_pos)))
end

function M.request_shutdown(jet, kernel_id, name)
    big_header("Requesting shutdown", name)
    print(M.dump(jet.request_shutdown(kernel_id)))
end

function M.request_restart(jet, kernel_id, name)
    big_header("Getting completeions", name)
    print(M.dump(jet.request_restart(kernel_id)))
end

function M.list_running_kernels(jet, name)
    big_header("Listing running kernels", name)
    print("Listing running kernels")
    M.cat_header(nil, "=")
    for id, kernel in pairs(jet.list_running_kernels()) do
        print(("* (%s) %s"):format(id:sub(1, 7), kernel.display_name))
    end
end

function M.discover_kernels(jet)
    big_header("Discovering available kernels")
    for id, kernel in pairs(jet.discover_kernels()) do
        print(("* (%s) %s"):format(id, kernel.display_name))
    end
end

return M
