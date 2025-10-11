# Ark: Panics in dummy kernel threads should be propagated to test harness

> <https://github.com/posit-dev/ark/issues/551>
>
> * Author: @lionel-
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwhpKcQ", name = "infra: tests", description = "", color = "bfdadc")

Currently, if a panic occurs in integration tests on a background thread involved in sending server-side zmq messages, the thread gets poisoned silently and the test harness hangs because the client is waiting for zmq messages that can no longer be sent. This makes it hard to debug issues both locally and on CI.

To fix that, we could use the same approach as for the Ark binary where we propagate panics to the main thread and then panic from there:

https://github.com/posit-dev/ark/blob/c6df0b2419dd3b5ee5a6a51bcef8d452b0245f9e/crates/ark/src/main.rs#L245-L249



## @lionel- at 2024-10-01T07:28:34Z

This approach doesn't really allow the panic to surface to the test harness. We only see an abnormal exit code without any panic message. Instead we should transmit the panic info to the dummy frontend thread and propagate from there.

Now the problem is that we can't currently receive messages with timeouts. We need to fix that using @DavisVaughan's approach in https://github.com/posit-dev/ark/pull/545/files/36357c43fc58a77641f4ef2a20d25bed3188a0fb..035ffcd9578a805152052b6e4f786c3a602dfeeb
