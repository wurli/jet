# Send console scopes from `ReadConsole`

> <https://github.com/posit-dev/ark/pull/359>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from https://github.com/posit-dev/amalthea/pull/358.

This PR is a preparation for the next one where I remove `diagnostics_id` from `Document`. This value is used in a debouncing mechanism for diagnostics which relies on mutex synchronisation of documents. The mutex needs to go away to make the `WorldState` a pure, non-mutable value.

I saw in a comment that the debouncer is motivated by the fact that it queries the current state of the search path via an `r_task()` which we don't want to do too aggressively. So this PR removes this concurrent `r_task()`, this way we won't need the debouncer and the mutable documents. In the new approach we send the console scopes from ReadConsole, after each top-level evaluation. This makes the design cleaner and we don't need to query R as often.

Progress towards posit-dev/ark#691 (Reduce concurrent R evaluations to a minimum): Querying the search path concurrently with R was unsafe I think, we might be in the middle of an `attach()` or similar. We don't want to know the state of the search path in the middle of a computation, we want to know its state _between_ top-level evaluations.

Progress towards posit-dev/positron#3180 (Decouple LSP from the kernel): Sending the console scopes from read-console also brings us closer to decoupling the LSP from the kernel. We send the scopes via a Rust channel but it might just as well be a Jupyter comm.

