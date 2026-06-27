-------------------------------------------------------------------------------
-- jet-lua smoke test: ark's 'lsp' comm. Open the comm with an ip_address,
-- then send a comm_info_request and assert the reply lists a comm whose
-- target_name is 'lsp'.
-------------------------------------------------------------------------------

local lib_ok, jet = pcall(require, "jet.core.engine")
if not lib_ok then
	jet = require("jet") --[[@as jet.engine]]
end

local script_dir = debug.getinfo(1, "S").source:sub(2):match("(.*/)") or "./"
package.path = script_dir .. "?.lua;" .. package.path
local utils = require("utils")

-- Start ark ------------------------------------------------------------------
local kernel = utils.start_kernel(jet, "/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json")

-- Open the 'lsp' comm. ark expects { ip_address = "<addr>" } — see
-- ark/crates/ark_test/src/dummy_frontend.rs:start_server. Drain to idle so
-- the open has been fully processed before we ask who's there.
local comm_id, messages = kernel.comm_open("lsp", { ip_address = "127.0.0.1" })
assert(type(comm_id) == "string" and #comm_id > 0, "expected comm_id from comm_open")

---@diagnostic disable-next-line: empty-block
for _ in messages do
	-- Drain to ensure we don't move on until the comm is really open
end

-- comm_info_request — filter by target_name to keep the assertion tight.
local found_lsp = false
for msg in kernel.comm_info("lsp") do
	if msg.type == "comm_info_reply" and msg.data and msg.data.comms then
		for _, info in pairs(msg.data.comms) do
			if info.target_name == "lsp" then
				found_lsp = true
			end
		end
	end
end
assert(found_lsp, "expected comm_info_reply to list a comm with target_name='lsp'")

kernel.stop()
