# Dispatch interrupts to StdIn and ReadConsole

> <https://github.com/posit-dev/ark/pull/60>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #58 
Addresses rstudio/positron#535.

The fix is in two parts:

- In `ReadConsole()`: Detect interrupts signaled during a user-prompt request, e.g. during `readline()`. In case of interrupt, quit the current Rust context and call `onintr()` to cause a longjump out of `readline()`. The longjump is called from a plain-old-frame function that doesn't have any destructors on the stack.

  The behaviour in that case is unspecified at the moment but it is also safe with the current implementation of rustc, and it should be made formally safe in a future version of rust: https://github.com/rust-lang/rfcs/blob/master/text/2945-c-unwind-abi.md. If that's a concern we could move our method to a `.c` file, but the implementation proposed in this PR seems safe enough.

- Dispatch an interrupt message from Control to StdIn so that the latter doesn't wait for an input reply that is never coming. It was easiest to implement this with a simple channel, but this did add some complications inside StdIn because of the need to pump the channel continuously. This could be simplified in the future if we implement a pub/sub mechanism.

I noticed another race condition between interrupts and exec-request messages, now documented in comments. To fix it, we could manage the Shell and Control sockets on the common message event thread implemented in #58. The Control messages would need to be handled in a blocking way to ensure subscribers are notified before the next incoming message is processed.

