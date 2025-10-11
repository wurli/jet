# Incorporate `Kernel` methods in the R thread

> <https://github.com/posit-dev/ark/pull/57>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Replaces #54.
Branched from #52

This refactoring intends to reduce the complexity of our concurrency model by incorporating as much functionality as possible into the R thread.

- The `ReadConsole()` callback now pulls execution requests from the `socket-shell` thread in place of the `ark-execution` thread which is now removed. Other requests that need to run concurrently to R are run from a new `ark-shell` thread owned by `Shell`.

- Correspondingly the `Request` enum is split into two enums `RRequest` (requests for the main thread) and `KernelRequest` (requests for the `Kernel` object shared among R, Shell, and LSP).

- The execute request/response methods now live in `interface.rs` and no longer communicate across threads via channels. Everything in `interface.rs` runs synchronously with R and so this code may now be considered single-threaded regarding the use of R and conceptually does not need the R lock (though we still should use `r_lock!` to protect against interrupts or perhaps add another macro for that purpose). This should make it easier to reason about, esp. in terms of message-passing races e.g. with interrupts.

- Inputs are now returned synchronously to `ReadConsole()` and represented by a new `ConsoleInput` enum with variants `EOF` and `Input(String)`.

- The `R_ERROR_OCCURRED` atomic and its friends are now simple variables in `RMain`.

- Some methods have been moved from `Kernel` to `RMain` methods. As discussed in #52, I would like to move all frontend methods to `RMain` in an ulterior PR, on the model of https://github.com/posit-dev/amalthea/commit/ffb6bbfd7355f7737ef1f0417d204f15a4565c00. This way the methods can be written more conventionally with state in `self`, and we just need to make sure the unsafe dereferences of the global `R_MAIN` object are indeed made from the R thread to ensure safety.

- For simplicity, the `StdIn` channel is no longer passed via a specific message but is passed to `Kernel.connect()` by argument. This channel is now owned by `RMain`.

