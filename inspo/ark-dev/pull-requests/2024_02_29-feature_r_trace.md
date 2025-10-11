# Include R-level backtrace in failing `RFunction` calls

> <https://github.com/posit-dev/ark/pull/257>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

This implements a new `r_safe_eval()` utility that combines `r_top_level_exec()` and an error + backtrace capture with `withCallingHandlers()`. This is now used for `RFunction::call()` to provide R-level backtraces for unexpected errors.

Here is an example for the bug introduced in https://github.com/posit-dev/amalthea/pull/254. Before:

<img width="748" alt="Screenshot 2024-02-29 at 12 02 00" src="https://github.com/posit-dev/amalthea/assets/4465050/b8b2c15d-21ad-4532-9e8f-5b36f6a0e06e">

After:

<img width="757" alt="Screenshot 2024-02-29 at 11 31 38" src="https://github.com/posit-dev/amalthea/assets/4465050/e2064ed1-9cdb-453b-9ae6-8ea5c4233ab9">

In the future we also need to reimplement `r_try_catch()` around `r_safe_eval()` to gain backtrace capture.

