# Extract `RCall` from `RFunction`

> <https://github.com/posit-dev/ark/pull/254>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

- `RFunction` is now built around `RCall`.

- `RCall` takes constructed functions of type `RObject`. The `RFunction` ctors now build them ahead of time.

- We have three `RFunction` variants: `new()`, `new_internal()` for `:::` access, and `new_inlined()` for bare closures.

  As before these don't type-check so runtime failures might happen if something is wrong (empty symbol or wrong inlined type). We could change this, no strong opinion on what's most appropriate. I'll note that the runtime failure will be reported via the `call()` method which returns a Result. Also we can't type-check ahead of time that the passed function is actually the intended one anyway, in which case a runtime error can't be helped. So for simplicity I'd lean into not introducing `Result` here.

- The building and calling logics are now split into `RCall::build()` and `RFunction::call()`. The `build()` method is useful to gain the nice Rust syntax to build R calls without evaluating them.

## @lionel- at 2024-02-28T15:00:42Z

I keep being bitten by the automatic enquoting of arguments so I've replaced this by an `expr_protect()` function to be called manually by the caller. No tests are broken and I couldn't find anything broken playing around with LSP and comms in positron.

## @lionel- at 2024-02-29T11:04:57Z

Here is one that was missing: https://github.com/posit-dev/amalthea/commit/2aaa7c532cdacd73123dfe3aeabf91a845b7e2fc
