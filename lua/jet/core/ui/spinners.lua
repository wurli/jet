local M = {}

local spinners = {
	arrows = { "←", "↖", "↑", "↗", "→", "↘", "↓", "↙" },
	blocks_h = { "▉", "▊", "▋", "▌", "▍", "▎", "▏", "▎", "▍", "▌", "▋", "▊", "▉" },
	blocks_v = { "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▁" },
	circles = { "◐", "◓", "◑", "◒" },
	circles2 = { "◴", "◷", "◶", "◵" },
	concentric = { "◉", "◎", "○", "◎" },
	corners = { "▖", "▘", "▝", "▗" },
	corners2 = { "◢", "◣", "◤", "◥" },
	corners3 = { "◰", "◳", "◲", "◱" },
	spin = { "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏" },
	spin2 = { "⢎⡰", "⢎⡡", "⢎⡑", "⢎⠱", "⠎⡱", "⢊⡱", "⢌⡱", "⢆⡱" },
}

---@alias Jet.Spinner.Preset
---|"arrows"
---|"blocks_h"
---|"blocks_v"
---|"circles"
---|"circles2"
---|"concentric"
---|"corners"
---|"corners2"
---|"corners3"
---|"spin"
---|"spin2"

---@alias Jet.Spinner Jet.Spinner.Preset | fun(): string[]

---Run a spinner animation
---
---@param on_tick fun(frame: string) A callback function that is called on each frame with
---@param on_complete fun() A callback function for when the spinner completes
---@param interval? number The interval in milliseconds between frames (default 100)
---@param spinner? Jet.Spinner A spinner name or a function returning an array of frames
---@return fun(): nil A function to stop the spinner
M.run = function(on_tick, on_complete, interval, spinner)
	interval = interval or 100
	local frames = type(spinner) == "function" and spinner() or spinners[spinner or "spin2"]
	local frame_count = #frames
	local current_frame = 0
	local timer = vim.uv.new_timer()

	timer:start(
		0,
		interval,
		vim.schedule_wrap(function()
			current_frame = (current_frame % frame_count) + 1
			on_tick(frames[current_frame])
		end)
	)

	return function()
		pcall(function()
			if on_complete then
				on_complete()
			end
			timer:stop()
			timer:close()
		end)
	end
end

return M
