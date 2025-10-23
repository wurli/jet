local function get_lib_extension()
    if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then return '.dylib' end
    if jit.os:lower() == 'windows' then return '.dll' end
    return '.so'
end

local base_path = vim.fn.simplify(debug.getinfo(1).source:match('@?(.*/)') .. '../../target/release/')
local lib_name = 'carpo'
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

return loader()
