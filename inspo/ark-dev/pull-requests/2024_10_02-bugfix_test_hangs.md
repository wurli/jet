# Fix hanging issues in integration tests

> <https://github.com/posit-dev/ark/pull/558>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes #551.
Branched from #547.

I've been facing issues locally with instability which in turned caused integration tests to hang indefinitely. And on CI we now also test on Windows, which is causing similar hang issues in the Amalthea integration tests (but due to other instability causes). This PR addresses my local instability issue and attempts to better propagate panics and problems in server-side threads so that the test threads do not hang waiting for messages.

It turns out that the instability was being caused by this top-level callback installed by the cli package: https://github.com/r-lib/cli/blob/1220ed092c03e167ff0062e9839c81d7258a4600/R/onload.R#L33-L40. This caused extra output containing an ANSI escape sequence to be streamed on IOpub at random times. To avoid this, we now pass `--vanilla` to R on startup. (Note that Positron will not see these "show cursor" escapes because we redirect stdout in production builds, which causes `istty(stdout())` to be `FALSE` and cli does not register that task callback.)

We now also use `assert_matches()` to get better information about actual incoming messages when they don't match what is expected. Unlike `assert_match()`, this shows the contents of the failing input which helps a lot.

Regarding tests hanging, the key point is that when a background thread panics, a panic hook is run to record information about the panic, the thread is unwound, but then nothing else happens if the thread is not joined. So when a test thread is waiting for an expected message and the server thread has panicked, the panic is not propagated to the client-side thread, which waits forever.

What did not work is setting a panic hook to propagate background panics to the main thread:

  - Notifying other threads from the panic hook is feasible but we need to specifically wait for notifications at the other end.
  - Another approach is to flip a global flag that other threads can check periodically, but that's manual and error-prone.
  - Calling the original panic hook and then exiting the process with `exit()` or `abort()` does not work out because the hard exit prevents the test runner from printing a summary of the test results.

Things that this PR does instead to (partially) address this:

- We really need the client test thread to panic when something goes wrong in the server threads. Propagating panics from the server threads is hard, so instead we now poll for expected incoming messages with a timeout. After one second the client thread panics.

- In the zmq/crossbeam merging threads, we now panic instead of logging if we're running tests and something unexpected happens, e.g. when an expected message doesn't come in or when a socket connection fails. This allows the test runner's panic hook to take note of the panic.

  Whether to panic is implemented by checking for `cfg(debug_assertions)`. This way we remain lenient (but verbose) in production builds.

- Sometimes the panic occurs even before the kernel starts up, e.g. because a zmq socket fails to connect. This is not a problem for the main Ark binary because this all happens on the main thread, where panics cause an exit. However in integration tests we run R in a background thread so it can be reused across multiple test threads. An early panic would then be silently ignored and the tests would be waiting indefinitely for R initialization.

  To fix this, I split `RMain::setup()` from `RMain::start()`. This allows the test init to run the former (via `start_kernel()`) independently on its own thread, before starting the R REPL on a background thread. Most init problems can then be propagated correctly to the test runner.


