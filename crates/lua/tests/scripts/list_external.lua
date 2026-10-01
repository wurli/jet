-------------------------------------------------------------------------------
-- jet-lua smoke test: `list_external`. Calls without a connection_file arg so
-- it scans $JUPYTER_RUNTIME_DIR (may be empty). Asserts that:
--   * the poll closure resolves to {status="ready", value=<table>}
--   * each entry in the array has the required fields
-------------------------------------------------------------------------------

local dbg = debug.getinfo(1, "S")
assert(dbg, "failed to determine script dir")
local script_dir = dbg.source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

local poll = utils.jet.list_external()

-- Drive the poll until ready (no kernel needed — just needs to resolve).
local result
local deadline = os.clock() + 10
while true do
    assert(os.clock() < deadline, "list_external poll timed out after 10s")
    local res = poll()
    assert(res.status ~= "done", "poll ended before producing a ready frame")
    if res.status == "ready" then
        result = res.value
        break
    end
end

assert(type(result) == "table", "expected result to be a table, got " .. type(result))

-- Validate each report's required fields.
for i, r in ipairs(result) do
    assert(type(r.connection_file_path) == "string" and #r.connection_file_path > 0,
        string.format("report[%d].connection_file_path must be a non-empty string", i))
    assert(type(r.alive) == "boolean",
        string.format("report[%d].alive must be a boolean", i))
    if r.error ~= nil then
        assert(type(r.error) == "string",
            string.format("report[%d].error must be a string when present", i))
    end
    if r.kernel_info ~= nil then
        assert(type(r.kernel_info) == "table",
            string.format("report[%d].kernel_info must be a table when present", i))
    end
end
