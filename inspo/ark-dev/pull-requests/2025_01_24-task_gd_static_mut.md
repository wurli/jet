# Remove static mut in `graphics_device.rs`

> <https://github.com/posit-dev/ark/pull/674>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Progress towards #661.

- `DEVICE_CONTEXT` is now a thread-local variable to reflect the fact it should only be accessed from the R thread.

- It's wrapped in a `RefCell` that is initialized from `RMain` on startup. All subsequent accesses are read-only and never panic.

- The device context requires mutable state. Where possible we use `Cell` which never panics. In a couple of places we use `RefCell` with care not to double-borrow on mutation, mainly by only performing short borrows to prevent holding a reference while recusing into a device context method by accident. (I haven't examined whether that would be possible in principle to recurse into a method as I'm not familiar with the graphics device code and its R hooks. I just made sure the borrows were short or at least only invoked pure Rust code.)

## @lionel- at 2025-01-27T10:47:42Z

Logging this from Slack for reference: Atomics are for multi-threaded accesses but here it's single-threaded so using `Cell` is more appropriate.

Since this is all single-threaded, using `RwLock` instead of `RefCell` would only be trading panics for deadlocks and we prefer panics because they are actionable and self documenting for users
