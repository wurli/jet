# IOPub Message Broker Architecture

## Overview

The IOPub message broker provides a robust solution for handling incoming messages from Jupyter kernels. It solves several key challenges:

1. **Message Correlation**: Routes messages to the correct execution request using parent headers
2. **Selective Routing**: Filters messages by type for different scenarios
3. **Message Retention**: Buffers orphan messages temporarily in case they're needed
4. **Automatic Cleanup**: Removes stale messages and requests to prevent memory leaks

## Architecture

### Core Components

#### `IopubBroker`
Central message router that:
- Maintains a registry of active requests
- Routes messages based on parent header correlation
- Buffers orphan messages (those without a matching request)
- Supports global subscribers for monitoring/debugging
- Performs periodic cleanup of stale data

#### `RequestChannels`
Separate channels for different message categories:
- `status_tx`: Kernel status messages (Busy, Idle, Starting)
- `execution_tx`: Execution-related messages (ExecuteInput, ExecuteResult, ExecuteError)
- `stream_tx`: stdout/stderr streams
- `display_tx`: Display data and updates
- `comm_tx`: Comm messages (for widgets, etc.)

#### `ExecutionCollector`
Receiver side of RequestChannels, provides methods to:
- Collect messages with timeouts
- Wait for specific message patterns
- Determine when execution is complete

### Message Flow

```
Kernel IOPub Socket
       ↓
   IOPub Thread (iopub_thread::start_iopub_thread)
       ↓
   IopubBroker::route_message
       ↓
  ┌────┴────┐
  ↓         ↓
Global    Extract Parent Header
Subscribers      ↓
            Match Request?
           ┌─────┴─────┐
          Yes          No
           ↓            ↓
    Route to      Buffer as
    Channels       Orphan
```

## Usage

### Basic Execution Pattern

```rust
use std::sync::Arc;
use std::time::Duration;
use carpo::msg::broker::{IopubBroker, ExecutionResult};

// 1. Create broker
let broker = Arc::new(IopubBroker::new());

// 2. Start IOPub thread
let broker_clone = Arc::clone(&broker);
iopub_thread::start_iopub_thread(iopub, broker_clone);

// 3. Execute code with message correlation
let (channels, collector) = ExecutionResult::create_channels();
let request_id = shell.send_execute_request(&code, options);
broker.register_request(request_id.clone(), channels);

// 4. Collect results
match collector.status_rx.recv_timeout(Duration::from_secs(5)) {
    Ok(Message::Status(msg)) => { /* handle busy */ }
    // ...
}

// 5. Cleanup
broker.unregister_request(&request_id);
```

### Global Monitoring

For debugging or logging all IOPub messages:

```rust
let (monitor_tx, monitor_rx) = channel();
broker.add_global_subscriber(monitor_tx);

// In another thread
std::thread::spawn(move || {
    while let Ok(msg) = monitor_rx.recv() {
        println!("IOPub: {:?}", msg.message_type());
    }
});
```

### Custom Configuration

```rust
use carpo::msg::broker::{IopubBroker, BrokerConfig};
use std::time::Duration;

let config = BrokerConfig {
    orphan_buffer_max: 500,
    orphan_max_age: Duration::from_secs(30),
    request_timeout: Duration::from_secs(120),
    cleanup_interval: Duration::from_secs(15),
};

let broker = Arc::new(IopubBroker::with_config(config));
```

## Key Benefits

### 1. Message Correlation
Messages are routed to the correct request using the parent header's `msg_id`:

```rust
// Request sent with msg_id: "abc-123"
shell.send_execute_request(&code);

// IOPub messages with parent_header.msg_id == "abc-123" 
// are automatically routed to this request's channels
```

### 2. Type-Based Channel Separation
Different message types go to different channels, making it easy to handle them separately:

```rust
// Status messages on one channel
let busy = collector.status_rx.recv()?;

// Results on another
let result = collector.execution_rx.recv()?;

// Streams on yet another
let stdout = collector.stream_rx.recv()?;
```

### 3. Orphan Message Handling
Messages that arrive without a registered request are buffered temporarily:

- Useful for messages that arrive before registration
- Prevents message loss due to timing issues
- Automatically cleaned up after timeout
- Can be queried for debugging

### 4. Automatic Cleanup
The broker periodically cleans up:

- **Stale requests**: Registered requests that exceed the timeout (default: 5 minutes)
- **Orphan messages**: Buffered messages older than max age (default: 1 minute)

This prevents memory leaks in long-running applications.

### 5. Thread Safety
All broker operations are thread-safe using:
- `Arc<RwLock<>>` for the request registry (many readers, few writers)
- `Arc<Mutex<>>` for the orphan buffer (less contention)
- Message passing via channels (lock-free)

## Design Decisions

### Why Parent Header Correlation?
The Jupyter protocol specifies that reply messages include the original request's header as the `parent_header`. This is the most reliable way to correlate messages since:

- Message IDs are unique per request
- The kernel guarantees to include the parent header
- Works across all Jupyter kernels (language-agnostic)

### Why Separate Channels?
Different message types have different consumption patterns:

- **Status**: Usually checked in a specific order (Busy → Idle)
- **Execution**: May or may not appear (e.g., invisible results)
- **Streams**: Can be buffered and arrive in chunks
- **Display**: May arrive asynchronously

Separate channels allow each to be handled appropriately without blocking others.

### Why Buffer Orphans?
Several scenarios can lead to orphan messages:

1. **Timing**: Message arrives before request is registered
2. **Cleanup**: Request was cleaned up but messages still arriving
3. **Global events**: Some messages (Welcome, Starting) have no parent
4. **Bugs**: Implementation errors or protocol violations

Buffering provides visibility into these cases for debugging.

## Performance Considerations

### Memory Usage
- Request registry: O(active_requests)
- Orphan buffer: O(orphan_buffer_max), default 1000 messages
- Global subscribers: O(subscribers) × O(messages)

For typical use cases (< 100 concurrent requests), memory overhead is minimal.

### Latency
- Message routing: O(1) hash map lookup
- Channel send: Lock-free, very fast
- Cleanup: Runs every 30 seconds by default, doesn't block routing

### Throughput
The broker can handle thousands of messages per second. The bottleneck is typically:
1. Network latency (ZeroMQ socket)
2. Message deserialization
3. Consumer processing speed

## Troubleshooting

### Messages Not Being Routed

Check that:
1. Request is registered before messages arrive
2. Request ID matches the message's parent_header.msg_id
3. Request hasn't been cleaned up as stale
4. Receiver channels haven't been dropped

Enable trace logging to see routing decisions:
```rust
env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Trace)
    .init();
```

### Orphan Messages Accumulating

Check broker stats:
```rust
let stats = broker.stats();
println!("Orphans: {}", stats.orphan_messages);
```

Reduce orphan_max_age if needed, or investigate why messages aren't being matched.

### Memory Leaks

Ensure requests are always unregistered:
```rust
// Use RAII pattern
struct ExecutionGuard<'a> {
    broker: &'a IopubBroker,
    request_id: String,
}

impl<'a> Drop for ExecutionGuard<'a> {
    fn drop(&mut self) {
        self.broker.unregister_request(&self.request_id);
    }
}
```

## Future Enhancements

Potential improvements for the broker:

1. **Request priorities**: High-priority requests get preferential routing
2. **Message filtering**: Filter messages by content, not just type
3. **Metrics**: Prometheus-style metrics for monitoring
4. **Replay**: Save orphan messages for later replay
5. **Backpressure**: Apply backpressure when channels are full
6. **Message tracing**: Distributed tracing support (OpenTelemetry)

## Related Files

- `src/msg/broker.rs`: Core broker implementation
- `src/frontend/iopub_thread.rs`: IOPub thread management
- `src/api.rs`: Example usage in execute_code
- `src/frontend/iopub.rs`: IOPub socket wrapper
