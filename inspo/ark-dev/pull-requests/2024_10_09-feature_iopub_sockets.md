# Implement JEP 65

> <https://github.com/posit-dev/ark/pull/577>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/ark/issues/569

This PR fixes a race condition regarding subscriptions to IOPub that causes clients to miss IOPub messages:

- On startup a client connects to the server sockets of a kernel.
- The client sends a request on Shell.
- The kernel starts processing the request and emits busy on IOPub.

If the client hasn't been able to fully subscribe to IOPub, messages can be lost, in particular the Busy message that encloses the request output.

On the Positron side we fixed it by sending kernel-info requests in a loop until we get a Ready message on IOPub. This signals Positron that the kernel is fully connected and in the Ready state: https://github.com/posit-dev/positron/pull/2207. We haven't implemented a similar fix in our dummy clients for integration tests and we believe this is what is causing the race condition described in #569.

As noted in https://github.com/posit-dev/positron/pull/2207, there is an accepted JEP proposal (JEP 65) that aims at solving this problem by switching to XPUB.

https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html
https://github.com/jupyter/enhancement-proposals/pull/65

The XPUB socket allows the server to get notified of all new subscriptions. A message of type `iopub_welcome` is sent to all connected clients. They should generally ignore it but clients that have just started up can use it as a cue that IOPub is correctly connected and that they won't miss any output from that point on.

Approach:

The subscription notification comes in as a message on the IOPub socket. This is problematic because the IOPub thread now needs to listens to its crossbeam channel and to the 0MQ socket at the same time, which isn't possible without resorting to timeout polling. So we use the same approach and infrastructure that we implemented in https://github.com/posit-dev/ark/pull/58 for listeing to both input replies on the StdIn socket and interrupt notifications on a crossbeam channel. The forwarding thread now owns the IOPub socket and listens for subscription notifications and fowrards IOPub messages coming from the kernel components.


Co-authored-by: Davis Vaughan <davis@posit.co>
Co-authored-by: Lionel Henry <lionel@posit.co>

