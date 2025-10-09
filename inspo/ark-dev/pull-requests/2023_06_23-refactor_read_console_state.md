# Collect all global state for R callbacks in a struct

> <https://github.com/posit-dev/ark/pull/52>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

This way all state needed by the callbacks can be inspected in one place and this makes the access syntax a bit lighter.
Also removes some unnecessary mutexes for state that is only accessed from frontend callbacks by the single R thread.

Branched from #51.

## @lionel- at 2023-06-26T19:35:13Z

Following this, we could move the frontend methods to `impl RMain` methods, here is an example: https://github.com/posit-dev/amalthea/commit/ffb6bbfd7355f7737ef1f0417d204f15a4565c00

The idea is that we'd call `let main = unsafe { R_MAIN.as_mut().unwrap() };` inside R extension points and then immediately the corresponding method on `main`. All these methods can then be written more conventionally with state in `self`, and without the R lock since the extension points are called synchronously by R (via frontend methods or `.Call()`). We just need to make sure the `unsafe` dereferences of the global `R_MAIN` object are indeed made from the R thread to ensure safety.