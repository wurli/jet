# Bump anyhow to 1.0.80 and drop backtrace crate

> <https://github.com/posit-dev/ark/pull/263>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

On the Windows VM, the `indexer::start()` command we use in LSP initialization takes over 40 seconds to complete when run in the `vctrs/` project. On a Mac, this typically takes a few milliseconds.

This is a bigger difference than just computer horsepower.

The way the indexer code is written is a little strange, `index_function()` and `index_comment()` end up propagating an _anyhow error_ upwards in the extremely common cases where no match is found, even though this error is immediately discarded in `index_node()`. These anyhow errors _always_ capture backtraces from what I can tell, so they actually have non zero cost to create, in particular on Windows apparently! The `index_function/comment()` functions are called hundreds of time, so this adds up.

With anyhow 1.0.71, the backtrace crate was used for backtrace capture, but in >=1.0.77 with Rust >=1.65, it started using the standard library's new backtrace support.
https://github.com/dtolnay/anyhow/pull/293
https://github.com/rust-lang/rust/pull/64154

They also encourage you to stop enabling the backtrace feature, as it is always on in Rust >=1.65 now:

> enabling the backtrace crate feature does nothing essentially (except downloading the dependency, so don't do that).

Switching to anyhow 1.0.80 (current release) somehow ends up fixing the performance issue. My guess is that std backtraces are either lazier or just way faster to capture on windows.

I consider this a "quick fix" and intend to rewrite the indexer code in a follow up PR to have an API that works more off `Option` than `Result`, avoiding this entirely (hopefully making it faster on all platforms).

---

Side note, I originally thought this was related to this anyhow performance regression:
https://github.com/dtolnay/anyhow/issues/347

But that seems to actually be unrelated and is a different scenario (there upgrading to 1.0.80 actually hurt performance, i think they previously were not capturing backtraces at all and all of a sudden they started to do so)

## @kevinushey at 2024-03-06T23:36:08Z

Belated LGTM; thanks for running that down.
