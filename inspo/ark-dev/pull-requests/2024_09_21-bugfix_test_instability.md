# Fix synchronisation of R accesses in tests

> <https://github.com/posit-dev/ark/pull/541>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/positron/issues/4222: Instability in Ark tests is fixed.

Reverts https://github.com/posit-dev/ark/pull/465 which is no longer needed to mitigate the crashes.

@DavisVaughan determined in https://github.com/posit-dev/positron/issues/4222 that tests can end up in a situation where `r_task()` is called from another thread than the thread where tests are executed. Because of this escape hatch for tests: https://github.com/posit-dev/ark/blob/35215ac5309739840e372bbd79f4662fe43517f3/crates/ark/src/r_task.rs#L149, there is no synchronisation when `r_task()` is called within an `r_test()`.

To fix this:

- We now lock harp's test mutex in `r_task()`.

- We're also initializing R from `r_task()` if not done already.

- The test mutex is now reentrant. This is necessary because we allow for the same leniency in the real `r_task()` where we just run the closure if we're already on the main thread. This should arguably be changed to represent a logic error instead, but it's probably too dangerous to do that before we have drastically reduced our reliance on `r_task()`.

In addition to these changes, I've renamed harp's `r_test()` to `r_task()`, and I've removed Ark's `r_test()` in favour to `r_task()`. This simplifies the API as we now only have one operation to run R code from other threads (two really, one in Harp and one in Ark, the latter being necessary in tests for Ark-specific initialization).

These changes allow for a new workflow for testing comms:

- Don't wrap your test in `r_task()` (previously `r_test()`)
- Instead use `r_task()` selectively when you need to access the R API.

This lets other threads, such as a comm thread, use `r_task()` to get access to the R API in a synchronised way and without causing deadlocks.

One big behaviour change is that tests that are no longer wrapped in `r_task()` no longer run sequentially. This is problematic for the data explorer tests because they depend on state in the global env. Ideally they should be rewritten to remove these dependencies @dfalbel.

In the mean time I've added `r_test_lock()` which uses a second lock to force these tests to run sequentially.

