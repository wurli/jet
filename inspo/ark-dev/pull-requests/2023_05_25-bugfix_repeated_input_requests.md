# Forward originator of input replies

> <https://github.com/posit-dev/ark/pull/9>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses rstudio/positron#623.

On repeated input requests, the originator is currently `None`, which causes the `StdIn` thread to hang while blocking on the 0MQ response. To fix that, we now retrieve the identity of input replies and pass that as originator for the corresponding `ExecuteCode` request. This way repeated input requests get the previous request as originator and the 0MQ message gets an ID that allows a response to come in and unblock the thread.

@jmcphers I'm now warning if an originator is not found but what is the best way to create a new 0MQ ID from Ark so that we can recover from this?

## @jmcphers at 2023-05-25T16:47:04Z

> @jmcphers I'm now warning if an originator is not found but what is the best way to create a new 0MQ ID from Ark so that we can recover from this?

In this case we should _always_ use the identity of the Shell socket.

https://jupyter-client.readthedocs.io/en/stable/messaging.html#messages-on-the-stdin-router-dealer-channel

> The stdin socket of the client is required to have the same zmq IDENTITY as the client’s shell socket. Because of this, the input_request must be sent with the same IDENTITY routing prefix as the execute_reply in order for the frontend to receive the message.

So for this change, I think we shouldn't pass the Originator into ARK (since socket identities are a protocol layer level detail); instead we should just pass the parent message ID, and then amend ARK's return value with the shell socket identity (in the Amalthea layer). 

## @lionel- at 2023-05-26T10:33:43Z

Thanks for reviewing. I did a quick read of the 0MQ book and the Jupyter protocol and examined the IPython and Jupyter client code to fix ideas. Please let me know if I'm missing anything:

- In principle there might be multiple frontends connected to the kernel.

- StdIn on the kernel side is a router socket, and on the frontend side it's a dealer. This is unusual because normally a dealer makes requests and a router manages the request origins and provides replies, but here it's the opposite.

  This is why Jupyter requires the StdIn and Shell sockets to have the same identity. We first get a message through the Shell router, which provides us the identity that StdIn needs to find the correct frontend socket. As currently implemented, this identity is forwarded with the message.

- Instead of forwarding identities, can we save the socket identity passed to Shell and use that for our StdIn requests? Yes and this is what IPython does:

  - The Shell message handler saves the originator (socket ID and header) for the current frontend request: https://github.com/ipython/ipykernel/blob/f7e5a8fb5c37d81c55b9ea785168c8a19d110eb4/ipykernel/kernelbase.py#L374

  - Input requests retrieve the saved originator and use that destination ID and parent header: https://github.com/ipython/ipykernel/blob/f7e5a8fb5c37d81c55b9ea785168c8a19d110eb4/ipykernel/kernelbase.py#L1191

  This works because a Shell request marks the kernel as busy for the duration of the request. Because of this there can't be any concurrent input request from another frontend, and so the shell identity remains valid for the duration of the outer request.

The implementation proposed in this PR is a little more flexible. By saving the originator as part of the message envelope (which I don't see as an abstraction breach), it allows in principle concurrent input requests to different frontends. Of course this isn't allowed by the protocol and I can see the value in reusing the IPython approach since it is a reference implementation. However in IPython the Shell and StdIn sockets are part of the same object and thus can inspect each other's state. In our case, StdIn and Shell are distinct objects with no knowledge of each other.

We could change that, but I feel like the design in this PR is cleaner: We're passing necessary data along with the message instead of saving it in the objects' states, which requires more assumptions and knowledge of the system to assess the correctness, and this allows the Shell and StdIn objects to remain independent. What do you think?


## @lionel- at 2023-05-26T10:45:13Z

On the other hand, always using the current Shell's ID solves the issue of a possibly absent originator (shouldn't happen but the possibility is looming behind that `Option<Originator>` field). We could store it as a global variable to avoid having to store a reference to Shell inside StdIn.

## @lionel- at 2023-05-26T11:54:06Z

> In our case, StdIn and Shell are distinct objects with no knowledge of each other.

Except for the fact that StdIn totally has a reference to Shell in its `handler` field!

## @lionel- at 2023-05-26T14:18:52Z

Following this fix, there is another ordering bug of activity items in the console that needs a fix: output generated after an input reply appears below the previous input item instead of below the input reply. That's because the current console context, normally managed by `IOPub` on busy events, is not updated after an input reply has been received. This causes further output to be attached to the previous input prompt.

https://github.com/posit-dev/amalthea/assets/4465050/73501f56-c7ca-44e9-ad88-c88c8cb7b41a

To fix this, the latest commit updates the IOPub message context when an input-reply is received. To support that the message context now lives in Kernel and is passed to StdIn and IOPub as a synchronised shared reference.

@jmcphers Please let me know if you'd still prefer to avoid passing originators with input replies and I'll make that change too.

## @jmcphers at 2023-05-26T16:56:05Z

> We're passing necessary data along with the message instead of saving it in the objects' states, which requires more assumptions and knowledge of the system to assess the correctness, and this allows the Shell and StdIn objects to remain independent. What do you think?

This is a good point. From this perspective we could the view `Originator` more as an opaque token than anything else (just hand this back so I can pair up your reply with the request), which doesn't break encapsulation. 

I also like your change to make the message context more accessible. 