# Forward 0MQ messages for StdIn over crossbeam channel

> <https://github.com/posit-dev/ark/pull/58>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards rstudio/positron#535.

We need to communicate an interrupt to the StdIn routine that waits for a reply on its 0MQ socket. Unfortunately there is no way to poll/select over a mix of crossbeam channels and 0MQ sockets. Instead of forwarding the interrupt as a 0MQ message so we could poll multiple sockets, which felt too low level, I though we'd forward 0MQ messages over a channel. This way we will be able to `select!` over two channels for incoming messages and interrupts.

Because 0MQ sockets can't be shared by multiple threads, and to keep things at the same level of abstraction, outgoing messages are also sent over a channel instead of a socket. StdIn no longer owns its socket and it exclusively communicates with channels.

The conversion between 0MQ and crossbeam is supported by two new threads:

- The notifier thread watches channels of outgoing messages for readiness. When a channel is readable, a notification is sent to the forwarding thread via a dedicated 0MQ socket.
- The forwarding thread polls 0MQ sockets for incoming messages and for notifications from the notifier threads. When an incoming message is ready, it's forwarded to the corresponding channel. When an outgoing message is ready, it's forwarded to the corresponding socket.

  This is implemented naively for now as we are only managing a single socket/channel for StdIn, but this can be extended to multiple sockets in the future. Some parts of the code are already implemented with multiple sockets in mind though (when this was easy to do so).

Other supporting features:

- `Socket::new_pair()` to create the `PAIR` sockets used for notification.
- `Socket::socket` is now public to allow lower level control, e.g. for polling.

As I'm reflecting on this PR, it does feel like this mechanism is a bit heavy with regards to the payoff of being able to `select!()` over channels in `StdIn`. Hopefully it will be useful for other cases as well in the future.

## @lionel- at 2023-06-28T16:37:09Z

Now I'm slightly worried about message ordering. Not an issue at the moment since only one socket is managed but extending this mechanism to multiple sockets/channels will require some thought about messaging consistency.

## @lionel- at 2023-06-28T17:11:44Z

Actually, if we manage to make the forwarding/notifier thread consistent regarding message ordering, it could help us solve message passing races like the one I just documented in https://github.com/posit-dev/amalthea/commit/59559e24eb36a2d4c4f3c5b167c5572ce36d9f4f.

The race happens between the StdIn and Shell sockets as they might send 0MQ messages out of order depending on how the threads are scheduled. It could be solved by pulling part of the `listen()` functionality out of the StdIn thread and into a function that would be called on the R thread. This function would block until the input request has been sent to the 0MQ socket, to ensure ordering of the messages.

However since sockets can't be shared across threads, we can't just send the 0MQ message from the R thread. That's where a notifier thread managing all sockets (or in this case both Shell and StdIn) would help since channels can be shared across threads. We could block until we've sent the message by channel and we'd have a guarantee that our message will be delivered before the Shell's message.

This race is not critical at all and very unlikely to happen in practice, but it's a good practical case to think about.


## @lionel- at 2023-06-29T10:40:19Z

Actually it was easy to ensure ordering of outgoing messages sent to the forwarding/notifier threads. We just need every outgoing messages to go through a single channel to inherit its FIFO property. The last commit implements that, with a new `enum OutgoingMessage` with variants like `StdIn(Message)` to associate an outgoing message to a socket.

With this setup we will be able to send messages through the outbound channel when ordering of outgoing messages is important, for instance to solve the race documented above.