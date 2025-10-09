# Run R on the main thread

> <https://github.com/posit-dev/ark/pull/128>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Follow-up to https://github.com/posit-dev/positron/issues/4
Addresses https://github.com/posit-dev/positron/issues/1420 (cc @jmcphers)

Fixes a crash with debug builds of R caused by a stack overflow. The default stack size of background threads on macOS is 512kb instead of 8mb for the main thread, so running R in the background is definitely too risky on some platforms. Also fixes stack overflow errors.

- With the refactor `kernel.connect()` now returns right away.

- Since we wait until after `connect()` to start R, I was able to remove all the waiting mechanism that was introduced to fix a multithreading issue between R and 0MQ init.

- The routine that checks if we're on the R thread now needs a non-static global variable holding an optional thread ID set on startup. This was needed because there's no straightforward cross-platform way to determine that we are on the main thread.

- The code disabling stack limit checks was removed (see https://github.com/posit-dev/positron/issues/4). This was causing Ark to crash when a function inf-recursed, e.g. `f <- function() f(); f()`. R is now able to detect this and throw an error instead. Our global handler is not called in case of stack overflows but I found a workaround (documented in the code).

![Screenshot 2023-10-25 at 14 58 25](https://github.com/posit-dev/amalthea/assets/4465050/d8616c0d-d0d4-4e12-afaf-c34a62424319)


