# Carpo Documentation

This directory contains comprehensive documentation for the Carpo Jupyter kernel client.

## IOPub Message Broker

The IOPub message broker is the core component for handling incoming messages from Jupyter kernels.

### Documentation Files

1. **[implementation-summary.md](implementation-summary.md)** - Start here!
   - Overview of what was implemented
   - List of files created and modified
   - Key features and benefits
   - Usage examples

2. **[iopub-broker.md](iopub-broker.md)** - Detailed technical guide
   - Architecture and core components
   - Usage patterns and code examples
   - Design decisions and rationale
   - Performance considerations
   - Troubleshooting guide

3. **[visual-guide.md](visual-guide.md)** - Visual diagrams and examples
   - System architecture diagram
   - Message flow timeline
   - Routing decision tree
   - Concurrent execution example
   - Memory layout analysis

### Example Code

See **[examples/broker_patterns.rs](../examples/broker_patterns.rs)** for:
- Basic code execution
- Global monitoring
- Custom configuration
- Stream handling
- Statistics monitoring
- RAII cleanup pattern
- Concurrent executions

## Quick Start

```rust
use carpo::msg::broker::{IopubBroker, ExecutionResult};
use std::sync::Arc;

// 1. Create and start broker
let broker = Arc::new(IopubBroker::new());
iopub_thread::start_iopub_thread(iopub, Arc::clone(&broker));

// 2. Execute code with message correlation
let (channels, collector) = ExecutionResult::create_channels();
let request_id = shell.send_execute_request(&code, options);
broker.register_request(request_id.clone(), channels);

// 3. Collect results
// ... use collector.status_rx, execution_rx, etc.

// 4. Cleanup
broker.unregister_request(&request_id);
```

## Key Concepts

### Message Correlation
Messages are routed to the correct request using the parent header's `msg_id`. This ensures that even with multiple concurrent requests, each one receives only its own messages.

### Typed Channels
Different message types go to different channels:
- `status_tx`: Kernel status (Busy, Idle)
- `execution_tx`: Results and errors
- `stream_tx`: stdout/stderr
- `display_tx`: Plots and images
- `comm_tx`: Widget messages

### Orphan Buffer
Messages without a matching request are temporarily buffered. This handles:
- Messages that arrive before registration
- Global events (Welcome, Starting)
- Timing issues and edge cases

### Automatic Cleanup
The broker automatically:
- Removes stale requests (default: 5 minutes)
- Cleans up old orphan messages (default: 60 seconds)
- Runs cleanup every 30 seconds (configurable)

## Benefits

✅ **Correct message routing** - Parent header correlation ensures accuracy

✅ **Concurrent execution** - Multiple requests can run simultaneously

✅ **Type safety** - Separate channels for different message types

✅ **Memory safety** - Automatic cleanup prevents leaks

✅ **Debuggability** - Global subscribers and orphan buffer aid debugging

✅ **Performance** - Lock-free message passing, O(1) routing

## Related Code

- **Core**: `src/msg/broker.rs`
- **Thread**: `src/frontend/iopub_thread.rs`
- **Usage**: `src/api.rs` (see `execute_code()`)
- **Examples**: `examples/broker_patterns.rs`

## Future Work

The broker provides a foundation for:
- Multi-kernel support (one broker per kernel)
- Request priorities
- Metrics and monitoring
- Message replay
- Backpressure handling
