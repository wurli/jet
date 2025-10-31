# Multi-Kernel Support

This branch implements support for managing multiple Jupyter kernels simultaneously.

## Changes

### Architecture

The main architectural change is replacing global singleton state with a `KernelManager` that maintains a collection of `KernelState` instances, one per kernel.

**Key components:**

1. **`KernelManager`** (`src/supervisor/manager.rs`)
   - Central registry for all active kernels
   - Maps `KernelId` (UUID string) to `KernelState`
   - Thread-safe using `RwLock<HashMap<KernelId, Arc<KernelState>>>`

2. **`KernelState`** (`src/supervisor/manager.rs`)
   - Encapsulates all state for a single kernel:
     - Kernel info (spec, capabilities)
     - ZMQ connections (shell, stdin)
     - Message brokers (iopub, shell, stdin)

3. **`Frontend`** (`src/supervisor/frontend.rs`)
   - Updated to work with kernel IDs
   - Methods now accept `kernel_id: &KernelId` parameter
   - Single static `KERNEL_MANAGER` instead of multiple globals

### API Changes

All API functions now require a `kernel_id` parameter:

**Rust API** (`src/api.rs`):
- `start_kernel(spec_path: String) -> anyhow::Result<KernelId>` - Returns kernel ID
- `list_kernels() -> Vec<KernelId>` - New function to list active kernels
- `execute_code(kernel_id: KernelId, code: String, ...) -> ...`
- `get_completions(kernel_id: KernelId, code: String, cursor_pos: u32) -> ...`
- `is_complete(kernel_id: KernelId, code: String) -> ...`
- `provide_stdin(kernel_id: KernelId, value: String) -> ...`

**Lua API** (`src/api_lua.rs`, `src/lib.rs`):
```lua
-- Start a kernel (returns kernel ID)
local kernel_id = carpo.start_kernel("/path/to/kernel.json")

-- List all running kernels
local kernels = carpo.list_kernels()

-- Execute code in a specific kernel
local callback = carpo.execute_code(kernel_id, "print('hello')", {})

-- Other operations require kernel ID
carpo.is_complete(kernel_id, "1 + ")
carpo.get_completions(kernel_id, "import ", 7)
carpo.provide_stdin(kernel_id, "user input")
```

### Implementation Details

**Benefits:**
- ✅ Multiple kernels can run concurrently
- ✅ Each kernel has isolated state and message routing
- ✅ Clean separation of concerns
- ✅ Type-safe kernel identification via UUIDs
- ✅ Connection files are unique per kernel

**Isolation:**
- Each kernel gets its own:
  - ZMQ connection file (named with kernel ID)
  - Shell/stdin sockets with independent locks
  - Message brokers with separate request tracking
  - IOPub listener thread

**Thread Safety:**
- `KernelManager` uses `RwLock` for concurrent access
- Individual kernel connections use `Mutex`
- Message brokers use `Arc` for shared ownership across threads

## Testing

See `lua_tests/test_multi_kernel.lua` for an example that:
1. Discovers available kernels
2. Starts Python and R kernels simultaneously
3. Executes code in both kernels
4. Verifies that kernel state is isolated

## Migration Guide

### For Lua Users

**Before:**
```lua
local banner = carpo.start_kernel("/path/to/kernel.json")
local callback = carpo.execute_code("1 + 1", {})
```

**After:**
```lua
local kernel_id = carpo.start_kernel("/path/to/kernel.json")
local callback = carpo.execute_code(kernel_id, "1 + 1", {})
```

### For Rust Users

All API functions in `carpo::api` now require a `KernelId` (which is just a `String` type alias):

```rust
let kernel_id = carpo::api::start_kernel(spec_path)?;
let result = carpo::api::execute_code(kernel_id, code, user_expressions);
```

## Future Enhancements

Potential improvements:
- Add `shutdown_kernel(kernel_id)` to cleanly stop individual kernels
- Add `get_kernel_info(kernel_id)` to query kernel metadata
- Resource limits per kernel (memory, CPU)
- Kernel lifecycle events/callbacks
- Auto-cleanup of stale/crashed kernels
