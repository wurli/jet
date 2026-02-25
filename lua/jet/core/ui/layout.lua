-- Some may argue this is too much abstraction.
--
-- Please stop saying that, I'm trying my best here.
--
-- This does some fancy stuff:
-- - If a window in a layout is closed, they are all closed
-- - If a window in a layout is opened, they are all opened (in the correct positions)
-- - If a window in a layout is moved, they are all moved

---@class Jet.Ui.Layout
---@field windows Jet.Ui.Win[]
local layout = {}
layout.__index = layout

setmetatable(layout, {
	---@return Jet.Ui.Layout
	__call = function(self, ...)
		return self.new(...)
	end,
})

---@param opts { windows: Jet.Ui.Win[] }
function layout.new(opts)
	local self = setmetatable({}, layout)
	self.windows = opts.windows

	for _, win in ipairs(self.windows) do
		win:autocmd("WinClosed", {
			callback = function()
				self:hide()
			end,
		})

		win:autocmd("BufWinLeave", {
			callback = function()
				self:hide()
			end,
		})

		win:autocmd("BufWinEnter", {
			callback = function()
				self:show()
			end,
		})

		-- Needs to be a general autocmd??
		-- win:autocmd("WinResized", {
		-- 	callback = function()
		-- 		self:hide()
		-- 	end,
		-- })
	end

	return self
end

function layout:hide()
	for _, win in pairs(self.windows) do
		win:hide()
	end
end

function layout:show()
	if not self.windows[1]:is_visible() then
		for _, win in ipairs(self.windows) do
			if win:is_visible() then
				local primary_win = self.windows[1]
				vim.api.win_set_buf(win.win, primary_win.buf)
				primary_win.win = win.win
				primary_win:show()
			end
		end
	end
end
