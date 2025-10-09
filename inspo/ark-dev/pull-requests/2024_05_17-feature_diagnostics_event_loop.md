# Simplify refresh and publication of diagnostics

> <https://github.com/posit-dev/ark/pull/360>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from https://github.com/posit-dev/amalthea/pull/359.

Addresses https://github.com/posit-dev/positron/issues/2630.
Follow-up to https://github.com/posit-dev/amalthea/pull/224 (Refresh diagnostics on open, on close, on change, and after R execution).

This PR focuses on diagnostics to tighten the control flow and concurrency between the different parts. Note that this will be improved further in the next PR so I wouldn't focus too much on the names of these events and how we send them. But I think it's still helpful to review this step separately.

- Introduce blocking tasks. We now run diagnostics in blocking tasks instead of async ones to avoid clogging up the async pool thread. The initial round of indexing is now also spawned in a blocking task instead of being run synchronously. The next PR will add more infrastructure around launching such tasks.

- New task events managed by our LSP loop introduced in posit-dev/positron#359 to refresh and publish diagnostics. This part is largely refactored in the next PR, but the main ideas remain.

- Remove the initialisation manager for the indexer. We now launch initial diagnostics from the indexer instead of in a timer.

- Remove the debouncer to make worldstate a pure value (the mutable diagnostic ID is removed from document). We could reintroduce a debouncer later on but I think that should be managed externally with a dedicated task managing the debouncing for each doc. That said I like the snappy diagnostics and immediate feedback.

- Thanks to these changes, diagnostics functions no longer take a clone of the backend. I've also reorganised things a bit so that there is a better file separation between diagnostics code and LSP handling code.

- I've commented out the "package not installed diagnostic" code for these reasons:

  - This lint steers users towards an action they don't necessarily want to take (installing a package), and is rather distracting in that case: https://github.com/posit-dev/positron/issues/2672

  - We'd like to avoid running R code during diagnostics: https://github.com/posit-dev/ark/issues/691

  - The diagnostic meshes R and tree-sitter objects in a way that's not perfectly safe and we have a known crash logged: https://github.com/posit-dev/positron/issues/2630. This diagnostic uses R for argument matching but since we prefer to avoid running `r_task()` in LSP code we should just reimplement argument matching on the Rust side.

  Given all these considerations I think it's best to just remove this feature for now.

