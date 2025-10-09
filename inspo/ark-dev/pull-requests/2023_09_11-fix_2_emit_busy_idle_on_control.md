# Emit busy/idle status from Control requests

> <https://github.com/posit-dev/ark/pull/92>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

PR 2 of 2
Replaces https://github.com/posit-dev/amalthea/pull/89. Same implementation, just no longer on top of https://github.com/posit-dev/amalthea/pull/87.
Requires https://github.com/posit-dev/amalthea/pull/91
Requires https://github.com/posit-dev/positron/pull/1265
Addresses https://github.com/posit-dev/positron/issues/1234

Already approved here https://github.com/posit-dev/amalthea/pull/89#pullrequestreview-1616913250, description from there is repeated below for cohesion

---

The problem in https://github.com/posit-dev/positron/issues/1234 was that our Control requests (for shutdown and interrupt) were not emitting IOPub busy/idle statuses, even though they definitely should be, per the Jupyter documentation ("Busy and idle messages should be sent before/after handling every request, not just execution.", https://jupyter-client.readthedocs.io/en/stable/messaging.html#kernel-status). This meant that the Console never receives any notification that it should switch out of the `interrupting` state, so it just sits there in that state indefinitely.

The solution presented in this PR is rather simple, we just change up the Control implementation to look more like the Shell implementation, which guards each request with busy/idle state messages. That fixes the `CTRL+C` bug:

https://github.com/posit-dev/amalthea/assets/19150088/e5ff21fd-0d5a-48fa-ac48-c31b79ec746a

The complicating factor is that now if you run something like `Sys.sleep(100)` and then press `CTRL+C`, then the interrupt occurs BUT then the Console is stuck in an infinite busy state. The issue has to do with the `msg_context` that we use as a parent in IOPub messages. Previously we only ever sent IOPub messages from the Shell, and the Shell requests are processed synchronously, so there was no way we could ever use an invalid parent `msg_context` there. However, we now emit IOPub Status messages from Control, and that occurs asynchronously, so we end up with the following when the user sends `Sys.sleep(100)` followed by `CTRL+C`: 

- A Shell `execute_request` sets the IOPub status to `busy` which sets the `msg_context` to the Shell `execute_request`
- The user triggers an interrupt. This sets the IOPub status to `busy` again and sets the `msg_context` to the Control `interrupt_request`
- The interrupt is processed (i.e. it has been acknowledged that the interrupt was at least sent) and the IOPub status is set back to `idle` via the Control thread (note the `msg_context` is still the Control `interrupt_request`)
- The code execution wraps up, due to the interrupt, and an IOPub `execute_result` message is sent back - **but with a faulty parent of the Control `interrupt_request`!!**
- The code execution is done, so we send back an `idle` state - **but with a faulty parent of the Control `interrupt_request`!!**

Since the original `execute_request`'s `busy` status never receives a corresponding `idle`, we end up in a weird place.

I noticed that ipykernel does a similar thing to our `msg_context`, but they use a _per channel_ context, one for Shell and one for Control
https://github.com/ipython/ipykernel/blob/c24b252dc4fc81e6cf6354f8d64ea06ed1ce3496/ipykernel/kernelbase.py#L608

So that is what PR 1 implements, and it does seem to work

