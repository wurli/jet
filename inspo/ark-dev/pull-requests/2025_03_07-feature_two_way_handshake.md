# Wait for an IOPub subscription during server startup

> <https://github.com/posit-dev/ark/pull/734>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

This PR addresses two issues:

- A small race condition where ark could bind to all of its sockets and then start sending out IOPub messages (unprompted by the client, i.e. if we process an `.Rprofile` all on our own). If we haven't received an IOPub subscription from the client yet, then on the ark side we'd simply _drop_ those IOPub messages on the way out. IOPub messages are very important and dropping them is extremely bad, so my solution here is to block in `kernel::connect()` until we have received an IOPub subscription.

- A test failure that I saw reliably on my local Windows machine, and that @jennybc also reported seeing. The client could receive an IOPub `KernelStatus::Starting` message before it received a `Welcome` message. This is because we really were trying to send out that `Starting` message first. From what I can tell, most of the time it was just getting dropped by us on the way out, otherwise it would have caused integration test failures. But on my local Windows machine I guess thread startup was slow enough that the client had fully subscribed on IOPub, so it received this message rather than us dropping it. I was tempted to just remove this `Starting` message entirely because Kallichore doesn't use it, but instead I've settled on sending it out right after `Welcome`, which happens after we have received the IOPub subscription message so we know someone is listening.

```
──── STDERR:             amalthea::client test_amalthea_input_request
thread 'test_amalthea_input_request' panicked at crates\amalthea\src\fixtures\dummy_frontend.rs:190:9:
assertion failed: `Status(JupyterMessage { zmq_identities: [], header: JupyterHeader { msg_id: "5c707901-7e47-4343-92c0-21e068512c41", session: "1eeb3d4b-fad5-4789-8ead-a32cf619b586", username: "kernel", date: "2025-03-19T20:18:00.897495300+00:00", msg_type: "status", version: "5.3" }, parent_header: None, content: KernelStatus { execution_state: Starting } })` does not match `Message::Welcome(data)`
```

@jennybc's failure happened on a Mac runner:
https://github.com/posit-dev/ark/actions/runs/13818568246/job/38658054678#step:8:939

With these issues out of the way we should be able to switch to `cargo nextest`

---

I thought this PR was going to be for https://github.com/posit-dev/positron/issues/6344 but it actually won't fix that. That's another race condition where sometimes we process an `.Rprofile` at the same time that we get a `kernel_info_request`. If that happens, Kallichore actually drops all IOPub messages until it gets the `kernel_info_reply`. We really need to come up with some proper timing of when to run `.Rprofile`, and it probably needs to be after we send out the `kernel_info_reply`.

