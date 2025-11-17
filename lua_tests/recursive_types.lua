---@class bar
---@field foo foo
---@field type string
local bar = {}
bar.__index = bar

setmetatable(bar, {
    __call = function(self, ...)
        return self.init(...)
    end,
})

function bar.init(foo)
    local self = setmetatable({}, bar)
    self.foo = foo
    self.type = "bar"
    return self
end


---@class foo
---@field bar bar
---@field type string
local foo = {}
foo.__index = foo

setmetatable(foo, {
    __call = function(self, ...)
        return self.init(...)
    end,
})

function foo.init()
    local self = setmetatable({}, foo)
    self.bar = bar(self)
    self.type = "foo"
    return self
end

local f = foo()


print(f.bar.foo.type)



