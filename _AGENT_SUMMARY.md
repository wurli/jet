# Investigation Summary: Cargo Test Hang

## Problem
`cargo test` currently hangs. The issue doesn't occur when ark's `is_complete` test is commented out.

## Root Cause
The `is_complete()` and `get_completions()` functions in `src/api.rs` were incorrectly waiting for iopub status messages that are never sent for these request types.

According to the Jupyter messaging protocol:
- `execute_request` publishes status messages (busy/idle) on the iopub channel
- Simple requests like `is_complete_request` and `complete_request` do NOT publish status messages
- They only send a direct reply on the shell channel

## The Bug
Both functions had this blocking loop:
```rust
while let Ok(reply) = receivers.iopub.recv() {
    match reply {
        Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
            log::trace!("Received iopub busy status for is_complete_request");
        }
        Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
            log::trace!("Received iopub idle status for is_complete_request");
            break;
        }
        _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
    }
}
```

The `receivers.iopub.recv()` is a **blocking** call that waits indefinitely for a message. 
Since the kernel never sends status messages for these request types, the code hangs forever.

## Solution
Removed the iopub waiting loop from both `is_complete()` and `get_completions()`. 
These functions now:
1. Send the shell request
2. Call `await_reply_shell()` to wait for the shell reply
3. Receive and return the shell reply

This matches the pattern used for `KernelInfoRequest` in the `subscribe()` function.

## Changes Made
- Modified `src/api.rs`:
  - `get_completions()`: Removed 16 lines of blocking iopub wait loop
  - `is_complete()`: Removed 16 lines of blocking iopub wait loop
  - Both functions now return errors properly instead of using a mutable `out` variable
  - Both functions now properly unregister requests before returning

## Testing
The code compiles successfully with `cargo build --tests`. The actual test execution would require setting up a Jupyter kernel (e.g., ark kernel), which is not available in this environment.
