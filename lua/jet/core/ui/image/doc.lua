---@class Jet.Ui.Image.doc
local M = {}

---@alias Jet.Ui.TSMatch {node:TSNode, meta:vim.treesitter.query.TSMetadata}
---@alias Jet.Ui.Image.transform fun(match: Jet.Ui.Image.match, ctx: Jet.Ui.Image.ctx)
---@alias Jet.Ui.Image.find fun(matches: Jet.Ui.Image.match[])

---@class Jet.Ui.Image.ctx
---@field buf number
---@field lang string
---@field meta vim.treesitter.query.TSMetadata
---@field pos? Jet.Ui.TSMatch
---@field src? Jet.Ui.TSMatch
---@field content? Jet.Ui.TSMatch

---@class Jet.Ui.Image.match
---@field id string
---@field pos Jet.Ui.Image.Pos
---@field src? string
---@field content? string
---@field content_id? string
---@field ext? string
---@field range? Range4
---@field lang string
---@field type Jet.Ui.Image.Type

local META_EXT = "image.ext"
local META_SRC = "image.src"
local META_TYPE = "image.type"
local META_IGNORE = "image.ignore"
local META_LANG = "image.lang"

---@type table<string, Jet.Ui.Image.transform>
M.transforms = {
	norg = function(img, ctx)
		local row, col = ctx.src.node:start()
		local line = vim.api.nvim_buf_get_lines(ctx.buf, row, row + 1, false)[1]
		img.src = line:sub(col + 1)
	end,
	data_img = function(img, ctx)
		if not vim.base64 then
			return
		end
		if not img.src then
			return
		end
		local ft, data = img.src:match("^data:(.-);base64,(.+)$")
		if not (ft and data) then
			return
		end
		img.content = vim.base64.decode(data)
		img.content_id = data:sub(1, 20)
		img.src = nil
		img.ext = ft:match("^image/(%w+)$") or "png"
	end,
}

local uv = vim.uv or vim.loop
local dir_cache = {} ---@type table<string, boolean>
local buf_cache = {} ---@type table<number,{tick: number, [string]:any}>

---@param buf number
---@param key string
---@param fn fun():any
function M._cache(buf, key, fn)
	if buf_cache[buf] and buf_cache[buf].tick ~= vim.api.nvim_buf_get_changedtick(buf) then
		buf_cache[buf] = nil
	end
	buf_cache[buf] = buf_cache[buf] or { tick = vim.api.nvim_buf_get_changedtick(buf) }
	if buf_cache[buf][key] == nil then
		buf_cache[buf][key] = fn()
	end
	return buf_cache[buf][key]
end

---@param buf number
function M.get_packages(buf)
	if vim.bo[buf].filetype ~= "tex" then
		return {}
	end
	return M._cache(buf, "packages", function()
		local ret = {} ---@type string[]
		for _, line in ipairs(vim.api.nvim_buf_get_lines(buf, 0, -1, false)) do
			line = line:match("(.-)%%") or line
			if line:find("\\usepackage", 1, true) then
				for _, p in ipairs(vim.split(line:match("\\usepackage.-{(.-)}") or "", ",%s*")) do
					if not vim.tbl_contains(ret, p) then
						ret[#ret + 1] = p
					end
				end
			elseif line:find("\\begin{document}", 1, true) then
				break
			end
		end
		return ret
	end)
end

---@param buf number
function M.get_header(buf)
	return M._cache(buf, "header", function()
		local header = {} ---@type string[]
		local in_header = false
		for _, line in ipairs(vim.api.nvim_buf_get_lines(buf, 0, -1, false)) do
			if line:find("snacks:%s*header%s*start") then
				in_header = true
			elseif line:find("snacks:%s*header%s*end") then
				in_header = false
			elseif in_header then
				header[#header + 1] = line
			end
		end
		return table.concat(header, "\n")
	end)
end

---@param str string
function M.url_decode(str)
	return str:gsub("+", " "):gsub("%%(%x%x)", function(hex)
		return string.char(tonumber(hex, 16))
	end)
end

---@param dir string
function M.is_dir(dir)
	if dir_cache[dir] == nil then
		dir_cache[dir] = vim.fn.isdirectory(dir) == 1
	end
	return dir_cache[dir]
end

---@param buf number
---@param src string
function M.resolve(buf, src)
	src = M.url_decode(src)
	local file = svim.fs.normalize(vim.api.nvim_buf_get_name(buf))
	local s = require("jet.core.ui.image.config").resolve and require("jet.core.ui.image.config").resolve(file, src) or nil
	if s then
		return s
	end
	if not src:find("^%w%w+://") then
		local cwd = uv.cwd() or "."
		local checks = { [src] = true }
		for _, root in ipairs({ cwd, vim.fs.dirname(file) }) do
			checks[root .. "/" .. src] = true
			for _, dir in ipairs(require("jet.core.ui.image.config").img_dirs) do
				dir = root .. "/" .. dir
				if M.is_dir(dir) then
					checks[dir .. "/" .. src] = true
				end
			end
		end
		for f in pairs(checks) do
			if vim.fn.filereadable(f) == 1 then
				src = uv.fs_realpath(f) or f
				break
			end
		end
		src = svim.fs.normalize(src)
	end
	return src
end

---@param buf number
---@param cb Jet.Ui.Image.find
function M.find_visible(buf, cb)
	local ret = {} ---@type table<string,Jet.Ui.Image.match>
	local wins = vim.fn.win_findbuf(buf)
	local count = #wins
	for _, win in ipairs(wins) do
		local info = vim.fn.getwininfo(win)[1]
		M.find(buf, function(mathes)
			for _, i in ipairs(mathes) do
				ret[i.id] = i
			end
			count = count - 1
			if count == 0 and cb then
				cb(vim.tbl_values(ret))
			end
		end, { from = math.max(info.topline - 1, 1), to = info.botline })
	end
end

---@param buf number
---@param cb Jet.Ui.Image.find
---@param opts? {from?: number, to?: number}
function M.find(buf, cb, opts)
	local ok, parser = pcall(vim.treesitter.get_parser, buf)
	if not ok or not parser then
		return cb({})
	end
	opts = opts or {}
	local from, to = opts.from, opts.to
	Snacks.util.parse(parser, from and to and { from, to } or true, function()
		local ret = {} ---@type Jet.Ui.Image.match[]
		parser:for_each_tree(function(tstree, tree)
			if not tstree then
				return
			end
			local query = vim.treesitter.query.get(tree:lang(), "images")
			if not query then
				return
			end
			for _, match, meta in query:iter_matches(tstree:root(), buf, from and from - 1 or nil, to) do
				if not meta[META_IGNORE] then
					---@type Jet.Ui.Image.ctx
					local ctx = {
						buf = buf,
						lang = tostring(meta[META_LANG] or meta["injection.language"] or tree:lang()),
						meta = meta,
					}
					for id, nodes in pairs(match) do
						nodes = type(nodes) == "userdata" and { nodes } or nodes
						local name = query.captures[id]
						local field = name == "image" and "pos" or name:match("^image%.(.*)$")
						if field then
							---@diagnostic disable-next-line: assign-type-mismatch
							ctx[field] = { node = nodes[1], meta = meta[id] or {} }
						end
					end
					ret[#ret + 1] = M._img(ctx)
				end
			end
		end)
		cb(ret)
	end)
end

---@param ctx Jet.Ui.Image.ctx
function M._img(ctx)
	ctx.pos = ctx.pos or ctx.src or ctx.content
	assert(ctx.pos, "no image node")

	local range6 = vim.treesitter.get_range(ctx.pos.node, ctx.buf, ctx.pos.meta)
	local range = { range6[1], range6[2], range6[4], range6[5] } ---@type Range4
	if range[3] > 0 and range[4] == 0 then
		range[3] = range[3] - 1
		local line = vim.api.nvim_buf_get_lines(ctx.buf, range[3], range[3] + 1, false)[1]
		range[4] = #line
	end
	---@type Jet.Ui.Image.match
	local img = {
		ext = ctx.meta[META_EXT],
		src = ctx.meta[META_SRC],
		lang = ctx.lang,
		id = ctx.pos.node:id(),
		range = { range[1] + 1, range[2], range[3] + 1, range[4] },
		pos = { range[1] + 1, range[2] },
		type = "image",
	}
	if ctx.meta[META_TYPE] then
		img.type = ctx.meta[META_TYPE]
	elseif img.ext then
		img.type = img.ext:match("^(%w+)%.") or img.type
	end
	if not require("jet.core.ui.image.config").math.enabled and img.type == "math" then
		return
	end
	if ctx.src then
		img.src = vim.treesitter.get_node_text(ctx.src.node, ctx.buf, { metadata = ctx.src.meta })
	end
	if ctx.content then
		img.content = vim.treesitter.get_node_text(ctx.content.node, ctx.buf, { metadata = ctx.content.meta })
	end
	assert(img.src or img.content, "no image src or content")

	local transform = M.transforms[ctx.lang]
	if img.src and img.src:find("^data:%w+/%w+;base64,") then
		transform = M.transforms["data_img"]
	end
	if transform then
		transform(img, ctx)
	end
	if img.src then
		img.src = M.resolve(ctx.buf, img.src)
	end
	if img.content and not img.src then
		local root = require("jet.core.ui.image.config").cache
		vim.fn.mkdir(root, "p")
		img.src = root
			.. "/"
			.. (img.content_id or vim.fn.sha256(img.content):sub(1, 8))
			.. "-content."
			.. (img.ext or "png")
		if vim.fn.filereadable(img.src) == 0 then
			local fd = assert(io.open(img.src, "w"), "failed to open " .. img.src)
			fd:write(img.content)
			fd:close()
		end
	end
	return img
end

--- Get the image at the cursor (if any)
---@param cb fun(image_src?:string, image_pos?: Jet.Ui.Image.Pos)
function M.at_cursor(cb)
	local cursor = vim.api.nvim_win_get_cursor(0)
	M.find(vim.api.nvim_get_current_buf(), function(imgs)
		for _, img in ipairs(imgs) do
			local range = img.range
			if range then
				if
					(range[1] == range[3] and cursor[2] >= range[2] and cursor[2] <= range[4])
					or (range[1] ~= range[3] and cursor[1] >= range[1] and cursor[1] <= range[3])
				then
					return cb(img.src, img.pos)
				end
			end
		end
		cb()
	end, { from = cursor[1], to = cursor[1] + 1 })
end

---@param buf number
function M._attach(buf)
	if not vim.api.nvim_buf_is_valid(buf) then
		return
	end
	if vim.b[buf].snacks_image_attached then
		return
	end
	vim.b[buf].snacks_image_attached = true
	local inline = require("jet.core.ui.image.config").doc.inline and require("jet.core.ui.image.terminal").env().placeholders
	local float = require("jet.core.ui.image.config").doc.float and not inline

	if not inline and not float then
		return
	end

	require("jet.core.ui.image.inline").new(buf)
end

---@param buf number
function M.attach(buf)
	if require("jet.core.ui.image.config").enabled == false then
		return
	end
	local Terminal = require("snacks.image.terminal")
	Terminal.detect(function()
		M._attach(buf)
	end)
end

return M
