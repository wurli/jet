# Run LSP handlers consecutively by default

> <https://github.com/posit-dev/ark/pull/361>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from https://github.com/posit-dev/amalthea/pull/360 (Follow this link to see preparations for this PR)

Addresses https://github.com/posit-dev/positron/issues/2692.
Closes https://github.com/posit-dev/positron/issues/2999.
Supersedes and closes https://github.com/posit-dev/amalthea/pull/340.

This PR refactors our LSP server to solve persistent ordering issues of message ordering and corruption of internal state that have caused Ark to crash periodically (posit-dev/positron#271, posit-dev/positron#340). We've implemented a number of workarounds over the last year (https://github.com/posit-dev/amalthea/commit/ffd8b2725aed3236a7b811fb50234295a24de79d, posit-dev/amalthea#45) but we still observe crashes caused by stale cursors and ranges (posit-dev/positron#2692). This has also been brought up by beta testers.

@DavisVaughan found out that the ruff project recently switched from tower-lsp to lsp-server (from the rust-analyzer project) for reasons relevant to us here: astral-sh/ruff#10158. See also this discussion on the tower-lsp repo: ebkalderon/tower-lsp#284. The gist is that while tower-lsp does call our message handlers in a single task (and thus on a single thread) in the correct order, any `await` point within the handler will cause a transfer of control to the next handler. In particular, we used to send log messages on handler entry and this was an await point so the ordering of our handlers was doomed from the get-go.

My first attempt to fix this was #340 which takes the approach of synchronising the handlers with a read-write lock. This lock allowed concurrent read handlers but forced write handlers that update the state to wait for the read handlers to finish before running. Any incoming requests or notifications at that point would also be queued by the RwLock until the write handler was done. However I ended up deciding against this approach for these reasons:

- To allow Ark to scale to more complex code analysis, we need to preserve the ability to spawn longer-running tasks. And we can't wait for these to complete everytime we get a write request on our lock, as that would overly reduce our throughput. (This reason is no longer relevant now that `WorldState` is safely clonable though.)

- I think it's safer to turn off all concurrency between handlers _by default_ and enable it on a case by case basis after giving appropriate consideration. While the LSP protocol allows for out of order responses, it vaguely stipulates that doing so should not affect the correctness of the responses. I interpret this as meaning that requests responding with text edits (formatting, refactoring, but also some special completions) should not be reordered. Handling messages sequentially by default is a safer stance. The new setup is easier to reason about as a result.

In this PR, we now handle each message in turn. The handlers can still be async (though almost all of them are now synchronous) but they must resolve completely before the next handler can run. This is supported by a "main loop" to which we relay messages from the client (and the Jupyter kernel, see https://github.com/posit-dev/amalthea/pull/359) via a channel. Handlers must return an `anyhow::Result` and errors are automatically logged (and propagated as a jsonrpc error response). The loop is very close to the one running in rust-analyzer: https://github.com/rust-lang/rust-analyzer/blob/83ba42043166948db91fcfcfe30e0b7eac10b3d5/crates/rust-analyzer/src/main_loop.rs#L155-L164. This loop owns the world state (see discussion in https://github.com/posit-dev/amalthea/pull/358) and dispatches it to handlers.

The second part of fixing integrity issues is that the world state has become a _pure value_. All synchronisation and interior mutability (e.g. through the dash map of documents) has been removed. This means that we can clone the state to create a snapshot and for handlers running on long blocking tasks. If a document update arrives concurrently, it will not affect the integrity of these background tasks.

Long-running handlers on spawned tasks will respond to the client in arbitrary order. In the future we could synchronise these responses if needed, for all tasks or a subset of them.

There is also a separate auxiliary loop for latency sensitive tasks, in particular logging, but also things like diagnostics publication. The main loop is not appropriate for these because each tick might take milliseconds. We don't want log messages to be queued there as the latency would make it harder to understand the causality of events. This loop is also in charge of joining background tasks to immediately log any errors or panics that might have occurred.

Logging is no longer async nor blocking, and no longer requires a reference to the backend. I've added new macros such as `lsp::log_error!()` that can now be used anywhere in the LSP, including in synchronous contexts. I propose that we now consistently use these to log messages from the LSP. This will unclutter the Jupyter kernel log and allow to see the messages in their context (logged requests).

I've also added some utils to the `lsp::` module, like `lsp::spawn_blocking()` or `lsp::publish_diagnostics()`. All of these are intended to be called with the `lsp::` prefix.

I've removed the workarounds we implemented in `Document::on_did_update()`. These should no longer be necessary and were also making the incorrect assumption that document versions were consecutively increasing, whereas the LSP protocol allows clients to skip versions. The only guarantee is that the versions are monotonically increasing. We still check for this invariant and panic if that's not the case. I think there is no way to keep running with out of sync state. If this panic comes up in practice and is not the result of a synchronisation bug, we could replace the panic with an orderly shutdown to start over.

Orderly shutdowns should be easy to implement as both async loops, and all their associated tasks and state, are automatically dropped when the tower-lsp backend is dropped.

I've organised files in the following way:

- `main_loop.rs`: Implements the main and auxiliary loops.
- `state.rs`: Defines `WorldState`, the source of inputs for LSP handlers.
- `state_handlers.rs`: Implements handlers for state-altering notifications. These require an exclusive reference to the world state.
- `handlers.rs`: Implements read-only handlers. These take LSP inputs and prepare them before calling other entry points of the LSP.

