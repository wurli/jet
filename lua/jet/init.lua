local manager = require("jet.core.manager")
local utils = require("jet.core.utils")

local Jet = {}

-- ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
--                API
-- ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
--
-- Main commands:
--
-- :Jet open <kernel>? <mode>     Start a new kernel or open an existing one.
--                                If currently in a notebook, this should open
--                                the REPL for the linked kernel.
--
-- :Jet repl <start|stop>         Start/stop a Jet REPL
--
-- :Jet notebook <start|stop>     Enable Jet for the current notebook
--
-- :Jet restart <kernel>?         Restart a running kernel
--
-- :Jet link <kernel>? <file>     Link a kernel to a file
--
-- :Jet unlink <file>             Unlink a kernel from a file. In notebook mode
--                                this should close any code cells and
--                                associate the buffer with a REPL.
--
-- All the above should use vim.ui.select if no kernel is provided. Can also
-- pass parameters as in Lua api, e.g. like trouble.nvim.

-- ---By default opens the kernel for the current buffer.
-- ---@param opts Jet.Manager.Filter
-- function Jet.open(opts)
-- 	manager:open_kernel(opts or { buf = 0 })
-- end

function Jet.repl()
	manager:open_repl()
end

function Jet.notebook()
	manager:open_notebook()
end

---@param opts Jet.Send.Opts?
function Jet.send_chunk(opts)
	opts = opts or {}

	local chunk = require("jet.core.execute").get_chunk()
	if not chunk then
		return
	end

	local kernel_criteria = opts.kernel
		or {
			status = "active",
			filetype = utils.get_cur_filetype(),
			buffer = 0,
		}

	manager:get_kernel(kernel_criteria, function(_, id)
		if id then
			local kernel = manager.running[id]
			if chunk and kernel.ui and kernel.ui.execute_chunk then
				kernel.ui:execute_chunk(chunk)
			end
		end
	end)
end

---@class Jet.Send.Opts
---
---Selection criteria used to choose the target kernel. By default will look
---for an active kernel for the filetype at the cursor position.
---@field kernel Jet.Manager.Filter?
---
---If `true`, don't show output in the UI.
---@field silent boolean?
---
---Will be passed any output from the kernel. Note that this may be called
---multiple times!
---@field callback fun()?
---
---Will be called once execution is complete.
---@field on_complete fun()?

---Execute code from the cursor position
---
---@param opts? Jet.Send.Opts
function Jet.send_from_cursor(opts)
	Jet.send_code(require("jet.core.execute").get_code_auto(), opts)
end

---Execute code from a motion
---
---``` lua
----- You can set an op-leading keymap like so:
---vim.keymap.set({ "n", "v" }, "gj", require("jet").send_motion(), { expr = true })
---```
---
---@param opts? Jet.Send.Opts
---@return fun(): "g@" # A function that can be used in an operator-pending mapping
function Jet.send_from_motion(opts)
	return require("jet.core.execute").handle_motion(function(code)
		Jet.send_code(code, opts)
	end)
end

---Execute code from Lua
---
---@param code string | string[]
---@param opts? Jet.Send.Opts
function Jet.send_code(code, opts)
	if type(code) == "string" then
		code = vim.split(code, "\n")
	end

	opts = opts or {}

	local kernel_criteria = opts.kernel or {
		status = "active",
		filetype = utils.get_cur_filetype(),
	}

	manager:get_kernel(kernel_criteria, function(_, id)
		if id then
			local kernel = manager.running[id]
			if opts.silent then
				kernel:execute(code, opts.callback, opts.on_complete)
			elseif kernel.ui and kernel.ui.execute_code then
				kernel.ui:execute_code(code, opts.callback, opts.on_complete)
			end
		end
	end)
end

-- How to design multiple UI options - e.g. repls and notebooks?
--
-- Execute is done abstractly. Some keymap, either in a repl or a notebook,
-- just sends an execute request to a kernel.
--
-- The kernel then has an active UI, which can either be a notebook or a repl,
-- to which it sends results.

-- ----------------------------------------------------------------------------
-- CONCEPTS:
--
-- LINKING
--   - A kernel may be linked with one or more buffers.
--   - Links are created manually
--   - Link information is stored in the kernel object (not buffer data)
--   - If linked, that kernel opens when you try to run code from the buffer
--   - Use `:Jet link` to create the link
--   - Use `:Jet unlink` to remove the link
--
-- PRIMARY KERNEL
-- - If there are multiple kernels running for the same filetype, only one may
--   be the 'primary' kernel, used to run code by default.
--
-- ----------------------------------------------------------------------------
-- IMPLEMENTATION:
--
-- Kernels should have the following extra fields:
-- - linked_buffers: number[]
--
-- Buffers should have the following data in `vim.b`:
-- - jet.mode: "notebook" | "repl" (if not present fall back to ft default or
--   "repl")
--
-- ----------------------------------------------------------------------------
-- PRIMARY REQUIREMENT:
-- - Run code with a simple <enter>
-- - The user should decide what to do if the repl is closed:
--    1. Don't execute
--    2. Open and execute
--    3. Execute but don't open
--
-- SECONDARY REQUIREMENTS:
-- - Associate a running kernel with a buffer/filetype. Should be (in order of
--   priority):
--   1. The kernel 'linked' to that buffer (if any)
--   3. The last used kernel associated with that filetype
--
-- IMPLEMENTATION NOTES:
-- - `jet.link(file, kernel_id)` to perform linking
-- - Store the link in the kernel rather than the buffer so that we can easily
--   find which kernels _not_ to use for other buffers.
--
-- ----------------------------------------------------------------------------
-- PRIMARY REQUIREMENT:
-- - Easily run code from different buffers *of the same filetype* with
--   separate dedicated kernel instances.
--
-- SECONDARY REQUIREMENTS:
-- - Manually link a kernel with a buffer.
-- - Once linked it will not be used with other buffers.
-- - But you can manually link additional buffers to it.
--
-- ----------------------------------------------------------------------------
-- PRIMARY REQUIREMENT:
-- - Isolate environments for notebooks
--
-- SECONDARY REQUIREMENTS:
-- - Automatically link buffers with some filetypes, e.g. markdown, to
--   dedicated kernels
-- - Although the user should be able to configure this somehow
--
-- IMPLEMENTATION NOTES:
-- - Config option `jet.isolate_filetypes = { "markdown", "quarto", "rmarkdown" }`
--
-- ----------------------------------------------------------------------------
-- PRIMARY REQUIREMENT:
-- - Choose between REPL and notebook style interaction for notebooks
--
-- SECONDARY REQUIREMENTS:
--
-- IMPLEMENTATION NOTES:
-- - Maybe best to always have a REPL buffer, even if hidden. Then choosing REPL
--   style just involves unhiding the REPL and not showing code cells.
--

local jet_augroup = vim.api.nvim_create_augroup("Jet", { clear = true })

Jet.setup = function(_)
	require("jet.core.ui.highlights").set()

	vim.api.nvim_create_autocmd("BufWinEnter", {
		pattern = "*",
		group = jet_augroup,
		callback = function(args)
			if vim.b[args.buf].jet and vim.b[args.buf].jet.type == "notebook" then
				for _, kernel in pairs(manager.map_kernel_buffer[args.buf]) do
					if kernel.ui and kernel.ui then
						kernel.ui:show()
					end
				end
			end
		end,
	})

	vim.api.nvim_create_autocmd("BufWinLeave", {
		pattern = "*",
		group = jet_augroup,
		callback = function(args)
			if vim.b[args.buf].jet and vim.b[args.buf].jet.type == "notebook" then
				for _, kernel in pairs(manager.map_kernel_buffer[args.buf]) do
					if kernel.ui and kernel.ui then
						kernel.ui:hide()
					end
				end
			end
		end,
	})

	vim.api.nvim_create_user_command("Jet", function(x)
		local args = x.fargs

		if args[1] == "notebook" then
			Jet.notebook()
			return
		end

		if args[1] == "repl" then
			Jet.repl()
			return
		end

		error(("Unsupported option '%s'"):format(args[1]))
	end, {
		nargs = "*",
		---@diagnostic disable-next-line: unused-local
		complete = function(prefix, line, col)
			local args = vim.split(line, " ", { trimempty = true })
			if args[1] ~= "Jet" then
				return {}
			end

			if #args == 1 then
				return {
					"notebook",
					"repl",
				}
			end

			if args[2] == "notebook" or args[2] == "repl" then
				return { "start", "stop" }
			end

			return {}
		end,
		desc = "Jupyter kernel management",
	})
end

return Jet
