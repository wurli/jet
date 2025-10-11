# Include error-site Rust backtraces on unexpected R-level errors

> <https://github.com/posit-dev/ark/pull/385>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

This PR improves the backtraces in our panic and error reports when an unexpected R error occurs somewhere in Ark internals.

Since https://github.com/posit-dev/amalthea/pull/257, when an R-level error occurs unexpectedly in an `RFunction` call, we capture an R backtrace _at the error site_ rather than the catch site. Because it's captured at the error site it contains the full context that lead up to the error. However this mechanism currently has limitations that this PR aims to fix:

- The backtrace capture mechanism is implemented in `safe_eval()` and only benefits to the callers of this function. There are many other contexts where we run inside a simple top-level-exec context. We also have a bunch of different ways of evaluating R code, e.g. using try-eval-silent from the R API. Instead we should use the same protection and capture mechanisms everywhere.

- The capture mechanism provides an intercepting backtrace for R but not for Rust, so we are missing part of the context. For instance in an R error happens inside `read_console()`, we currently only get a Rust backtrace up to the sandbox call in which our read-console implementation lives and we can only guess what happened in there.

Here is the strategy taken by this PR to fix these limitations:

- `top_level_exec()` hasn't changed, it is the lowest level protection mechanism. It insulates from condition handlers and captures any R longjumps. It returns an `Error::TopLevelExecError` in that case.

- `try_catch()` is no longer a wrapper around `R_tryCatch()`. It now wraps `top_level_exec()` and a new R operation (from 4.0): `R_withCallingErrorHandler()`. Unlike the former, the latter doesn't need to call into R to install condition handlers so it's faster, and it handles errors at the error site rather than the catch site. We now capture both an R and a Rust backtrace in this handler. Note that calling handlers are not capable of capturing stack overflow errors but these are still captured by top-level-exec, with a relevant message taken from the R error buffer.

- `try_catch()` has a more general interface so it can be called in more contexts. It no longer requires a closure that returns an object coercible to `RObject`, instead it's fully generic in the return type. This means you need to perform the conversion yourself at the call site if needed.

- `try_eval()` and `try_eval_silent()` are wrappers around `try_catch()` to evaluate R expressions. The silent version is supported by a new RAII struct `raii::RLocalShowErrorMessageOption()`. `r_parse_eval()` now calls `try_catch()` instead of doing its own thing.

- Thanks to this consolidation `TryEvalError` has been removed. We how only have `TryCatchError` and `TopLevelExec` errors.

Style-wise, the `r_` prefix was removed from all these functions. Instead they are reexported from the top-level module and intended to called with a `harp::` prefix, e.g. `harp::try_catch()`. This prefix signals you should only call the function on the R thread.

Following these changes, when an unexpected R error occurs in our implementation, you can expect to see 2 or 3 backtraces: The R backtrace, the Rust-level backtrace for the R thread, and possibly the Rust backtrace of the calling thread if the error occurred in an R task. Excerpt:

<img width="710" alt="Screenshot 2024-06-05 at 14 39 09" src="https://github.com/posit-dev/amalthea/assets/4465050/b2b6de93-9416-4800-9041-d47dae4375f9">


