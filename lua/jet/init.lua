local manager = require("jet.core.manager")

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
-- :Jet mode <repl|notebook|both> Set the current buffer's interaction mode.
--
-- :Jet start <kernel>?           Start a new kernel
--
-- :Jet stop <kernel>?            Stop a running kernel
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

---@class Jet.Manager.Filter
---
---A buffer number; 0 for the current buffer. Note: this filters for (a) the
---linked kernel for the buffer if it exists, and if not, (b) the primary
---kernel for the buffer's filetype.
---@field buf? number
---
---Case-insensitive Lua pattern; matched against the kernel spec path
---@field spec_path? string
---
---Case-insensitive language name (not a pattern); matched against the language
---as given in the kernel spec
---@field language? string
---
---Case-insensitive pattern; matched against the kernel display name
---@field name? string
---
---The ID of an existing kernel instance to get
---@field id? string
---
---Active status
---@field status? "active" | "inactive"

---By default opens the kernel for the current buffer.
---@param opts Jet.Manager.Filter
function Jet.open(opts)
	manager:open_kernel(opts or { buf = 0 })
end

function Jet.send()
    local pos = vim.fn.getpos(".")
	manager:get_kernel(function(_, id)
		if id then
            -- Restore the cursor position after getting the kernel (e.g.
            -- in case the user had to enter a dialog to choose a kernel)
            -- so  the kernel can resolve the code to send.
            vim.fn.setpos(".", pos)
			manager.running[id]:send_from_buf()
		end
	end, { buf = 0 })
end

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

Jet.setup = function(_) end

return Jet
