# Extract LSP state in own object

> <https://github.com/posit-dev/ark/pull/358>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from #357.

This PR starts the work of gathering all LSP inputs to a new `WorldState` struct. This data structure will eventually be a pure value without any interior mutability (e.g. no DashMap of Documents). The goal is that if you take a clone of it, you can send it off to a long-running background task without running the risk of invalidated assumptions due to input mutation after an LSP update.

Progress towards posit-dev/positron#3181 (Incremental and cancellable computations with salsa): In the longer term this could serve as the source of root Salsa inputs for incremental computation. Querying an outdated Salsa input would cancel the background task with a special unwinding panic.

