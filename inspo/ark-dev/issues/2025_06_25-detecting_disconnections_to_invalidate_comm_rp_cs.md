# Detecting disconnections to invalidate comm RPCs

> <https://github.com/posit-dev/ark/issues/841>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

When a request is sent via a comm, it's important that the comm eventually gets an error or a result, otherwise the sender waits undefinitely for a response (response times are variable so timeouts are not always sound). For this reason RPC mechanisms can't reliably work if messages are silently dropped.

For regular Jupyter messages initiated by the kernel we at least get an error of type `EHOSTUNREACH` because we have set `ZMQ_ROUTER_MANDATORY` on our ROUTER sockets (otherwise messages are silently dropped, see http://api.zeromq.org/3-3:zmq-setsockopt). This allows our infrastructure to detect delivery failures and take appropriate actions to recover (fail an StdIn request made to the client for instance). We set the router to mandatory here: https://github.com/posit-dev/ark/blob/d838eefd1d2534c0f023a7f2d9477e088d1eb63e/crates/amalthea/src/socket/socket.rs#L83-L93

However we don't have this guarantee for the OpenRPC mechanism of our custom comms because comm messages originating from the kernel are sent over IOPub. With a (X)PUB socket, messages are silently dropped if no one is there to listen.

To work around this we could listen for "unsubscribe" events on our XPUB socket (see below). The ability of detecting disconnections is one of the perks of having switched to XPUB when we implemented JEP65 (https://github.com/posit-dev/ark/pull/577/files). Whereas our ROUTER sockets aren't notified of disconnections, XPUB are. From https://rfc.zeromq.org/spec/29:

> SHALL receive subscribe and unsubscribe requests from subscribers depending   on the transport protocol used.
> SHALL, if the subscriber peer disconnects prematurely, generate a suitable unsubscribe request for the calling application.

We actually already handle (with a no-op handler) the unsubscribe notification here: https://github.com/posit-dev/ark/blob/d838eefd1d2534c0f023a7f2d9477e088d1eb63e/crates/amalthea/src/socket/iopub.rs#L270-L275. From there we should call a "disconnection" handler that downstream crates like Ark could implement to perform cleanups.

How we handle the disconnection depends on the comm type:

- For persistent comms like plots, which hold state for the frontend, we just invalidate pending requests. There is a slight race condition here: we might invalidate requests for incoming responses that were emitted before the disconnection.

- For all other comms (the default), we just close them and call a cleanup handler. This is the safest option and gets us ahead as we should destroy existing comms on reconnect anyway (see posit-dev/positron#1126).


## @DavisVaughan at 2025-06-16T16:00:49Z

Shouldn't we just shut down on an IOPub unsubscribe? I think Kallichore ensures our connection is persistent so we should never have an unsubscribe

## @lionel- at 2025-06-23T08:55:25Z

From what I've seen Kallichore only ensures a persistent connection for 90s, after this delay it does disconnect. @jmcphers I assumed it is still important to allow reconnections after the 90s, is this assumption correct?

Also we might be connected to another frontend. Shutting down on IOPub disconnects would make Ark unreliable with these other frontends that don't have buffering via Kallichore. Worth noting that we currently don't support Positron comms outside Positron (at least not officially, a third-party vim project does integrate with our LSP comm) so robustness of comm RPCs are not that important for these.

## @DavisVaughan at 2025-06-23T13:02:51Z

> Shutting down on IOPub disconnects would make Ark unreliable with these other frontends

I'd argue that you'd miss so much critical information after an IOPub disconnect that a shutdown seems like the right thing to do, it feels unrecoverable to me

## @lionel- at 2025-06-23T17:14:03Z

hmm to me it feels like we should be able to clean things up and recover to an acceptable level (maybe with some special behaviour for edge cases like interrupting readline). I agree that the client will potentially have missed important _output_ (unless they captured it), but not the _workspace_ or e.g. any results saved to disk after a long computation.

Jonathan can comment more on the Workbench side of things but for Jupyter apps, AFAIK, resilience to disconnections is important for long running sessions. Tabs might be refreshed, go asleep, etc. Here is one article that discusses measures to make output resilient to disconnections: https://saturncloud.io/blog/long-running-notebooks, and this post about persistency of kernel sessions upon disconnect: https://discourse.jupyter.org/t/jupyterhub-server-shuts-down-after-period-of-inactivity/11124. This tells me that the Jupyter community expects kernels to persists after disconnects.