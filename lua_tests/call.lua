--------------------------------------------------
-- Get the carpo library
--------------------------------------------------
local libname = "carpo"

local function get_lib_extension()
    if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then return '.dylib' end
    if jit.os:lower() == 'windows' then return '.dll' end
    return '.so'
end

local carpo_loader = package.loadlib(
    "./target/release/lib" .. libname .. get_lib_extension(),
    "luaopen_" .. libname
)

--------------------------------------------------
-- Helpers
--------------------------------------------------

--- Get the number of keys in a table
local tbl_len = function(t)
    local count = 0
    for _ in pairs(t) do count = count + 1 end
    return count
end

--- Dump a Lua object as a string
local function dump(o, level)
    level = level or 4
    local indent = (" "):rep(level)
    local prev_indent = (" "):rep(level - 4)
    if type(o) == "table" and tbl_len(o) > 0 then
        local s = "{\n"
        for k, v in pairs(o) do
            if type(k) ~= "number" then k = '"' .. k .. '"' end
            s = s .. indent .. "[" .. k .. "] = " .. dump(v, level + 4) .. ",\n"
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

---@param x string
local cat_header = function(x, pad)
    local out_len = 80
    pad = pad or "-"
    x = x and " " .. x .. " " or ""
    local x_len = x:len()
    print(pad:rep(2) .. x .. pad:rep(math.max(out_len - 2 - x_len, 0)))
end

--- Execute code in the carpo kernel and print results until the execution finishes
local function execute(carpo, code)
    cat_header("Executing code", "=")
    print("```")
    print(code)
    print("```")
    local callback = carpo.execute_code(code)
    local i = 0
    while true do
        i = i + 1
        cat_header("Result " .. i)
        local result = callback()
        print(dump(result))
        if result.is_complete then break end

        if result.type == "input_request" then
            local stdin = "Hello from Lua!"
            cat_header(("Sending dummy val '%s'"):format(stdin), "*")
            carpo.provide_stdin(stdin)
        end

        os.execute("sleep 0.1")
    end
end

--------------------------------------------------
-- Start the kernel
--------------------------------------------------
local carpo = carpo_loader()

local startup_message = carpo.start_kernel("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- Print the startup message
cat_header("startup message", "=")
print(startup_message)

-- Try running some code
execute(carpo, "1 + 1")
execute(carpo, "readline('Enter something: ')")
