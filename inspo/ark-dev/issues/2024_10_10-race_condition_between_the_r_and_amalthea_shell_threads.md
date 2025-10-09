# Race condition between the R and Amalthea Shell threads

> <https://github.com/posit-dev/ark/issues/569>
> 
> * Author: @lionel-
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695"), list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6"), list(id = "LA_kwDOJkuGPc8AAAABwhpKcQ", name = "infra: tests", description = "", color = "bfdadc")

One run of Windows CI failed because a `status` message came in on IOPub before the expected `execute_result`:

```
thread 'test_notebook_execute_request_incomplete_multiple_lines' panicked at crates\amalthea\src\fixtures\dummy_frontend.rs:219:9:
assertion failed: `ExecuteError(JupyterMessage {
  zmq_identities: [],
  header: JupyterHeader {
    msg_id: "45d7303b-4f2b-49de-ad9a-561425034687",
    session: "bc8fb405-6169-4601-9caf-507488d75ee6",
    username: "kernel",
    date: "2024-10-04T14:48:27.391849800+00:00",
    msg_type: "error",
    version: "5.3"
  },
  parent_header: Some(JupyterHeader {
    msg_id: "38b5f3ed-3087-4c14-a0c4-38af2ddb1f87",
    session: "59960967-e8b2-4ba6-86f9-c7cbda70e3f6",
    username: "kernel",
    date: "2024-10-04T14:48:27.145261900+00:00",
    msg_type: "execute_request", version: "5.3"
    }),
  content: ExecuteError {
    exception: Exception {
      ename: "",
      evalue: "Error:\n! \nCan't execute incomplete input:\n1 +\n2 +",
      traceback: ["Error:\n! \nCan't execute incomplete input:\n1 +\n2 +"] } } })`
      does not match `Message::Status(data)`
```

<details>

I think it's caused by us handling requests on two different threads (Amalthea Shell and R) interacting with a third thread (Amalthea IOPub).

When a Shell request comes in, the Amalthea's Shell socket thread (dispatch in https://github.com/posit-dev/ark/blob/cac78eb3d92d54687845ddba9402054ba907b510/crates/amalthea/src/socket/shell.rs#L160, `execute_request` handler in https://github.com/posit-dev/ark/blob/cac78eb3d92d54687845ddba9402054ba907b510/crates/amalthea/src/socket/shell.rs#L221) performs this sequence:

- Send a `Busy` message on IOPub
- Invoke Ark's handler for Shell messages
- Block until Ark's handler responds
- Send the response on its Shell socket ()
- Send `Idle` to IOPub.

Note that IOPub's messages are not sent immediately on the IOPub socket, they are sent to the IOPub thread which relays the messages to the IOPub socket which it owns.

While Ark's `execute_request` handler (https://github.com/posit-dev/ark/blob/cac78eb3d92d54687845ddba9402054ba907b510/crates/ark/src/shell.rs#L205) is invoked on Amalthea's Shell socket thread, it immediately relays the request to the R thread which then executes the code.

When the request has been completed, as determined by iterating back into `ReadConsole`, the R thread sends an `execute_result` on IOPub: https://github.com/posit-dev/ark/blob/cac78eb3d92d54687845ddba9402054ba907b510/crates/ark/src/interface.rs#L1242.

Note how there are three interacting threads:

- Amalthea Shell sends status messages on IOPub
- The R thread sends stream and result messages on IOPub
- The IOPub thread relays these messages on its socket

This triangle configuration is bound to have race conditions regarding message ordering of outbound IOPub messages.

This concerns the status and result messages as in the test failure above but it's also possible for other IOPub messages sent by R thread to lag behing past an Idle boundary. It's problematic because the Jupyter protocol states:

> When that kernel has completed processing the request and has finished publishing associated IOPub messages, if any, it shall publish a status message with execution_state: 'idle'

So clients are not required to track the parent header of IOPub messages to match them to already closed requests. They can just assume these are nested in status messages.

So there are two things we can do here:

- [ ] To fix the status/result ordering, we can simply move the emission of results to the Amalthea socket thread, i.e. in Ark's `execute_request` handler. We already send a response to unblock the thread so that's easy.

- [ ] A more comprehensive fix that would ensure the IOPub messages are enclosed in Busy/Idle statuses would be to move the entire handling of Shell messages to the R thread, which would own the Shell socket. Then there would be only two threads involved: the R/Shell thread and IOPub. This would solve the triangle race conditions and would significantly simplify our asynchronous system. Related goal: https://github.com/posit-dev/ark/issues/689

  A simpler fix would be to send IOPub messages from the R thread to the Amalthea Shell thread, which would then relay them to the IOPub thread. This would merge the flows of IOPub messages and ensure proper ordering.

</details>

I'm now doubting my analysis in the details box. The three-thread issue arises when there's a continuous flow of messages from threads A and B to thread C. But here the flow of messages from R and Shell to IOPub are nested.

## @DavisVaughan at 2024-10-07T16:37:01Z

One potential hypothesis is:
- IOPub `Busy` arrives on the IOPub thread and can't be forwarded on to ZMQ for some reason
    - Note that we would be able to record the parent as the `shell_context` before the failure occurs, which would explain why we see a `parent_header` above
- IOPub `ExecuteInput` arrives on the IOPub thread and can't be forwarded on to ZMQ for some reason
- IOPub `ExecuteError` arrives on the IOPub thread, and somehow this succeeds
- `recv_iopub_busy()` gets this `ExecuteError` and panics

I don't think we can see this right now because we wouldn't see these logs:
https://github.com/posit-dev/ark/blob/cac78eb3d92d54687845ddba9402054ba907b510/crates/amalthea/src/socket/iopub.rs#L122

It's possible this should be a panic - dropping a Busy or Idle message is quite bad, there is no way to recover from that.