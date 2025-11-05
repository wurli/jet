# Task: Implement Non-Blocking Execution via Neovim RPC

## Problem
Currently, code execution blocks Neovim:
- `lua/jet/kernel.lua` (lines 38-47): Calls `execute_code()` which returns a callback
- The callback must be polled in a while loop: `while true do callback() end`
- This blocks Neovim's main thread, preventing real-time output

## Solution
Change from **pull model** (Lua polls callback) to **push model** (Rust sends results to Neovim via RPC).

## Architecture Overview

### Current Flow
1. Lua calls `jet_engine.execute_code(id, code, {})` → returns callback
2. Lua polls callback in blocking loop
3. Callback retrieves messages from kernel

### New Flow
1. Lua calls `jet_engine.execute_code(id, code)` → returns immediately
2. Rust spawns async task to listen for kernel messages
3. Rust sends each message to Neovim via RPC as it arrives
4. Lua handler processes messages without blocking

## Implementation Details

### Rust Changes (`src/api.rs` and `src/api_lua.rs`)

**Current signature** (line 44-47 in `api.rs`):
```rust
pub fn execute_code(
    kernel_id: Id,
    code: String,
    user_expressions: HashMap<String, String>,
) -> anyhow::Result<impl Fn() -> Option<Message>>
```

**New signature needed**:
```rust
pub fn execute_code(
    kernel_id: Id,
    code: String,
    user_expressions: HashMap<String, String>,
    nvim_handler: String,  // Name of Lua function to call
) -> anyhow::Result<()>
```

**Implementation approach**:
- Spawn thread/task to handle kernel message loop (current callback logic from lines 63-122)
- For each message received, send RPC call to Neovim
- Use Neovim's msgpack-RPC protocol to invoke Lua handler
- Connection: Get Neovim address from `$NVIM` env var or passed from Lua

**Dependencies to add** (in `Cargo.toml`):
- `nvim-rs` or `neovim-lib` for Neovim RPC client
- OR implement minimal msgpack-RPC client using existing `serde_json` + a msgpack crate

### Lua Changes (`lua/jet/kernel.lua`)

**Current code** (lines 37-47):
```lua
function jet_kernel:execute(code)
    self:_handle_text_output("\n> " .. code .. "\n")
    local callback = jet_engine.execute_code(self.id, code, {})

    while true do
        local msg = callback()
        if vim.tbl_count(msg) > 0 then
            self:_handle_result(msg)
        else
            break
        end
    end
end
```

**New code needed**:
```lua
function jet_kernel:execute(code)
    self:_handle_text_output("\n> " .. code .. "\n")

    -- Register global handler for this kernel
    _G["jet_output_" .. self.id] = function(msg)
        vim.schedule(function()
            self:_handle_result(msg)
        end)
    end

    -- Execute asynchronously (returns immediately)
    jet_engine.execute_code(self.id, code, {}, "jet_output_" .. self.id)
end
```

## Key Files to Modify

1. **`src/api.rs`**:
   - Line 44-122: Replace callback return with async RPC push

2. **`src/api_lua.rs`**:
   - Line 20-41: Update `execute_code` wrapper to accept handler name

3. **`lua/jet/kernel.lua`**:
   - Line 31-47: Replace polling loop with async handler registration

4. **`Cargo.toml`**:
   - Add Neovim RPC client dependency

## Testing
Run integration tests with `RUST_LOG=trace luajit lua_tests/test_*.lua` to verify non-blocking execution.

## Notes
- `_handle_result()` (line 117-125) already processes all message types correctly
- Use `vim.schedule()` in Lua handler to ensure UI updates happen on main thread
- Consider cleanup: unregister global handlers when kernel shuts down
