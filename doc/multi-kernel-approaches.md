# Multi-Kernel Support: Architectural Approaches

## Current Architecture Analysis

The current `carpo` implementation uses a single-kernel architecture with:

- **Global State**: `OnceLock` static variables (`KERNEL_INFO`, `EXECUTE_RX`, `STREAM_CHANNEL`, `SHELL`)
- **Single Frontend**: One `Frontend` instance managing 5 ZMQ sockets (shell, iopub, stdin, control, heartbeat)
- **Thread Model**:
  - Heartbeat thread for kernel health monitoring
  - IOPub thread for receiving execution results and streams
  - Main thread for synchronous code execution
- **Lua API**: Simple functions that operate on the singular kernel instance

## Proposed Approaches for Multi-Kernel Support

### Approach 1: Kernel Manager with HashMap Registry

**Overview**: Replace static globals with a centralized `KernelManager` that maintains a registry of active kernels.

**Architecture**:
```rust
pub struct KernelManager {
    kernels: Arc<RwLock<HashMap<KernelId, KernelInstance>>>,
    active_kernel: Arc<RwLock<Option<KernelId>>>,
}

pub struct KernelInstance {
    spec: KernelSpec,
    info: KernelInfoReply,
    frontend: Frontend,
    execute_rx: Receiver<Message>,
    stream_rx: Receiver<Message>,
}

pub type KernelId = String; // e.g., "python-1", "r-main", or UUID
```

**Implementation Details**:
- Use `Arc<RwLock<HashMap>>` for thread-safe kernel registry (single writer, multiple readers pattern)
- Each kernel gets a unique ID (UUID or user-defined name)
- Maintain an "active kernel" concept for default operations
- Kernel threads remain isolated per instance

**Lua API Changes**:
```lua
-- Start kernels with explicit IDs
carpo.start_kernel(spec_path, "python-main")
carpo.start_kernel(spec_path, "python-worker")

-- Execute on specific kernel
carpo.execute_code(code, "python-main")

-- Execute on active kernel (for convenience)
carpo.execute_code(code)  -- uses active kernel

-- Switch active kernel
carpo.set_active_kernel("python-worker")

-- List all running kernels
kernels = carpo.list_kernels()

-- Shutdown specific kernel
carpo.shutdown_kernel("python-worker")
```

**Pros**:
- Clean separation of kernel instances
- Easy to implement kernel lifecycle management
- Straightforward API with explicit kernel targeting
- Maintains backward compatibility (active kernel concept)

**Cons**:
- Need to refactor all static globals
- Requires passing manager instance or making it a singleton
- Some overhead from HashMap lookups

---

### Approach 2: Per-Language Kernel Pool

**Overview**: Organize kernels by language type, allowing multiple instances per language.

**Architecture**:
```rust
pub struct KernelPool {
    pools: Arc<RwLock<HashMap<String, LanguagePool>>>,
}

pub struct LanguagePool {
    language: String,
    spec: KernelSpec,
    instances: Vec<KernelInstance>,
    round_robin_idx: AtomicUsize,
}
```

**Implementation Details**:
- Group kernels by language (Python, R, Julia, etc.)
- Support load balancing within a language pool
- Automatic instance selection (round-robin, least-busy, etc.)
- Pool size management (min/max instances)

**Lua API Changes**:
```lua
-- Create pool with 3 Python kernel instances
carpo.create_pool("python", spec_path, {size = 3})

-- Execute on any available Python kernel
carpo.execute_code(code, {language = "python"})

-- Execute on specific instance
carpo.execute_code(code, {language = "python", instance = 2})

-- Scale pool
carpo.scale_pool("python", 5)
```

**Pros**:
- Excellent for parallel computation workloads
- Natural load balancing
- Efficient resource utilization
- Good for notebook servers with many users

**Cons**:
- More complex implementation
- State sharing between instances requires explicit handling
- May be overkill for simple use cases

---

### Approach 3: Actor-Based Model with Message Passing

**Overview**: Treat each kernel as an independent actor with message-based communication.

**Architecture**:
```rust
pub enum KernelCommand {
    Execute { code: String, reply_to: Sender<ExecuteResult> },
    Complete { code: String, pos: usize, reply_to: Sender<CompleteResult> },
    Shutdown,
}

pub struct KernelActor {
    id: KernelId,
    frontend: Frontend,
    command_rx: Receiver<KernelCommand>,
}

pub struct KernelHandle {
    id: KernelId,
    command_tx: Sender<KernelCommand>,
}
```

**Implementation Details**:
- Each kernel runs in its own thread with a command queue
- Use `crossbeam` channels for command/response communication
- Async-style API with callbacks or futures
- Complete isolation between kernels

**Lua API Changes**:
```lua
-- Start kernel and get handle
kernel = carpo.start_kernel(spec_path)

-- Execute with callback
kernel:execute_code(code, function(result)
    print(result)
end)

-- Or synchronous blocking
result = kernel:execute_code_sync(code)

-- Multiple kernels
py = carpo.start_kernel(python_spec)
r = carpo.start_kernel(r_spec)

py:execute_code("import numpy")
r:execute_code("library(dplyr)")
```

**Pros**:
- Strong isolation between kernels
- Natural concurrency model
- Easy to add features (timeouts, cancellation)
- Clean, message-oriented architecture

**Cons**:
- More complex error handling
- Requires more boilerplate
- Potential for deadlocks if not careful
- Callback-style can be harder to reason about in Lua

---

### Approach 4: Hybrid Registry + Context Pattern

**Overview**: Combine kernel registry with execution contexts for fine-grained control.

**Architecture**:
```rust
pub struct KernelRegistry {
    kernels: Arc<RwLock<HashMap<KernelId, Arc<Mutex<KernelInstance>>>>>,
}

pub struct ExecutionContext {
    kernel_id: KernelId,
    variables: HashMap<String, Value>,
    working_dir: PathBuf,
}
```

**Implementation Details**:
- Global registry for kernel discovery
- Each execution can specify context (kernel + environment)
- Support for kernel sessions/workspaces
- Context switching without kernel restart

**Lua API Changes**:
```lua
-- Create kernels with contexts
ctx1 = carpo.create_context("python-main", {
    kernel = python_spec,
    cwd = "/workspace/project1"
})

ctx2 = carpo.create_context("python-analysis", {
    kernel = python_spec,
    cwd = "/workspace/project2"
})

-- Execute in specific context
ctx1:execute("import pandas")
ctx2:execute("import matplotlib")

-- Switch between contexts
carpo.use_context(ctx1)
carpo.execute("df = pd.read_csv('data.csv')")

carpo.use_context(ctx2)
carpo.execute("plt.plot([1,2,3])")
```

**Pros**:
- Most flexible approach
- Supports complex workflows
- Can simulate "kernel restart" without actually restarting
- Great for notebook-style workflows

**Cons**:
- Most complex to implement
- Context management adds cognitive overhead
- Potential for state confusion

---

### Approach 5: Fork/Clone Pattern (Lightweight Kernels)

**Overview**: Start with one kernel and "clone" it for parallel execution.

**Architecture**:
```rust
pub struct KernelTree {
    root: KernelId,
    children: HashMap<KernelId, KernelInstance>,
    parent_map: HashMap<KernelId, KernelId>,
}
```

**Implementation Details**:
- Single primary kernel with state
- Spawn child kernels that inherit parent state
- Copy-on-write semantics for variables
- Useful for what-if analysis

**Lua API Changes**:
```lua
-- Start main kernel
main = carpo.start_kernel(spec_path)
main:execute("x = 10")

-- Fork kernel for experimentation
fork1 = main:fork("experiment-1")
fork1:execute("x = x + 5")  -- doesn't affect main

fork2 = main:fork("experiment-2")
fork2:execute("x = x * 2")  -- doesn't affect main or fork1

-- Get results
print(main:eval("x"))   -- 10
print(fork1:eval("x"))  -- 15
print(fork2:eval("x"))  -- 20
```

**Pros**:
- Intuitive for exploratory data analysis
- Efficient for parallel experimentation
- Natural way to handle "undo" scenarios
- Familiar from Git-like workflows

**Cons**:
- Not all kernels support state serialization
- Memory overhead for duplicated state
- Complex to implement kernel state snapshotting
- Limited by kernel's internal state management

---

## Recommendation Matrix

| Use Case                          | Best Approach        | Why                              |
| ----------                        | --------------       | -----                            |
| Simple multi-language support     | Approach 1 (Manager) | Clean, simple, explicit control  |
| High-performance parallel compute | Approach 2 (Pool)    | Built-in load balancing          |
| Complex async workflows           | Approach 3 (Actor)   | Natural concurrency primitives   |
| Notebook server / IDE             | Approach 4 (Hybrid)  | Flexible context management      |
| Exploratory data analysis         | Approach 5 (Fork)    | Natural experimentation workflow |

## Implementation Roadmap

### Phase 1: Minimal Multi-Kernel (Approach 1)
1. Create `KernelManager` struct
2. Replace static `OnceLock` with registry
3. Add kernel ID parameter to API functions
4. Implement `active_kernel` pattern for backward compatibility

### Phase 2: Enhanced Features
- Add kernel lifecycle events (starting, ready, busy, idle, dead)
- Implement kernel health monitoring
- Add kernel resource usage tracking
- Support kernel environment variables

### Phase 3: Advanced Patterns
- Evaluate need for Approach 2, 3, 4, or 5 based on user feedback
- Consider hybrid implementation combining multiple approaches
- Add kernel communication primitives (if needed)

## Technical Considerations

### Thread Safety
- All kernel registry access must be thread-safe
- Consider using `Arc<RwLock<>>` for read-heavy workloads
- Use `Arc<Mutex<>>` for simpler write-heavy scenarios
- Avoid deadlocks by establishing lock ordering

### Resource Management
- Implement proper kernel shutdown on drop
- Add timeout mechanisms for hanging kernels
- Monitor ZMQ socket health
- Consider connection pooling for efficiency

### Error Handling
- Kernel crashes should not affect other kernels
- Provide clear error messages with kernel ID context
- Implement automatic kernel restart policies (optional)
- Add circuit breaker pattern for failing kernels

### State Management
- Decide on shared vs isolated state model
- Consider serialization for kernel state snapshots
- Handle variable namespace collisions
- Support cross-kernel data sharing (if needed)

### Testing Strategy
- Unit tests for kernel registry operations
- Integration tests with multiple real kernels
- Stress tests for concurrent execution
- Race condition detection

## Future Enhancements

1. **Kernel Communication**: Allow kernels to send data to each other
2. **Distributed Kernels**: Support kernels on remote machines
3. **Kernel Clusters**: Orchestrate kernel pools across multiple nodes
4. **Language Bridges**: Enable Python-R interop through kernel proxying
5. **Kernel Snapshots**: Save/restore kernel state to disk
6. **Jupyter Compatibility**: Support standard Jupyter multi-kernel protocols
