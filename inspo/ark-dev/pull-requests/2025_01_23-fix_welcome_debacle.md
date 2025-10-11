# Set the client side SUB IOPub socket subscription *before* we connect

> <https://github.com/posit-dev/ark/pull/673>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

I am hoping this fixes some very strange weirdness we've been seeing ever since we added the `Welcome` message handshake infrastructure. In particular, this error we see in CI a lot:

> thread 'test_env_vars' panicked at crates/amalthea/src/fixtures/dummy_frontend.rs:195:9:
assertion failed: `Status(JupyterMessage { zmq_identities: [], header: JupyterHeader { msg_id: "24ea361b-b116-462e-a868-47d503ae01be", session: "5ccdaa85-4179-49b6-b8e2-97b484081e7e", username: "kernel", date: "2025-01-23T21:23:15.387272+00:00", msg_type: "status", version: "5.3" }, parent_header: None, content: KernelStatus { execution_state: Starting } })` does not match `Message::Welcome(data)`

That error comes from here:
https://github.com/posit-dev/ark/blob/c7cfe6b16f595e6cd7328d87af915eb46868a6d0/crates/amalthea/src/fixtures/dummy_frontend.rs#L185-L192

Essentially "I was expecting to get a `Welcome` message, but instead got a `KernelStatus { Starting }` message"

We've been confused about this because we thought that an XPUB socket was supposed to send out a `Welcome` message before _anything_ else when a SUB connects, which should make this impossible. But we've been thinking about it wrong! It's not that the XPUB socket is sending the `KernelStatus { Starting }` message first, it's that _we've already missed_ the `Welcome` message!

Here's my theory about the race condition we are dealing with:
- XPUB `bind()`s first
- SUB `connect()`s
- XPUB processes that connection, and says `Welcome`. But SUB has not set any subscriptions, so `Welcome` is dropped
- SUB `subscribe()`s, and then starts waiting on `Welcome`
- XPUB sends out `KernelStatus { Starting }`
- SUB is very confused when we get `KernelStatus { Starting }` instead of `Welcome`, resulting in the above message

The solution is so simple. Just subscribe _before_ calling `connect()`, so the welcome message can't get dropped. This is even confirmed by the documentation of `ZMQ_XPUB_WELCOME_MSG`:

> Subscriber must subscribe to the Welcome message before connecting.

In this case `Subscriber = our client side SUB`. So yea, that would explain it.

I even dug all the way into the zmq C++ to see where the welcome message is sent out from. It does indeed look like it is sent out exactly once, at `connect()` time, providing a window where we can miss it if we are not subscribed yet!
https://github.com/zeromq/libzmq/blob/34f7fa22022bed9e0e390ed3580a1c83ac4a2834/src/xpub.cpp#L56-L65

## @DavisVaughan at 2025-01-23T22:56:40Z

@jmcphers I'm pretty convinced from this that kallichore should probably be subscribing before connecting as well, right here:
https://github.com/posit-dev/kallichore/blob/6a5d4877cd81b14e13e8c2a096bbb41c2cd22452/crates/kcserver/src/zmq_ws_proxy.rs#L155-L173

Which would really come into play when kallichore supports the welcome message (https://github.com/posit-dev/kallichore/issues/1) like our test client does.
